// SPDX-License-Identifier: AGPL-3.0-or-later

use folioscope_core::penpot::{decode, summarize};

#[test]
fn navigation_lists_boards_flows_and_links() {
    let path = format!(
        "{}/../../fixtures/penpot/navigation.penpot",
        env!("CARGO_MANIFEST_DIR")
    );
    let file = decode(&std::fs::read(path).unwrap()).unwrap();
    insta::assert_snapshot!(summarize(&file).to_string());
}
