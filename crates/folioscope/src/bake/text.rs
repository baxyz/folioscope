// SPDX-License-Identifier: AGPL-3.0-or-later

//! Text laid out from fonts (ADR 0011), where the design file stores no glyph outlines: parley
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
        layout.break_all_lines(paragraph.width);
        let alignment = match paragraph.align {
            Align::Left => Alignment::Left,
            Align::Center => Alignment::Center,
            Align::Right => Alignment::Right,
            Align::Justify => Alignment::Justify,
        };
        layout.align(alignment, AlignmentOptions::default());

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
