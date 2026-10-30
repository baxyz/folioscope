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
