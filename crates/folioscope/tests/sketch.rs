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
