// SPDX-License-Identifier: AGPL-3.0-or-later

//! Phase 0 spike: renders a `.fig` node to SVG, straight from the decoded tree (there is no IR
//! yet). Shapes come from the geometry Figma stores with each node, text from its glyph outlines,
//! so no font is needed.
//!
//! Not handled yet: instance overrides, radial and angular gradients, blend modes, blurs, image
//! transforms and tiling, auto-layout changes since the file was saved.

use std::fmt::Write;

use folioscope_core::fig::tree::{Node, NodeId, Tree, node_id};
use folioscope_core::kiwi::Value;

pub struct Svg {
    pub width: f32,
    pub height: f32,
    pub markup: String,
}

/// Renders `id` and its descendants, at the origin, in its own coordinate space.
#[must_use]
pub fn render_node(tree: &Tree<'_>, id: NodeId) -> Option<Svg> {
    let node = tree.get(id)?;
    let (width, height) = size(node.value);
    let mut ctx = Ctx {
        tree,
        defs: String::new(),
        body: String::new(),
        next_id: 0,
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
}

impl Ctx<'_, '_> {
    fn id(&mut self, prefix: &str) -> String {
        self.next_id += 1;
        format!("{prefix}{}", self.next_id)
    }

    fn node(&mut self, node: &Node<'_>, root: bool) {
        let v = node.value;
        if matches!(v.get("visible"), Some(Value::Bool(false))) {
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

    /// Fill attributes for a paint, adding gradients and patterns to `defs`.
    fn paint(&mut self, paint: &Value, w: f32, h: f32) -> Option<String> {
        let opacity = paint.get("opacity").and_then(Value::as_f32).unwrap_or(1.0);
        match paint.get("type").and_then(Value::as_str)? {
            "SOLID" => {
                let (rgb, alpha) = color(paint.get("color")?);
                Some(format!(
                    r#"fill="{rgb}" fill-opacity="{}""#,
                    alpha * opacity
                ))
            }
            "GRADIENT_LINEAR" => {
                let ((x1, y1), (x2, y2)) = gradient_line(paint.get("transform"), w, h)?;
                let id = self.id("g");
                let _ = write!(
                    self.defs,
                    r#"<linearGradient id="{id}" gradientUnits="userSpaceOnUse" x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}">"#
                );
                for stop in paint.get("stops").map_or(&[][..], Value::as_array) {
                    let position = stop.get("position").and_then(Value::as_f32).unwrap_or(0.0);
                    let (rgb, alpha) = stop.get("color").map_or(("#000".to_owned(), 1.0), color);
                    let _ = write!(
                        self.defs,
                        r#"<stop offset="{position}" stop-color="{rgb}" stop-opacity="{alpha}"/>"#
                    );
                }
                self.defs.push_str("</linearGradient>");
                Some(format!(r#"fill="url(#{id})" fill-opacity="{opacity}""#))
            }
            "IMAGE" => {
                let Some(Value::Bytes(hash)) = paint.get("image").and_then(|i| i.get("hash"))
                else {
                    return None;
                };
                let bytes = self.tree.image(hash)?;
                let aspect = match paint.get("imageScaleMode").and_then(Value::as_str) {
                    Some("FIT") => "xMidYMid meet",
                    Some("STRETCH") => "none",
                    _ => "xMidYMid slice",
                };
                let id = self.id("p");
                let _ = write!(
                    self.defs,
                    r#"<pattern id="{id}" patternUnits="userSpaceOnUse" width="{w}" height="{h}"><image href="data:{};base64,{}" width="{w}" height="{h}" preserveAspectRatio="{aspect}"/></pattern>"#,
                    mime(bytes),
                    helpers4::hex::base64_encode(bytes)
                );
                Some(format!(r#"fill="url(#{id})" fill-opacity="{opacity}""#))
            }
            // Radial, angular and diamond gradients: their first stop, for now.
            _ => {
                let stop = paint.get("stops")?.as_array().first()?;
                let (rgb, alpha) = color(stop.get("color")?);
                Some(format!(
                    r#"fill="{rgb}" fill-opacity="{}""#,
                    alpha * opacity
                ))
            }
        }
    }

    fn drop_shadow(&mut self, effect: &Value) -> String {
        let id = self.id("f");
        let offset = |axis: &str| {
            effect
                .get("offset")
                .and_then(|o| o.get(axis))
                .and_then(Value::as_f32)
                .unwrap_or(0.0)
        };
        let radius = effect.get("radius").and_then(Value::as_f32).unwrap_or(0.0);
        let (rgb, alpha) = effect.get("color").map_or(("#000".to_owned(), 0.25), color);
        // Figma's blur radius is about twice the Gaussian standard deviation.
        let _ = write!(
            self.defs,
            r#"<filter id="{id}" x="-50%" y="-50%" width="200%" height="200%"><feDropShadow dx="{}" dy="{}" stdDeviation="{}" flood-color="{rgb}" flood-opacity="{alpha}"/></filter>"#,
            offset("x"),
            offset("y"),
            radius / 2.0
        );
        id
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

/// Appends the SVG path data of a geometry blob: a command byte (0 close, 1 move, 2 line,
/// 3 quadratic, 4 cubic) followed by little-endian `f32` points. Stops at an unknown command.
/// Glyph outlines use quadratic curves, as TrueType fonts do.
fn path_commands(bytes: &[u8], d: &mut String, map: impl Fn(f32, f32) -> (f32, f32)) {
    let mut i = 0;
    let point = |i: &mut usize| -> Option<(f32, f32)> {
        let x = f32::from_le_bytes(bytes.get(*i..*i + 4)?.try_into().ok()?);
        let y = f32::from_le_bytes(bytes.get(*i + 4..*i + 8)?.try_into().ok()?);
        *i += 8;
        Some(map(x, y))
    };
    // Glyph outlines start with a close command; SVG rejects a path that does.
    let mut open = false;
    while let Some(&cmd) = bytes.get(i) {
        i += 1;
        let ok = match cmd {
            0 => {
                if open {
                    d.push('Z');
                    open = false;
                }
                Some(())
            }
            1 | 2 => point(&mut i).map(|(x, y)| {
                let _ = write!(d, "{}{x} {y}", if cmd == 1 { 'M' } else { 'L' });
                open = true;
            }),
            3 => (|| {
                let (x1, y1) = point(&mut i)?;
                let (x, y) = point(&mut i)?;
                let _ = write!(d, "Q{x1} {y1} {x} {y}");
                Some(())
            })(),
            4 => (|| {
                let (x1, y1) = point(&mut i)?;
                let (x2, y2) = point(&mut i)?;
                let (x, y) = point(&mut i)?;
                let _ = write!(d, "C{x1} {y1} {x2} {y2} {x} {y}");
                Some(())
            })(),
            _ => None,
        };
        if ok.is_none() {
            return;
        }
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

/// `(css rgb, alpha)` from a Figma color with 0–1 channels.
fn color(c: &Value) -> (String, f32) {
    let ch = |k: &str| c.get(k).and_then(Value::as_f32).unwrap_or(0.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let byte = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    (
        format!("rgb({},{},{})", byte(ch("r")), byte(ch("g")), byte(ch("b"))),
        ch("a"),
    )
}

/// Start and end of a linear gradient, in node pixels. The paint transform maps node space
/// (normalized to 0–1) to gradient space, where the gradient runs from (0, 0.5) to (1, 0.5).
// TODO(helpers4): affine inversion belongs to the helpers4 `geometry` module.
#[allow(clippy::many_single_char_names)] // matrix notation
fn gradient_line(transform: Option<&Value>, w: f32, h: f32) -> Option<((f32, f32), (f32, f32))> {
    let m = |k: &str| transform.and_then(|t| t.get(k)).and_then(Value::as_f32);
    let (a, c, e) = (
        m("m00").unwrap_or(1.0),
        m("m01").unwrap_or(0.0),
        m("m02").unwrap_or(0.0),
    );
    let (b, d, f) = (
        m("m10").unwrap_or(0.0),
        m("m11").unwrap_or(1.0),
        m("m12").unwrap_or(0.0),
    );
    let det = a * d - b * c;
    if det.abs() < 1e-12 {
        return None;
    }
    let (ia, ic, ie) = (d / det, -c / det, (c * f - d * e) / det);
    let (ib, id, iff) = (-b / det, a / det, (b * e - a * f) / det);
    let point = |gx: f32, gy: f32| ((ia * gx + ic * gy + ie) * w, (ib * gx + id * gy + iff) * h);
    Some((point(0.0, 0.5), point(1.0, 0.5)))
}

fn mime(bytes: &[u8]) -> &'static str {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => "image/png",
        [0xff, 0xd8, ..] => "image/jpeg",
        [b'G', b'I', b'F', ..] => "image/gif",
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'E',
            b'B',
            b'P',
            ..,
        ] => "image/webp",
        _ => "application/octet-stream",
    }
}
