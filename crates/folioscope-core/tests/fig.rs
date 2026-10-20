// SPDX-License-Identifier: AGPL-3.0-or-later

use folioscope_core::fig::{decode, prototype::summarize};

fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/../../fixtures/fig/{name}", env!("CARGO_MANIFEST_DIR"));
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
    assert!(decode(b"not a fig file").is_err());
}

/// Real files in the git-ignored `fixtures/external/fig/` (see `fixtures/README.md`): they must all
/// decode. Skipped when the folder is absent, as in CI.
#[test]
fn external_files_decode() {
    let dir = format!("{}/../../fixtures/external/fig", env!("CARGO_MANIFEST_DIR"));
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "fig") {
            let bytes = std::fs::read(&path).unwrap();
            let file = decode(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert!(!summarize(&file).pages.is_empty(), "{}", path.display());
        }
    }
}
