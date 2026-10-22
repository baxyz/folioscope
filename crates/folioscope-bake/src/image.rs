// SPDX-License-Identifier: AGPL-3.0-or-later

//! Layer image formats: lossless WebP by default, AVIF when it is much smaller (photos).

use resvg::tiny_skia::Pixmap;
use serde::Serialize;

use crate::RenderError;

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
}

impl ImageFormat {
    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            ImageFormat::Webp => "webp",
            ImageFormat::Avif => "avif",
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

/// Encodes `pixmap` in `format`.
///
/// # Errors
///
/// Fails when the encoder fails.
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
