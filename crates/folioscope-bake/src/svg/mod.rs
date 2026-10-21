// SPDX-License-Identifier: AGPL-3.0-or-later

//! Phase 0 spike: renders a `.fig` node to SVG, straight from the decoded tree (there is no IR
//! yet). Shapes come from the geometry Figma stores with each node, text from its glyph outlines,
//! so no font is needed.
//!
//! Not handled yet: instance overrides, radial and angular gradients, blend modes, blurs, image
//! transforms and tiling, auto-layout changes since the file was saved.

use std::fmt::Write;

use folioscope_core::figma::tree::{Node, NodeId, Tree, node_id};
use folioscope_core::kiwi::Value;

use path::path_commands;

mod paint;
mod path;

pub struct Svg {
    pub width: f32,
    pub height: f32,
    pub markup: String,
}

/// Renders `id` and its descendants, at the origin, in its own coordinate space.
#[must_use]
pub fn render_node(tree: &Tree<'_>, id: NodeId) -> Option<Svg> {
    render_node_without(tree, id, &[])
}

/// Like [`render_node`], leaving out the `hidden` descendants: what the bake does to cut a frame
/// into a background and the layers drawn above it.
#[must_use]
pub fn render_node_without(tree: &Tree<'_>, id: NodeId, hidden: &[NodeId]) -> Option<Svg> {
    let node = tree.get(id)?;
    let (width, height) = size(node.value);
    let mut ctx = Ctx {
        tree,
        defs: String::new(),
        body: String::new(),
        next_id: 0,
        hidden,
    };
    ctx.node(node, true);
    let markup = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}"><defs>{}</defs>{}</svg>"#,
        ctx.defs, ctx.body
    );
    Some(Svg {
        width,
        height,
        markup,
    })
}

struct Ctx<'t, 'a> {
    tree: &'t Tree<'a>,
    defs: String,
    body: String,
    next_id: u32,
    hidden: &'t [NodeId],
}

impl Ctx<'_, '_> {
    fn id(&mut self, prefix: &str) -> String {
        self.next_id += 1;
        format!("{prefix}{}", self.next_id)
    }

    fn node(&mut self, node: &Node<'_>, root: bool) {
        let v = node.value;
        if matches!(v.get("visible"), Some(Value::Bool(false))) || self.hidden.contains(&node.id) {
            return;
        }

        let mut open = 0;
        let mut attrs = String::new();
        if !root && let Some(m) = v.get("transform") {
            let f = |k: &str| m.get(k).and_then(Value::as_f32).unwrap_or(0.0);
            // SVG's matrix(a b c d e f) is column-major: Figma's m00 m10 m01 m11 m02 m12.
            let _ = write!(
                attrs,
                r#" transform="matrix({} {} {} {} {} {})""#,
                f("m00"),
                f("m10"),
                f("m01"),
                f("m11"),
                f("m02"),
                f("m12")
            );
        }
        if let Some(opacity) = v.get("opacity").and_then(Value::as_f32)
            && opacity < 1.0
        {
            let _ = write!(attrs, r#" opacity="{opacity}""#);
        }
        let _ = write!(self.body, "<g{attrs}>");
        open += 1;

        for effect in v.get("effects").map_or(&[][..], Value::as_array) {
            if effect.get("type").and_then(Value::as_str) == Some("DROP_SHADOW") && visible(effect)
            {
                let filter = self.drop_shadow(effect);
                let _ = write!(self.body, r#"<g filter="url(#{filter})">"#);
                open += 1;
            }
        }

        if node.kind == "TEXT" {
            self.text(node);
        } else {
            self.paint_geometry(v, "fillGeometry", "fillPaints", true);
            self.children(node);
            self.paint_geometry(v, "strokeGeometry", "strokePaints", false);
        }

        for _ in 0..open {
            self.body.push_str("</g>");
        }
    }

    fn children(&mut self, node: &Node<'_>) {
        let container = matches!(node.kind, "FRAME" | "SYMBOL" | "INSTANCE" | "COMPONENT_SET");
        // Instances do not store their children: draw the component's, without overrides.
        let source = if node.kind == "INSTANCE" {
            node.value
                .get("symbolData")
                .and_then(|s| s.get("symbolID"))
                .and_then(node_id)
                .unwrap_or(node.id)
        } else {
            node.id
        };
        let children: Vec<NodeId> = self.tree.children(source).map(|c| c.id).collect();
        if children.is_empty() {
            return;
        }
        let clip =
            container && !matches!(node.value.get("frameMaskDisabled"), Some(Value::Bool(true)));
        if clip {
            let id = self.id("c");
            let shape = self.outline(node.value);
            let _ = write!(self.defs, r#"<clipPath id="{id}">{shape}</clipPath>"#);
            let _ = write!(self.body, r#"<g clip-path="url(#{id})">"#);
        }
        for child in children {
            if let Some(child) = self.tree.get(child) {
                self.node(child, false);
            }
        }
        if clip {
            self.body.push_str("</g>");
        }
    }

    /// The node's own shape, for clipping: its first fill geometry, or its rounded bounds.
    fn outline(&self, v: &Value) -> String {
        if let Some(path) = v
            .get("fillGeometry")
            .map_or(&[][..], Value::as_array)
            .first()
            .and_then(|g| self.path_data(g))
        {
            return format!(r#"<path d="{path}"/>"#);
        }
        let (w, h) = size(v);
        let r = v.get("cornerRadius").and_then(Value::as_f32).unwrap_or(0.0);
        format!(r#"<rect width="{w}" height="{h}" rx="{r}"/>"#)
    }

    fn paint_geometry(
        &mut self,
        v: &Value,
        geometry: &str,
        paints: &str,
        fallback_to_bounds: bool,
    ) {
        let paints: Vec<&Value> = v
            .get(paints)
            .map_or(&[][..], Value::as_array)
            .iter()
            .filter(|p| visible(p))
            .collect();
        if paints.is_empty() {
            return;
        }
        let mut shapes: Vec<String> = v
            .get(geometry)
            .map_or(&[][..], Value::as_array)
            .iter()
            .filter_map(|g| {
                let d = self.path_data(g)?;
                let rule = if g.get("windingRule").and_then(Value::as_str) == Some("EVENODD") {
                    "evenodd"
                } else {
                    "nonzero"
                };
                Some(format!(r#"<path d="{d}" fill-rule="{rule}""#))
            })
            .collect();
        // Files written by other tools may omit the cached geometry of simple shapes.
        if shapes.is_empty() && fallback_to_bounds {
            let (w, h) = size(v);
            let r = v.get("cornerRadius").and_then(Value::as_f32).unwrap_or(0.0);
            shapes.push(format!(r#"<rect width="{w}" height="{h}" rx="{r}""#));
        }
        let (w, h) = size(v);
        for paint in paints {
            let Some(fill) = self.paint(paint, w, h) else {
                continue;
            };
            for shape in &shapes {
                let _ = write!(self.body, "{shape} {fill}/>");
            }
        }
    }

    /// Text from the glyph outlines Figma stored after laying it out.
    fn text(&mut self, node: &Node<'_>) {
        let v = node.value;
        let Some(fill) = v
            .get("fillPaints")
            .map_or(&[][..], Value::as_array)
            .iter()
            .find(|p| visible(p))
        else {
            return;
        };
        let (w, h) = size(v);
        let Some(fill) = self.paint(fill, w, h) else {
            return;
        };
        let glyphs = v
            .get("derivedTextData")
            .and_then(|d| d.get("glyphs"))
            .map_or(&[][..], Value::as_array);
        let mut d = String::new();
        for glyph in glyphs {
            let Some(blob) = glyph.get("commandsBlob").and_then(Value::as_u32) else {
                continue;
            };
            let Some(bytes) = self.tree.blob(blob) else {
                continue;
            };
            let size = glyph.get("fontSize").and_then(Value::as_f32).unwrap_or(0.0);
            let pos = |axis: &str| {
                glyph
                    .get("position")
                    .and_then(|p| p.get(axis))
                    .and_then(Value::as_f32)
                    .unwrap_or(0.0)
            };
            // Outlines are in em units with y up; place them on the baseline at `position`.
            path_commands(bytes, &mut d, |x, y| {
                (pos("x") + x * size, pos("y") - y * size)
            });
        }
        if !d.is_empty() {
            let _ = write!(self.body, r#"<path d="{d}" {fill}/>"#);
        }
    }

    fn path_data(&self, geometry: &Value) -> Option<String> {
        let bytes = self.tree.blob(geometry.get("commandsBlob")?.as_u32()?)?;
        let mut d = String::new();
        path_commands(bytes, &mut d, |x, y| (x, y));
        (!d.is_empty()).then_some(d)
    }
}

fn size(v: &Value) -> (f32, f32) {
    let axis = |a: &str| {
        v.get("size")
            .and_then(|s| s.get(a))
            .and_then(Value::as_f32)
            .unwrap_or(0.0)
    };
    (axis("x"), axis("y"))
}

fn visible(paint: &Value) -> bool {
    !matches!(paint.get("visible"), Some(Value::Bool(false)))
}
