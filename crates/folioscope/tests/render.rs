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
    let svg = folioscope::bake::svg::render_node(&tree, "1:1".parse().unwrap()).unwrap();
    insta::assert_snapshot!(svg.markup);
    let png = folioscope::bake::rasterize(&svg.markup, 2.0).unwrap();
    assert!(png.starts_with(b"\x89PNG"));
}

#[test]
fn single_node_renders_at_the_origin() {
    let file = navigation();
    let tree = Tree::new(&file);
    let svg = folioscope::bake::svg::render_node(&tree, "1:4".parse().unwrap()).unwrap();
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
        let svg = folioscope::bake::svg::render_node(&tree, "1:1".parse().unwrap()).unwrap();
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
