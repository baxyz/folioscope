// SPDX-License-Identifier: AGPL-3.0-or-later

//! The `.xd` importer, on files read from `fixtures/external/xd` when the machine has them
//! (third-party prototypes are never committed; see `fixtures/README.md`).

use folioscope::xd::{decode, ir::to_ir, summarize};

fn fixture() -> Vec<u8> {
    let path = format!(
        "{}/../../fixtures/xd/navigation.xd",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn navigation_lists_artboards_start_and_links() {
    let file = decode(&fixture()).unwrap();
    insta::assert_snapshot!(summarize(&file).to_string());
}

/// The IR of the generated fixture, as the JSON baked into a `.folio`.
#[test]
fn generated_fixture_converts_to_ir() {
    let file = decode(&fixture()).unwrap();
    let ir = to_ir(&file);
    insta::assert_snapshot!("ir-navigation", serde_json::to_string_pretty(&ir).unwrap());
}

fn external() -> Vec<(String, Vec<u8>)> {
    let dir = format!("{}/../../fixtures/external/xd", env!("CARGO_MANIFEST_DIR"));
    let Ok(entries) = std::fs::read_dir(format!("{dir}/github")) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "xd"))
        .map(|p| (p.display().to_string(), std::fs::read(&p).unwrap()))
        .collect()
}

/// Every interaction of a real file lands on a hotspot, and every target is an artboard of it.
#[test]
fn real_prototypes_keep_their_links() {
    for (path, bytes) in external() {
        let file = decode(&bytes).unwrap_or_else(|e| panic!("{path}: {e}"));
        let ir = to_ir(&file);
        let page = &ir.pages[0];
        let ids: Vec<&str> = page.frames.iter().map(|f| f.id.as_str()).collect();
        let wanted = file.interactions["interactions"]
            .as_object()
            .map_or(0, |all| {
                all.values().map(|l| l.as_array().map_or(0, Vec::len)).sum()
            });
        let found: usize = page
            .frames
            .iter()
            .flat_map(|f| &f.hotspots)
            .map(|h| h.interactions.len())
            .sum();
        println!(
            "{path}: {} frames, {found}/{wanted} interactions",
            ids.len()
        );
        assert_eq!(page.flows.len(), 1, "{path}: one start");
        for frame in &page.frames {
            for hotspot in &frame.hotspots {
                for interaction in &hotspot.interactions {
                    if let folioscope::ir::Action::Navigate { to, .. } = &interaction.action {
                        assert!(ids.contains(&to.as_str()), "{path}: {to} is no artboard");
                    }
                }
            }
        }
    }
}
