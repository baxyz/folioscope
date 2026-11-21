// SPDX-License-Identifier: AGPL-3.0-or-later

//! Paints and effects: solid, gradient (linear, radial, angular, diamond) and image fills;
//! drop and inner shadows, layer blur.

use std::fmt::Write;

use crate::kiwi::Value;
use helpers4::color::Rgba;
use helpers4::geometry::{Matrix, Point};

use super::{Ctx, visible};
use crate::bake::fx::{self, Shadow, Stop};

impl Ctx<'_, '_> {
    /// Fill attributes for a paint, adding gradients and patterns to `defs`.
    pub(super) fn paint(&mut self, paint: &Value, w: f32, h: f32) -> Option<String> {
        let opacity = paint.get("opacity").and_then(Value::as_f32).unwrap_or(1.0);
        match paint.get("type").and_then(Value::as_str)? {
            "SOLID" => {
                let (rgb, alpha) = color(paint.get("color")?);
                Some(format!(
                    r#"fill="{rgb}" fill-opacity="{}""#,
                    alpha * opacity
                ))
            }
            "GRADIENT_LINEAR" => {
                let ((x1, y1), (x2, y2)) = gradient_line(paint.get("transform"), w, h)?;
                let id = self.id("g");
                let _ = write!(
                    self.defs,
                    r#"<linearGradient id="{id}" gradientUnits="userSpaceOnUse" x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}">"#
                );
                for stop in paint.get("stops").map_or(&[][..], Value::as_array) {
                    let position = stop.get("position").and_then(Value::as_f32).unwrap_or(0.0);
                    let (rgb, alpha) = stop.get("color").map_or(("#000".to_owned(), 1.0), color);
                    let _ = write!(
                        self.defs,
                        r#"<stop offset="{position}" stop-color="{rgb}" stop-opacity="{alpha}"/>"#
                    );
                }
                self.defs.push_str("</linearGradient>");
                Some(format!(r#"fill="url(#{id})" fill-opacity="{opacity}""#))
            }
            "IMAGE" => self.image_paint(paint, w, h, opacity),
            kind @ ("GRADIENT_RADIAL" | "GRADIENT_ANGULAR" | "GRADIENT_DIAMOND") => {
                let m = unit_to_node(paint.get("transform"), w, h)?;
                let stops = stops(paint);
                let id = self.id("g");
                let fill = match kind {
                    "GRADIENT_RADIAL" => {
                        let _ = write!(
                            self.defs,
                            r#"<radialGradient id="{id}" gradientUnits="userSpaceOnUse" cx="0.5" cy="0.5" r="0.5" gradientTransform="matrix({} {} {} {} {} {})">"#,
                            m[0], m[1], m[2], m[3], m[4], m[5]
                        );
                        for stop in &stops {
                            let _ = write!(
                                self.defs,
                                r#"<stop offset="{}" stop-color="{}" stop-opacity="{}"/>"#,
                                stop.offset,
                                stop.color.to_rgb().to_css(),
                                stop.color.opacity()
                            );
                        }
                        self.defs.push_str("</radialGradient>");
                        format!(r#"fill="url(#{id})""#)
                    }
                    "GRADIENT_ANGULAR" => {
                        fx::angular(&mut self.defs, &id, &stops, m, f64::from(w), f64::from(h))
                    }
                    _ => fx::diamond(&mut self.defs, &id, &stops, m, f64::from(w), f64::from(h)),
                };
                Some(format!(r#"{fill} fill-opacity="{opacity}""#))
            }
            _ => None,
        }
    }

    /// An image fill: stretched, fit, filled, tiled, or cropped (stretched with a transform).
    fn image_paint(&mut self, paint: &Value, w: f32, h: f32, opacity: f32) -> Option<String> {
        let Some(Value::Bytes(hash)) = paint.get("image").and_then(|i| i.get("hash")) else {
            return None;
        };
        let bytes = self.tree.image(hash)?;
        let href = format!(
            "data:{};base64,{}",
            mime(bytes),
            helpers4::hex::base64_encode(bytes)
        );
        let id = self.id("p");
        let mode = paint.get("imageScaleMode").and_then(Value::as_str);
        let number = |k: &str| paint.get(k).and_then(Value::as_f32);
        match mode {
            // Tiles at the image's own size, times its scale, from the node's corner.
            Some("TILE") => {
                let (iw, ih) = number("originalImageWidth")
                    .zip(number("originalImageHeight"))
                    .or_else(|| image_size(bytes))?;
                let scale = number("scale").unwrap_or(1.0);
                let (tw, th) = (iw * scale, ih * scale);
                let _ = write!(
                    self.defs,
                    r#"<pattern id="{id}" patternUnits="userSpaceOnUse" width="{tw}" height="{th}"><image href="{href}" width="{tw}" height="{th}" preserveAspectRatio="none"/></pattern>"#
                );
            }
            // A stretched image with a transform is a crop: the image's unit square,
            // placed by the inverse of the paint transform.
            Some("STRETCH") if paint.get("transform").is_some() => {
                let m = unit_to_node(paint.get("transform"), w, h)?;
                let _ = write!(
                    self.defs,
                    r#"<pattern id="{id}" patternUnits="userSpaceOnUse" width="{w}" height="{h}"><image href="{href}" width="1" height="1" preserveAspectRatio="none" transform="matrix({} {} {} {} {} {})"/></pattern>"#,
                    m[0], m[1], m[2], m[3], m[4], m[5]
                );
            }
            _ => {
                let aspect = match mode {
                    Some("FIT") => "xMidYMid meet",
                    Some("STRETCH") => "none",
                    _ => "xMidYMid slice",
                };
                let _ = write!(
                    self.defs,
                    r#"<pattern id="{id}" patternUnits="userSpaceOnUse" width="{w}" height="{h}"><image href="{href}" width="{w}" height="{h}" preserveAspectRatio="{aspect}"/></pattern>"#
                );
            }
        }
        Some(format!(r#"fill="url(#{id})" fill-opacity="{opacity}""#))
    }

    /// Opens the groups that draw a node's visible effects: drop shadows, inner shadows and layer
    /// blur. Returns how many it opened. Background blurs are not drawn.
    pub(super) fn effects(&mut self, node: &Value) -> usize {
        let effects: Vec<&Value> = node
            .get("effects")
            .map_or(&[][..], Value::as_array)
            .iter()
            .filter(|e| visible(e))
            .collect();

        let shadows = |name: &str| -> Vec<Shadow> {
            effects
                .iter()
                .filter(|e| kind(e) == name)
                .map(|e| shadow(e))
                .collect()
        };
        let mut open = 0;
        let drops = shadows("DROP_SHADOW");
        if !drops.is_empty() {
            let id = self.id("f");
            fx::drop_shadows(&mut self.defs, &id, &drops);
            let _ = write!(self.body, r#"<g filter="url(#{id})">"#);
            open += 1;
        }
        if let Some(blur) = effects.iter().find(|e| kind(e) == "FOREGROUND_BLUR") {
            let id = self.id("f");
            let radius = blur.get("radius").and_then(Value::as_f32).unwrap_or(0.0);
            fx::layer_blur(&mut self.defs, &id, f64::from(radius));
            let _ = write!(self.body, r#"<g filter="url(#{id})">"#);
            open += 1;
        }
        let inner = shadows("INNER_SHADOW");
        if !inner.is_empty() {
            let id = self.id("f");
            fx::inner_shadows(&mut self.defs, &id, &inner);
            let _ = write!(self.body, r#"<g filter="url(#{id})">"#);
            open += 1;
        }
        open
    }
}

/// An effect's `type`.
fn kind(e: &Value) -> &str {
    e.get("type").and_then(Value::as_str).unwrap_or("")
}

/// A Figma shadow effect.
fn shadow(effect: &Value) -> Shadow {
    let number = |v: Option<&Value>| v.and_then(Value::as_f32).map_or(0.0, f64::from);
    let offset = effect.get("offset");
    Shadow {
        dx: number(offset.and_then(|o| o.get("x"))),
        dy: number(offset.and_then(|o| o.get("y"))),
        blur: number(effect.get("radius")),
        spread: number(effect.get("spread")),
        color: effect.get("color").map_or(Rgba::new(0, 0, 0, 64), rgba),
    }
}

/// A Figma colour with 0–1 channels and its alpha.
fn rgba(c: &Value) -> Rgba {
    let ch = |k: &str| c.get(k).and_then(Value::as_f32).map_or(0.0, f64::from);
    Rgba::from_unit(ch("r"), ch("g"), ch("b"), ch("a"))
}

/// A gradient paint's stops.
fn stops(paint: &Value) -> Vec<Stop> {
    paint
        .get("stops")
        .map_or(&[][..], Value::as_array)
        .iter()
        .map(|stop| Stop {
            offset: stop
                .get("position")
                .and_then(Value::as_f32)
                .map_or(0.0, f64::from),
            color: stop.get("color").map_or(Rgba::new(0, 0, 0, 255), rgba),
        })
        .collect()
}

/// The matrix taking the gradient's unit square to the node's pixels: the inverse of the paint
/// transform (node, normalized to 0–1, to gradient space), then the node's size.
fn unit_to_node(transform: Option<&Value>, width: f32, height: f32) -> Option<[f64; 6]> {
    let entry = |name: &str, default: f32| {
        f64::from(
            transform
                .and_then(|t| t.get(name))
                .and_then(Value::as_f32)
                .unwrap_or(default),
        )
    };
    let to_gradient = Matrix::new(
        entry("m00", 1.0),
        entry("m10", 0.0),
        entry("m01", 0.0),
        entry("m11", 1.0),
        entry("m02", 0.0),
        entry("m12", 0.0),
    );
    let to_node = to_gradient.invert()?;
    let (w, h) = (f64::from(width), f64::from(height));
    Some([
        to_node.a * w,
        to_node.b * h,
        to_node.c * w,
        to_node.d * h,
        to_node.e * w,
        to_node.f * h,
    ])
}

/// `(css rgb, alpha)` from a Figma color with 0–1 channels.
fn color(c: &Value) -> (String, f32) {
    let ch = |k: &str| c.get(k).and_then(Value::as_f32).unwrap_or(0.0);
    let unit = |k: &str| f64::from(ch(k));
    (
        Rgba::from_unit(unit("r"), unit("g"), unit("b"), 1.0)
            .to_rgb()
            .to_css(),
        ch("a"),
    )
}

/// Start and end of a linear gradient, in node pixels. The paint transform maps node space
/// (normalized to 0–1) to gradient space, where the gradient runs from (0, 0.5) to (1, 0.5).
#[allow(clippy::cast_possible_truncation)] // design coordinates fit in f32
fn gradient_line(
    transform: Option<&Value>,
    width: f32,
    height: f32,
) -> Option<((f32, f32), (f32, f32))> {
    let entry = |name: &str, default: f32| {
        f64::from(
            transform
                .and_then(|t| t.get(name))
                .and_then(Value::as_f32)
                .unwrap_or(default),
        )
    };
    let to_gradient = Matrix::new(
        entry("m00", 1.0),
        entry("m10", 0.0),
        entry("m01", 0.0),
        entry("m11", 1.0),
        entry("m02", 0.0),
        entry("m12", 0.0),
    );
    let to_node = to_gradient.invert()?;
    let in_node = |gx: f64, gy: f64| {
        let p = to_node.apply(Point::new(gx, gy));
        (p.x as f32 * width, p.y as f32 * height)
    };
    Some((in_node(0.0, 0.5), in_node(1.0, 0.5)))
}

/// The pixel size of a PNG or JPEG image, from its header.
pub(crate) fn image_size(bytes: &[u8]) -> Option<(f32, f32)> {
    let be16 = |at: usize| {
        Some(f32::from(u16::from_be_bytes([
            *bytes.get(at)?,
            *bytes.get(at + 1)?,
        ])))
    };
    let be32 = |at: usize| {
        let b: [u8; 4] = bytes.get(at..at + 4)?.try_into().ok()?;
        #[allow(clippy::cast_precision_loss)] // image sides fit in f32
        Some(u32::from_be_bytes(b) as f32)
    };
    if bytes.starts_with(b"\x89PNG") {
        return Some((be32(16)?, be32(20)?));
    }
    if bytes.starts_with(&[0xFF, 0xD8]) {
        // Walk the JPEG segments up to a start of frame (SOF0 to SOF15, but DHT, JPG and DAC).
        let mut at = 2;
        while at + 9 < bytes.len() {
            if bytes[at] != 0xFF {
                return None;
            }
            let marker = bytes[at + 1];
            let length = usize::from(u16::from_be_bytes([bytes[at + 2], bytes[at + 3]]));
            if (0xC0..=0xCF).contains(&marker) && !matches!(marker, 0xC4 | 0xC8 | 0xCC) {
                return Some((be16(at + 7)?, be16(at + 5)?));
            }
            at += 2 + length;
        }
    }
    None
}

pub(crate) fn mime(bytes: &[u8]) -> &'static str {
    helpers4::bytes::media_type_of(bytes).unwrap_or("application/octet-stream")
}
