// SPDX-License-Identifier: AGPL-3.0-or-later

//! How a design file's frames become SVG: one [`Draw`] per format, so the bake itself
//! (`folio`) only knows the IR.

use std::collections::BTreeSet;

use crate::figma::tree::{NodeId, Tree};
use crate::ir::{self, Rect};
use crate::kiwi::Value;
use helpers4::geometry::Matrix;

use crate::bake::folio::BakeError;
use crate::bake::svg;
use crate::bake::text::Typesetter;

/// Draws the frames of one design file as SVG.
pub trait Draw {
    /// The whole frame as it shows: the placeholder and the poster are made from it.
    fn frame(&mut self, frame: &ir::Frame) -> Option<String>;

    /// The frame's layers, bottom first, as `(markup, bounds, scrolls)`; `whole` is what
    /// [`Draw::frame`] gave. Interactive components are left out: [`Draw::state`] draws them.
    ///
    /// # Errors
    ///
    /// Fails when a part of the frame cannot be drawn.
    fn layers(
        &mut self,
        frame: &ir::Frame,
        whole: &str,
    ) -> Result<Vec<(String, Rect, bool)>, BakeError>;

    /// One state of an interactive component.
    fn state(&mut self, component: &ir::Component, state: &ir::ComponentState) -> Option<String>;

    /// Fonts some text needed and was not given (`Family Style`), over every call so far.
    fn missing_fonts(&mut self) -> BTreeSet<String>;

    /// The parts smart animate pairs (docs/spec/player.md): the frame's background, named `""`,
    /// then its layers, each drawn without its own parts. None by default.
    fn parts(&mut self, _frame: &ir::Frame) -> Vec<Part> {
        Vec::new()
    }

    /// The parts of a component's state, for smart animate between states, in the state's
    /// coordinates. None by default.
    fn state_parts(
        &mut self,
        _component: &ir::Component,
        _state: &ir::ComponentState,
    ) -> Vec<Part> {
        Vec::new()
    }
}

/// A part of a frame that smart animate moves on its own.
#[derive(Debug, Clone)]
pub struct Part {
    /// Its layer's name, under its parents' (`Card/Title`); the background is `""`.
    pub name: String,
    pub markup: String,
    /// Where its image goes, unrotated, in the frame's coordinates.
    pub bounds: Rect,
    /// Its transform in the frame's coordinates, its image's top-left corner at the origin:
    /// `[a, b, c, d, e, f]`, when it is turned or flipped.
    pub matrix: Option<[f32; 6]>,
    /// A plain rounded rectangle: its fill and corner radius, which the player can animate
    /// without stretching the corners.
    pub shape: Option<PartShape>,
}

/// A rounded rectangle with a solid fill.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PartShape {
    /// CSS colour.
    pub fill: String,
    pub radius: f32,
}

/// Draws a `.fig` from its node tree.
pub struct FigmaDraw<'t, 'a> {
    tree: &'t Tree<'a>,
    typesetter: Typesetter,
    missing_fonts: BTreeSet<String>,
}

impl<'t, 'a> FigmaDraw<'t, 'a> {
    #[must_use]
    pub fn new(tree: &'t Tree<'a>, typesetter: Typesetter) -> Self {
        Self {
            tree,
            typesetter,
            missing_fonts: BTreeSet::new(),
        }
    }
}

impl Draw for FigmaDraw<'_, '_> {
    fn frame(&mut self, frame: &ir::Frame) -> Option<String> {
        svg::render_node(self.tree, frame.id.parse().ok()?).map(|svg| svg.markup)
    }

    fn layers(
        &mut self,
        frame: &ir::Frame,
        whole_markup: &str,
    ) -> Result<Vec<(String, Rect, bool)>, BakeError> {
        let missing = || BakeError::MissingFrame(frame.id.clone());
        let id: NodeId = frame.id.parse().map_err(|_| missing())?;
        let tree = self.tree;
        // Interactive components are left out of the frame's layers: they get their own.
        let hidden: Vec<NodeId> = frame
            .components
            .iter()
            .filter_map(|c| c.id.rsplit(';').next()?.parse().ok())
            .collect();
        let whole = Rect {
            x: 0.0,
            y: 0.0,
            width: frame.bounds.width,
            height: frame.bounds.height,
        };
        let Some(scroll) = &frame.scroll else {
            let markup = if hidden.is_empty() {
                whole_markup.to_owned()
            } else {
                svg::render_node_without(tree, id, &hidden)
                    .ok_or_else(missing)?
                    .markup
            };
            return Ok(vec![(markup, whole, false)]);
        };
        let fixed: Vec<NodeId> = scroll.fixed.iter().filter_map(|f| f.parse().ok()).collect();
        let part = |part| {
            svg::render_part(tree, id, part, &hidden)
                .map(|svg| svg.markup)
                .ok_or_else(missing)
        };
        let size = (scroll.content.width, scroll.content.height);
        let mut parts = vec![
            (part(svg::Part::Background)?, whole, false),
            (
                part(svg::Part::Content {
                    fixed: &fixed,
                    size,
                })?,
                scroll.content,
                true,
            ),
        ];
        if !fixed.is_empty() {
            parts.push((part(svg::Part::Fixed { fixed: &fixed })?, whole, false));
        }
        Ok(parts)
    }

    fn state(&mut self, component: &ir::Component, state: &ir::ComponentState) -> Option<String> {
        let variant = state.variant.parse().ok()?;
        let path: Vec<NodeId> = component
            .id
            .split(';')
            .map(str::parse)
            .collect::<Result<_, _>>()
            .ok()?;
        // Components nested in the state are drawn on their own, over it.
        let hidden: Vec<NodeId> = state
            .components
            .iter()
            .filter_map(|c| c.id.rsplit(';').next()?.parse().ok())
            .collect();
        let svg = svg::render_component_state(
            self.tree,
            &path,
            variant,
            Some(&mut self.typesetter),
            &hidden,
        )?;
        self.missing_fonts.extend(svg.missing_fonts);
        Some(svg.markup)
    }

    fn missing_fonts(&mut self) -> BTreeSet<String> {
        std::mem::take(&mut self.missing_fonts)
    }

    fn parts(&mut self, frame: &ir::Frame) -> Vec<Part> {
        let Ok(id) = frame.id.parse::<NodeId>() else {
            return Vec::new();
        };
        let mut parts = Vec::new();
        if let Some(background) = svg::render_part(self.tree, id, svg::Part::Background, &[]) {
            let bounds = Rect {
                x: 0.0,
                y: 0.0,
                width: frame.bounds.width,
                height: frame.bounds.height,
            };
            parts.push(Part {
                name: String::new(),
                markup: background.markup,
                bounds,
                matrix: None,
                shape: None,
            });
        }
        figma_parts(self.tree, id, "", Matrix::IDENTITY, 1, &mut parts);
        parts
    }

    fn state_parts(&mut self, _component: &ir::Component, state: &ir::ComponentState) -> Vec<Part> {
        let Ok(variant) = state.variant.parse::<NodeId>() else {
            return Vec::new();
        };
        let mut parts = Vec::new();
        if let Some(background) = svg::render_part(self.tree, variant, svg::Part::Background, &[]) {
            parts.push(Part {
                name: String::new(),
                markup: background.markup,
                bounds: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: background.width,
                    height: background.height,
                },
                matrix: None,
                shape: None,
            });
        }
        figma_parts(self.tree, variant, "", Matrix::IDENTITY, 1, &mut parts);
        parts
    }
}

/// How deep smart animate pairs nested layers, and how many parts a frame has at most.
const PART_DEPTH: usize = 3;
const MAX_PARTS: usize = 80;

/// The parts of `parent`'s visible children, then of theirs, each drawn without its own parts;
/// `to_frame` maps `parent`'s coordinates to the frame's.
fn figma_parts(
    tree: &Tree<'_>,
    parent: NodeId,
    path: &str,
    to_frame: Matrix,
    depth: usize,
    parts: &mut Vec<Part>,
) {
    let children: Vec<_> = tree
        .children(parent)
        .filter(|c| !matches!(c.value.get("visible"), Some(Value::Bool(false))))
        .collect();
    for child in children {
        if parts.len() >= MAX_PARTS {
            return;
        }
        let name = if path.is_empty() {
            child.name.to_owned()
        } else {
            format!("{path}/{}", child.name)
        };
        let number = |k: &str, default: f32| {
            f64::from(
                child
                    .value
                    .get("transform")
                    .and_then(|m| m.get(k))
                    .and_then(Value::as_f32)
                    .unwrap_or(default),
            )
        };
        let local = Matrix::new(
            number("m00", 1.0),
            number("m10", 0.0),
            number("m01", 0.0),
            number("m11", 1.0),
            number("m02", 0.0),
            number("m12", 0.0),
        );
        let matrix = to_frame.multiply(local);
        // Containers split into their children, which move on their own; instances, texts and
        // shapes are drawn whole.
        let nested = depth < PART_DEPTH
            && matches!(child.kind, "FRAME" | "GROUP" | "SECTION")
            && tree.children(child.id).next().is_some();
        let hidden: Vec<NodeId> = if nested {
            tree.children(child.id).map(|c| c.id).collect()
        } else {
            Vec::new()
        };
        let Some(svg) = svg::render_node_without(tree, child.id, &hidden) else {
            continue;
        };
        #[allow(clippy::cast_possible_truncation)] // design coordinates fit in f32
        let turned = (matrix.a - 1.0).abs() > 1e-6
            || matrix.b.abs() > 1e-6
            || matrix.c.abs() > 1e-6
            || (matrix.d - 1.0).abs() > 1e-6;
        #[allow(clippy::cast_possible_truncation)]
        let as_f32 = |m: Matrix| [m.a, m.b, m.c, m.d, m.e, m.f].map(|v| v as f32);
        #[allow(clippy::cast_possible_truncation)]
        parts.push(Part {
            name: name.clone(),
            markup: svg.markup,
            bounds: Rect {
                x: matrix.e as f32,
                y: matrix.f as f32,
                width: svg.width,
                height: svg.height,
            },
            matrix: turned.then(|| as_f32(matrix)),
            shape: if nested { None } else { plain_box(child.value) },
        });
        if nested {
            figma_parts(tree, child.id, &name, matrix, depth + 1, parts);
        }
    }
}

/// A rectangle (or a childless frame) with one solid fill and no stroke or effect: what the
/// player can animate as a box, its corners kept round whatever its size.
fn plain_box(node: &Value) -> Option<PartShape> {
    let kind = node.get("type").and_then(Value::as_str)?;
    if !matches!(kind, "RECTANGLE" | "ROUNDED_RECTANGLE" | "FRAME") {
        return None;
    }
    let visible = |v: &&Value| !matches!(v.get("visible"), Some(Value::Bool(false)));
    let list = |k: &str| -> Vec<&Value> {
        node.get(k)
            .map_or(&[][..], Value::as_array)
            .iter()
            .filter(visible)
            .collect()
    };
    let fills = list("fillPaints");
    if fills.len() != 1 || !list("strokePaints").is_empty() || !list("effects").is_empty() {
        return None;
    }
    let fill = fills[0];
    if fill.get("type").and_then(Value::as_str) != Some("SOLID") {
        return None;
    }
    let color = fill.get("color")?;
    let ch = |k: &str| color.get(k).and_then(Value::as_f32).map_or(0.0, f64::from);
    let opacity = fill
        .get("opacity")
        .and_then(Value::as_f32)
        .map_or(1.0, f64::from);
    let rgba = helpers4::color::Rgba::from_unit(ch("r"), ch("g"), ch("b"), ch("a") * opacity);
    Some(PartShape {
        fill: rgba.to_css(),
        radius: node
            .get("cornerRadius")
            .and_then(Value::as_f32)
            .unwrap_or(0.0),
    })
}
