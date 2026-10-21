// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::{Path, PathBuf};

use folioscope_core::{Format, detect, summarize};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            files(&path, out);
        } else if path
            .extension()
            .is_some_and(|e| ["fig", "sketch", "xd", "penpot"].iter().any(|x| e == *x))
        {
            out.push(path);
        }
    }
}

#[test]
fn detects_formats_by_content() {
    let read = |p: &str| std::fs::read(fixtures().join(p)).unwrap();
    assert_eq!(detect(&read("figma/navigation.fig")), Some(Format::Fig));
    assert_eq!(
        detect(&read("sketch/navigation.sketch")),
        Some(Format::Sketch)
    );
    assert_eq!(
        detect(&read("penpot/navigation.penpot")),
        Some(Format::Penpot)
    );
    assert_eq!(detect(b"not a zip"), None);
}

/// Real files in the git-ignored `fixtures/external/` (see `fixtures/README.md`): every one must
/// decode. Nothing to check when the folder is absent, as in CI.
#[test]
fn external_files_decode() {
    let mut paths = Vec::new();
    files(&fixtures().join("external"), &mut paths);
    for path in paths {
        let bytes = std::fs::read(&path).unwrap();
        let summary = summarize(&bytes).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(!summary.pages.is_empty(), "{}", path.display());
        if detect(&bytes) == Some(Format::Fig) {
            let ir =
                folioscope_core::figma::ir::to_ir(&folioscope_core::figma::decode(&bytes).unwrap());
            assert!(!ir.pages.is_empty(), "{}", path.display());
        }
    }
}
