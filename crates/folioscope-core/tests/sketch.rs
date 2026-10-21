// SPDX-License-Identifier: AGPL-3.0-or-later

use folioscope_core::sketch::{decode, summarize};

#[test]
fn navigation_lists_artboards_start_and_links() {
    let path = format!(
        "{}/../../fixtures/sketch/navigation.sketch",
        env!("CARGO_MANIFEST_DIR")
    );
    let file = decode(&std::fs::read(path).unwrap()).unwrap();
    insta::assert_snapshot!(summarize(&file).to_string());
}
