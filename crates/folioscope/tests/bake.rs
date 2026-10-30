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
