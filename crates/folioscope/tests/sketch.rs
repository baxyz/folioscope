// SPDX-License-Identifier: AGPL-3.0-or-later

use folioscope::sketch::{decode, summarize};

fn fixture() -> Vec<u8> {
    let path = format!(
        "{}/../../fixtures/sketch/navigation.sketch",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn navigation_lists_artboards_start_and_links() {
    let file = decode(&fixture()).unwrap();
    insta::assert_snapshot!(summarize(&file).to_string());
}

/// The IR of the generated fixture, as the JSON baked into a `.folio`.
#[test]
fn generated_fixture_converts_to_ir() {
    let file = decode(&fixture()).unwrap();
    let ir = folioscope::sketch::ir::to_ir(&file);
    insta::assert_snapshot!("ir-navigation", serde_json::to_string_pretty(&ir).unwrap());
}

#[cfg(feature = "bake")]
mod features {
    //! `fixtures/sketch/features.sketch`: every Sketch scenario, one test per feature.
    use folioscope::bake::draw::Draw;
    use folioscope::bake::fonts::FontLibrary;
    use folioscope::bake::sketch::SketchDraw;
    use folioscope::bake::text::Typesetter;
    use folioscope::ir::{Action, Document, Frame, TransitionKind};
    use folioscope::sketch::{SketchFile, decode, ir::to_ir};

    fn file() -> SketchFile {
        let path = format!(
            "{}/../../fixtures/sketch/features.sketch",
            env!("CARGO_MANIFEST_DIR")
        );
        decode(&std::fs::read(path).unwrap()).unwrap()
    }

    fn frame<'d>(ir: &'d Document, name: &str) -> &'d Frame {
        ir.pages
            .iter()
            .flat_map(|p| &p.frames)
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("no frame {name}"))
    }

    fn svg(name: &str) -> String {
        let file = file();
        let ir = to_ir(&file);
        let mut draw = SketchDraw::new(&file, Typesetter::new(FontLibrary::default()));
        draw.frame(frame(&ir, name)).unwrap()
    }

    #[test]
    fn sketch_flows_and_pages() {
        let ir = to_ir(&file());
        let pages: Vec<_> = ir.pages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(pages, ["Prototype", "Second page"]);
        let starts: Vec<_> = ir
            .pages
            .iter()
            .flat_map(|p| &p.flows)
            .map(|f| frame(&ir, "Home").id == f.start || frame(&ir, "About").id == f.start)
            .collect();
        assert_eq!(starts, [true, true]);
        let home = frame(&ir, "Home");
        let open = &home.hotspots[0].interactions[0].action;
        assert!(matches!(
            open,
            Action::Navigate { transition, .. } if matches!(transition.kind, TransitionKind::SlideIn { .. })
        ));
        assert!(matches!(
            frame(&ir, "Details").hotspots[0].interactions[0].action,
            Action::Back
        ));
    }

    #[test]
    fn sketch_draws_shapes() {
        let svg = svg("Home");
        assert!(
            svg.contains("<image") && svg.contains("data:image/png;base64,"),
            "bitmap"
        );
        assert!(svg.contains("clip-path") || svg.contains("<mask"), "mask");
        assert!(
            svg.contains("feDropShadow") || svg.contains("feGaussianBlur"),
            "shadow"
        );
        folioscope::bake::rasterize(&svg, 1.0).unwrap();
    }

    #[test]
    fn sketch_gradients() {
        let svg = svg("Home");
        assert!(svg.contains("<linearGradient"), "linear");
        assert!(svg.contains("<radialGradient"), "radial");
    }
}

#[cfg(feature = "bake")]
mod effects {
    //! The effects of `fixtures/sketch/features.sketch`, drawn with Inter.
    use folioscope::bake::draw::Draw;
    use folioscope::bake::fonts::FontLibrary;
    use folioscope::bake::sketch::SketchDraw;
    use folioscope::bake::text::Typesetter;
    use folioscope::sketch::{decode, ir::to_ir};

    fn svg() -> String {
        let root = format!("{}/../../fixtures", env!("CARGO_MANIFEST_DIR"));
        let file =
            decode(&std::fs::read(format!("{root}/sketch/features.sketch")).unwrap()).unwrap();
        let ir = to_ir(&file);
        let mut fonts = FontLibrary::default();
        let inter = std::fs::read(format!("{root}/fonts/Inter-Latin.ttf")).unwrap();
        fonts.add("Inter-Latin.ttf", &inter).unwrap();
        let mut draw = SketchDraw::new(&file, Typesetter::new(fonts));
        let svg = draw.frame(&ir.pages[0].frames[0]).unwrap();
        folioscope::bake::rasterize(&svg, 1.0).unwrap();
        svg
    }

    #[test]
    fn sketch_effects() {
        let svg = svg();
        assert!(
            svg.contains(r#"operator="arithmetic" k1="-1""#),
            "inner shadow"
        );
        assert!(
            svg.contains("<feGaussianBlur stdDeviation=\"4\"/>"),
            "layer blur"
        );
        assert!(
            svg.contains("frame\"><") && svg.contains("<use href"),
            "background blur"
        );
        assert!(svg.contains("mix-blend-mode:multiply"), "blend mode");
    }

    #[test]
    fn sketch_borders_and_masks() {
        let svg = svg();
        // Inside and outside borders: twice as wide, clipped.
        assert!(svg.contains(r#"stroke-width="8""#), "aligned borders");
        assert!(svg.contains(r#"mask-type="alpha""#), "alpha mask");
        // The angular gradient is a pattern of thin wedges.
        assert!(svg.contains("<pattern"), "angular gradient");
    }

    #[test]
    fn sketch_tiles() {
        // The 8 px checker tiles at its own size.
        assert!(svg().contains(r#"width="8" height="8" preserveAspectRatio="none""#));
    }

    #[test]
    fn sketch_rich_text() {
        let svg = svg();
        // Three runs, three colours; the underline and strikethrough are drawn.
        for color in ["rgb(17 24 39)", "rgb(37 99 235)", "rgb(107 114 128)"] {
            assert!(svg.contains(color), "{color}");
        }
    }
}
