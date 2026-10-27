// SPDX-License-Identifier: AGPL-3.0-or-later

use folioscope_core::penpot::{decode, summarize};

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
    let ir = folioscope_core::penpot::ir::to_ir(&file);
    insta::assert_snapshot!("ir-navigation", serde_json::to_string_pretty(&ir).unwrap());
}
