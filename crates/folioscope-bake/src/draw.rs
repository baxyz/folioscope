// SPDX-License-Identifier: AGPL-3.0-or-later

//! How a design file's frames become SVG: one [`Draw`] per format, so the bake itself
//! (`folio`) only knows the IR.

use std::collections::BTreeSet;

use folioscope_core::figma::tree::{NodeId, Tree};
use folioscope_core::ir::{self, Rect};

use crate::folio::BakeError;
use crate::svg;
use crate::text::Typesetter;

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
        let svg =
            svg::render_component_state(self.tree, &path, variant, Some(&mut self.typesetter))?;
        self.missing_fonts.extend(svg.missing_fonts);
        Some(svg.markup)
    }

    fn missing_fonts(&mut self) -> BTreeSet<String> {
        std::mem::take(&mut self.missing_fonts)
    }
}
