// SPDX-License-Identifier: AGPL-3.0-or-later

//! Writes a `.folio` folder (ADR 0004): `index.json` (the IR plus each frame's layers), hashed
//! layer images at every density, and `poster.webp`. A frame is one layer, or, when it scrolls,
//! three: its background, the content that scrolls, and the fixed children on top.

use std::fmt::Write as _;
use std::path::Path;

use folioscope_core::figma::{
    FigFile,
    ir::to_ir,
    tree::{NodeId, Tree},
};
use folioscope_core::ir::{self, Flow, Rect};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::image::{ImageFormat, encode, encode_best};
use crate::{RenderError, render_pixmap, svg};

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
    /// Encode large, photo-like layers as AVIF when it beats lossless WebP (see `image`).
    pub avif: bool,
}

impl Default for BakeOptions {
    fn default() -> Self {
        Self {
            densities: vec![1, 2],
            avif: true,
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
    /// [ThumbHash](https://evanw.github.io/thumbhash/), base64: a blurred preview shown until the
    /// layers load.
    placeholder: String,
}

/// An image drawn at `bounds` (frame coordinates); its files are
/// `layers/<src>@<density>x.<format>`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Layer {
    src: String,
    format: ImageFormat,
    bounds: Rect,
    /// The frame's scrolling content: moves with the scroll, under the layers that follow.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    scrolls: bool,
}

/// `(markup, bounds, scrolls)` of each layer of `frame`, bottom first: the whole frame, or the
/// three parts of a scrolling one.
fn frame_parts(
    tree: &Tree<'_>,
    id: NodeId,
    frame: &ir::Frame,
    whole_markup: &str,
) -> Result<Vec<(String, Rect, bool)>, BakeError> {
    let whole = Rect {
        x: 0.0,
        y: 0.0,
        width: frame.bounds.width,
        height: frame.bounds.height,
    };
    let Some(scroll) = &frame.scroll else {
        return Ok(vec![(whole_markup.to_owned(), whole, false)]);
    };
    let fixed: Vec<NodeId> = scroll.fixed.iter().filter_map(|f| f.parse().ok()).collect();
    let part = |part| {
        svg::render_part(tree, id, part)
            .map(|svg| svg.markup)
            .ok_or_else(|| BakeError::MissingFrame(frame.id.clone()))
    };
    let size = (scroll.content.width, scroll.content.height);
    let mut parts = vec![
        (part(svg::Part::Background)?, whole, false),
        (
            part(svg::Part::Content {
                fixed: &fixed,
                size,
            })?,
            scroll.content,
            true,
        ),
    ];
    if !fixed.is_empty() {
        parts.push((part(svg::Part::Fixed { fixed: &fixed })?, whole, false));
    }
    Ok(parts)
}

enum LayerError {
    Render(RenderError),
    Sink(BakeError),
}

/// Encodes one layer at every density, highest first (it picks the format), into `sink`.
fn encode_layer(
    markup: &str,
    densities: &[u8],
    options: &BakeOptions,
    report: &mut BakeReport,
    sink: &mut Sink<'_>,
) -> Result<(String, ImageFormat), LayerError> {
    let src = hash(markup);
    let mut format = None;
    for &density in densities {
        let pixmap = render_pixmap(markup, f32::from(density)).map_err(LayerError::Render)?;
        let (chosen, bytes) = match format {
            None => encode_best(&pixmap, options.avif).map_err(LayerError::Render)?,
            Some(f) => (f, encode(&pixmap, f).map_err(LayerError::Render)?),
        };
        format = Some(chosen);
        report.bytes += bytes.len();
        report.layers += 1;
        sink(
            &format!("layers/{src}@{density}x.{}", chosen.extension()),
            bytes.as_slice(),
        )
        .map_err(LayerError::Sink)?;
    }
    Ok((src, format.unwrap_or(ImageFormat::Webp)))
}

/// Receives each baked file: its path inside the `.folio` folder, and its bytes.
pub type Sink<'a> = dyn FnMut(&str, &[u8]) -> Result<(), BakeError> + 'a;

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
    create_dir(&out.join("layers"))?;
    bake_fig_into(file, options, &mut |path, bytes| {
        write(&out.join(path), bytes)
    })
}

/// Bakes a decoded `.fig`, handing each file to `sink` with its path inside the `.folio` folder
/// (`index.json`, `poster.webp`, `layers/…`). The WASM build writes to memory this way.
///
/// # Errors
///
/// Fails when a frame cannot be rendered or `sink` fails.
pub fn bake_fig_into(
    file: &FigFile,
    options: &BakeOptions,
    sink: &mut Sink<'_>,
) -> Result<BakeReport, BakeError> {
    let document = to_ir(file);
    let tree = Tree::new(file);
    let mut report = BakeReport::default();
    let mut poster: Option<Vec<u8>> = None;
    // The highest density is encoded first: it decides the layer's format for every density.
    let mut densities = options.densities.clone();
    densities.sort_unstable_by(|a, b| b.cmp(a));
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
            let render_error = |source| BakeError::Render {
                frame: frame.id.clone(),
                source,
            };
            let placeholder = crate::placeholder(&rendered.markup).map_err(render_error)?;
            let mut layers = Vec::new();
            for (markup, bounds, scrolls) in frame_parts(&tree, id, frame, &rendered.markup)? {
                let (src, format) = encode_layer(&markup, &densities, options, &mut report, sink)
                    .map_err(|e| match e {
                    LayerError::Render(source) => render_error(source),
                    LayerError::Sink(e) => e,
                })?;
                layers.push(Layer {
                    src,
                    format,
                    bounds,
                    scrolls,
                });
            }
            // The poster stands in for the starting frame until the player runs: 1x, lossless.
            if Some(frame.id.as_str()) == first_start {
                let pixmap = render_pixmap(&rendered.markup, 1.0).map_err(render_error)?;
                poster = Some(encode(&pixmap, ImageFormat::Webp).map_err(render_error)?);
            }
            report.frames += 1;
            frames.push(FolioFrame {
                frame,
                layers,
                placeholder,
            });
        }
        pages.push(FolioPage {
            id: &page.id,
            name: &page.name,
            flows: &page.flows,
            frames,
        });
    }

    if let Some(webp) = poster {
        sink("poster.webp", &webp)?;
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
    sink("index.json", json.as_bytes())?;
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
