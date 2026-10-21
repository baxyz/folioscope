// SPDX-License-Identifier: AGPL-3.0-or-later

use folioscope_core::figma::{decode, prototype::summarize};

fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/../../fixtures/figma/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// Every fixture written by `tools/fixtures`, summarized: pages, frames, flows and links.
#[test]
fn generated_fixtures_list_frames_flows_and_links() {
    for name in [
        "navigation",
        "overlay",
        "scroll",
        "triggers",
        "flows-and-pages",
    ] {
        let file = decode(&fixture(&format!("{name}.fig"))).unwrap();
        insta::assert_snapshot!(name, summarize(&file).to_string());
    }
}

#[test]
fn rejects_a_file_that_is_not_a_zip() {
    assert!(decode(b"not a .fig file").is_err());
}
