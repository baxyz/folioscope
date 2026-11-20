// SPDX-License-Identifier: AGPL-3.0-or-later

//! Layer image formats (ADR 0004, ADR 0006): lossless WebP by default, AVIF when it is much
//! smaller (photos), and the layer's SVG itself when it holds no image or filter and weighs less
//! than its densest raster.

use resvg::tiny_skia::Pixmap;
use serde::Serialize;

use crate::bake::RenderError;

/// Below this size, lossless WebP is kept without trying AVIF: flat UI screens compress well
/// losslessly, and AVIF costs about a second per large layer.
const AVIF_THRESHOLD: usize = 64 * 1024;
/// AVIF replaces WebP only when it saves at least this share of the bytes.
const AVIF_MIN_GAIN: f32 = 0.3;
const AVIF_QUALITY: f32 = 80.0;
/// Slower than ravif's fastest, but much smaller: the bake runs once per file, at build time.
const AVIF_SPEED: u8 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageFormat {
    Webp,
    Avif,
    /// The layer's SVG, one file for every density.
    Svg,
}

impl ImageFormat {
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            ImageFormat::Webp => "webp",
            ImageFormat::Avif => "avif",
            ImageFormat::Svg => "svg",
        }
    }
}

/// Encodes `pixmap` as lossless WebP, or as AVIF when `allow_avif` and it is much smaller.
///
/// # Errors
///
/// Fails when an encoder fails.
pub fn encode_best(
    pixmap: &Pixmap,
    allow_avif: bool,
) -> Result<(ImageFormat, Vec<u8>), RenderError> {
    let webp = encode(pixmap, ImageFormat::Webp)?;
    if !allow_avif || webp.len() < AVIF_THRESHOLD {
        return Ok((ImageFormat::Webp, webp));
    }
    let avif = encode(pixmap, ImageFormat::Avif)?;
    // Lossless sizes as usize fit in f32 well enough for a ratio.
    #[allow(clippy::cast_precision_loss)]
    let gain = 1.0 - avif.len() as f32 / webp.len() as f32;
    Ok(if gain >= AVIF_MIN_GAIN {
        (ImageFormat::Avif, avif)
    } else {
        (ImageFormat::Webp, webp)
    })
}

/// Whether a layer may be served as its SVG (ADR 0006): it holds no image, which an SVG shown
/// by `<img>` can only carry inlined (no AVIF, no cache of its own), and no filter (blurs,
/// shadows), which browsers draw slowly and not quite as `resvg` does.
#[must_use]
pub fn svg_eligible(markup: &str) -> bool {
    !markup.contains("<image") && !markup.contains("<filter")
}

/// The size of `markup` once gzipped, as a web server sends it.
#[must_use]
pub fn gzipped_len(markup: &str) -> usize {
    use std::io::Write as _;
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    // Writing to a Vec cannot fail.
    let _ = encoder.write_all(markup.as_bytes());
    encoder.finish().map_or(markup.len(), |out| out.len())
}

/// Encodes `pixmap` in `format`, a raster one.
///
/// # Errors
///
/// Fails when the encoder fails, or `format` is [`ImageFormat::Svg`].
pub fn encode(pixmap: &Pixmap, format: ImageFormat) -> Result<Vec<u8>, RenderError> {
    let rgba = straight_rgba(pixmap);
    let error = |e: String| RenderError::Encode(e);
    match format {
        ImageFormat::Webp => {
            let mut out = Vec::new();
            image_webp::WebPEncoder::new(&mut out)
                .encode(
                    &rgba,
                    pixmap.width(),
                    pixmap.height(),
                    image_webp::ColorType::Rgba8,
                )
                .map_err(|e| error(e.to_string()))?;
            Ok(out)
        }
        ImageFormat::Avif => {
            let pixels: Vec<ravif::RGBA8> = rgba
                .as_chunks::<4>()
                .0
                .iter()
                .map(|&[r, g, b, a]| ravif::RGBA8::new(r, g, b, a))
                .collect();
            let image = ravif::Img::new(
                &pixels[..],
                pixmap.width() as usize,
                pixmap.height() as usize,
            );
            ravif::Encoder::new()
                .with_quality(AVIF_QUALITY)
                .with_speed(AVIF_SPEED)
                .encode_rgba(image)
                .map(|encoded| encoded.avif_file)
                .map_err(|e| error(e.to_string()))
        }
        ImageFormat::Svg => Err(error("SVG is not a raster format".to_owned())),
    }
}

/// RGBA with straight alpha: tiny-skia keeps pixels premultiplied, encoders expect them not to be.
#[must_use]
pub fn straight_rgba(pixmap: &Pixmap) -> Vec<u8> {
    pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect()
}
