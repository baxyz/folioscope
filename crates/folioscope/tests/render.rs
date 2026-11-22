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
