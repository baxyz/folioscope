// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bakes prototypes into `.folio` folders (ADR 0004): renders nodes to SVG, rasterizes them with
//! `resvg` (ADR 0001), and writes the layers and `index.json`.

pub mod draw;
pub mod folio;
pub mod fonts;
pub mod image;
pub mod penpot;
pub mod sketch;
pub mod svg;
pub mod text;
pub mod xd;

use resvg::{tiny_skia, usvg};

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("invalid SVG: {0}")]
    Svg(#[from] usvg::Error),
    #[error("empty or oversized image")]
    Size,
    #[error("cannot encode the image: {0}")]
    Encode(String),
}

/// Rasterizes an SVG document to PNG, at `scale` times its size.
///
/// # Errors
///
/// Fails on invalid SVG, or when the scaled image is empty or too large.
pub fn rasterize(svg: &str, scale: f32) -> Result<Vec<u8>, RenderError> {
    render_pixmap(svg, scale)?
        .encode_png()
        .map_err(|e| RenderError::Encode(e.to_string()))
}

/// A [ThumbHash](https://evanw.github.io/thumbhash/) of the SVG: a blurred placeholder of about
/// 25 bytes, base64-encoded, that the
/// player shows while the real layers load.
///
/// # Errors
///
/// Fails on invalid SVG or an empty image.
pub fn placeholder(svg: &str) -> Result<String, RenderError> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default())?;
    let size = tree.size();
    // ThumbHash takes images of at most 100x100.
    let scale = (100.0 / size.width().max(size.height())).min(1.0);
    let pixmap = render_pixmap(svg, scale)?;
    let rgba = image::straight_rgba(&pixmap);
    let hash =
        thumbhash::rgba_to_thumb_hash(pixmap.width() as usize, pixmap.height() as usize, &rgba);
    Ok(helpers4::hex::base64_encode(&hash))
}

/// Renders an SVG document at `scale` times its size.
///
/// # Errors
///
/// Fails on invalid SVG, or when the scaled image is empty or too large.
pub fn render_pixmap(svg: &str, scale: f32) -> Result<tiny_skia::Pixmap, RenderError> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default())?;
    let size = tree
        .size()
        .to_int_size()
        .scale_by(scale)
        .ok_or(RenderError::Size)?;
    let mut pixmap =
        tiny_skia::Pixmap::new(size.width(), size.height()).ok_or(RenderError::Size)?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}
