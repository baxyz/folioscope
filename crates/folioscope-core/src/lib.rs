// SPDX-License-Identifier: AGPL-3.0-or-later

//! Importers and intermediate model (IR) for interactive `.fig`, `.sketch`, `.xd` and `.penpot`
//! prototypes. See `PROJECT.md` at the repository root.

pub mod figma;
pub mod ir;
pub mod kiwi;
pub mod penpot;
pub mod sketch;
pub mod summary;
pub mod xd;

use std::io::{Cursor, Read};

/// Version of the IR schema, written into every baked `.folio` so players can refuse
/// incompatible files.
pub const IR_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Fig,
    Sketch,
    Xd,
    Penpot,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not a .fig, .sketch, .xd or .penpot file")]
    UnknownFormat,
    #[error("{0:?} files are not supported yet")]
    Unsupported(Format),
    #[error(transparent)]
    Fig(#[from] figma::FigError),
    #[error(transparent)]
    Sketch(#[from] sketch::SketchError),
    #[error(transparent)]
    Xd(#[from] xd::XdError),
    #[error(transparent)]
    Penpot(#[from] penpot::PenpotError),
}

/// Detects the format from the archive's contents, never from a file name.
#[must_use]
pub fn detect(archive: &[u8]) -> Option<Format> {
    let mut zip = zip::ZipArchive::new(Cursor::new(archive)).ok()?;
    let names: Vec<String> = zip
        .file_names()
        .filter_map(|name| name.ok().map(std::borrow::Cow::into_owned))
        .collect();
    let has = |name: &str| names.iter().any(|n| n == name);
    if has("canvas.fig") {
        return Some(Format::Fig);
    }
    if has("document.json") {
        return Some(Format::Sketch);
    }
    if has("mimetype") {
        let mut mimetype = String::new();
        zip.by_name("mimetype")
            .ok()?
            .read_to_string(&mut mimetype)
            .ok()?;
        if mimetype.contains("application/vnd.adobe.sparkler.project+dcxucf") {
            return Some(Format::Xd);
        }
    }
    if has("manifest.json") && names.iter().any(|n| n.starts_with("files/")) {
        return Some(Format::Penpot);
    }
    None
}

/// Decodes any supported design file and summarizes its prototype.
///
/// # Errors
///
/// Fails on an unknown or not yet supported format, or when the file cannot be decoded.
pub fn summarize(archive: &[u8]) -> Result<summary::Summary, Error> {
    match detect(archive).ok_or(Error::UnknownFormat)? {
        Format::Fig => Ok(figma::prototype::summarize(&figma::decode(archive)?)),
        Format::Sketch => Ok(sketch::summarize(&sketch::decode(archive)?)),
        Format::Xd => Ok(xd::summarize(&xd::decode(archive)?)),
        Format::Penpot => Ok(penpot::summarize(&penpot::decode(archive)?)),
    }
}
