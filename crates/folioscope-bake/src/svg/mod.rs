// SPDX-License-Identifier: AGPL-3.0-or-later

//! Phase 0 spike: renders a `.fig` node to SVG, straight from the decoded tree (there is no IR
//! yet). Shapes come from the geometry Figma stores with each node, text from its glyph outlines,
//! so no font is needed.
//!
//! Not handled yet: instance overrides, radial and angular gradients, blend modes, blurs, image
//! transforms and tiling, auto-layout changes since the file was saved.

use std::collections::BTreeSet;
use std::fmt::Write;

use folioscope_core::figma::instance::Overrides;
use folioscope_core::figma::tree::{Node, NodeId, Tree, node_id};
use folioscope_core::kiwi::Value;

use path::path_commands;

use crate::fonts::weight_of_style;
use crate::text::{Align, Paragraph, Typesetter};

mod paint;
mod path;

pub struct Svg {
    pub width: f32,
    pub height: f32,
    pub markup: String,
    /// Fonts the text needed and was not given (`Family Style`): their text keeps the file's
    /// glyphs, which may be of another style.
    pub missing_fonts: BTreeSet<String>,
}

/// Renders `id` and its descendants, at the origin, in its own coordinate space.
#[must_use]
pub fn render_node(tree: &Tree<'_>, id: NodeId) -> Option<Svg> {
    let node = tree.get(id)?;
    let (width, height) = size(node.value);
    Some(render(tree, node, Root::WHOLE, Vec::new(), (width, height)))
}

/// Like [`render_node`], leaving out the `hidden` descendants: the interactive components, drawn
/// as layers of their own.
#[must_use]
pub fn render_node_without(tree: &Tree<'_>, id: NodeId, hidden: &[NodeId]) -> Option<Svg> {
    let node = tree.get(id)?;
    let (width, height) = size(node.value);
    Some(render(
        tree,
        node,
        Root::WHOLE,
        hidden.to_vec(),
        (width, height),
    ))
}

/// Renders one state of an interactive component: the instance at the end of `path` (the
/// instances from the frame down to it) showing `variant`, through every instance's overrides.
/// The variant it shows in the file is drawn as the instance; another variant is drawn in its
/// place at the variant's own size, with the overrides that still apply (layout is not redone).
#[must_use]
pub fn render_component_state<'t>(
    tree: &'t Tree<'_>,
    path: &[NodeId],
    variant: NodeId,
    typesetter: Option<&'t mut Typesetter>,
) -> Option<Svg> {
    let (last, outer) = path.split_last()?;
    let mut overrides = Overrides::default();
    for id in outer {
        let node = tree.get(*id)?;
        let resolved = overrides.resolve(node);
        overrides.enter(node, &resolved);
    }
    let instance = tree.get(*last)?;
    let resolved = overrides.resolve(instance);
    let shows = resolved
        .get("symbolData")
        .and_then(|s| s.get("symbolID"))
        .and_then(node_id);
    let other;
    let root = if shows == Some(variant) {
        instance
    } else {
        // The instance's overrides follow it to the other variant, matched by layer names.
        if let Some(shows) = shows {
            overrides.alias(tree, shows, variant);
        }
        let target = tree.get(variant)?;
        // A variant laid out like the one shown takes the instance's size and shapes too.
        let same_size = shows
            .and_then(|s| tree.get(s))
            .is_some_and(|s| size(s.value) == size(target.value));
        let mut value = target.value.clone();
        if same_size {
            take_fields(
                &mut value,
                &resolved,
                &["size", "fillGeometry", "strokeGeometry"],
            );
        }
        overrides.enter(instance, &resolved);
        other = value;
        &Node {
            id: target.id,
            kind: target.kind,
            name: target.name,
            value: &other,
        }
    };
    let size = size(&overrides.resolve(root));
    let mut ctx = Ctx {
        tree,
        defs: String::new(),
        body: String::new(),
        next_id: 0,
        hidden: Vec::new(),
        root: Root::WHOLE,
        overrides,
        typesetter,
        missing_fonts: BTreeSet::new(),
    };
    ctx.node(root, true);
    Some(svg(&ctx, size))
}

/// A layer of a scrolling frame (ADR 0004's layer split).
#[derive(Debug, Clone, Copy)]
pub enum Part<'f> {
    /// The frame's own fills and strokes, without its children: stays in place.
    Background,
    /// The children but the `fixed` ones, unclipped, on a transparent canvas of `size`: scrolls.
    Content {
        fixed: &'f [NodeId],
        size: (f32, f32),
    },
    /// Only the `fixed` children, on a transparent canvas of the frame's size: drawn on top.
    Fixed { fixed: &'f [NodeId] },
}

/// Renders one [`Part`] of the frame `id`, leaving out the `hidden` descendants.
#[must_use]
pub fn render_part(tree: &Tree<'_>, id: NodeId, part: Part<'_>, hidden: &[NodeId]) -> Option<Svg> {
    let node = tree.get(id)?;
    let frame_size = size(node.value);
    let mut hidden = hidden.to_vec();
    Some(match part {
        Part::Background => render(tree, node, Root::BACKGROUND, hidden, frame_size),
        Part::Content { fixed, size } => {
            hidden.extend_from_slice(fixed);
            render(tree, node, Root::CONTENT, hidden, size)
        }
        Part::Fixed { fixed } => {
            hidden.extend(
                tree.children(id)
                    .map(|c| c.id)
                    .filter(|c| !fixed.contains(c)),
            );
            render(tree, node, Root::CONTENT, hidden, frame_size)
        }
    })
}

/// What to draw of the root node itself.
#[derive(Debug, Clone, Copy)]
struct Root {
    paint: bool,
    children: bool,
    clip: bool,
}

impl Root {
    const WHOLE: Root = Root {
        paint: true,
        children: true,
        clip: true,
    };
    const BACKGROUND: Root = Root {
        paint: true,
        children: false,
        clip: true,
    };
    const CONTENT: Root = Root {
        paint: false,
        children: true,
        clip: false,
    };
}

fn render(
    tree: &Tree<'_>,
    node: &Node<'_>,
    root: Root,
    hidden: Vec<NodeId>,
    (width, height): (f32, f32),
) -> Svg {
    let mut ctx = Ctx {
        tree,
        defs: String::new(),
        body: String::new(),
        next_id: 0,
        hidden,
        root,
        overrides: Overrides::default(),
        typesetter: None,
        missing_fonts: BTreeSet::new(),
    };
    ctx.node(node, true);
    svg(&ctx, (width, height))
}

/// Copies the `names` fields of `from` onto `to`.
fn take_fields(to: &mut Value, from: &Value, names: &[&str]) {
    let Value::Object(fields) = to else {
        return;
    };
    for &name in names {
        let Some(value) = from.get(name) else {
            continue;
        };
        match fields.iter_mut().find(|(n, _)| &**n == name) {
            Some((_, old)) => *old = value.clone(),
            None => fields.push((name.into(), value.clone())),
        }
    }
}

fn svg(ctx: &Ctx<'_, '_>, (width, height): (f32, f32)) -> Svg {
    let markup = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}"><defs>{}</defs>{}</svg>"#,
        ctx.defs, ctx.body
    );
    Svg {
        width,
        height,
        markup,
        missing_fonts: ctx.missing_fonts.clone(),
    }
}

struct Ctx<'t, 'a> {
    tree: &'t Tree<'a>,
    defs: String,
    body: String,
    next_id: u32,
    hidden: Vec<NodeId>,
    root: Root,
    /// The instances being drawn, whose overrides apply to what is inside them.
    overrides: Overrides,
    /// Lays text out where the file's glyphs do not fit it; none when no font was given.
    typesetter: Option<&'t mut Typesetter>,
    missing_fonts: BTreeSet<String>,
}

impl Ctx<'_, '_> {
    fn id(&mut self, prefix: &str) -> String {
        self.next_id += 1;
        format!("{prefix}{}", self.next_id)
    }

    fn node(&mut self, node: &Node<'_>, root: bool) {
        let resolved = self.overrides.resolve(node);
        let node = &Node {
            id: node.id,
            kind: node.kind,
            name: node.name,
            value: &resolved,
        };
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

        let paint = !root || self.root.paint;
        for effect in v
            .get("effects")
            .map_or(&[][..], Value::as_array)
            .iter()
            .filter(|_| paint)
        {
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
            if paint {
                self.paint_geometry(v, "fillGeometry", "fillPaints", true);
            }
            if !root || self.root.children {
                self.children(node, root);
            }
            if paint {
                self.paint_geometry(v, "strokeGeometry", "strokePaints", false);
            }
        }

        for _ in 0..open {
            self.body.push_str("</g>");
        }
    }

    fn children(&mut self, node: &Node<'_>, root: bool) {
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
        let clip = container
            && (!root || self.root.clip)
            && !matches!(node.value.get("frameMaskDisabled"), Some(Value::Bool(true)));
        if clip {
            let id = self.id("c");
            let shape = self.outline(node.value);
            let _ = write!(self.defs, r#"<clipPath id="{id}">{shape}</clipPath>"#);
            let _ = write!(self.body, r#"<g clip-path="url(#{id})">"#);
        }
        let instance = node.kind == "INSTANCE";
        if instance {
            self.overrides.enter(node, node.value);
        }
        for child in children {
            if let Some(child) = self.tree.get(child) {
                self.node(child, false);
            }
        }
        if instance {
            self.overrides.leave();
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
        let data = v.get("derivedTextData");
        let glyphs = data
            .and_then(|d| d.get("glyphs"))
            .map_or(&[][..], Value::as_array);
        // "Truncate text": Figma stores the ellipsis among the glyphs, and the index of the first
        // glyph it hides.
        let shown = match data.and_then(|d| d.get("truncationStartIndex")) {
            Some(Value::Int(i)) => usize::try_from(*i).unwrap_or(glyphs.len()),
            _ => glyphs.len(),
        };
        // The file's glyphs, unless they were laid out in another font (a variant other than
        // the one stored, with another style) and the right one was given.
        let mut path = if stale_glyphs(v) {
            self.lay_out(v).unwrap_or_default()
        } else {
            String::new()
        };
        if path.is_empty() {
            for glyph in glyphs.iter().take(shown) {
                self.glyph(glyph, &mut path);
            }
        }
        if !path.is_empty() {
            let _ = write!(self.body, r#"<path d="{path}" {fill}/>"#);
        }
    }

    /// The text laid out with the fonts given; records its font as missing when there is none.
    fn lay_out(&mut self, text: &Value) -> Option<String> {
        let style = TextStyle::of(text)?;
        let laid_out = self
            .typesetter
            .as_deref_mut()
            .and_then(|t| t.path(&style.paragraph()));
        if laid_out.is_none() {
            self.missing_fonts
                .insert(format!("{} {}", style.family, style.style));
        }
        laid_out
    }

    /// Appends a glyph's outline to `d`.
    fn glyph(&self, glyph: &Value, d: &mut String) {
        let Some(bytes) = glyph
            .get("commandsBlob")
            .and_then(Value::as_u32)
            .and_then(|blob| self.tree.blob(blob))
        else {
            return;
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
        path_commands(bytes, d, |x, y| (pos("x") + x * size, pos("y") - y * size));
    }

    fn path_data(&self, geometry: &Value) -> Option<String> {
        let bytes = self.tree.blob(geometry.get("commandsBlob")?.as_u32()?)?;
        let mut d = String::new();
        path_commands(bytes, &mut d, |x, y| (x, y));
        (!d.is_empty()).then_some(d)
    }
}

/// Whether a text's stored glyphs were laid out in another font than its own: Figma lists the
/// fonts of the glyphs in `fontMetaData`.
fn stale_glyphs(text: &Value) -> bool {
    let Some(font) = text.get("fontName") else {
        return false;
    };
    let fonts = text
        .get("derivedTextData")
        .and_then(|d| d.get("fontMetaData"))
        .map_or(&[][..], Value::as_array);
    let same = |a: Option<&Value>, b: Option<&Value>, field: &str| {
        a.and_then(|v| v.get(field)).and_then(Value::as_str)
            == b.and_then(|v| v.get(field)).and_then(Value::as_str)
    };
    !fonts.is_empty()
        && !fonts.iter().any(|f| {
            same(f.get("key"), Some(font), "family") && same(f.get("key"), Some(font), "style")
        })
}

/// A `.fig` text's style, as a paragraph to lay out.
struct TextStyle {
    characters: String,
    family: String,
    style: String,
    size: f32,
    line_height: Option<f32>,
    letter_spacing: f32,
    align: Align,
    width: Option<f32>,
}

impl TextStyle {
    fn of(text: &Value) -> Option<Self> {
        let font = text.get("fontName")?;
        let field = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_owned();
        let font_size = text.get("fontSize").and_then(Value::as_f32).unwrap_or(12.0);
        // `{ value, units }`: pixels, a percentage of the font size, or a multiple of it.
        let measure = |name: &str| {
            let m = text.get(name)?;
            let value = m.get("value").and_then(Value::as_f32)?;
            Some(match m.get("units").and_then(Value::as_str) {
                Some("PERCENT") => value / 100.0 * font_size,
                Some("RAW") => value * font_size,
                _ => value,
            })
        };
        Some(Self {
            characters: text
                .get("textData")
                .and_then(|t| t.get("characters"))
                .and_then(Value::as_str)?
                .to_owned(),
            family: field(font, "family"),
            style: field(font, "style"),
            size: font_size,
            line_height: measure("lineHeight"),
            letter_spacing: measure("letterSpacing").unwrap_or(0.0),
            align: match text.get("textAlignHorizontal").and_then(Value::as_str) {
                Some("CENTER") => Align::Center,
                Some("RIGHT") => Align::Right,
                Some("JUSTIFIED") => Align::Justify,
                _ => Align::Left,
            },
            // A text that grows with its content does not wrap; others wrap at their width.
            width: match text.get("textAutoResize").and_then(Value::as_str) {
                Some("WIDTH_AND_HEIGHT") => None,
                _ => Some(size(text).0),
            },
        })
    }

    fn paragraph(&self) -> Paragraph<'_> {
        Paragraph {
            text: &self.characters,
            family: &self.family,
            weight: weight_of_style(&self.style),
            italic: self.style.to_ascii_lowercase().contains("italic"),
            size: self.size,
            line_height: self.line_height,
            letter_spacing: self.letter_spacing,
            align: self.align,
            width: self.width,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn object(fields: Vec<(&str, Value)>) -> Value {
        Value::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    fn text(family: &str, style: &str, glyphs_in: (&str, &str)) -> Value {
        let font = |family: &str, style: &str| {
            object(vec![
                ("family", Value::String(family.into())),
                ("style", Value::String(style.into())),
            ])
        };
        object(vec![
            ("fontName", font(family, style)),
            ("fontSize", Value::Float(16.0)),
            (
                "lineHeight",
                object(vec![
                    ("value", Value::Float(150.0)),
                    ("units", Value::Enum("PERCENT".into())),
                ]),
            ),
            ("textAlignHorizontal", Value::Enum("CENTER".into())),
            ("textAutoResize", Value::Enum("HEIGHT".into())),
            (
                "size",
                object(vec![("x", Value::Float(120.0)), ("y", Value::Float(24.0))]),
            ),
            (
                "textData",
                object(vec![("characters", Value::String("Mains".into()))]),
            ),
            (
                "derivedTextData",
                object(vec![(
                    "fontMetaData",
                    Value::Array(vec![object(vec![("key", font(glyphs_in.0, glyphs_in.1))])]),
                )]),
            ),
        ])
    }

    #[test]
    fn glyphs_of_another_font_are_stale() {
        assert!(!stale_glyphs(&text(
            "Karla",
            "ExtraBold",
            ("Karla", "ExtraBold")
        )));
        assert!(stale_glyphs(&text(
            "Karla",
            "Regular",
            ("Karla", "ExtraBold")
        )));
    }

    #[test]
    fn reads_a_text_style() {
        let style = TextStyle::of(&text("Karla", "Semi Bold Italic", ("Karla", "Bold"))).unwrap();
        let paragraph = style.paragraph();
        assert_eq!(paragraph.text, "Mains");
        assert_eq!(paragraph.weight, 600.0);
        assert!(paragraph.italic);
        assert_eq!(paragraph.line_height, Some(24.0));
        assert_eq!(paragraph.align, Align::Center);
        assert_eq!(paragraph.width, Some(120.0));
    }
}
