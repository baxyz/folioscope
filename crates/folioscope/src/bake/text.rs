// SPDX-License-Identifier: AGPL-3.0-or-later

//! Text laid out from fonts (docs/spec/bake.md), where the design file stores no glyph outlines: parley
//! shapes the characters and breaks the lines, skrifa gives each glyph's outline, drawn as an SVG
//! path like the outlines a `.fig` stores.

use std::collections::HashSet;
use std::fmt::Write as _;
use std::sync::Arc;

use parley::fontique::{Blob, Collection, CollectionOptions, SourceCache};
use parley::{
    Alignment, AlignmentOptions, FontContext, FontFamily, FontStyle, FontWeight, Layout,
    LayoutContext, LineHeight, PositionedLayoutItem, StyleProperty,
};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::raw::types::F2Dot14;
use skrifa::{FontRef, GlyphId, MetadataProvider};

use crate::bake::fonts::FontLibrary;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

/// One paragraph in one style.
#[derive(Debug, Clone)]
pub struct Paragraph<'t> {
    pub text: &'t str,
    pub family: &'t str,
    pub weight: f32,
    pub italic: bool,
    /// Font size, in pixels.
    pub size: f32,
    /// Distance between baselines, in pixels; the font's own when absent.
    pub line_height: Option<f32>,
    /// Extra space between characters, in pixels.
    pub letter_spacing: f32,
    pub align: Align,
    /// Lines break at this width; one line per paragraph when absent.
    pub width: Option<f32>,
}

/// Lays text out with the fonts of a [`FontLibrary`], and nothing else.
pub struct Typesetter {
    library: FontLibrary,
    fonts: FontContext,
    layouts: LayoutContext<()>,
    rich: LayoutContext<usize>,
}

/// A run of a [`Rich`] text in its own style: bytes `range` of the text.
#[derive(Debug, Clone)]
pub struct Span<'t> {
    pub range: std::ops::Range<usize>,
    pub family: &'t str,
    pub weight: f32,
    pub italic: bool,
    pub size: f32,
    pub letter_spacing: f32,
    pub underline: bool,
    pub strikethrough: bool,
}

/// A paragraph whose runs have styles of their own.
#[derive(Debug, Clone)]
pub struct Rich<'t> {
    pub text: &'t str,
    pub spans: Vec<Span<'t>>,
    pub line_height: Option<f32>,
    pub align: Align,
    pub width: Option<f32>,
}

/// A [`Rich`] text laid out: one path per span (glyphs, then underline and strikethrough as
/// rectangles), and the size it takes.
#[derive(Debug, Clone)]
pub struct SetRich {
    pub paths: Vec<String>,
    pub width: f32,
    pub height: f32,
    pub baseline: f32,
    /// The spans whose family no font answers to: they are not drawn.
    pub missing: Vec<usize>,
}

impl Typesetter {
    #[must_use]
    pub fn new(library: FontLibrary) -> Self {
        let mut collection = Collection::new(CollectionOptions {
            shared: false,
            system_fonts: false,
        });
        // Faces of one file share their bytes: register each file once.
        let mut seen = HashSet::new();
        for face in library.faces() {
            if seen.insert(Arc::as_ptr(&face.data)) {
                let bytes: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(face.data.to_vec());
                collection.register_fonts(Blob::new(bytes), None);
            }
        }
        Self {
            library,
            fonts: FontContext {
                collection,
                source_cache: SourceCache::default(),
            },
            layouts: LayoutContext::new(),
            rich: LayoutContext::new(),
        }
    }

    #[must_use]
    pub fn library(&self) -> &FontLibrary {
        &self.library
    }

    /// The paragraph's glyphs as SVG path data, its top-left corner at the origin; `None` when no
    /// font of its family was given.
    #[must_use]
    pub fn path(&mut self, paragraph: &Paragraph<'_>) -> Option<String> {
        self.set(paragraph).map(|text| text.path)
    }

    /// Like [`Typesetter::path`], with the size of the laid-out text.
    #[must_use]
    pub fn set(&mut self, paragraph: &Paragraph<'_>) -> Option<SetText> {
        let face = self
            .library
            .resolve(paragraph.family, paragraph.weight, paragraph.italic)?;
        // The family as the font names itself, so the collection finds it whatever its case.
        let family = face.family.clone();
        let mut builder = self
            .layouts
            .ranged_builder(&mut self.fonts, paragraph.text, 1.0, false);
        builder.push_default(StyleProperty::FontFamily(FontFamily::from(family.as_str())));
        builder.push_default(StyleProperty::FontSize(paragraph.size));
        builder.push_default(StyleProperty::FontWeight(FontWeight::new(paragraph.weight)));
        if paragraph.italic {
            builder.push_default(StyleProperty::FontStyle(FontStyle::Italic));
        }
        if let Some(height) = paragraph.line_height {
            builder.push_default(StyleProperty::LineHeight(LineHeight::Absolute(height)));
        }
        builder.push_default(StyleProperty::LetterSpacing(paragraph.letter_spacing));
        let mut layout: Layout<()> = builder.build(paragraph.text);
        let alignment = match paragraph.align {
            Align::Left => Alignment::Left,
            Align::Center => Alignment::Center,
            Align::Right => Alignment::Right,
            Align::Justify => Alignment::Justify,
        };
        // Applied when the lines are broken, within the width given or the longest line.
        layout.set_alignment(alignment, AlignmentOptions::default());
        layout.break_all_lines(paragraph.width);

        let (width, height) = (layout.width(), layout.height());
        let baseline = layout
            .lines()
            .next()
            .map_or(0.0, |line| line.metrics().baseline);
        let mut d = String::new();
        for line in layout.lines() {
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyphs) = item else {
                    continue;
                };
                let run = glyphs.run();
                let data = run.font();
                let Ok(font) = FontRef::from_index(data.data.as_ref(), data.index) else {
                    continue;
                };
                let outlines = font.outline_glyphs();
                // Where a variable font sits on its axes, for this run's weight.
                let coords: Vec<F2Dot14> = run
                    .normalized_coords()
                    .iter()
                    .map(|c| F2Dot14::from_bits(c.to_bits()))
                    .collect();
                let size = Size::new(run.font_size());
                let location = LocationRef::new(&coords);
                for glyph in glyphs.positioned_glyphs() {
                    let Some(outline) = outlines.get(GlyphId::new(glyph.id)) else {
                        continue;
                    };
                    let mut pen = SvgPen {
                        d: &mut d,
                        x: glyph.x,
                        y: glyph.y,
                    };
                    let _ = outline.draw(DrawSettings::unhinted(size, location), &mut pen);
                }
            }
        }
        Some(SetText {
            path: d,
            width,
            height,
            baseline,
        })
    }
}

impl Typesetter {
    /// Lays out a text whose runs have styles of their own. Runs without a font are left out and
    /// listed in [`SetRich::missing`].
    #[must_use]
    #[allow(clippy::too_many_lines)] // styles in, then one pass over the laid-out runs
    pub fn set_rich(&mut self, rich: &Rich<'_>) -> SetRich {
        let mut missing = Vec::new();
        let families: Vec<Option<String>> = rich
            .spans
            .iter()
            .enumerate()
            .map(|(i, span)| {
                let face = self.library.resolve(span.family, span.weight, span.italic);
                if face.is_none() {
                    missing.push(i);
                }
                face.map(|f| f.family.clone())
            })
            .collect();
        let mut builder = self
            .rich
            .ranged_builder(&mut self.fonts, rich.text, 1.0, false);
        if let Some(height) = rich.line_height {
            builder.push_default(StyleProperty::LineHeight(LineHeight::Absolute(height)));
        }
        // Text no span covers gets an unreachable brush, and is not drawn.
        builder.push_default(StyleProperty::Brush(usize::MAX));
        for (i, span) in rich.spans.iter().enumerate() {
            let range = span.range.clone();
            let family = families[i].as_deref().unwrap_or(span.family);
            builder.push(
                StyleProperty::FontFamily(FontFamily::from(family)),
                range.clone(),
            );
            builder.push(StyleProperty::FontSize(span.size), range.clone());
            builder.push(
                StyleProperty::FontWeight(FontWeight::new(span.weight)),
                range.clone(),
            );
            if span.italic {
                builder.push(StyleProperty::FontStyle(FontStyle::Italic), range.clone());
            }
            builder.push(
                StyleProperty::LetterSpacing(span.letter_spacing),
                range.clone(),
            );
            builder.push(StyleProperty::Underline(span.underline), range.clone());
            builder.push(
                StyleProperty::Strikethrough(span.strikethrough),
                range.clone(),
            );
            builder.push(StyleProperty::Brush(i), range);
        }
        let mut layout: Layout<usize> = builder.build(rich.text);
        let alignment = match rich.align {
            Align::Left => Alignment::Left,
            Align::Center => Alignment::Center,
            Align::Right => Alignment::Right,
            Align::Justify => Alignment::Justify,
        };
        // Applied when the lines are broken, within the width given or the longest line.
        layout.set_alignment(alignment, AlignmentOptions::default());
        layout.break_all_lines(rich.width);
        let mut paths = vec![String::new(); rich.spans.len()];
        let baseline = layout
            .lines()
            .next()
            .map_or(0.0, |line| line.metrics().baseline);
        for line in layout.lines() {
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyphs) = item else {
                    continue;
                };
                let style = glyphs.style();
                let index = style.brush;
                if index >= paths.len() || missing.contains(&index) {
                    continue;
                }
                let run = glyphs.run();
                let size = run.font_size();
                let d = &mut paths[index];
                let data = run.font();
                if let Ok(font) = FontRef::from_index(data.data.as_ref(), data.index) {
                    let outlines = font.outline_glyphs();
                    let coords: Vec<F2Dot14> = run
                        .normalized_coords()
                        .iter()
                        .map(|c| F2Dot14::from_bits(c.to_bits()))
                        .collect();
                    let location = LocationRef::new(&coords);
                    for glyph in glyphs.positioned_glyphs() {
                        let Some(outline) = outlines.get(GlyphId::new(glyph.id)) else {
                            continue;
                        };
                        let mut pen = SvgPen {
                            d,
                            x: glyph.x,
                            y: glyph.y,
                        };
                        let _ = outline
                            .draw(DrawSettings::unhinted(Size::new(size), location), &mut pen);
                    }
                }
                // Decorations, from the font size: below the baseline, and through the x-height.
                let (x, width, y) = (glyphs.offset(), glyphs.advance(), glyphs.baseline());
                let thickness = (size / 14.0).max(1.0);
                if style.underline.is_some() {
                    let top = y + size * 0.1;
                    let _ = write!(d, "M{x} {top}h{width}v{thickness}h{}Z", -width);
                }
                if style.strikethrough.is_some() {
                    let top = y - size * 0.3;
                    let _ = write!(d, "M{x} {top}h{width}v{thickness}h{}Z", -width);
                }
            }
        }
        SetRich {
            paths,
            width: layout.width(),
            height: layout.height(),
            baseline,
            missing,
        }
    }
}

/// Text laid out: its glyphs as SVG path data, and the size they take.
#[derive(Debug, Clone)]
pub struct SetText {
    pub path: String,
    pub width: f32,
    pub height: f32,
    /// Distance from the top of the text to the baseline of its first line.
    pub baseline: f32,
}

/// Writes outline commands as SVG path data, at a glyph's origin, y pointing down.
struct SvgPen<'d> {
    d: &'d mut String,
    x: f32,
    y: f32,
}

impl SvgPen<'_> {
    fn at(&self, x: f32, y: f32) -> (f32, f32) {
        (self.x + x, self.y - y)
    }
}

impl OutlinePen for SvgPen<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.at(x, y);
        let _ = write!(self.d, "M{x} {y}");
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = self.at(x, y);
        let _ = write!(self.d, "L{x} {y}");
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        let (cx0, cy0) = self.at(cx0, cy0);
        let (x, y) = self.at(x, y);
        let _ = write!(self.d, "Q{cx0} {cy0} {x} {y}");
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let (cx0, cy0) = self.at(cx0, cy0);
        let (cx1, cy1) = self.at(cx1, cy1);
        let (x, y) = self.at(x, y);
        let _ = write!(self.d, "C{cx0} {cy0} {cx1} {cy1} {x} {y}");
    }

    fn close(&mut self) {
        self.d.push('Z');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typesetter() -> Typesetter {
        let path = format!(
            "{}/../../fixtures/fonts/Inter-Latin.ttf",
            env!("CARGO_MANIFEST_DIR")
        );
        let mut library = FontLibrary::default();
        library
            .add("Inter-Latin.ttf", &std::fs::read(path).unwrap())
            .unwrap();
        Typesetter::new(library)
    }

    fn paragraph(text: &str, width: Option<f32>) -> Paragraph<'_> {
        Paragraph {
            text,
            family: "Inter",
            weight: 400.0,
            italic: false,
            size: 16.0,
            line_height: Some(24.0),
            letter_spacing: 0.0,
            align: Align::Left,
            width,
        }
    }

    /// The lowest point any command of `d` reaches.
    fn bottom(d: &str) -> f32 {
        d.split(|c: char| c.is_ascii_alphabetic())
            .filter_map(|cmd| {
                let numbers: Vec<f32> = cmd
                    .split_whitespace()
                    .filter_map(|n| n.parse().ok())
                    .collect();
                numbers
                    .chunks(2)
                    .map(|p| p.get(1).copied())
                    .max_by(|a, b| a.partial_cmp(b).unwrap())?
            })
            .fold(0.0, f32::max)
    }

    #[test]
    fn draws_text_and_breaks_lines_in_the_box() {
        let mut typesetter = typesetter();
        let one_line = typesetter
            .path(&paragraph("Lunch and dinner", None))
            .unwrap();
        assert!(one_line.contains('M') && one_line.ends_with('Z'));
        let wrapped = typesetter
            .path(&paragraph("Lunch and dinner", Some(60.0)))
            .unwrap();
        // Three lines of 24 px instead of one.
        assert!(bottom(&wrapped) > bottom(&one_line) + 40.0);
    }

    #[test]
    fn rich_text_draws_each_span_with_its_decorations() {
        let mut typesetter = typesetter();
        let span = |range, weight, underline| Span {
            range,
            family: "Inter",
            weight,
            italic: false,
            size: 16.0,
            letter_spacing: 0.0,
            underline,
            strikethrough: false,
        };
        let set = typesetter.set_rich(&Rich {
            text: "Lunch dinner",
            spans: vec![
                span(0..6, 400.0, false),
                span(6..12, 700.0, true),
                Span {
                    family: "Unknown",
                    ..span(12..12, 400.0, false)
                },
            ],
            line_height: Some(24.0),
            align: Align::Left,
            width: None,
        });
        assert_eq!(set.paths.len(), 3);
        assert!(set.paths[0].contains('M') && set.paths[1].contains('M'));
        // The underline: a rectangle as wide as the run, below its baseline.
        assert!(set.paths[1].contains('h') && !set.paths[0].contains('h'));
        assert_eq!(set.missing, [2]);
    }

    #[test]
    fn bolder_text_is_wider_and_unknown_families_are_reported() {
        let mut typesetter = typesetter();
        let regular = typesetter.path(&paragraph("Mains", None)).unwrap();
        let bold = typesetter
            .path(&Paragraph {
                weight: 800.0,
                ..paragraph("Mains", None)
            })
            .unwrap();
        assert_ne!(regular, bold);
        assert!(
            typesetter
                .path(&Paragraph {
                    family: "Roboto",
                    ..paragraph("Mains", None)
                })
                .is_none()
        );
    }
}
