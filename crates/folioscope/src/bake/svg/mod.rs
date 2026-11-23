// SPDX-License-Identifier: AGPL-3.0-or-later

//! Phase 0 spike: renders a `.fig` node to SVG, straight from the decoded tree (there is no IR
//! yet). Shapes come from the geometry Figma stores with each node, text from its glyph outlines,
//! so no font is needed.
//!
//! Where the file stores no layout, for a variant other than the one an instance shows, the bake
//! lays out again what the instance's overrides change: texts that grow with their content, and
//! the auto-layout frames that hug them ([`Ctx::fit`]).

use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Write;

use crate::figma::instance::{Overrides, instance_symbol};
use crate::figma::tree::{Node, NodeId, Tree, node_id};
use crate::kiwi::Value;

use path::path_commands;

use crate::bake::fonts::weight_of_style;
use crate::bake::fx;
use crate::bake::text::{Align, Paragraph, Typesetter};
use helpers4::geometry::Matrix;

mod paint;
pub(crate) use paint::{image_size, mime};
mod path;

pub struct Svg {
    pub width: f32,
    pub height: f32,
    pub markup: String,
    /// Fonts the text needed and was not given (`Family Style`): their text keeps the file's
    /// glyphs, which may be of another style.
    pub missing_fonts: BTreeSet<String>,
}

/// Renders `id` and its descendants, at the origin, in its own coordinate space. The
/// `typesetter` lays out the text whose glyphs the file did not store.
#[must_use]
pub fn render_node<'t>(
    tree: &'t Tree<'_>,
    id: NodeId,
    typesetter: Option<&'t mut Typesetter>,
) -> Option<Svg> {
    let node = tree.get(id)?;
    let (width, height) = size(node.value);
    Some(render(
        tree,
        node,
        Root::WHOLE,
        Vec::new(),
        (width, height),
        typesetter,
    ))
}

/// Like [`render_node`], leaving out the `hidden` descendants: the interactive components, drawn
/// as layers of their own.
#[must_use]
pub fn render_node_without<'t>(
    tree: &'t Tree<'_>,
    id: NodeId,
    hidden: &[NodeId],
    typesetter: Option<&'t mut Typesetter>,
) -> Option<Svg> {
    let node = tree.get(id)?;
    let (width, height) = size(node.value);
    Some(render(
        tree,
        node,
        Root::WHOLE,
        hidden.to_vec(),
        (width, height),
        typesetter,
    ))
}

/// Renders one state of an interactive component: the instance at the end of `path` (the
/// instances from the frame down to it) showing `variant`, through every instance's overrides.
/// The variant it shows in the file is drawn as the instance; another variant is drawn in its
/// place at the variant's own size, with the overrides that still apply, its texts and the
/// auto-layout frames that hug them laid out again ([`Ctx::fit`]).
#[must_use]
pub fn render_component_state<'t>(
    tree: &'t Tree<'_>,
    path: &[NodeId],
    variant: NodeId,
    typesetter: Option<&'t mut Typesetter>,
    hidden: &[NodeId],
) -> Option<Svg> {
    let (mut ctx, root) = state_context(tree, path, variant, typesetter, hidden)?;
    let node = root.node();
    let size = size(&ctx.resolve(&node));
    ctx.node(&node, true);
    Some(svg(&ctx, size))
}

/// A part of a component's state, for smart animate between states: see [`state_parts`].
pub struct StatePart {
    /// Its layer's name, under its parents' (`Row/Label`); the background is `""`.
    pub name: String,
    pub svg: Svg,
    /// The room around its box in its image, for what overflows it: its box starts at `pad`.
    pub pad: f32,
    /// From its box's top-left corner to the state's.
    pub matrix: Matrix,
    /// Its fields, through the instance's overrides and the layout done again.
    pub value: Value,
    /// Whether its children are parts of their own (it is drawn without them).
    pub split: bool,
}

/// The parts of a component's state, drawn as [`render_component_state`] draws the whole: the
/// root's own fills, then each layer, a frame's children three levels down as parts of their
/// own, through the instance's overrides and with the layout done again. Also the fonts the text
/// needed and was not given.
#[must_use]
pub fn state_parts<'t>(
    tree: &'t Tree<'_>,
    path: &[NodeId],
    variant: NodeId,
    typesetter: Option<&'t mut Typesetter>,
    hidden: &[NodeId],
) -> Option<(Vec<StatePart>, BTreeSet<String>)> {
    let (mut ctx, root) = state_context(tree, path, variant, typesetter, hidden)?;
    let node = root.node();
    let resolved = ctx.resolve(&node).into_owned();
    ctx.root = Root::BACKGROUND;
    let background = ctx.alone(&node, hidden.to_vec(), size(&resolved), 0.0);
    ctx.root = Root::WHOLE;
    let mut parts = vec![StatePart {
        name: String::new(),
        svg: background,
        pad: 0.0,
        matrix: Matrix::IDENTITY,
        value: resolved.clone(),
        split: true,
    }];
    // An instance's children are its component's, inside its overrides.
    let source = if node.kind == "INSTANCE" {
        ctx.overrides.enter(&node, &resolved);
        instance_symbol(&resolved)?
    } else {
        node.id
    };
    ctx.parts(
        source,
        &resolved,
        "",
        Matrix::IDENTITY,
        1,
        hidden,
        &mut parts,
    );
    Some((parts, std::mem::take(&mut ctx.missing_fonts)))
}

/// How deep the parts of a state go, and how many it has at most.
const PART_DEPTH: usize = 3;
const MAX_PARTS: usize = 80;

/// The node a component's state is drawn from: the instance, or another variant in its place.
struct StateRoot<'a> {
    id: NodeId,
    kind: &'a str,
    name: &'a str,
    value: Value,
}

impl StateRoot<'_> {
    fn node(&self) -> Node<'_> {
        Node {
            id: self.id,
            kind: self.kind,
            name: self.name,
            value: &self.value,
        }
    }
}

/// The context that draws a component's state, inside the instances of `path`, and its root.
fn state_context<'t, 'a>(
    tree: &'t Tree<'a>,
    path: &[NodeId],
    variant: NodeId,
    typesetter: Option<&'t mut Typesetter>,
    hidden: &[NodeId],
) -> Option<(Ctx<'t, 'a>, StateRoot<'a>)> {
    let (last, outer) = path.split_last()?;
    let mut overrides = Overrides::default();
    for id in outer {
        let node = tree.get(*id)?;
        let resolved = overrides.resolve(node);
        overrides.enter(node, &resolved);
    }
    let instance = tree.get(*last)?;
    let resolved = overrides.resolve(instance);
    let shows = instance_symbol(&resolved);
    let root = if shows == Some(variant) {
        StateRoot {
            id: instance.id,
            kind: instance.kind,
            name: instance.name,
            value: instance.value.clone(),
        }
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
        StateRoot {
            id: target.id,
            kind: target.kind,
            name: target.name,
            value,
        }
    };
    let mut ctx = Ctx {
        tree,
        defs: String::new(),
        body: String::new(),
        next_id: 0,
        hidden: hidden.to_vec(),
        root: Root::WHOLE,
        overrides,
        typesetter,
        missing_fonts: BTreeSet::new(),
        shifts: HashMap::new(),
        resized: HashMap::new(),
        relaid: HashSet::new(),
        backdrop: Backdrop::default(),
    };
    if shows == Some(variant) {
        // The variant shown: Figma stored its layout, but a file written by another tool may
        // not have; only texts without stored glyphs are laid out again.
        let node = root.node();
        let resolved = ctx.resolve(&node).into_owned();
        if let Some(symbol) = shows {
            ctx.overrides.enter(&node, &resolved);
            let fitted = ctx.fit_stack(symbol, &resolved, true);
            ctx.overrides.leave();
            let own = size(&resolved);
            if (fitted.0 - own.0).abs() > 0.5 || (fitted.1 - own.1).abs() > 0.5 {
                ctx.resized.insert(node.id, fitted);
            }
        }
    } else {
        ctx.fit(&root.node(), false);
    }
    Some((ctx, root))
}

/// A layer of a scrolling frame (the layer split of docs/spec/folio.md).
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

/// How far a node draws outside its box: its stroke, unless inside, and its shadows and blurs.
/// A part of a frame drawn on its own is given that much room, or it would be cut off.
#[must_use]
pub fn overflow(tree: &Tree<'_>, v: &Value) -> f32 {
    // Paints through the shared style a node uses, if any.
    let list = |k: &str| paints_of(tree, v, k).iter().filter(|p| visible(p));
    let number = |e: &Value, k: &str| e.get(k).and_then(Value::as_f32).unwrap_or(0.0);
    let mut pad: f32 = 0.0;
    if list("strokePaints").next().is_some() {
        let weight = v.get("strokeWeight").and_then(Value::as_f32).unwrap_or(1.0);
        pad = match v.get("strokeAlign").and_then(Value::as_str) {
            Some("INSIDE") => 0.0,
            Some("OUTSIDE") => weight,
            _ => weight / 2.0,
        };
    }
    // The outlines Figma stored win over the settings: they are what is drawn, and they may
    // reach further (a stroke said inside, stored centred).
    let (width, height) = size(v);
    let reach = std::cell::Cell::new(pad);
    for key in ["fillGeometry", "strokeGeometry"] {
        for geometry in v.get(key).map_or(&[][..], Value::as_array) {
            if let Some(bytes) = geometry
                .get("commandsBlob")
                .and_then(Value::as_u32)
                .and_then(|blob| tree.blob(blob))
            {
                path_commands(bytes, &mut String::new(), |x, y| {
                    reach.set(reach.get().max(-x).max(-y).max(x - width).max(y - height));
                    (x, y)
                });
            }
        }
    }
    // So may the glyphs of a text, around its box (descenders, a line taller than the box).
    for glyph in v
        .get("derivedTextData")
        .and_then(|d| d.get("glyphs"))
        .map_or(&[][..], Value::as_array)
    {
        let Some(bytes) = glyph
            .get("commandsBlob")
            .and_then(Value::as_u32)
            .and_then(|blob| tree.blob(blob))
        else {
            continue;
        };
        let size = number(glyph, "fontSize");
        let at = |axis: &str| glyph.get("position").map_or(0.0, |p| number(p, axis));
        let (gx, gy) = (at("x"), at("y"));
        path_commands(bytes, &mut String::new(), |x, y| {
            let (x, y) = (gx + x * size, gy - y * size);
            reach.set(reach.get().max(-x).max(-y).max(x - width).max(y - height));
            (x, y)
        });
    }
    pad = reach.get();
    for effect in list("effects") {
        // A Gaussian blur fades out at about one and a half times its radius.
        let blur = number(effect, "radius") * 1.5;
        let reach = match effect.get("type").and_then(Value::as_str) {
            Some("DROP_SHADOW") => {
                let offset = effect.get("offset");
                let shift = offset.map_or(0.0, |o| number(o, "x").abs().max(number(o, "y").abs()));
                blur + number(effect, "spread").max(0.0) + shift
            }
            Some("FOREGROUND_BLUR" | "LAYER_BLUR") => blur,
            _ => 0.0,
        };
        pad = pad.max(reach);
    }
    pad.ceil()
}

/// Renders `id` on its own, as a part for smart animate: like [`render_node_without`], with
/// room around it for what overflows its box ([`overflow`]). Also the room given.
#[must_use]
pub fn render_layer<'t>(
    tree: &'t Tree<'_>,
    id: NodeId,
    hidden: &[NodeId],
    typesetter: Option<&'t mut Typesetter>,
) -> Option<(Svg, f32)> {
    let node = tree.get(id)?;
    let pad = overflow(tree, node.value);
    let svg = render_padded(
        tree,
        node,
        Root::WHOLE,
        hidden.to_vec(),
        size(node.value),
        typesetter,
        pad,
    );
    Some((svg, pad))
}

/// Renders one [`Part`] of the frame `id`, leaving out the `hidden` descendants.
#[must_use]
pub fn render_part<'t>(
    tree: &'t Tree<'_>,
    id: NodeId,
    part: Part<'_>,
    hidden: &[NodeId],
    typesetter: Option<&'t mut Typesetter>,
) -> Option<Svg> {
    let node = tree.get(id)?;
    let frame_size = size(node.value);
    let mut hidden = hidden.to_vec();
    Some(match part {
        Part::Background => render(tree, node, Root::BACKGROUND, hidden, frame_size, typesetter),
        Part::Content { fixed, size } => {
            hidden.extend_from_slice(fixed);
            render(tree, node, Root::CONTENT, hidden, size, typesetter)
        }
        Part::Fixed { fixed } => {
            hidden.extend(
                tree.children(id)
                    .map(|c| c.id)
                    .filter(|c| !fixed.contains(c)),
            );
            render(tree, node, Root::CONTENT, hidden, frame_size, typesetter)
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

fn render<'t>(
    tree: &'t Tree<'_>,
    node: &Node<'_>,
    root: Root,
    hidden: Vec<NodeId>,
    size: (f32, f32),
    typesetter: Option<&'t mut Typesetter>,
) -> Svg {
    render_padded(tree, node, root, hidden, size, typesetter, 0.0)
}

fn render_padded<'t>(
    tree: &'t Tree<'_>,
    node: &Node<'_>,
    root: Root,
    hidden: Vec<NodeId>,
    size: (f32, f32),
    typesetter: Option<&'t mut Typesetter>,
    pad: f32,
) -> Svg {
    let mut ctx = Ctx {
        tree,
        defs: String::new(),
        body: String::new(),
        next_id: 0,
        hidden,
        root,
        overrides: Overrides::default(),
        typesetter,
        missing_fonts: BTreeSet::new(),
        shifts: HashMap::new(),
        resized: HashMap::new(),
        relaid: HashSet::new(),
        backdrop: Backdrop {
            root: Some(node.id),
            ..Backdrop::default()
        },
    };
    ctx.node(node, true);
    svg_padded(&ctx, size, pad)
}

/// A rectangle with rounded corners, as SVG path data.
fn rounded_rect(left: f32, top: f32, width: f32, height: f32, radius: f32) -> String {
    let (width, height) = (width.max(0.0), height.max(0.0));
    let r = radius.min(width / 2.0).min(height / 2.0).max(0.0);
    let (right, bottom) = (left + width, top + height);
    format!(
        "M{} {top}H{}A{r} {r} 0 0 1 {right} {}V{}A{r} {r} 0 0 1 {} {bottom}H{}A{r} {r} 0 0 1 {left} {}V{}A{r} {r} 0 0 1 {} {top}Z",
        left + r,
        right - r,
        top + r,
        bottom - r,
        right - r,
        left + r,
        bottom - r,
        top + r,
        left + r,
    )
}

/// Whether an auto-layout frame's size along an axis follows its content ("hug").
fn hugs(frame: &Value, field: &str) -> bool {
    frame
        .get(field)
        .and_then(Value::as_str)
        .is_some_and(|sizing| sizing.starts_with("RESIZE_TO_FIT"))
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

fn svg(ctx: &Ctx<'_, '_>, size: (f32, f32)) -> Svg {
    svg_padded(ctx, size, 0.0)
}

/// The SVG document of what `ctx` drew, a node of `width` × `height` with `pad` of room around it
/// for what overflows its box (strokes, shadows, blurs): the document's origin is `-pad`.
fn svg_padded(ctx: &Ctx<'_, '_>, (width, height): (f32, f32), pad: f32) -> Svg {
    let (width, height) = (width + 2.0 * pad, height + 2.0 * pad);
    // `0.0 - pad`, not `-pad`: no room is `0`, not `-0`.
    let origin = 0.0 - pad;
    let markup = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="{origin} {origin} {width} {height}"><defs>{}</defs>{}</svg>"#,
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
    /// How far auto layout moves a node of an instance from where its component has it: `(x, y)`.
    shifts: HashMap<NodeId, (f32, f32)>,
    /// Sizes laid out again by [`Ctx::fit`], replacing the file's.
    resized: HashMap<NodeId, (f32, f32)>,
    /// Texts [`Ctx::fit`] measured: drawn with the font, not the file's glyphs.
    relaid: HashSet<NodeId>,
    /// What background blurs need: see [`Backdrop`].
    backdrop: Backdrop,
}

/// A background blur shows what is drawn behind its node, blurred, inside the node's outline. SVG
/// cannot read what is behind an element, so the frame is drawn a second time, up to that node,
/// into `defs`, and used there.
#[derive(Default)]
struct Backdrop {
    /// The frame drawn, when a second drawing of it is possible (not for a component's state).
    root: Option<NodeId>,
    /// Prefix of the ids of a second drawing, unique in the document.
    prefix: String,
    /// In a second drawing: the node to stop before, and whether it was reached.
    stop_at: Option<NodeId>,
    stopped: bool,
    /// Each open node's transform from the frame, the innermost last.
    matrices: Vec<Matrix>,
}

impl Ctx<'_, '_> {
    fn id(&mut self, prefix: &str) -> String {
        self.next_id += 1;
        format!("{}{prefix}{}", self.backdrop.prefix, self.next_id)
    }

    /// `node`'s fields through the instances entered, at the size [`Ctx::fit`] gave it, if any:
    /// its stored outlines, made for the old size, are then dropped.
    fn resolve<'v>(&self, node: &Node<'v>) -> Cow<'v, Value> {
        let resolved = self.swapped(node, self.overrides.resolve(node));
        let Some(&(w, h)) = self.resized.get(&node.id) else {
            return resolved;
        };
        let mut value = resolved.into_owned();
        if let Value::Object(fields) = &mut value {
            fields
                .retain(|(name, _)| !matches!(&**name, "size" | "fillGeometry" | "strokeGeometry"));
            fields.push((
                "size".into(),
                Value::Object(vec![
                    ("x".into(), Value::Float(w)),
                    ("y".into(), Value::Float(h)),
                ]),
            ));
        }
        Cow::Owned(value)
    }

    /// An instance swapped for another component (`overriddenSymbolID`) looks like that one: the
    /// fields of its own look that no override changed come from the new component, as in Figma.
    fn swapped<'v>(&self, node: &Node<'v>, resolved: Cow<'v, Value>) -> Cow<'v, Value> {
        const LOOK: [&str; 10] = [
            "fillPaints",
            "strokePaints",
            "strokeWeight",
            "strokeAlign",
            "effects",
            "cornerRadius",
            "fillGeometry",
            "strokeGeometry",
            "styleIdForFill",
            "styleIdForStrokeFill",
        ];
        if node.kind != "INSTANCE" {
            return resolved;
        }
        let own = node
            .value
            .get("symbolData")
            .and_then(|s| s.get("symbolID"))
            .and_then(node_id);
        let Some(component) = instance_symbol(&resolved)
            .filter(|&id| Some(id) != own)
            .and_then(|id| self.tree.get(id))
        else {
            return resolved;
        };
        // The component's outlines fit its own size: at another size, they are drawn from the
        // instance's rounded bounds instead.
        let same_size = size(component.value) == size(&resolved);
        let mut value = resolved.into_owned();
        if let Value::Object(fields) = &mut value {
            for name in LOOK {
                // Overridden for this instance: kept.
                if fields.iter().find(|(n, _)| &**n == name).map(|(_, v)| v) != node.value.get(name)
                {
                    continue;
                }
                fields.retain(|(n, _)| &**n != name);
                let outline = matches!(name, "fillGeometry" | "strokeGeometry");
                if let Some(v) = component.value.get(name)
                    && (same_size || !outline)
                {
                    fields.push((name.into(), v.clone()));
                }
            }
        }
        Cow::Owned(value)
    }

    /// Draws `node` on its own, at the origin, leaving out `hidden`, on a canvas of `size`.
    fn alone(&mut self, node: &Node<'_>, hidden: Vec<NodeId>, size: (f32, f32), pad: f32) -> Svg {
        let kept = std::mem::replace(&mut self.hidden, hidden);
        self.body.clear();
        self.defs.clear();
        self.node(node, true);
        let drawn = svg_padded(self, size, pad);
        self.hidden = kept;
        drawn
    }

    /// The parts of `source`'s children (the parent's fields are `parent`), then of theirs: see
    /// [`state_parts`]. `to_root` maps the parent's coordinates to the state's.
    #[allow(clippy::too_many_arguments)]
    fn parts(
        &mut self,
        source: NodeId,
        parent: &Value,
        path: &str,
        to_root: Matrix,
        depth: usize,
        hidden: &[NodeId],
        out: &mut Vec<StatePart>,
    ) {
        let children: Vec<NodeId> = self.tree.children(source).map(|c| c.id).collect();
        self.reflow(parent, &children);
        for id in children {
            if out.len() >= MAX_PARTS {
                return;
            }
            let Some(child) = self.tree.get(id) else {
                continue;
            };
            let resolved = self.resolve(child).into_owned();
            if hidden.contains(&id) || matches!(resolved.get("visible"), Some(Value::Bool(false))) {
                continue;
            }
            let name = if path.is_empty() {
                child.name.to_owned()
            } else {
                format!("{path}/{}", child.name)
            };
            let (dx, dy) = self.shifts.get(&id).copied().unwrap_or((0.0, 0.0));
            let m = |k: &str, default: f32| {
                f64::from(
                    resolved
                        .get("transform")
                        .and_then(|m| m.get(k))
                        .and_then(Value::as_f32)
                        .unwrap_or(default),
                )
            };
            let local = Matrix::new(
                m("m00", 1.0),
                m("m10", 0.0),
                m("m01", 0.0),
                m("m11", 1.0),
                m("m02", 0.0) + f64::from(dx),
                m("m12", 0.0) + f64::from(dy),
            );
            let matrix = to_root.multiply(local);
            // Containers split into their children, which move on their own; instances, texts
            // and shapes are drawn whole.
            let split = depth < PART_DEPTH
                && matches!(child.kind, "FRAME" | "GROUP" | "SECTION")
                && self.tree.children(id).next().is_some();
            let mut leave_out = hidden.to_vec();
            if split {
                leave_out.extend(self.tree.children(id).map(|c| c.id));
            }
            let pad = overflow(self.tree, &resolved);
            let drawn = self.alone(child, leave_out, size(&resolved), pad);
            out.push(StatePart {
                name: name.clone(),
                svg: drawn,
                pad,
                matrix,
                value: resolved.clone(),
                split,
            });
            if split {
                self.parts(id, &resolved, &name, matrix, depth + 1, hidden, out);
            }
        }
    }

    /// Lays out again a variant other than the one an instance shows, for the instance's
    /// overrides, as Figma does when the instance changes variant (the file holds the layout of
    /// the variant shown only): a text whose characters changed and that grows with its content
    /// is measured with its font, and an auto-layout frame that hugs its content takes the size
    /// of what it holds. Returns the node's size.
    /// With `stored_wins`, a text whose glyphs the file stored keeps its layout.
    fn fit(&mut self, node: &Node<'_>, stored_wins: bool) -> (f32, f32) {
        let resolved = self.resolve(node).into_owned();
        let own = size(&resolved);
        match node.kind {
            "TEXT" => self.fit_text(node, &resolved, stored_wins).unwrap_or(own),
            "FRAME" | "SYMBOL" => {
                let fitted = self.fit_stack(node.id, &resolved, stored_wins);
                if (fitted.0 - own.0).abs() > 0.5 || (fitted.1 - own.1).abs() > 0.5 {
                    self.resized.insert(node.id, fitted);
                }
                fitted
            }
            _ => own,
        }
    }

    fn fit_text(&mut self, node: &Node<'_>, text: &Value, stored_wins: bool) -> Option<(f32, f32)> {
        let characters = |v: &Value| v.get("textData").and_then(|t| t.get("characters")).cloned();
        if characters(text) == characters(node.value) {
            return None;
        }
        let stored = text
            .get("derivedTextData")
            .and_then(|d| d.get("glyphs"))
            .is_some_and(|g| !g.as_array().is_empty());
        if stored_wins && stored {
            return None;
        }
        let grows = text.get("textAutoResize").and_then(Value::as_str);
        if !matches!(grows, Some("WIDTH_AND_HEIGHT" | "HEIGHT")) {
            return None;
        }
        let style = TextStyle::of(text)?;
        let set = self.typesetter.as_deref_mut()?.set(&style.paragraph())?;
        let width = if grows == Some("HEIGHT") {
            size(text).0
        } else {
            set.width
        };
        self.relaid.insert(node.id);
        self.resized.insert(node.id, (width, set.height));
        Some((width, set.height))
    }

    /// `source` holds the children: the frame itself, or an instance's component.
    fn fit_stack(&mut self, source: NodeId, frame: &Value, stored_wins: bool) -> (f32, f32) {
        let own = size(frame);
        let children: Vec<NodeId> = self.tree.children(source).map(|c| c.id).collect();
        let mut sizes = Vec::new();
        for &id in &children {
            let Some(child) = self.tree.get(id) else {
                continue;
            };
            let fitted = self.fit(child, stored_wins);
            let child = self.resolve(child);
            if !matches!(child.get("visible"), Some(Value::Bool(false)))
                && child.get("stackPositioning").and_then(Value::as_str) != Some("ABSOLUTE")
            {
                sizes.push(fitted);
            }
        }
        let horizontal = match frame.get("stackMode").and_then(Value::as_str) {
            Some("HORIZONTAL") => true,
            Some("VERTICAL") => false,
            _ => return own,
        };
        if !children.iter().any(|id| self.resized.contains_key(id)) {
            return own;
        }
        let number = |k: &str| frame.get(k).and_then(Value::as_f32);
        let left = number("stackHorizontalPadding").unwrap_or(0.0);
        let top = number("stackVerticalPadding").unwrap_or(0.0);
        let padding = (
            left + number("stackPaddingRight").unwrap_or(left),
            top + number("stackPaddingBottom").unwrap_or(top),
        );
        let spacing = number("stackSpacing").unwrap_or(0.0);
        let gaps: f32 = sizes.iter().skip(1).map(|_| spacing).sum();
        let (along, across): (Vec<f32>, Vec<f32>) = sizes
            .iter()
            .map(|&(w, h)| if horizontal { (w, h) } else { (h, w) })
            .unzip();
        let along = along.iter().sum::<f32>() + gaps;
        let across = across.iter().copied().fold(0.0, f32::max);
        let (mut w, mut h) = own;
        let (primary, counter) = if horizontal {
            ((&mut w, padding.0), (&mut h, padding.1))
        } else {
            ((&mut h, padding.1), (&mut w, padding.0))
        };
        if hugs(frame, "stackPrimarySizing") {
            *primary.0 = along + primary.1;
        }
        if hugs(frame, "stackCounterSizing") {
            *counter.0 = across + counter.1;
        }
        (w, h)
    }

    fn node(&mut self, node: &Node<'_>, root: bool) {
        let resolved = self.resolve(node);
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
        if self.backdrop.stopped || self.backdrop.stop_at == Some(node.id) {
            self.backdrop.stopped = true;
            return;
        }

        let mut open = 0;
        let mut attrs = String::new();
        let parent = self
            .backdrop
            .matrices
            .last()
            .copied()
            .unwrap_or(Matrix::IDENTITY);
        let mut local = Matrix::IDENTITY;
        if !root && let Some(m) = v.get("transform") {
            let f = |k: &str| m.get(k).and_then(Value::as_f32).unwrap_or(0.0);
            let (dx, dy) = self.shifts.get(&node.id).copied().unwrap_or((0.0, 0.0));
            local = Matrix::new(
                f64::from(f("m00")),
                f64::from(f("m10")),
                f64::from(f("m01")),
                f64::from(f("m11")),
                f64::from(f("m02") + dx),
                f64::from(f("m12") + dy),
            );
            // SVG's matrix(a b c d e f) is column-major: Figma's m00 m10 m01 m11 m02 m12.
            let _ = write!(
                attrs,
                r#" transform="matrix({} {} {} {} {} {})""#,
                f("m00"),
                f("m10"),
                f("m01"),
                f("m11"),
                f("m02") + dx,
                f("m12") + dy
            );
        }
        if let Some(opacity) = v.get("opacity").and_then(Value::as_f32)
            && opacity < 1.0
        {
            let _ = write!(attrs, r#" opacity="{opacity}""#);
        }
        if !root {
            attrs.push_str(&fx::blend_style(
                v.get("blendMode")
                    .and_then(Value::as_str)
                    .and_then(fx::blend),
            ));
        }
        let _ = write!(self.body, "<g{attrs}>");
        open += 1;

        self.backdrop.matrices.push(parent.multiply(local));
        let paint = !root || self.root.paint;
        if paint && !root {
            self.background_blur(node);
        }
        if paint {
            open += self.effects(v);
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
                if self.resized.contains_key(&node.id) {
                    self.stroke_bounds(v);
                } else {
                    self.paint_geometry(v, "strokeGeometry", "strokePaints", false);
                }
            }
        }

        for _ in 0..open {
            self.body.push_str("</g>");
        }
        self.backdrop.matrices.pop();
    }

    /// Draws what is behind `node`, blurred, inside its outline, when it has a background blur.
    fn background_blur(&mut self, node: &Node<'_>) {
        let Some(radius) = node
            .value
            .get("effects")
            .map_or(&[][..], Value::as_array)
            .iter()
            .find(|e| {
                visible(e) && e.get("type").and_then(Value::as_str) == Some("BACKGROUND_BLUR")
            })
            .and_then(|e| e.get("radius").and_then(Value::as_f32))
        else {
            return;
        };
        let Some(root) = self.backdrop.root.and_then(|id| self.tree.get(id)) else {
            return;
        };
        // The frame again, up to this node, with ids of its own.
        let prefix = format!("{}b{}_", self.backdrop.prefix, self.next_id + 1);
        self.next_id += 1;
        let mut behind = Ctx {
            tree: self.tree,
            defs: String::new(),
            body: String::new(),
            next_id: 0,
            hidden: self.hidden.clone(),
            root: self.root,
            overrides: Overrides::default(),
            typesetter: None,
            missing_fonts: BTreeSet::new(),
            shifts: HashMap::new(),
            resized: HashMap::new(),
            relaid: HashSet::new(),
            backdrop: Backdrop {
                root: None,
                prefix: prefix.clone(),
                stop_at: Some(node.id),
                ..Backdrop::default()
            },
        };
        behind.node(root, true);
        let Some(to_node) = self.backdrop.matrices.last().and_then(|m| m.invert()) else {
            return;
        };
        let (clip, blur) = (self.id("c"), self.id("f"));
        let outline = self.outline(node.value);
        let _ = write!(
            self.defs,
            r#"{}<g id="{prefix}frame">{}</g><clipPath id="{clip}">{outline}</clipPath>"#,
            behind.defs, behind.body
        );
        fx::layer_blur(&mut self.defs, &blur, f64::from(radius));
        let _ = write!(
            self.body,
            r##"<g clip-path="url(#{clip})"><g transform="matrix({} {} {} {} {} {})"><g filter="url(#{blur})"><use href="#{prefix}frame"/></g></g></g>"##,
            to_node.a, to_node.b, to_node.c, to_node.d, to_node.e, to_node.f
        );
    }

    fn children(&mut self, node: &Node<'_>, root: bool) {
        // A boolean operation is drawn from the outline Figma computed (its own `fillGeometry`,
        // painted with its own fill): its children are the operands, not drawn.
        if node.kind == "BOOLEAN_OPERATION" {
            return;
        }
        let container = matches!(node.kind, "FRAME" | "SYMBOL" | "INSTANCE" | "COMPONENT_SET");
        // Instances do not store their children: draw the component's, without overrides.
        let source = if node.kind == "INSTANCE" {
            instance_symbol(node.value).unwrap_or(node.id)
        } else {
            node.id
        };
        // A component drawn inside itself would never end.
        if node.kind == "INSTANCE" && self.overrides.inside(source) {
            return;
        }
        let children: Vec<NodeId> = self.tree.children(source).map(|c| c.id).collect();
        if children.is_empty() {
            return;
        }
        self.reflow(node.value, &children);
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
        // A mask is not drawn: it masks the siblings above it, up to the end of the parent.
        let mut masks = 0;
        for child in children {
            let Some(child) = self.tree.get(child) else {
                continue;
            };
            let resolved = self.resolve(child);
            if !matches!(resolved.get("mask"), Some(Value::Bool(true))) {
                self.node(child, false);
                continue;
            }
            let start = self.body.len();
            self.node(child, false);
            let drawn = self.body.split_off(start);
            let id = self.id("m");
            let kind = match resolved.get("maskType").and_then(Value::as_str) {
                Some("LUMINANCE") => "luminance",
                _ => "alpha",
            };
            let _ = write!(
                self.defs,
                r#"<mask id="{id}" mask-type="{kind}" maskUnits="userSpaceOnUse" x="-100000" y="-100000" width="200000" height="200000">{drawn}</mask>"#
            );
            let _ = write!(self.body, r#"<g mask="url(#{id})">"#);
            masks += 1;
        }
        for _ in 0..masks {
            self.body.push_str("</g>");
        }
        if instance {
            self.overrides.leave();
        }
        if clip {
            self.body.push_str("</g>");
        }
    }

    /// Where auto layout puts the children of a stack inside an instance. Figma writes the sizes it
    /// computed for the instance, not always the positions that follow: when a child grew (a
    /// longer label, a spacer that fills), those after it move along the stack. Only a stack that
    /// starts at its beginning is reflowed, from the sizes and spacing it has; anything else keeps
    /// the positions of the component.
    fn reflow(&mut self, parent: &Value, children: &[NodeId]) {
        if !self.overrides.active() {
            return;
        }
        let horizontal = match parent.get("stackMode").and_then(Value::as_str) {
            Some("HORIZONTAL") => true,
            Some("VERTICAL") => false,
            _ => return,
        };
        // A stack that hugs its content along its axis has no room to align it in.
        if !hugs(parent, "stackPrimarySizing")
            && parent
                .get("stackPrimaryAlignItems")
                .and_then(Value::as_str)
                .is_some_and(|align| align != "MIN")
        {
            return;
        }
        let number = |v: &Value, k: &str| v.get(k).and_then(Value::as_f32).unwrap_or(0.0);
        let mut cursor = number(
            parent,
            if horizontal {
                "stackHorizontalPadding"
            } else {
                "stackVerticalPadding"
            },
        );
        let spacing = number(parent, "stackSpacing");
        let mut shifts = Vec::new();
        for &id in children {
            let Some(child) = self.tree.get(id) else {
                continue;
            };
            let child = self.resolve(child);
            if matches!(child.get("visible"), Some(Value::Bool(false)))
                || child.get("stackPositioning").and_then(Value::as_str) == Some("ABSOLUTE")
            {
                continue;
            }
            let Some(m) = child.get("transform") else {
                return;
            };
            let (w, h) = size(&child);
            let f = |k: &str| number(m, k);
            // The child's extent along the stack, whatever its rotation.
            let (origin, along_w, along_h) = if horizontal {
                (f("m02"), f("m00") * w, f("m01") * h)
            } else {
                (f("m12"), f("m10") * w, f("m11") * h)
            };
            let corners = [0.0, along_w, along_h, along_w + along_h];
            let low = origin + corners.iter().copied().fold(f32::INFINITY, f32::min);
            let high = origin + corners.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            if (cursor - low).abs() > 0.5 {
                shifts.push((id, cursor - low));
            }
            cursor += high - low + spacing;
        }
        for (id, by) in shifts {
            self.shifts
                .insert(id, if horizontal { (by, 0.0) } else { (0.0, by) });
        }
    }

    /// The stroke of a node laid out again, whose stored stroke outline no longer fits: a ring
    /// along its rounded bounds, inside, centred on or outside them as the stroke is aligned.
    fn stroke_bounds(&mut self, v: &Value) {
        let weight = v.get("strokeWeight").and_then(Value::as_f32).unwrap_or(0.0);
        if weight <= 0.0 {
            return;
        }
        let (w, h) = size(v);
        let r = v.get("cornerRadius").and_then(Value::as_f32).unwrap_or(0.0);
        let (outward, inward) = match v.get("strokeAlign").and_then(Value::as_str) {
            Some("INSIDE") => (0.0, weight),
            Some("OUTSIDE") => (weight, 0.0),
            _ => (weight / 2.0, weight / 2.0),
        };
        let ring = format!(
            "{}{}",
            rounded_rect(
                -outward,
                -outward,
                w + 2.0 * outward,
                h + 2.0 * outward,
                r + outward
            ),
            rounded_rect(
                inward,
                inward,
                w - 2.0 * inward,
                h - 2.0 * inward,
                r - inward
            )
        );
        let paints: Vec<Value> = paints_of(self.tree, v, "strokePaints")
            .iter()
            .filter(|p| visible(p))
            .cloned()
            .collect();
        for paint in paints {
            if let Some(fill) = self.paint(&paint, w, h) {
                let _ = write!(
                    self.body,
                    r#"<path d="{ring}" fill-rule="evenodd" {fill}/>"#
                );
            }
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
        let paints: Vec<&Value> = paints_of(self.tree, v, paints)
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
            if v.get("type").and_then(Value::as_str) == Some("ELLIPSE") {
                shapes.push(format!(
                    r#"<ellipse cx="{}" cy="{}" rx="{}" ry="{}""#,
                    w / 2.0,
                    h / 2.0,
                    w / 2.0,
                    h / 2.0
                ));
            } else {
                let r = v.get("cornerRadius").and_then(Value::as_f32).unwrap_or(0.0);
                shapes.push(format!(r#"<rect width="{w}" height="{h}" rx="{r}""#));
            }
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

    /// Text from the glyph outlines Figma stored after laying it out. A run of the text can have
    /// a fill of its own (`textData.styleOverrideTable`): its glyphs are drawn with it.
    fn text(&mut self, node: &Node<'_>) {
        let v = node.value;
        let (w, h) = size(v);
        let base = paints_of(self.tree, v, "fillPaints")
            .iter()
            .find(|p| visible(p))
            .cloned();
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
        // the one stored, with another style) and the right one was given, the text was laid out
        // again ([`Ctx::fit`]), or a file written by another tool stored none.
        let unstored = glyphs.is_empty()
            && v.get("textData")
                .and_then(|t| t.get("characters"))
                .and_then(Value::as_str)
                .is_some_and(|c| !c.is_empty());
        if stale_glyphs(v) || unstored || self.relaid.contains(&node.id) {
            let path = self.lay_out(v).unwrap_or_default();
            if !path.is_empty()
                && let Some(base) = base
                && let Some(fill) = self.paint(&base, w, h)
            {
                let _ = write!(self.body, r#"<path d="{path}" {fill}/>"#);
            }
            return;
        }
        // The glyphs to draw with each fill: the run's own, or the text's.
        let runs = run_fills(self.tree, v);
        let mut paths: Vec<(u32, String)> = Vec::new();
        for glyph in glyphs.iter().take(shown) {
            let style = glyph
                .get("firstCharacter")
                .and_then(Value::as_u32)
                .and_then(|c| runs.styles.get(c as usize))
                .copied()
                .filter(|s| runs.fills.iter().any(|(id, _)| id == s))
                .unwrap_or(0);
            let at = if let Some(at) = paths.iter().position(|(id, _)| *id == style) {
                at
            } else {
                paths.push((style, String::new()));
                paths.len() - 1
            };
            self.glyph(glyph, &mut paths[at].1);
        }
        for (style, path) in paths {
            let paint = runs
                .fills
                .iter()
                .find(|(id, _)| *id == style)
                .and_then(|(_, paints)| paints.iter().find(|p| visible(p)))
                .or(base.as_ref());
            if path.is_empty() {
                continue;
            }
            if let Some(fill) = paint.and_then(|p| self.paint(p, w, h)) {
                let _ = write!(self.body, r#"<path d="{path}" {fill}/>"#);
            }
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

/// The paints of a node: those of the shared style it uses, when it has one, else its own.
/// Figma keeps a node's own paints beside the style's, and they go stale when the style is
/// edited: the style is what Figma draws.
fn paints_of<'v>(tree: &'v Tree<'_>, v: &'v Value, key: &str) -> &'v [Value] {
    let style = match key {
        "fillPaints" => "styleIdForFill",
        "strokePaints" => "styleIdForStrokeFill",
        _ => "",
    };
    v.get(style)
        .and_then(|s| s.get("guid"))
        .and_then(node_id)
        .and_then(|id| tree.get(id))
        .and_then(|style| style.value.get("fillPaints"))
        .or_else(|| v.get(key))
        .map_or(&[][..], Value::as_array)
}

/// What a text's runs say about their fill: the style of each character, and the paints of the
/// styles that have some (their own, or a shared fill style's).
struct RunFills {
    styles: Vec<u32>,
    fills: Vec<(u32, Vec<Value>)>,
}

fn run_fills(tree: &Tree<'_>, text: &Value) -> RunFills {
    let data = text.get("textData");
    let styles = data
        .and_then(|d| d.get("characterStyleIDs"))
        .map_or(&[][..], Value::as_array)
        .iter()
        .map(|s| s.as_u32().unwrap_or(0))
        .collect();
    let fills = data
        .and_then(|d| d.get("styleOverrideTable"))
        .map_or(&[][..], Value::as_array)
        .iter()
        .filter_map(|o| {
            let id = o.get("styleID").and_then(Value::as_u32)?;
            let paints = paints_of(tree, o, "fillPaints");
            (!paints.is_empty()).then(|| (id, paints.to_vec()))
        })
        .collect();
    RunFills { styles, fills }
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
