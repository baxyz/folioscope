// SPDX-License-Identifier: AGPL-3.0-or-later

//! Writes a `.folio` folder (docs/spec/folio.md): `index.json` (the IR plus each frame's layers), hashed
//! layer images at every density, and `poster.webp`. A frame is one layer, or, when it scrolls,
//! three: its background, the content that scrolls, and the fixed children on top.

use std::collections::{BTreeSet, HashSet};
use std::fmt::Write as _;
use std::path::Path;

use crate::Format;
use crate::figma::{FigFile, ir::to_ir, tree::Tree};
use crate::ir::{self, Document, Flow, Rect};
use crate::penpot::{PenpotFile, ir::to_ir as penpot_ir};
use crate::sketch::{SketchFile, ir::to_ir as sketch_ir};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::bake::draw::{Draw, FigmaDraw};
use crate::bake::fonts::FontLibrary;
use crate::bake::image::{ImageFormat, encode, encode_best, gzipped_len, svg_eligible};
use crate::bake::penpot::PenpotDraw;
use crate::bake::sketch::SketchDraw;
use crate::bake::text::Typesetter;
use crate::bake::xd::XdDraw;
use crate::bake::{RenderError, render_pixmap};
use crate::xd::{XdFile, ir::to_ir as xd_ir};

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
    #[error("{0}")]
    Decode(String),
}

#[derive(Debug, Clone)]
pub struct BakeOptions {
    /// Pixel densities to render, e.g. `[1, 2]`.
    pub densities: Vec<u8>,
    /// Encode large, photo-like layers as AVIF when it beats lossless WebP (see `image`).
    pub avif: bool,
    /// Serve a layer as its SVG when it holds no image or filter and weighs less than its lightest
    /// image (docs/spec/bake.md).
    pub svg: bool,
    /// The most states baked per interactive component (docs/spec/folio.md); the others stay unreachable,
    /// and the report lists the component.
    pub max_states: usize,
    /// Fonts for text the file stores no glyphs for (docs/spec/bake.md).
    pub fonts: FontLibrary,
    /// Bake the pages that have no flow too. By default, when some page has one, the others (a
    /// design system's components, say) are left out: they are not part of the prototype.
    pub all_pages: bool,
}

impl Default for BakeOptions {
    fn default() -> Self {
        Self {
            densities: vec![1, 2],
            avif: true,
            svg: true,
            max_states: DEFAULT_MAX_STATES,
            fonts: FontLibrary::default(),
            all_pages: false,
        }
    }
}

/// What a bake wrote.
#[derive(Debug, Default)]
pub struct BakeReport {
    pub frames: usize,
    pub layers: usize,
    /// Of `layers`, those served as their SVG.
    pub svg_layers: usize,
    pub bytes: usize,
    /// Interactive components with more states than `max_states`: the states past it were left
    /// out.
    pub capped_components: Vec<CappedComponent>,
    /// Fonts some text needed and was not given (`Family Style`).
    pub missing_fonts: BTreeSet<String>,
    /// Frames left out because an image of them would be wider or taller than the formats allow
    /// (a 28,000 px strip of icons, say): by name.
    pub skipped_frames: Vec<String>,
    /// Pages left out for having no flow while another page has one: by name.
    pub skipped_pages: Vec<String>,
}

/// An interactive component the bake kept only some states of (docs/spec/folio.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CappedComponent {
    /// The component's name, then the frame's.
    pub name: String,
    pub frame: String,
    /// States baked, of `states` reachable.
    pub kept: usize,
    pub states: usize,
}

/// States baked per interactive component by default (docs/spec/folio.md).
pub const DEFAULT_MAX_STATES: usize = 16;

/// The longest side of an image WebP can hold.
const MAX_IMAGE_SIDE: f32 = 16383.0;

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
    /// The IR frame, its components left out: they follow, with their layers.
    #[serde(flatten)]
    frame: ir::Frame,
    layers: Vec<Layer>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    components: Vec<FolioComponent<'a>>,
    /// For smart animate (docs/spec/player.md): the background, then each direct child, by name.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    parts: Vec<FolioPart>,
    /// [ThumbHash](https://evanw.github.io/thumbhash/), base64: a blurred preview shown until the
    /// layers load.
    placeholder: String,
}

/// A part of a frame that smart animate moves: the background is named `""`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FolioPart {
    name: String,
    layer: Layer,
}

/// An interactive component, with one layer per state.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FolioComponent<'a> {
    id: &'a str,
    name: &'a str,
    bounds: Rect,
    initial: &'a str,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    fixed: bool,
    states: Vec<FolioState<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FolioState<'a> {
    variant: &'a str,
    name: &'a str,
    bounds: Rect,
    hotspots: &'a [ir::Hotspot],
    /// Drawn at the state's bounds, from the component's top-left corner.
    layer: Layer,
    /// Interactive components inside the state, drawn over its layer.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    components: Vec<FolioComponent<'a>>,
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

/// Renders every state of the frame's interactive components, each as a layer.
fn bake_components<'a>(
    draw: &mut dyn Draw,
    frame: &'a ir::Frame,
    options: &BakeOptions,
    report: &mut BakeReport,
    sink: &mut Sink<'_>,
) -> Result<Vec<FolioComponent<'a>>, BakeError> {
    let mut components = Vec::new();
    for component in &frame.components {
        components.push(bake_component(
            draw, frame, component, options, report, sink,
        )?);
    }
    Ok(components)
}

/// Renders one interactive component's states, and those of the components nested in them.
fn bake_component<'a>(
    draw: &mut dyn Draw,
    frame: &'a ir::Frame,
    component: &'a ir::Component,
    options: &BakeOptions,
    report: &mut BakeReport,
    sink: &mut Sink<'_>,
) -> Result<FolioComponent<'a>, BakeError> {
    let densities = &sorted_densities(options);
    let kept = component.states.len().min(options.max_states.max(1));
    if kept < component.states.len() {
        report.capped_components.push(CappedComponent {
            name: component.name.clone(),
            frame: frame.name.clone(),
            kept,
            states: component.states.len(),
        });
    }
    let mut states = Vec::new();
    for state in &component.states[..kept] {
        let markup = draw
            .state(component, state)
            .ok_or_else(|| BakeError::MissingFrame(state.variant.clone()))?;
        let (src, format) =
            encode_layer(&markup, densities, options, report, sink).map_err(|e| match e {
                LayerError::Render(source) => BakeError::Render {
                    frame: frame.id.clone(),
                    source,
                },
                LayerError::Sink(e) => e,
            })?;
        let mut nested = Vec::new();
        for inner in &state.components {
            nested.push(bake_component(draw, frame, inner, options, report, sink)?);
        }
        states.push(FolioState {
            variant: &state.variant,
            name: &state.name,
            bounds: state.bounds,
            hotspots: &state.hotspots,
            layer: Layer {
                src,
                format,
                bounds: state.bounds,
                scrolls: false,
            },
            components: nested,
        });
    }
    Ok(FolioComponent {
        id: &component.id,
        name: &component.name,
        bounds: component.bounds,
        initial: &component.initial,
        fixed: component.fixed,
        states,
    })
}

/// Frames a smart animate leaves or reaches: they need their parts (docs/spec/player.md).
fn smart_animated(document: &Document) -> HashSet<&str> {
    let mut frames = HashSet::new();
    for frame in document.pages.iter().flat_map(|p| &p.frames) {
        for interaction in frame.hotspots.iter().flat_map(|h| &h.interactions) {
            if let ir::Action::Navigate { to, transition } = &interaction.action
                && matches!(transition.kind, ir::TransitionKind::SmartAnimate)
            {
                frames.insert(frame.id.as_str());
                frames.insert(to.as_str());
            }
        }
    }
    frames
}

/// Renders the parts of a frame smart animate pairs, each as a layer.
fn bake_parts(
    draw: &mut dyn Draw,
    frame: &ir::Frame,
    options: &BakeOptions,
    report: &mut BakeReport,
    sink: &mut Sink<'_>,
) -> Result<Vec<FolioPart>, BakeError> {
    let densities = &sorted_densities(options);
    let mut parts = Vec::new();
    for (name, markup, bounds) in draw.parts(frame) {
        let (src, format) =
            encode_layer(&markup, densities, options, report, sink).map_err(|e| match e {
                LayerError::Render(source) => BakeError::Render {
                    frame: frame.id.clone(),
                    source,
                },
                LayerError::Sink(e) => e,
            })?;
        parts.push(FolioPart {
            name,
            layer: Layer {
                src,
                format,
                bounds,
                scrolls: false,
            },
        });
    }
    Ok(parts)
}

/// The densities to render, highest first: it decides each layer's format for the others.
fn sorted_densities(options: &BakeOptions) -> Vec<u8> {
    let mut densities = options.densities.clone();
    densities.sort_unstable_by(|a, b| b.cmp(a));
    densities
}

enum LayerError {
    Render(RenderError),
    Sink(BakeError),
}

/// Encodes one layer into `sink`: as its SVG when that is allowed and lighter (docs/spec/bake.md), else at
/// every density, highest first (it picks the raster format).
fn encode_layer(
    markup: &str,
    densities: &[u8],
    options: &BakeOptions,
    report: &mut BakeReport,
    sink: &mut Sink<'_>,
) -> Result<(String, ImageFormat), LayerError> {
    let src = hash(markup);
    let mut format = None;
    let mut files = Vec::with_capacity(densities.len());
    for &density in densities {
        let pixmap = render_pixmap(markup, f32::from(density)).map_err(LayerError::Render)?;
        let (chosen, bytes) = match format {
            None => encode_best(&pixmap, options.avif).map_err(LayerError::Render)?,
            Some(f) => (f, encode(&pixmap, f).map_err(LayerError::Render)?),
        };
        format = Some(chosen);
        files.push((density, bytes));
    }
    // The SVG against the lightest image, both as a server sends them (WebP and AVIF do not
    // shrink when gzipped): served as SVG, the layer is never heavier for any visitor, and it is
    // one file for every density, sharp at any zoom.
    let lightest = files.iter().map(|(_, bytes)| bytes.len()).min();
    if options.svg
        && svg_eligible(markup)
        && lightest.is_some_and(|lightest| gzipped_len(markup) < lightest)
    {
        report.bytes += markup.len();
        report.layers += 1;
        report.svg_layers += 1;
        sink(&format!("layers/{src}.svg"), markup.as_bytes()).map_err(LayerError::Sink)?;
        return Ok((src, ImageFormat::Svg));
    }
    let format = format.unwrap_or(ImageFormat::Webp);
    for (density, bytes) in files {
        report.bytes += bytes.len();
        report.layers += 1;
        sink(
            &format!("layers/{src}@{density}x.{}", format.extension()),
            bytes.as_slice(),
        )
        .map_err(LayerError::Sink)?;
    }
    Ok((src, format))
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

/// Bakes a decoded `.penpot` (see [`bake_fig_into`]).
///
/// # Errors
///
/// Fails when a frame cannot be rendered or `sink` fails.
pub fn bake_penpot_into(
    file: &PenpotFile,
    options: &BakeOptions,
    sink: &mut Sink<'_>,
) -> Result<BakeReport, BakeError> {
    let mut draw = PenpotDraw::new(file, Typesetter::new(options.fonts.clone()));
    bake_into(&penpot_ir(file), &mut draw, options, sink)
}

/// Bakes a design file of any format the bake draws into the folder `out`, created if needed.
///
/// # Errors
///
/// Fails when the file cannot be read or rendered, or a file cannot be written.
pub fn bake_file(bytes: &[u8], out: &Path, options: &BakeOptions) -> Result<BakeReport, BakeError> {
    create_dir(&out.join("layers"))?;
    bake_file_into(bytes, options, &mut |path, bytes| {
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
    let tree = Tree::new(file);
    let mut draw = FigmaDraw::new(&tree, Typesetter::new(options.fonts.clone()));
    bake_into(&to_ir(file), &mut draw, options, sink)
}

/// Bakes a decoded `.sketch` (see [`bake_fig_into`]); fonts embedded in the file (`fonts/…`)
/// join the ones in `options`.
///
/// # Errors
///
/// Fails when a frame cannot be rendered or `sink` fails.
pub fn bake_sketch_into(
    file: &SketchFile,
    options: &BakeOptions,
    sink: &mut Sink<'_>,
) -> Result<BakeReport, BakeError> {
    let mut fonts = options.fonts.clone();
    let mut embedded: Vec<_> = file
        .files
        .iter()
        .filter(|(name, _)| name.starts_with("fonts/"))
        .collect();
    embedded.sort_by_key(|(name, _)| *name);
    for (name, bytes) in embedded {
        // A file that is not a font is left out; the text that needed it reports it missing.
        let _ = fonts.add(name, bytes);
    }
    let mut draw = SketchDraw::new(file, Typesetter::new(fonts));
    bake_into(&sketch_ir(file), &mut draw, options, sink)
}

/// Bakes a decoded `.xd` (see [`bake_fig_into`]).
///
/// # Errors
///
/// Fails when a frame cannot be rendered or `sink` fails.
pub fn bake_xd_into(
    file: &XdFile,
    options: &BakeOptions,
    sink: &mut Sink<'_>,
) -> Result<BakeReport, BakeError> {
    let mut draw = XdDraw::new(file, Typesetter::new(options.fonts.clone()));
    bake_into(&xd_ir(file), &mut draw, options, sink)
}

/// Bakes a design file of any format the bake draws (`.fig`, `.sketch`, `.xd`, `.penpot`).
///
/// # Errors
///
/// Fails when the file is not one of those, cannot be decoded, or a frame cannot be rendered.
pub fn bake_file_into(
    bytes: &[u8],
    options: &BakeOptions,
    sink: &mut Sink<'_>,
) -> Result<BakeReport, BakeError> {
    let error = |e: &dyn std::fmt::Display| BakeError::Decode(e.to_string());
    match crate::detect(bytes) {
        Some(Format::Fig) => {
            let file = crate::figma::decode(bytes).map_err(|e| error(&e))?;
            bake_fig_into(&file, options, sink)
        }
        Some(Format::Sketch) => {
            let file = crate::sketch::decode(bytes).map_err(|e| error(&e))?;
            bake_sketch_into(&file, options, sink)
        }
        Some(Format::Penpot) => {
            let file = crate::penpot::decode(bytes).map_err(|e| error(&e))?;
            bake_penpot_into(&file, options, sink)
        }
        Some(Format::Xd) => {
            let file = crate::xd::decode(bytes).map_err(|e| error(&e))?;
            bake_xd_into(&file, options, sink)
        }
        None => Err(BakeError::Decode(
            "not a .fig, .sketch, .xd or .penpot file".to_owned(),
        )),
    }
}

/// Bakes a document whose frames `draw` renders, whatever its format.
///
/// # Errors
///
/// Fails when a frame cannot be rendered or `sink` fails.
pub fn bake_into(
    document: &Document,
    draw: &mut dyn Draw,
    options: &BakeOptions,
    sink: &mut Sink<'_>,
) -> Result<BakeReport, BakeError> {
    let mut report = BakeReport::default();
    let mut poster: Option<Vec<u8>> = None;
    // The highest density is encoded first: it decides the layer's format for every density.
    let densities = sorted_densities(options);
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

    let smart = smart_animated(document);
    let mut pages = Vec::new();
    let has_flows = document.pages.iter().any(|p| !p.flows.is_empty());
    for page in &document.pages {
        if has_flows && page.flows.is_empty() && !options.all_pages {
            report
                .skipped_pages
                .extend((!page.frames.is_empty()).then_some(page.name.clone()));
            continue;
        }
        let mut frames = Vec::new();
        for frame in &page.frames {
            if too_large(frame, &densities) {
                report.skipped_frames.push(frame.name.clone());
                continue;
            }
            let whole = draw
                .frame(frame)
                .ok_or_else(|| BakeError::MissingFrame(frame.id.clone()))?;
            let render_error = |source| BakeError::Render {
                frame: frame.id.clone(),
                source,
            };
            let placeholder = crate::bake::placeholder(&whole).map_err(render_error)?;
            let mut layers = Vec::new();
            for (markup, bounds, scrolls) in draw.layers(frame, &whole)? {
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
                let pixmap = render_pixmap(&whole, 1.0).map_err(render_error)?;
                poster = Some(encode(&pixmap, ImageFormat::Webp).map_err(render_error)?);
            }
            report.frames += 1;
            let components = bake_components(draw, frame, options, &mut report, sink)?;
            let parts = if smart.contains(frame.id.as_str()) {
                bake_parts(draw, frame, options, &mut report, sink)?
            } else {
                Vec::new()
            };
            frames.push(FolioFrame {
                frame: ir::Frame {
                    components: Vec::new(),
                    ..frame.clone()
                },
                layers,
                components,
                parts,
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
    report.missing_fonts.extend(draw.missing_fonts());

    if let Some(webp) = poster {
        sink("poster.webp", &webp)?;
    }

    let index = Index {
        folio: FOLIO_VERSION,
        ir_version: crate::IR_VERSION,
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

/// Whether an image of the frame (or of its scrolling content, when that is larger), at the top
/// density, would be longer than WebP can hold.
fn too_large(frame: &ir::Frame, densities: &[u8]) -> bool {
    let size = frame.scroll.as_ref().map_or(frame.bounds, |s| s.content);
    let densest = f32::from(densities.first().copied().unwrap_or(1));
    size.width.max(size.height) * densest > MAX_IMAGE_SIDE
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
