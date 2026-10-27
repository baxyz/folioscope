// SPDX-License-Identifier: AGPL-3.0-or-later

//! Paints and effects: solid, gradient and image fills, drop shadows.

use std::fmt::Write;

use folioscope_core::kiwi::Value;
use helpers4::geometry::{Matrix, Point};

use super::Ctx;

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
            "IMAGE" => {
                let Some(Value::Bytes(hash)) = paint.get("image").and_then(|i| i.get("hash"))
                else {
                    return None;
                };
                let bytes = self.tree.image(hash)?;
                let aspect = match paint.get("imageScaleMode").and_then(Value::as_str) {
                    Some("FIT") => "xMidYMid meet",
                    Some("STRETCH") => "none",
                    _ => "xMidYMid slice",
                };
                let id = self.id("p");
                let _ = write!(
                    self.defs,
                    r#"<pattern id="{id}" patternUnits="userSpaceOnUse" width="{w}" height="{h}"><image href="data:{};base64,{}" width="{w}" height="{h}" preserveAspectRatio="{aspect}"/></pattern>"#,
                    mime(bytes),
                    helpers4::hex::base64_encode(bytes)
                );
                Some(format!(r#"fill="url(#{id})" fill-opacity="{opacity}""#))
            }
            // Radial, angular and diamond gradients: their first stop, for now.
            _ => {
                let stop = paint.get("stops")?.as_array().first()?;
                let (rgb, alpha) = color(stop.get("color")?);
                Some(format!(
                    r#"fill="{rgb}" fill-opacity="{}""#,
                    alpha * opacity
                ))
            }
        }
    }

    pub(super) fn drop_shadow(&mut self, effect: &Value) -> String {
        let id = self.id("f");
        let offset = |axis: &str| {
            effect
                .get("offset")
                .and_then(|o| o.get(axis))
                .and_then(Value::as_f32)
                .unwrap_or(0.0)
        };
        let radius = effect.get("radius").and_then(Value::as_f32).unwrap_or(0.0);
        let (rgb, alpha) = effect.get("color").map_or(("#000".to_owned(), 0.25), color);
        // Figma's blur radius is about twice the Gaussian standard deviation.
        let _ = write!(
            self.defs,
            r#"<filter id="{id}" x="-50%" y="-50%" width="200%" height="200%"><feDropShadow dx="{}" dy="{}" stdDeviation="{}" flood-color="{rgb}" flood-opacity="{alpha}"/></filter>"#,
            offset("x"),
            offset("y"),
            radius / 2.0
        );
        id
    }
}

/// `(css rgb, alpha)` from a Figma color with 0–1 channels.
fn color(c: &Value) -> (String, f32) {
    let ch = |k: &str| c.get(k).and_then(Value::as_f32).unwrap_or(0.0);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let byte = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    (
        format!("rgb({},{},{})", byte(ch("r")), byte(ch("g")), byte(ch("b"))),
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

// TODO(helpers4): media type sniffing (docs/helpers4-requests.md § 5).
pub(crate) fn mime(bytes: &[u8]) -> &'static str {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => "image/png",
        [0xff, 0xd8, ..] => "image/jpeg",
        [b'G', b'I', b'F', ..] => "image/gif",
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'E',
            b'B',
            b'P',
            ..,
        ] => "image/webp",
        _ => "application/octet-stream",
    }
}
