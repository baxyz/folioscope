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

#[cfg(feature = "bake")]
mod symbols {
    //! `fixtures/sketch/symbols.sketch`: instances of a button (its label overridden, one
    //! stretched), a card holding one (overridden through the card, then swapped for a dark
    //! button), and a back button whose link is in its master. Checked on the pixels drawn.
    use folioscope::bake::draw::Draw;
    use folioscope::bake::fonts::FontLibrary;
    use folioscope::bake::render_pixmap;
    use folioscope::bake::sketch::SketchDraw;
    use folioscope::bake::text::Typesetter;
    use folioscope::ir::{Action, Rect, TransitionKind};
    use folioscope::sketch::{SketchFile, decode, ir::to_ir};
    use serde_json::Value;

    const BLUE: [u8; 3] = [37, 99, 235];
    const INK: [u8; 3] = [17, 24, 39];
    const WHITE: [u8; 3] = [255, 255, 255];

    fn root() -> String {
        format!("{}/../../fixtures", env!("CARGO_MANIFEST_DIR"))
    }

    fn file() -> SketchFile {
        decode(&std::fs::read(format!("{}/sketch/symbols.sketch", root())).unwrap()).unwrap()
    }

    /// The colour of each pixel asked for, on the Home artboard drawn with Inter.
    fn pixels(file: &SketchFile, at: &[(u32, u32)]) -> Vec<[u8; 3]> {
        let mut fonts = FontLibrary::default();
        let inter = std::fs::read(format!("{}/fonts/Inter-Latin.ttf", root())).unwrap();
        fonts.add("Inter-Latin.ttf", &inter).unwrap();
        let mut draw = SketchDraw::new(file, Typesetter::new(fonts));
        let svg = draw.frame(&to_ir(file).pages[0].frames[0]).unwrap();
        assert!(draw.missing_fonts().is_empty(), "every label has its font");
        let pixmap = render_pixmap(&svg, 1.0).unwrap();
        at.iter()
            .map(|&(x, y)| {
                let pixel = pixmap.pixel(x, y).unwrap().demultiply();
                [pixel.red(), pixel.green(), pixel.blue()]
            })
            .collect()
    }

    /// Whether some pixel of the row `y`, from `from` to `to`, is white: a label's glyph.
    fn label_reaches(file: &SketchFile, y: std::ops::Range<u32>, x: std::ops::Range<u32>) -> bool {
        let at: Vec<_> = y.flat_map(|y| x.clone().map(move |x| (x, y))).collect();
        pixels(file, &at).contains(&WHITE)
    }

    /// The file with no override: every instance as its master.
    fn without_overrides(mut file: SketchFile) -> SketchFile {
        fn clear(value: &mut Value) {
            match value {
                Value::Object(map) => {
                    if let Some(Value::Array(overrides)) = map.get_mut("overrideValues") {
                        overrides.clear();
                    }
                    map.values_mut().for_each(clear);
                }
                Value::Array(list) => list.iter_mut().for_each(clear),
                _ => {}
            }
        }
        file.pages.iter_mut().for_each(clear);
        file
    }

    #[test]
    fn sketch_symbols_carry_their_links() {
        let ir = to_ir(&file());
        let frames = &ir.pages[0].frames;
        // A link on an instance: its own box.
        let open = &frames[0].hotspots[0];
        assert_eq!(open.name, "Open details");
        assert_eq!(
            open.bounds,
            Rect {
                x: 24.0,
                y: 120.0,
                width: 160.0,
                height: 48.0
            }
        );
        assert!(matches!(
            &open.interactions[0].action,
            Action::Navigate { to, transition }
                if *to == frames[1].id && matches!(transition.kind, TransitionKind::SlideIn { .. })
        ));
        // A link inside a master: where its instance puts it.
        let back = &frames[1].hotspots[0];
        assert_eq!(
            back.bounds,
            Rect {
                x: 16.0,
                y: 56.0,
                width: 80.0,
                height: 32.0
            }
        );
        assert!(matches!(back.interactions[0].action, Action::Back));
        // The masters are not screens: two artboards, none on the symbols' page.
        let screens: Vec<usize> = ir.pages.iter().map(|page| page.frames.len()).collect();
        assert_eq!(screens, [2, 0]);
    }

    #[test]
    fn sketch_symbols_draw_their_master_scaled() {
        // "Large" is 240 by 72, its master 160 by 48: blue reaches where the master would not.
        let drawn = pixels(&file(), &[(60, 144), (250, 236), (250, 144)]);
        assert_eq!(drawn, [BLUE, BLUE, WHITE]);
    }

    #[test]
    fn sketch_symbols_override_text() {
        // "Label" ends before x = 100; "Open details" goes on past it.
        let past_the_label = |file: &SketchFile| label_reaches(file, 134..154, 104..150);
        assert!(past_the_label(&file()));
        assert!(!past_the_label(&without_overrides(file())));
        // The label of the button inside the card, overridden through the card: "Read more".
        let in_the_card = |file: &SketchFile| label_reaches(file, 390..410, 255..290);
        assert!(in_the_card(&file()));
        assert!(!in_the_card(&without_overrides(file())));
    }

    #[test]
    fn sketch_symbols_swap_a_nested_symbol() {
        // The button of each card, left of its label: the second card swaps it for the dark one.
        let buttons = [(185, 400), (185, 550)];
        assert_eq!(pixels(&file(), &buttons), [BLUE, INK]);
        assert_eq!(pixels(&without_overrides(file()), &buttons), [BLUE, BLUE]);
    }
}
