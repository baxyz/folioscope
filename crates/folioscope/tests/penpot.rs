// SPDX-License-Identifier: AGPL-3.0-or-later

use folioscope::penpot::{decode, summarize};

fn fixture() -> Vec<u8> {
    let path = format!(
        "{}/../../fixtures/penpot/navigation.penpot",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn navigation_lists_boards_flows_and_links() {
    let file = decode(&fixture()).unwrap();
    insta::assert_snapshot!(summarize(&file).to_string());
}

/// The IR of the generated fixture, as the JSON baked into a `.folio`.
#[test]
fn generated_fixture_converts_to_ir() {
    let file = decode(&fixture()).unwrap();
    let ir = folioscope::penpot::ir::to_ir(&file);
    insta::assert_snapshot!("ir-navigation", serde_json::to_string_pretty(&ir).unwrap());
}

mod features {
    //! `fixtures/penpot/features.penpot`: every Penpot scenario, one test per feature.
    use folioscope::bake::draw::Draw;
    use folioscope::bake::fonts::FontLibrary;
    use folioscope::bake::penpot::PenpotDraw;
    use folioscope::bake::text::Typesetter;
    use folioscope::ir::{Action, Document, Frame, OverlayPosition, Trigger};
    use folioscope::penpot::{PenpotFile, decode, ir::to_ir};

    fn root() -> String {
        format!("{}/../../fixtures", env!("CARGO_MANIFEST_DIR"))
    }

    fn file() -> PenpotFile {
        decode(&std::fs::read(format!("{}/penpot/features.penpot", root())).unwrap()).unwrap()
    }

    fn frame<'d>(ir: &'d Document, name: &str) -> &'d Frame {
        ir.pages
            .iter()
            .flat_map(|p| &p.frames)
            .find(|f| f.name == name)
            .unwrap_or_else(|| panic!("no frame {name}"))
    }

    /// The SVG of a frame, with Inter given for its text.
    fn svg(name: &str) -> String {
        let file = file();
        let ir = to_ir(&file);
        let mut fonts = FontLibrary::default();
        let inter = std::fs::read(format!("{}/fonts/Inter-Latin.ttf", root())).unwrap();
        fonts.add("Inter-Latin.ttf", &inter).unwrap();
        let mut draw = PenpotDraw::new(&file, Typesetter::new(fonts));
        let markup = draw.frame(frame(&ir, name)).unwrap();
        assert!(
            draw.missing_fonts().is_empty(),
            "{:?}",
            draw.missing_fonts()
        );
        markup
    }

    /// The interactions of a hotspot, by its node's name.
    fn on<'d>(frame: &'d Frame, name: &str) -> Vec<&'d folioscope::ir::Interaction> {
        frame
            .hotspots
            .iter()
            .filter(|h| h.name == name)
            .flat_map(|h| &h.interactions)
            .collect()
    }

    #[test]
    fn penpot_flows_and_pages() {
        let ir = to_ir(&file());
        let pages: Vec<_> = ir.pages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(pages, ["Prototype", "Second page"]);
        let flows: Vec<_> = ir
            .pages
            .iter()
            .flat_map(|p| &p.flows)
            .map(|f| f.name.as_str())
            .collect();
        assert_eq!(flows, ["Main flow", "Skip the splash", "About flow"]);
    }

    #[test]
    fn penpot_triggers() {
        let ir = to_ir(&file());
        let splash = on(frame(&ir, "Splash"), "Splash");
        assert_eq!(splash[0].trigger, Trigger::AfterDelay { delay_ms: 1500 });
        let home = frame(&ir, "Home");
        assert_eq!(on(home, "Hold me")[0].trigger, Trigger::Press);
        let hover = on(home, "Hover me");
        assert_eq!(hover[0].trigger, Trigger::MouseEnter);
        assert_eq!(hover[1].trigger, Trigger::MouseLeave);
        assert!(matches!(hover[1].action, Action::Close));
        assert!(matches!(
            &on(home, "Website")[0].action,
            Action::OpenUrl { url, .. } if url == "https://folioscope.baxyz.dev"
        ));
    }

    #[test]
    fn penpot_overlays() {
        let ir = to_ir(&file());
        let home = frame(&ir, "Home");
        assert!(matches!(
            on(home, "Menu button")[0].action,
            Action::Overlay { toggle: true, .. }
        ));
        let menu = frame(&ir, "Menu").overlay.as_ref().unwrap();
        assert_eq!(menu.position, OverlayPosition::Center);
        assert!(menu.close_on_click_outside);
        assert!(menu.backdrop.is_some());
        let tip = frame(&ir, "Tip").overlay.as_ref().unwrap();
        assert_eq!(tip.position, OverlayPosition::Manual);
        assert!(matches!(
            on(home, "Hover me")[0].action,
            Action::Overlay {
                offset: Some(_),
                ..
            }
        ));
        assert!(matches!(
            on(frame(&ir, "Menu"), "Close")[0].action,
            Action::Close
        ));
    }

    #[test]
    fn penpot_draws_shapes() {
        let svg = svg("Home");
        // The path, the image, the masked group, the dashed stroke and the drop shadow.
        assert!(svg.contains("L547 328"), "path");
        assert!(
            svg.contains("<image") && svg.contains("data:image/png;base64,"),
            "image"
        );
        assert!(svg.contains("<clipPath") || svg.contains("<mask"), "mask");
        assert!(svg.contains("stroke-dasharray"), "dashed stroke");
        assert!(
            svg.contains("feDropShadow") || svg.contains("feGaussianBlur"),
            "shadow"
        );
        // And it renders.
        folioscope::bake::rasterize(&svg, 1.0).unwrap();
    }

    #[test]
    fn penpot_gradients() {
        let svg = svg("Home");
        assert!(svg.contains("<linearGradient"), "linear");
        assert!(svg.contains("<radialGradient"), "radial");
        assert!(
            svg.contains("rgb(37 99 235)") || svg.contains("#2563EB"),
            "a stop colour"
        );
    }

    #[test]
    fn penpot_text() {
        let svg = svg("Home");
        // Two runs, two colours, both laid out with Inter (one family quoted).
        assert!(
            svg.contains("fill=\"rgb(17 24 39)\"") || svg.contains("#111827"),
            "first run"
        );
        assert!(
            svg.contains("fill=\"rgb(37 99 235)\"") || svg.contains("#2563EB"),
            "second run"
        );
    }
}
