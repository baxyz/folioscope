// SPDX-License-Identifier: AGPL-3.0-or-later

use folioscope::bake::folio::{BakeOptions, bake_fig};
use folioscope::figma::decode;

#[test]
fn navigation_bakes_to_a_folio_folder() {
    let input = format!(
        "{}/../../fixtures/figma/navigation.fig",
        env!("CARGO_MANIFEST_DIR")
    );
    let out = std::env::temp_dir().join(format!("folioscope-bake-{}", std::process::id()));
    let file = decode(&std::fs::read(input).unwrap()).unwrap();

    let report = bake_fig(&file, &out, &BakeOptions::default()).unwrap();
    assert_eq!((report.frames, report.layers), (2, 4));

    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(out.join("index.json")).unwrap()).unwrap();
    assert_eq!(index["folio"], 1);
    assert_eq!(index["irVersion"], folioscope::IR_VERSION);
    let home = &index["pages"][0]["frames"][0];
    assert_eq!(home["name"], "Home");
    let layer = home["layers"][0]["src"].as_str().unwrap();
    // A flat screen stays lossless WebP: AVIF is only tried on large, photo-like layers.
    assert_eq!(home["layers"][0]["format"], "webp");
    for density in [1, 2] {
        assert!(
            out.join(format!("layers/{layer}@{density}x.webp"))
                .is_file()
        );
    }
    assert!(out.join("poster.webp").is_file());

    // Layers are named by content: baking again changes nothing.
    let again = out.join("again");
    bake_fig(&file, &again, &BakeOptions::default()).unwrap();
    assert_eq!(
        std::fs::read(out.join("index.json")).unwrap(),
        std::fs::read(again.join("index.json")).unwrap()
    );
    std::fs::remove_dir_all(&out).unwrap();
}

/// A `.sketch` stores no glyphs: its text is drawn with the fonts given, matched by name.
#[test]
fn sketch_text_needs_its_fonts() {
    use folioscope::bake::folio::bake_file_into;
    use folioscope::bake::fonts::{FontHints, FontLibrary};

    let root = env!("CARGO_MANIFEST_DIR");
    let sketch = std::fs::read(format!("{root}/../../fixtures/sketch/text.sketch")).unwrap();
    let inter = std::fs::read(format!("{root}/../../fixtures/fonts/Inter-Latin.ttf")).unwrap();

    // Bakes with `hints` for the one font, and returns the missing fonts and the first layer.
    let bake = |hints: Option<FontHints>| {
        let mut fonts = FontLibrary::default();
        if let Some(hints) = hints {
            fonts.add_with("Inter-Latin.ttf", &inter, &hints).unwrap();
        }
        let options = BakeOptions {
            fonts,
            ..BakeOptions::default()
        };
        let mut layer = Vec::new();
        let report = bake_file_into(&sketch, &options, &mut |path, bytes| {
            if path.ends_with("@1x.webp") {
                layer = bytes.to_vec();
            }
            Ok(())
        })
        .unwrap();
        (report.missing_fonts.into_iter().collect::<Vec<_>>(), layer)
    };

    let (nothing, blank) = bake(None);
    assert_eq!(nothing, ["Brand-Bold", "Inter-Regular"]);

    // Inter alone gives the first text; the brand font stays missing.
    let (brand_missing, with_inter) = bake(Some(FontHints::default()));
    assert_eq!(brand_missing, ["Brand-Bold"]);
    assert_ne!(with_inter, blank, "the text is drawn");

    // Inter standing in for the brand font, by name, or by pattern: nothing is missing, and
    // there is more text to draw.
    for hints in [
        FontHints {
            family: Some("Brand".into()),
            ..FontHints::default()
        },
        FontHints {
            matches: vec!["Br?nd*".into()],
            ..FontHints::default()
        },
    ] {
        let (none, both) = bake(Some(hints));
        assert!(none.is_empty(), "{none:?}");
        assert_ne!(both, with_inter, "the second text is drawn too");
    }
}

/// An `.xd` stores no glyphs either: its text is laid out where XD put it, with the fonts given.
#[test]
fn xd_text_needs_its_fonts() {
    use folioscope::bake::folio::bake_file_into;
    use folioscope::bake::fonts::FontLibrary;

    let root = env!("CARGO_MANIFEST_DIR");
    let xd = std::fs::read(format!("{root}/../../fixtures/xd/text.xd")).unwrap();
    let inter = std::fs::read(format!("{root}/../../fixtures/fonts/Inter-Latin.ttf")).unwrap();

    let bake = |fonts: FontLibrary| {
        let options = BakeOptions {
            fonts,
            ..BakeOptions::default()
        };
        let mut layer = Vec::new();
        let report = bake_file_into(&xd, &options, &mut |path, bytes| {
            if path.ends_with("@1x.webp") {
                layer = bytes.to_vec();
            }
            Ok(())
        })
        .unwrap();
        (report.missing_fonts.into_iter().collect::<Vec<_>>(), layer)
    };

    let (missing, blank) = bake(FontLibrary::default());
    assert_eq!(missing, ["Inter Regular"]);

    let mut fonts = FontLibrary::default();
    fonts.add("Inter-Latin.ttf", &inter).unwrap();
    let (missing, drawn) = bake(fonts);
    assert!(missing.is_empty(), "{missing:?}");
    assert_ne!(drawn, blank, "the text is drawn");
}

/// Draws every frame as a flat white rectangle, to test the bake without a design file.
mod flat {
    use folioscope::bake::draw::Draw;
    use folioscope::bake::folio::BakeError;
    use folioscope::ir::{
        Component, ComponentState, Document, Flow, Frame, Page, Rect, SourceFormat,
    };
    use std::collections::BTreeSet;

    pub struct Flat;

    impl Draw for Flat {
        fn frame(&mut self, frame: &Frame) -> Option<String> {
            let (w, h) = (frame.bounds.width, frame.bounds.height);
            Some(format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}"><rect width="{w}" height="{h}" fill="#fff"/></svg>"##
            ))
        }

        fn layers(
            &mut self,
            frame: &Frame,
            whole: &str,
        ) -> Result<Vec<(String, Rect, bool)>, BakeError> {
            Ok(vec![(whole.to_owned(), frame.bounds, false)])
        }

        fn state(&mut self, _: &Component, _: &ComponentState) -> Option<String> {
            None
        }

        fn missing_fonts(&mut self) -> BTreeSet<String> {
            BTreeSet::new()
        }
    }

    pub fn frame(name: &str, width: f32) -> Frame {
        Frame {
            id: name.to_owned(),
            name: name.to_owned(),
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                width,
                height: 10.0,
            },
            hotspots: Vec::new(),
            overlay: None,
            scroll: None,
            components: Vec::new(),
        }
    }

    pub fn page(name: &str, start: Option<&str>, frames: Vec<Frame>) -> Page {
        Page {
            id: name.to_owned(),
            name: name.to_owned(),
            flows: start
                .map(|s| Flow {
                    name: "Flow".to_owned(),
                    description: String::new(),
                    start: s.to_owned(),
                })
                .into_iter()
                .collect(),
            frames,
        }
    }

    pub fn document(pages: Vec<Page>) -> Document {
        Document {
            source: SourceFormat::Fig,
            pages,
        }
    }
}

fn bake_flat(
    document: &folioscope::ir::Document,
    options: &BakeOptions,
) -> (folioscope::bake::folio::BakeReport, serde_json::Value) {
    let mut index = Vec::new();
    let report = folioscope::bake::folio::bake_into(
        document,
        &mut flat::Flat,
        options,
        &mut |path, bytes| {
            if path == "index.json" {
                index = bytes.to_vec();
            }
            Ok(())
        },
    )
    .unwrap();
    (report, serde_json::from_slice(&index).unwrap())
}

/// A frame too wide to be an image (a strip of icons, 28,000 px wide) is left out and reported:
/// it must not fail the bake of everything else.
#[test]
fn a_frame_too_large_for_an_image_is_skipped() {
    use flat::{document, frame, page};
    let document = document(vec![page(
        "Page",
        None,
        vec![frame("Strip", 28_000.0), frame("Small", 100.0)],
    )]);
    let (report, _) = bake_flat(&document, &BakeOptions::default());
    assert_eq!(report.skipped_frames, ["Strip"]);
    assert_eq!(report.frames, 1);
}

/// With a flow somewhere, the pages without one (a component library) are not part of the
/// prototype; `all_pages` keeps them, and a file with no flow at all keeps everything.
#[test]
fn pages_without_a_flow_are_left_out() {
    use flat::{document, frame, page};
    let pages = || {
        vec![
            page("Prototype", Some("Home"), vec![frame("Home", 100.0)]),
            page("Library", None, vec![frame("Button", 100.0)]),
            page("Empty", None, Vec::new()),
        ]
    };
    let (report, index) = bake_flat(&document(pages()), &BakeOptions::default());
    assert_eq!(report.skipped_pages, ["Library"]);
    assert_eq!(index["pages"].as_array().unwrap().len(), 1);

    let all = BakeOptions {
        all_pages: true,
        ..BakeOptions::default()
    };
    let (report, index) = bake_flat(&document(pages()), &all);
    assert_eq!(report.skipped_pages, Vec::<String>::new());
    assert_eq!(index["pages"].as_array().unwrap().len(), 3);

    let no_flow = document(vec![page("A", None, vec![frame("A", 100.0)])]);
    let (report, index) = bake_flat(&no_flow, &BakeOptions::default());
    assert_eq!(report.skipped_pages, Vec::<String>::new());
    assert_eq!(index["pages"].as_array().unwrap().len(), 1);
}
