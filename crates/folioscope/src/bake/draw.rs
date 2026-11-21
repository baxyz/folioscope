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

    /// The layers that scroll inside the screen and the sticky ones: each scroller's content,
    /// and the children that stick, of a scroller or of the frame. None by default.
    ///
    /// # Errors
    ///
    /// Fails when a part of the frame cannot be drawn.
    fn scroll_layers(&mut self, _frame: &ir::Frame) -> Result<Vec<ScrollLayer>, BakeError> {
        Ok(Vec::new())
    }

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

/// A layer that scrolls inside the screen, or that sticks once scrolled to.
#[derive(Debug, Clone)]
pub struct ScrollLayer {
    pub markup: String,
    /// In the scroller's content (or the frame's, for a sticky child of the frame).
    pub bounds: Rect,
    /// The scroller it is in, by id; none for a sticky child of the frame itself.
    pub scroller: Option<String>,
    pub sticky: bool,
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
        let svg = svg::render_node(
            self.tree,
            frame.id.parse().ok()?,
            Some(&mut self.typesetter),
        )?;
        self.missing_fonts.extend(svg.missing_fonts);
        Some(svg.markup)
    }

    fn layers(
        &mut self,
        frame: &ir::Frame,
        whole_markup: &str,
    ) -> Result<Vec<(String, Rect, bool)>, BakeError> {
        let missing = || BakeError::MissingFrame(frame.id.clone());
        let id: NodeId = frame.id.parse().map_err(|_| missing())?;
        let FigmaDraw {
            tree,
            typesetter,
            missing_fonts,
        } = self;
        let tree = *tree;
        // Interactive components are left out of the frame's layers: they get their own. So are
        // what scrolls inside the screen and what sticks.
        let mut hidden: Vec<NodeId> = frame
            .components
            .iter()
            .filter_map(|c| c.id.rsplit(';').next()?.parse().ok())
            .collect();
        for scroller in &frame.scrollers {
            let scroller: NodeId = scroller.id.parse().map_err(|_| missing())?;
            hidden.extend(tree.children(scroller).map(|c| c.id));
        }
        if let Some(scroll) = &frame.scroll {
            hidden.extend(
                scroll
                    .sticky
                    .iter()
                    .filter_map(|s| s.parse::<NodeId>().ok()),
            );
        }
        let simple = hidden.is_empty();
        let whole = Rect {
            x: 0.0,
            y: 0.0,
            width: frame.bounds.width,
            height: frame.bounds.height,
        };
        let Some(scroll) = &frame.scroll else {
            let markup = if simple {
                whole_markup.to_owned()
            } else {
                let svg = svg::render_node_without(tree, id, &hidden, Some(typesetter))
                    .ok_or_else(missing)?;
                missing_fonts.extend(svg.missing_fonts);
                svg.markup
            };
            return Ok(vec![(markup, whole, false)]);
        };
        let fixed: Vec<NodeId> = scroll.fixed.iter().filter_map(|f| f.parse().ok()).collect();
        let mut part = |part| {
            let svg = svg::render_part(tree, id, part, &hidden, Some(&mut *typesetter))
                .ok_or_else(missing)?;
            missing_fonts.extend(svg.missing_fonts);
            Ok::<String, BakeError>(svg.markup)
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

    fn scroll_layers(&mut self, frame: &ir::Frame) -> Result<Vec<ScrollLayer>, BakeError> {
        let missing = || BakeError::MissingFrame(frame.id.clone());
        let FigmaDraw {
            tree,
            typesetter,
            missing_fonts,
        } = self;
        let tree = *tree;
        let components: Vec<NodeId> = frame
            .components
            .iter()
            .filter_map(|c| c.id.rsplit(';').next()?.parse().ok())
            .collect();
        let mut layers = Vec::new();
        // A sticky child, drawn on its own at its place in its parent's content.
        let sticky = |layers: &mut Vec<ScrollLayer>,
                      typesetter: &mut Typesetter,
                      missing_fonts: &mut BTreeSet<String>,
                      id: &str,
                      scroller: Option<String>| {
            let node: NodeId = id.parse().map_err(|_| missing())?;
            let child = tree.get(node).ok_or_else(missing)?;
            let svg = svg::render_node(tree, node, Some(typesetter)).ok_or_else(missing)?;
            missing_fonts.extend(svg.missing_fonts);
            let offset = |k: &str| {
                child
                    .value
                    .get("transform")
                    .and_then(|m| m.get(k))
                    .and_then(Value::as_f32)
                    .unwrap_or(0.0)
            };
            layers.push(ScrollLayer {
                markup: svg.markup,
                bounds: Rect {
                    x: offset("m02"),
                    y: offset("m12"),
                    width: svg.width,
                    height: svg.height,
                },
                scroller,
                sticky: true,
            });
            Ok::<(), BakeError>(())
        };
        for scroller in &frame.scrollers {
            let id: NodeId = scroller.id.parse().map_err(|_| missing())?;
            let fixed: Vec<NodeId> = scroller
                .sticky
                .iter()
                .filter_map(|s| s.parse().ok())
                .collect();
            let size = (scroller.content.width, scroller.content.height);
            let content = svg::render_part(
                tree,
                id,
                svg::Part::Content {
                    fixed: &fixed,
                    size,
                },
                &components,
                Some(&mut *typesetter),
            )
            .ok_or_else(missing)?;
            missing_fonts.extend(content.missing_fonts);
            layers.push(ScrollLayer {
                markup: content.markup,
                bounds: scroller.content,
                scroller: Some(scroller.id.clone()),
                sticky: false,
            });
            for child in &scroller.sticky {
                sticky(
                    &mut layers,
                    typesetter,
                    missing_fonts,
                    child,
                    Some(scroller.id.clone()),
                )?;
            }
        }
        if let Some(scroll) = &frame.scroll {
            for child in &scroll.sticky {
                sticky(&mut layers, typesetter, missing_fonts, child, None)?;
            }
        }
        Ok(layers)
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
        if let Some(background) = svg::render_part(
            self.tree,
            id,
            svg::Part::Background,
            &[],
            Some(&mut self.typesetter),
        ) {
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
        figma_parts(
            self.tree,
            &mut self.typesetter,
            &mut self.missing_fonts,
            id,
            "",
            Matrix::IDENTITY,
            1,
            &mut parts,
        );
        parts
    }

    fn state_parts(&mut self, component: &ir::Component, state: &ir::ComponentState) -> Vec<Part> {
        let Ok(variant) = state.variant.parse::<NodeId>() else {
            return Vec::new();
        };
        let Ok(path) = component
            .id
            .split(';')
            .map(str::parse)
            .collect::<Result<Vec<NodeId>, _>>()
        else {
            return Vec::new();
        };
        // Components nested in the state are drawn on their own, over it.
        let hidden: Vec<NodeId> = state
            .components
            .iter()
            .filter_map(|c| c.id.rsplit(';').next()?.parse().ok())
            .collect();
        let Some((drawn, missing)) = svg::state_parts(
            self.tree,
            &path,
            variant,
            Some(&mut self.typesetter),
            &hidden,
        ) else {
            return Vec::new();
        };
        self.missing_fonts.extend(missing);
        drawn
            .into_iter()
            .map(|part| {
                let m = part.matrix;
                let turned = (m.a - 1.0).abs() > 1e-6
                    || m.b.abs() > 1e-6
                    || m.c.abs() > 1e-6
                    || (m.d - 1.0).abs() > 1e-6;
                #[allow(clippy::cast_possible_truncation)] // design coordinates fit in f32
                let as_f32 = [m.a, m.b, m.c, m.d, m.e, m.f].map(|v| v as f32);
                Part {
                    shape: if part.split || part.name.is_empty() {
                        None
                    } else {
                        plain_box(&part.value)
                    },
                    name: part.name,
                    bounds: Rect {
                        x: as_f32[4],
                        y: as_f32[5],
                        width: part.svg.width,
                        height: part.svg.height,
                    },
                    matrix: turned.then_some(as_f32),
                    markup: part.svg.markup,
                }
            })
            .collect()
    }
}

/// How deep smart animate pairs nested layers, and how many parts a frame has at most.
const PART_DEPTH: usize = 3;
const MAX_PARTS: usize = 80;

/// The parts of `parent`'s visible children, then of theirs, each drawn without its own parts;
/// `to_frame` maps `parent`'s coordinates to the frame's.
#[allow(clippy::too_many_arguments)]
fn figma_parts(
    tree: &Tree<'_>,
    typesetter: &mut Typesetter,
    missing_fonts: &mut BTreeSet<String>,
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
        let Some(svg) = svg::render_node_without(tree, child.id, &hidden, Some(&mut *typesetter))
        else {
            continue;
        };
        missing_fonts.extend(svg.missing_fonts.iter().cloned());
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
            figma_parts(
                tree,
                typesetter,
                missing_fonts,
                child.id,
                &name,
                matrix,
                depth + 1,
                parts,
            );
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
