// SPDX-License-Identifier: AGPL-3.0-or-later

//! Writes a `.folio` folder (ADR 0004): `index.json` (the IR plus each frame's layers), hashed
//! layer images at every density, and `poster.png`. Phase 1: one layer per frame.

use std::fmt::Write as _;
use std::path::Path;

use folioscope_core::figma::{FigFile, ir::to_ir, tree::Tree};
use folioscope_core::ir::{self, Flow, Rect};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{RenderError, rasterize, svg};

/// Version of the `.folio` layout, independent of the IR's.
pub const FOLIO_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum BakeError {
    #[error("cannot write {path}: {source}")]
    Write {
        path: String,
        source: std::io::Error,
    },
    #[error("frame {frame}: {source}")]
    Render { frame: String, source: RenderError },
    #[error("frame {0} is not a node of the file")]
    MissingFrame(String),
}

#[derive(Debug, Clone)]
pub struct BakeOptions {
    /// Pixel densities to render, e.g. `[1, 2]`.
    pub densities: Vec<u8>,
}

impl Default for BakeOptions {
    fn default() -> Self {
        Self {
            densities: vec![1, 2],
        }
    }
}

/// What a bake wrote.
#[derive(Debug, Default)]
pub struct BakeReport {
    pub frames: usize,
    pub layers: usize,
    pub bytes: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Index<'a> {
    folio: u32,
    ir_version: u32,
    densities: &'a [u8],
    source: ir::SourceFormat,
    pages: Vec<FolioPage<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FolioPage<'a> {
    id: &'a str,
    name: &'a str,
    flows: &'a [Flow],
    frames: Vec<FolioFrame<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FolioFrame<'a> {
    #[serde(flatten)]
    frame: &'a ir::Frame,
    layers: Vec<Layer>,
}

/// An image drawn at `bounds` (frame coordinates); its files are `layers/<src>@<density>x.png`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Layer {
    src: String,
    bounds: Rect,
}

/// Bakes a decoded `.fig` into the folder `out`, created if needed.
///
/// # Errors
///
/// Fails when a frame cannot be rendered or a file cannot be written.
pub fn bake_fig(
    file: &FigFile,
    out: &Path,
    options: &BakeOptions,
) -> Result<BakeReport, BakeError> {
    let document = to_ir(file);
    let tree = Tree::new(file);
    create_dir(&out.join("layers"))?;
    let mut report = BakeReport::default();
    let mut poster: Option<String> = None;
    let first_start = document
        .pages
        .iter()
        .find_map(|p| p.flows.first().map(|f| f.start.as_str()))
        .or_else(|| {
            document
                .pages
                .first()?
                .frames
                .first()
                .map(|f| f.id.as_str())
        });

    let mut pages = Vec::new();
    for page in &document.pages {
        let mut frames = Vec::new();
        for frame in &page.frames {
            let id = frame
                .id
                .parse()
                .map_err(|_| BakeError::MissingFrame(frame.id.clone()))?;
            let rendered = svg::render_node(&tree, id)
                .ok_or_else(|| BakeError::MissingFrame(frame.id.clone()))?;
            let src = hash(&rendered.markup);
            for &density in &options.densities {
                let png = rasterize(&rendered.markup, f32::from(density)).map_err(|source| {
                    BakeError::Render {
                        frame: frame.id.clone(),
                        source,
                    }
                })?;
                report.bytes += png.len();
                report.layers += 1;
                write(&out.join(format!("layers/{src}@{density}x.png")), &png)?;
            }
            if Some(frame.id.as_str()) == first_start {
                poster = Some(src.clone());
            }
            report.frames += 1;
            frames.push(FolioFrame {
                frame,
                layers: vec![Layer {
                    src,
                    bounds: Rect {
                        x: 0.0,
                        y: 0.0,
                        width: frame.bounds.width,
                        height: frame.bounds.height,
                    },
                }],
            });
        }
        pages.push(FolioPage {
            id: &page.id,
            name: &page.name,
            flows: &page.flows,
            frames,
        });
    }

    // The poster is the starting frame at the highest density.
    if let (Some(src), Some(density)) = (poster, options.densities.iter().max()) {
        let from = out.join(format!("layers/{src}@{density}x.png"));
        let bytes = std::fs::read(&from).map_err(|source| BakeError::Write {
            path: from.display().to_string(),
            source,
        })?;
        write(&out.join("poster.png"), &bytes)?;
    }

    let index = Index {
        folio: FOLIO_VERSION,
        ir_version: folioscope_core::IR_VERSION,
        densities: &options.densities,
        source: document.source,
        pages,
    };
    let json = serde_json::to_string(&index).map_err(|e| BakeError::Write {
        path: "index.json".to_owned(),
        source: e.into(),
    })?;
    report.bytes += json.len();
    write(&out.join("index.json"), json.as_bytes())?;
    Ok(report)
}

/// First 16 hex digits of the SHA-256 of a layer's SVG: stable across bakes and densities.
fn hash(markup: &str) -> String {
    let digest = Sha256::digest(markup.as_bytes());
    digest[..8].iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn create_dir(path: &Path) -> Result<(), BakeError> {
    std::fs::create_dir_all(path).map_err(|source| BakeError::Write {
        path: path.display().to_string(),
        source,
    })
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), BakeError> {
    std::fs::write(path, bytes).map_err(|source| BakeError::Write {
        path: path.display().to_string(),
        source,
    })
}
