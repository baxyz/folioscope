// SPDX-License-Identifier: AGPL-3.0-or-later

use folioscope::figma::{decode, tree::Tree};

fn navigation() -> folioscope::figma::FigFile {
    let path = format!(
        "{}/../../fixtures/figma/navigation.fig",
        env!("CARGO_MANIFEST_DIR")
    );
    decode(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn frame_renders_to_svg_and_png() {
    let file = navigation();
    let tree = Tree::new(&file);
    let svg = folioscope::bake::svg::render_node(&tree, "1:1".parse().unwrap(), None).unwrap();
    insta::assert_snapshot!(svg.markup);
    let png = folioscope::bake::rasterize(&svg.markup, 2.0).unwrap();
    assert!(png.starts_with(b"\x89PNG"));
}

#[test]
fn single_node_renders_at_the_origin() {
    let file = navigation();
    let tree = Tree::new(&file);
    let svg = folioscope::bake::svg::render_node(&tree, "1:6".parse().unwrap(), None).unwrap();
    assert_eq!((svg.width, svg.height), (200.0, 48.0));
    assert!(!svg.markup.contains("matrix"));
}

mod features {
    //! `fixtures/figma/features.fig`: the `.fig` drawing features and the "open link" action.
    use folioscope::figma::{decode, ir::to_ir, tree::Tree};
    use folioscope::ir::Action;

    fn file() -> folioscope::figma::FigFile {
        let path = format!(
            "{}/../../fixtures/figma/features.fig",
            env!("CARGO_MANIFEST_DIR")
        );
        decode(&std::fs::read(path).unwrap()).unwrap()
    }

    fn svg() -> String {
        let file = file();
        let tree = Tree::new(&file);
        let svg = folioscope::bake::svg::render_node(&tree, "1:1".parse().unwrap(), None).unwrap();
        folioscope::bake::rasterize(&svg.markup, 1.0).unwrap();
        svg.markup
    }

    /// A part drawn on its own, for smart animate, keeps what overflows its box: the shadow of
    /// a card, the blur of a circle. A plain gradient square needs no room.
    #[test]
    fn a_layer_drawn_alone_keeps_its_shadow_and_blur() {
        let file = file();
        let tree = Tree::new(&file);
        let id = |name: &str| tree.nodes().find(|n| n.name == name).unwrap().id;
        let pad = |name: &str| {
            let (svg, pad) =
                folioscope::bake::svg::render_layer(&tree, id(name), &[], None).unwrap();
            let node = tree.get(id(name)).unwrap();
            let width = node
                .value
                .get("size")
                .and_then(|s| s.get("x"))
                .and_then(folioscope::kiwi::Value::as_f32)
                .unwrap();
            assert!((svg.width - (width + 2.0 * pad)).abs() < 0.01, "{name}");
            assert!(
                svg.markup
                    .contains(&format!(r#"viewBox="{0} {0}"#, 0.0 - pad)),
                "{name}"
            );
            pad
        };
        // A shadow 4 px down with a 12 px blur reaches 4 + 18 px.
        assert_eq!(pad("Drop shadow"), 22.0);
        assert_eq!(pad("Layer blur"), 12.0);
        assert_eq!(pad("GRADIENT_LINEAR"), 0.0);
    }

    #[test]
    fn figma_ir_reads_open_url() {
        let ir = to_ir(&file());
        let site = ir.pages[0].frames[0]
            .hotspots
            .iter()
            .find(|h| h.name == "Website")
            .unwrap();
        assert!(matches!(
            &site.interactions[0].action,
            Action::OpenUrl { url, new_tab: true } if url == "https://folioscope.baxyz.dev"
        ));
    }

    #[test]
    fn figma_linear_gradient() {
        assert!(svg().contains("<linearGradient"));
    }

    #[test]
    fn figma_other_gradients() {
        let svg = svg();
        assert!(svg.contains("<radialGradient"), "radial");
        // Angular and diamond gradients are patterns of thin shapes.
        assert!(svg.matches("<pattern").count() >= 2, "angular and diamond");
    }

    #[test]
    fn figma_shadows() {
        let svg = svg();
        // Two drop shadows in one filter, and an inner shadow.
        assert!(svg.contains(r#"result="s1""#), "two drop shadows");
        assert!(
            svg.contains(r#"operator="arithmetic" k1="-1""#),
            "inner shadow"
        );
    }

    #[test]
    fn figma_blurs() {
        let svg = svg();
        assert!(
            svg.contains("<feGaussianBlur stdDeviation=\"4\"/>"),
            "layer blur"
        );
        // The background blur draws the frame again behind the glass, blurred, in its outline.
        assert!(
            svg.contains("frame\"><use") || svg.contains("<use href=\"#b"),
            "background blur"
        );
    }

    #[test]
    fn figma_masks_and_images() {
        let svg = svg();
        assert!(
            svg.contains("<mask id=") && svg.contains(r#"mask-type="alpha""#),
            "mask"
        );
        // A tile at the image's own size, and a crop placing the image's unit square.
        assert!(
            svg.contains(r#"width="16" height="16" preserveAspectRatio="none""#),
            "tile"
        );
        assert!(
            svg.contains(r#"width="1" height="1" preserveAspectRatio="none" transform="#),
            "crop"
        );
    }

    #[test]
    fn figma_blend_modes() {
        assert!(svg().contains("mix-blend-mode:multiply"));
    }
}

/// `fixtures/figma/instances.fig`: a header swaps its tab for the active variant (whose icon a
/// property of its own hides) and its account icon for a person. Drawn from the components
/// alone, the gear and the inactive tab would show instead.
#[test]
fn swapped_instances_draw_the_component_they_were_swapped_for() {
    let path = format!(
        "{}/../../fixtures/figma/instances.fig",
        env!("CARGO_MANIFEST_DIR")
    );
    let file = folioscope::figma::decode(&std::fs::read(path).unwrap()).unwrap();
    let tree = Tree::new(&file);
    let home = tree.nodes().find(|n| n.name == "Home").unwrap().id;
    let svg = folioscope::bake::svg::render_node(&tree, home, None)
        .unwrap()
        .markup;
    let has = |rgb: &str| svg.contains(&format!("rgb({rgb})"));
    assert!(has("37 99 235"), "the active tab");
    assert!(!has("229 231 235"), "no inactive tab");
    assert!(has("22 163 74"), "the person icon");
    assert!(
        !has("220 38 38"),
        "no gear: the account's is swapped, the tab's hidden"
    );
}
