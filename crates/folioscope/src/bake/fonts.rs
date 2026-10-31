// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fonts given to the bake (ADR 0011): read for their family, weight and style, and matched to
//! what a text asks for. Never the system's fonts, so a bake gives the same result everywhere.

use std::sync::Arc;

use helpers4::font::{FontFace, FontRequest, FontStyle, is_italic, pick_closest, split_postscript};
use helpers4::string::match_glob_ignore_case;
use serde::{Deserialize, Serialize};
use skrifa::raw::FileRef;
use skrifa::{FontRef, MetadataProvider, Tag, attribute::Style, string::StringId};

#[derive(Debug, thiserror::Error)]
pub enum FontError {
    #[error("{0}: not a WOFF, WOFF2, TrueType or OpenType font")]
    Format(String),
}

/// One face of a font file: a single font, or one of a collection.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Face {
    /// Typographic family (`name` ID 16), else the family (ID 1).
    pub family: String,
    /// PostScript name (ID 6), as Sketch names fonts (`Inter-Bold`); empty when absent.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub postscript: String,
    /// Weights it covers: one, or the range of a variable font's `wght` axis.
    pub weight: [f32; 2],
    pub italic: bool,
    /// Where it came from: a path, or a name the caller gave.
    pub source: String,
    /// Index in a collection, 0 otherwise.
    #[serde(skip_serializing_if = "is_zero")]
    pub index: u32,
    /// Other family names this face answers to, case-insensitively: a name, or a pattern with `*`
    /// (any run of characters) and `?` (one character), as `SF Pro*`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// The font as TrueType or OpenType (WOFF and WOFF2 decoded).
    #[serde(skip)]
    pub data: Arc<Vec<u8>>,
}

/// What to say about a font file instead of what it says itself (`<folioscope-font family="…">`).
#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
pub struct FontHints {
    /// A family name the font also answers to, besides its own.
    #[serde(default)]
    pub family: Option<String>,
    /// Family names or patterns it also answers to: it substitutes for them.
    #[serde(default, rename = "match")]
    pub matches: Vec<String>,
    /// The weight it is, instead of the one in the file.
    #[serde(default)]
    pub weight: Option<f32>,
    /// Whether it is italic, instead of what the file says.
    #[serde(default)]
    pub italic: Option<bool>,
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde's signature
fn is_zero(index: &u32) -> bool {
    *index == 0
}

impl Face {
    #[must_use]
    pub fn font(&self) -> Option<FontRef<'_>> {
        FontRef::from_index(&self.data, self.index).ok()
    }

    /// Whether the face is the one for `family`: its own family, or one of its aliases.
    fn answers(&self, family: &str) -> bool {
        self.family.eq_ignore_ascii_case(family)
            || self
                .aliases
                .iter()
                .any(|alias| match_glob_ignore_case(alias, family))
    }

    /// The face as helpers4 matches it.
    fn css(&self) -> FontFace {
        let style = if self.italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        };
        FontFace::variable(self.weight[0].into(), self.weight[1].into(), style)
    }
}

/// The face among `faces` that CSS font matching picks for a weight and a style.
fn closest<'a>(faces: &[&'a Face], weight: f32, italic: bool) -> Option<&'a Face> {
    let css: Vec<FontFace> = faces.iter().map(|face| face.css()).collect();
    let style = if italic {
        FontStyle::Italic
    } else {
        FontStyle::Normal
    };
    pick_closest(&css, FontRequest::new(weight.into(), style)).map(|index| faces[index])
}

/// The fonts a bake may use.
#[derive(Debug, Default, Clone)]
pub struct FontLibrary {
    faces: Vec<Face>,
}

impl FontLibrary {
    /// Adds every face of a font file; returns how many.
    ///
    /// # Errors
    ///
    /// Fails when `bytes` are not a font this library reads.
    pub fn add(&mut self, source: &str, bytes: &[u8]) -> Result<usize, FontError> {
        self.add_with(source, bytes, &FontHints::default())
    }

    /// Like [`FontLibrary::add`], with what the caller says about the font: the family names it
    /// answers to, and its weight and style when the file says otherwise or nothing.
    ///
    /// # Errors
    ///
    /// Fails when `bytes` are not a font this library reads.
    pub fn add_with(
        &mut self,
        source: &str,
        bytes: &[u8],
        hints: &FontHints,
    ) -> Result<usize, FontError> {
        let error = || FontError::Format(source.to_owned());
        let data = Arc::new(match bytes.get(..4) {
            Some(b"wOFF") => wuff::decompress_woff1(bytes).map_err(|_| error())?,
            Some(b"wOF2") => wuff::decompress_woff2(bytes).map_err(|_| error())?,
            _ => bytes.to_vec(),
        });
        let fonts: Vec<FontRef<'_>> = match FileRef::new(&data).map_err(|_| error())? {
            FileRef::Font(font) => vec![font],
            FileRef::Collection(collection) => collection.iter().filter_map(Result::ok).collect(),
        };
        let before = self.faces.len();
        for (index, font) in (0u32..).zip(fonts) {
            let Some(family) = family(&font) else {
                continue;
            };
            let attributes = font.attributes();
            let weight = font
                .axes()
                .get_by_tag(Tag::new(b"wght"))
                .map_or([attributes.weight.value(); 2], |axis| {
                    [axis.min_value(), axis.max_value()]
                });
            let postscript = font
                .localized_strings(StringId::POSTSCRIPT_NAME)
                .english_or_first()
                .map(|name| name.to_string())
                .unwrap_or_default();
            self.faces.push(Face {
                family,
                postscript,
                weight: hints.weight.map_or(weight, |w| [w, w]),
                italic: hints
                    .italic
                    .unwrap_or(!matches!(attributes.style, Style::Normal)),
                source: source.to_owned(),
                index,
                aliases: hints.family.iter().chain(&hints.matches).cloned().collect(),
                data: Arc::clone(&data),
            });
        }
        if self.faces.len() == before {
            return Err(error());
        }
        Ok(self.faces.len() - before)
    }

    #[must_use]
    pub fn faces(&self) -> &[Face] {
        &self.faces
    }

    /// The face for `family` (case-insensitive) closest to `weight` and `italic`, as CSS font
    /// matching picks it: the style first, then the weight.
    #[must_use]
    pub fn resolve(&self, family: &str, weight: f32, italic: bool) -> Option<&Face> {
        let candidates: Vec<&Face> = self.faces.iter().filter(|f| f.answers(family)).collect();
        closest(&candidates, weight, italic)
    }
}

impl FontLibrary {
    /// The face a PostScript name designates (`NeutrifPro-SemiBold`): the face of that name,
    /// else the closest face of the family before the dash (spaces ignored), at the weight and
    /// style its suffix says. A variable font's instances have no name of their own.
    #[must_use]
    pub fn resolve_postscript(&self, name: &str) -> Option<&Face> {
        if let Some(face) = self
            .faces
            .iter()
            .find(|f| f.postscript.eq_ignore_ascii_case(name))
        {
            return Some(face);
        }
        let (family_name, style) = split_postscript(name);
        let squeeze = |s: &str| s.replace(' ', "").to_ascii_lowercase();
        let family = squeeze(family_name);
        let candidates: Vec<&Face> = self
            .faces
            .iter()
            .filter(|f| {
                // PostScript names have no spaces: compare the aliases without theirs too.
                squeeze(&f.family) == family
                    || f.aliases
                        .iter()
                        .any(|alias| match_glob_ignore_case(&squeeze(alias), &family))
            })
            .collect();
        closest(&candidates, weight_of_style(style), is_italic(style))
    }
}

fn family(font: &FontRef<'_>) -> Option<String> {
    [StringId::TYPOGRAPHIC_FAMILY_NAME, StringId::FAMILY_NAME]
        .into_iter()
        .find_map(|id| font.localized_strings(id).english_or_first())
        .map(|name| name.to_string())
}

/// Weight of a style name as design tools write it ("Semi Bold Italic" → 600).
#[must_use]
#[allow(clippy::cast_possible_truncation)] // 100 to 950
pub fn weight_of_style(style: &str) -> f32 {
    helpers4::font::weight_of_style(style) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inter(ext: &str) -> Vec<u8> {
        let path = format!(
            "{}/../../fixtures/fonts/Inter-Latin.{ext}",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
    }

    #[test]
    fn reads_a_variable_font_and_its_woff() {
        let mut library = FontLibrary::default();
        assert_eq!(library.add("Inter-Latin.ttf", &inter("ttf")).unwrap(), 1);
        assert_eq!(library.add("Inter-Latin.woff", &inter("woff")).unwrap(), 1);
        for face in library.faces() {
            assert_eq!(face.family, "Inter");
            assert_eq!(face.weight, [100.0, 900.0]);
            assert!(!face.italic);
        }
        assert!(library.add("not.ttf", b"nope").is_err());
    }

    #[test]
    fn matches_family_style_then_weight() {
        let mut library = FontLibrary::default();
        library.add("Inter-Latin.ttf", &inter("ttf")).unwrap();
        assert!(library.resolve("inter", 650.0, false).is_some());
        // No italic face: the upright one stands in, as in CSS.
        assert!(library.resolve("Inter", 400.0, true).is_some());
        assert!(library.resolve("Roboto", 400.0, false).is_none());
    }

    #[test]
    fn finds_a_face_by_postscript_name() {
        let mut library = FontLibrary::default();
        library.add("Inter-Latin.ttf", &inter("ttf")).unwrap();
        // The variable font is named for its default instance; a bold name falls back to it.
        assert!(library.resolve_postscript("Inter-Bold").is_some());
        assert!(library.resolve_postscript("Montserrat-Bold").is_none());
    }

    #[test]
    fn a_font_answers_to_the_names_it_is_given() {
        let mut library = FontLibrary::default();
        let hints = FontHints {
            family: Some("Brand".into()),
            matches: vec!["SF Pro*".into()],
            weight: Some(500.0),
            italic: Some(true),
        };
        library
            .add_with("Inter-Latin.ttf", &inter("ttf"), &hints)
            .unwrap();
        for name in ["Inter", "brand", "SF Pro Display"] {
            assert!(library.resolve(name, 400.0, false).is_some(), "{name}");
        }
        assert!(library.resolve("Helvetica", 400.0, false).is_none());
        // Substitutes by name, for Sketch's PostScript names too.
        assert!(library.resolve_postscript("Brand-Bold").is_some());
        assert!(library.resolve_postscript("SFProText-Regular").is_some());
        assert!(library.resolve_postscript("Helvetica-Bold").is_none());
        let face = &library.faces()[0];
        assert_eq!((face.weight, face.italic), ([500.0, 500.0], true));
    }

    #[test]
    fn weights_from_style_names() {
        assert_eq!(weight_of_style("Semi Bold Italic"), 600.0);
        assert_eq!(weight_of_style("ExtraBold"), 800.0);
        assert_eq!(weight_of_style("Regular"), 400.0);
        assert_eq!(weight_of_style("Light"), 300.0);
    }
}
