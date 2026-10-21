// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bakes prototypes into `.folio` folders (ADR 0004): renders nodes to SVG, rasterizes them with
//! `resvg` (ADR 0001), and writes the layers and `index.json`.

pub mod folio;
pub mod svg;

use resvg::{tiny_skia, usvg};

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("invalid SVG: {0}")]
    Svg(#[from] usvg::Error),
    #[error("empty or oversized image")]
    Size,
    #[error("cannot encode the PNG: {0}")]
    Png(String),
}

/// Rasterizes an SVG document to PNG, at `scale` times its size.
///
/// # Errors
///
/// Fails on invalid SVG, or when the scaled image is empty or too large.
pub fn rasterize(svg: &str, scale: f32) -> Result<Vec<u8>, RenderError> {
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
    pixmap
        .encode_png()
        .map_err(|e| RenderError::Png(e.to_string()))
}
