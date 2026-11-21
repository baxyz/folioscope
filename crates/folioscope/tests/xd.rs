// SPDX-License-Identifier: AGPL-3.0-or-later

//! The `.xd` importer, on files read from `fixtures/external/xd` when the machine has them
//! (third-party prototypes are never committed; see `fixtures/README.md`).

use folioscope::xd::{decode, ir::to_ir, summarize};

fn fixture() -> Vec<u8> {
    let path = format!(
        "{}/../../fixtures/xd/navigation.xd",
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
    let ir = to_ir(&file);
    insta::assert_snapshot!("ir-navigation", serde_json::to_string_pretty(&ir).unwrap());
}

/// An artboard taller than its viewport is a frame of the viewport's height that scrolls over the
/// whole artboard; the bake draws its background in place and its content as the scrolling layer.
#[cfg(feature = "bake")]
#[test]
fn a_tall_artboard_scrolls() {
    use folioscope::bake::folio::{BakeOptions, bake_file_into};
    use folioscope::ir::ScrollDirection;

    let path = format!("{}/../../fixtures/xd/scroll.xd", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let ir = to_ir(&decode(&bytes).unwrap());
    let [feed, details] = ir.pages[0].frames.as_slice() else {
        panic!("two artboards expected");
    };
    assert_eq!((feed.bounds.width, feed.bounds.height), (375.0, 812.0));
    let scroll = feed.scroll.as_ref().expect("the feed scrolls");
    assert_eq!(scroll.direction, ScrollDirection::Vertical);
    assert_eq!(
        (scroll.content.width, scroll.content.height),
        (375.0, 1600.0)
    );
    assert!(details.scroll.is_none());

    let mut index = Vec::new();
    bake_file_into(&bytes, &BakeOptions::default(), &mut |path, bytes| {
        if path == "index.json" {
            index = bytes.to_vec();
        }
        Ok(())
    })
    .unwrap();
    let index: serde_json::Value = serde_json::from_slice(&index).unwrap();
    let layers = index["pages"][0]["frames"][0]["layers"].as_array().unwrap();
    let scrolls: Vec<bool> = layers
        .iter()
        .map(|l| l["scrolls"].as_bool().unwrap_or(false))
        .collect();
    assert_eq!(scrolls, [false, true]);
    assert_eq!(layers[1]["bounds"]["height"], 1600.0);
}

/// An artboard opened as an overlay shows where the action puts it, given from the hotspot's
/// corner, and "previous artboard" takes it off.
#[test]
fn an_overlay_opens_at_its_position() {
    use folioscope::ir::{Action, OverlayPosition, Point};

    let path = format!(
        "{}/../../fixtures/xd/overlay.xd",
        env!("CARGO_MANIFEST_DIR")
    );
    let ir = to_ir(&decode(&std::fs::read(&path).unwrap()).unwrap());
    let [home, sheet] = ir.pages[0].frames.as_slice() else {
        panic!("two artboards expected");
    };
    let settings = sheet.overlay.as_ref().expect("the sheet is an overlay");
    assert_eq!(settings.position, OverlayPosition::Manual);
    assert!(home.overlay.is_none());
    let Action::Overlay { to, offset, .. } = &home.hotspots[0].interactions[0].action else {
        panic!("the button opens an overlay");
    };
    assert_eq!(to, &sheet.id);
    // The sheet's corner is at (0, 449) of the screen; the button's is at (88, 640).
    assert_eq!(
        *offset,
        Some(Point {
            x: -88.0,
            y: -191.0
        })
    );
    assert_eq!(sheet.hotspots[0].interactions[0].action, Action::Back);
}

fn external() -> Vec<(String, Vec<u8>)> {
    let dir = format!("{}/../../fixtures/external/xd", env!("CARGO_MANIFEST_DIR"));
    let Ok(entries) = std::fs::read_dir(format!("{dir}/github")) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "xd"))
        .map(|p| (p.display().to_string(), std::fs::read(&p).unwrap()))
        .collect()
}

/// Every interaction of a real file lands on a hotspot, and every target is an artboard of it.
#[test]
fn real_prototypes_keep_their_links() {
    for (path, bytes) in external() {
        let file = decode(&bytes).unwrap_or_else(|e| panic!("{path}: {e}"));
        let ir = to_ir(&file);
        let page = &ir.pages[0];
        let ids: Vec<&str> = page.frames.iter().map(|f| f.id.as_str()).collect();
        let wanted = file.interactions["interactions"]
            .as_object()
            .map_or(0, |all| {
                all.values().map(|l| l.as_array().map_or(0, Vec::len)).sum()
            });
        let found: usize = page
            .frames
            .iter()
            .flat_map(|f| &f.hotspots)
            .map(|h| h.interactions.len())
            .sum();
        println!(
            "{path}: {} frames, {found}/{wanted} interactions",
            ids.len()
        );
        assert_eq!(page.flows.len(), 1, "{path}: one start");
        for frame in &page.frames {
            for hotspot in &frame.hotspots {
                for interaction in &hotspot.interactions {
                    if let folioscope::ir::Action::Navigate { to, .. } = &interaction.action {
                        assert!(ids.contains(&to.as_str()), "{path}: {to} is no artboard");
                    }
                }
            }
        }
    }
}

#[cfg(feature = "bake")]
mod features {
    //! `fixtures/xd/features.xd`: every XD scenario, one test per feature.
    use folioscope::bake::draw::Draw;
    use folioscope::bake::fonts::FontLibrary;
    use folioscope::bake::text::Typesetter;
    use folioscope::bake::xd::XdDraw;
    use folioscope::ir::{Action, Document, Frame, TransitionKind, Trigger};
    use folioscope::xd::{XdFile, decode, ir::to_ir};

    fn file() -> XdFile {
        let path = format!(
            "{}/../../fixtures/xd/features.xd",
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
        let mut draw = XdDraw::new(&file, Typesetter::new(FontLibrary::default()));
        draw.frame(frame(&ir, name)).unwrap()
    }

    #[test]
    fn xd_triggers() {
        let ir = to_ir(&file());
        let splash = &frame(&ir, "Splash").hotspots[0].interactions[0];
        assert_eq!(splash.trigger, Trigger::AfterDelay { delay_ms: 1500 });
        let open = &frame(&ir, "Home").hotspots[0].interactions[0];
        assert_eq!(open.trigger, Trigger::Click);
        let peek = frame(&ir, "Home")
            .hotspots
            .iter()
            .find(|h| h.name == "Peek")
            .unwrap();
        assert_eq!(peek.interactions[0].trigger, Trigger::Hover);
        assert!(matches!(
            frame(&ir, "Details").hotspots[0].interactions[0].action,
            Action::Back
        ));
    }

    #[test]
    fn xd_auto_animate() {
        let ir = to_ir(&file());
        let open = &frame(&ir, "Home").hotspots[0].interactions[0];
        assert!(matches!(
            &open.action,
            Action::Navigate { transition, .. } if transition.kind == TransitionKind::SmartAnimate
        ));
    }

    #[test]
    fn xd_draws_shapes() {
        let svg = svg("Home");
        assert!(
            svg.contains("<image") && svg.contains("data:image/png;base64,"),
            "image fill"
        );
        assert!(svg.contains("<clipPath"), "clip path");
        assert!(svg.contains(r#"result="s0""#), "shadow");
        folioscope::bake::rasterize(&svg, 1.0).unwrap();
    }

    #[test]
    fn xd_gradients() {
        let svg = svg("Home");
        assert!(svg.contains("<linearGradient"), "linear");
        assert!(svg.contains("<radialGradient"), "radial");
    }
}

#[cfg(feature = "bake")]
mod effects {
    //! The effects of `fixtures/xd/features.xd`, drawn with Inter.
    use folioscope::bake::draw::Draw;
    use folioscope::bake::fonts::FontLibrary;
    use folioscope::bake::text::Typesetter;
    use folioscope::bake::xd::XdDraw;
    use folioscope::xd::{decode, ir::to_ir};

    fn svg() -> String {
        let root = format!("{}/../../fixtures", env!("CARGO_MANIFEST_DIR"));
        let file = decode(&std::fs::read(format!("{root}/xd/features.xd")).unwrap()).unwrap();
        let ir = to_ir(&file);
        let mut fonts = FontLibrary::default();
        let inter = std::fs::read(format!("{root}/fonts/Inter-Latin.ttf")).unwrap();
        fonts.add("Inter-Latin.ttf", &inter).unwrap();
        let mut draw = XdDraw::new(&file, Typesetter::new(fonts));
        let home = ir.pages[0]
            .frames
            .iter()
            .find(|f| f.name == "Home")
            .unwrap();
        let svg = draw.frame(home).unwrap();
        assert!(draw.missing_fonts().is_empty());
        folioscope::bake::rasterize(&svg, 1.0).unwrap();
        svg
    }

    #[test]
    fn xd_blurs() {
        let svg = svg();
        assert!(
            svg.contains("<feGaussianBlur stdDeviation=\"4\"/>"),
            "object blur"
        );
        assert!(
            svg.contains("frame\"><") && svg.contains("<use href"),
            "background blur"
        );
    }

    #[test]
    fn xd_blend_and_image_fit() {
        let svg = svg();
        assert!(svg.contains("mix-blend-mode:multiply"), "blend mode");
        assert!(
            svg.contains(r#"preserveAspectRatio="xMidYMid meet""#),
            "image fit whole"
        );
    }

    #[test]
    fn xd_text_decoration() {
        assert!(svg().contains("v1.2857143h-"), "underline");
    }
}
