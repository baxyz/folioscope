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
