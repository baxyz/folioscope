// SPDX-License-Identifier: AGPL-3.0-or-later

//! Effects and paints shared by the four renderers, written as SVG for `resvg` (and the browser,
//! for SVG layers): drop and inner shadows, layer blur, blend modes, and the gradients SVG lacks
//! (angular, diamond), drawn as many thin solid shapes.

use std::fmt::Write;

use helpers4::color::Rgba;
use helpers4::geometry::Matrix;

/// A gradient stop: its offset (0–1) and colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stop {
    pub offset: f64,
    pub color: Rgba,
}

/// Slices of an angular gradient, and rings of a diamond one: enough for smooth colour at 2x.
const STEPS: u32 = 180;

/// An SVG `matrix(…)` for a 2×3 transform.
fn matrix(m: [f64; 6]) -> String {
    format!(
        "matrix({} {} {} {} {} {})",
        m[0], m[1], m[2], m[3], m[4], m[5]
    )
}

/// The colour of a gradient at `t` (0–1), between its stops.
#[must_use]
pub fn color_at(stops: &[Stop], t: f64) -> Rgba {
    let Some(first) = stops.first() else {
        return Rgba::new(0, 0, 0, 0);
    };
    if t <= first.offset {
        return first.color;
    }
    for pair in stops.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if t <= b.offset {
            let span = (b.offset - a.offset).max(f64::EPSILON);
            let k = ((t - a.offset) / span).clamp(0.0, 1.0);
            let [ar, ag, ab, aa] = a.color.to_unit();
            let [br, bg, bb, ba] = b.color.to_unit();
            let mix = |x: f64, y: f64| x + (y - x) * k;
            return Rgba::from_unit(mix(ar, br), mix(ag, bg), mix(ab, bb), mix(aa, ba));
        }
    }
    stops.last().map_or(first.color, |s| s.color)
}

/// `fill` and `fill-opacity` attributes for a colour.
#[must_use]
pub fn fill(color: Rgba) -> String {
    format!(
        r#"fill="{}" fill-opacity="{}""#,
        color.to_rgb().to_css(),
        color.opacity()
    )
}

/// Writes an angular (conic) gradient into `defs` as pattern `id`, and returns its fill.
///
/// `transform` maps the gradient's unit square (centre `0.5, 0.5`, angle 0 towards `1, 0.5`,
/// turning clockwise) to the shape's pixels, as an SVG `matrix(a b c d e f)`; `width` and `height`
/// are the shape's.
pub fn angular(
    defs: &mut String,
    id: &str,
    stops: &[Stop],
    transform: [f64; 6],
    width: f64,
    height: f64,
) -> String {
    let _ = write!(
        defs,
        r#"<pattern id="{id}" patternUnits="userSpaceOnUse" width="{width}" height="{height}"><g transform="{}">"#,
        matrix(transform)
    );
    // Wedges from the centre, reaching far past the unit square so they cover any shape.
    let count = f64::from(STEPS);
    let step = std::f64::consts::TAU / count;
    for i in 0..STEPS {
        let i = f64::from(i);
        let offset = (i + 0.5) / count;
        // Each wedge overlaps the next one's start, and is outlined in its own colour, so no seam
        // of the background shows between them.
        let (from, to) = (i * step, (i + 2.0) * step);
        let point = |angle: f64| (0.5 + 4.0 * angle.cos(), 0.5 + 4.0 * angle.sin());
        let (x1, y1) = point(from);
        let (x2, y2) = point(to);
        let color = color_at(stops, offset);
        let _ = write!(
            defs,
            r#"<path d="M0.5 0.5L{x1:.4} {y1:.4}L{x2:.4} {y2:.4}Z" {} stroke="{}" stroke-opacity="{}" stroke-width="0.004"/>"#,
            fill(color),
            color.to_rgb().to_css(),
            color.opacity()
        );
    }
    defs.push_str("</g></pattern>");
    format!(r#"fill="url(#{id})""#)
}

/// Writes a diamond gradient into `defs` as pattern `id`, and returns its fill: the colour goes by
/// the sum of the distances to the centre along both axes, reaching the last stop at the unit
/// square's edges. `transform` as for [`angular`].
pub fn diamond(
    defs: &mut String,
    id: &str,
    stops: &[Stop],
    transform: [f64; 6],
    width: f64,
    height: f64,
) -> String {
    let last = stops.last().map_or(Rgba::new(0, 0, 0, 0), |s| s.color);
    let _ = write!(
        defs,
        r#"<pattern id="{id}" patternUnits="userSpaceOnUse" width="{width}" height="{height}"><g transform="{}"><rect x="-4" y="-4" width="9" height="9" {}/>"#,
        matrix(transform),
        fill(last)
    );
    // Concentric diamonds, the outermost first.
    for i in (0..STEPS).rev() {
        let offset = (f64::from(i) + 1.0) / f64::from(STEPS);
        let r = 0.5 * offset;
        let _ = write!(
            defs,
            r#"<path d="M0.5 {:.5}L{:.5} 0.5L0.5 {:.5}L{:.5} 0.5Z" {}/>"#,
            0.5 - r,
            0.5 + r,
            0.5 + r,
            0.5 - r,
            fill(color_at(stops, offset))
        );
    }
    defs.push_str("</g></pattern>");
    format!(r#"fill="url(#{id})""#)
}

/// Where a stroke sits on its path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrokeAlign {
    Center,
    Inside,
    Outside,
}

/// Writes a stroke of `path` into `body`. `attributes` are its paint (`stroke`, `stroke-opacity`,
/// dashes…). Inside and outside strokes are drawn twice as wide, clipped to the path's inside or
/// outside with clip path `clip`, written into `defs`.
pub fn stroke(
    defs: &mut String,
    body: &mut String,
    clip: &str,
    path: &str,
    attributes: &str,
    width: f64,
    align: StrokeAlign,
) {
    let clip_path = match align {
        StrokeAlign::Center => {
            let _ = write!(
                body,
                r#"<path d="{path}" fill="none" {attributes} stroke-width="{width}"/>"#
            );
            return;
        }
        StrokeAlign::Inside => format!(r#"<path d="{path}"/>"#),
        StrokeAlign::Outside => format!(
            r#"<path clip-rule="evenodd" d="M-100000 -100000H100000V100000H-100000Z{path}"/>"#
        ),
    };
    let _ = write!(defs, r#"<clipPath id="{clip}">{clip_path}</clipPath>"#);
    let _ = write!(
        body,
        r#"<g clip-path="url(#{clip})"><path d="{path}" fill="none" {attributes} stroke-width="{}"/></g>"#,
        width * 2.0
    );
}

/// What a background blur needs while a frame is drawn. SVG cannot read what is behind an
/// element, so a renderer draws the frame a second time, stopping before the blurred node, and
/// shows that drawing blurred inside the node's outline ([`backdrop`]).
#[derive(Debug, Default, Clone)]
pub struct Backdrop {
    /// Prefix of the ids of a second drawing, unique in the document.
    pub prefix: String,
    /// In a second drawing: the node to stop before (its id), and whether it was reached.
    pub stop_at: Option<String>,
    pub stopped: bool,
    /// Each open node's transform from the frame, the innermost last.
    pub matrices: Vec<Matrix>,
}

impl Backdrop {
    /// Whether node `id` must not be drawn: it is where a second drawing stops, or comes after.
    pub fn stops_at(&mut self, id: &str) -> bool {
        if self.stopped || self.stop_at.as_deref() == Some(id) {
            self.stopped = true;
        }
        self.stopped
    }

    /// Enters a node whose transform from its parent is `local`.
    pub fn enter(&mut self, local: Matrix) {
        let parent = self.matrices.last().copied().unwrap_or(Matrix::IDENTITY);
        self.matrices.push(parent.multiply(local));
    }

    pub fn leave(&mut self) {
        self.matrices.pop();
    }

    /// The transform from the current node back to the frame's coordinates.
    #[must_use]
    pub fn to_node(&self) -> Option<Matrix> {
        self.matrices
            .last()
            .copied()
            .unwrap_or(Matrix::IDENTITY)
            .invert()
    }
}

/// Writes a background blur: `behind` (the frame drawn up to the node, its defs and body, with
/// ids prefixed by `prefix`) blurred by `radius`, clipped to `outline` (the node's shape, in its
/// coordinates). `to_node` brings the frame's coordinates into the node's.
#[allow(clippy::too_many_arguments)]
pub fn backdrop(
    defs: &mut String,
    body: &mut String,
    prefix: &str,
    behind: (&str, &str),
    outline: &str,
    (clip, blur): (&str, &str),
    radius: f64,
    to_node: Matrix,
) {
    let _ = write!(
        defs,
        r#"{}<g id="{prefix}frame">{}</g><clipPath id="{clip}">{outline}</clipPath>"#,
        behind.0, behind.1
    );
    layer_blur(defs, blur, radius);
    let m = to_node;
    let _ = write!(
        body,
        r##"<g clip-path="url(#{clip})"><g transform="matrix({} {} {} {} {} {})"><g filter="url(#{blur})"><use href="#{prefix}frame"/></g></g></g>"##,
        m.a, m.b, m.c, m.d, m.e, m.f
    );
}

/// A shadow, as the design tools describe it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadow {
    pub dx: f64,
    pub dy: f64,
    /// The design tool's blur radius: about twice the Gaussian standard deviation.
    pub blur: f64,
    /// Grows (or shrinks, when negative) the shape before blurring.
    pub spread: f64,
    pub color: Rgba,
}

/// Writes filter `id`: every drop shadow under the shape, the first one on top.
pub fn drop_shadows(defs: &mut String, id: &str, shadows: &[Shadow]) {
    let _ = write!(
        defs,
        r#"<filter id="{id}" x="-50%" y="-50%" width="200%" height="200%" color-interpolation-filters="sRGB">"#
    );
    for (i, s) in shadows.iter().enumerate() {
        let source = spread(defs, "SourceAlpha", s.spread, i);
        let _ = write!(
            defs,
            r#"<feOffset in="{source}" dx="{}" dy="{}" result="o{i}"/><feGaussianBlur in="o{i}" stdDeviation="{}" result="b{i}"/><feFlood flood-color="{}" flood-opacity="{}"/><feComposite in2="b{i}" operator="in" result="s{i}"/>"#,
            s.dx,
            s.dy,
            s.blur / 2.0,
            s.color.to_rgb().to_css(),
            s.color.opacity()
        );
    }
    defs.push_str("<feMerge>");
    for i in (0..shadows.len()).rev() {
        let _ = write!(defs, r#"<feMergeNode in="s{i}"/>"#);
    }
    defs.push_str(r#"<feMergeNode in="SourceGraphic"/></feMerge></filter>"#);
}

/// Writes filter `id`: the shape, with every inner shadow drawn inside it.
pub fn inner_shadows(defs: &mut String, id: &str, shadows: &[Shadow]) {
    let _ = write!(
        defs,
        r#"<filter id="{id}" x="-50%" y="-50%" width="200%" height="200%" color-interpolation-filters="sRGB">"#
    );
    for (i, s) in shadows.iter().enumerate() {
        // The shape's alpha, shrunk by the spread, offset and blurred: where it does not reach,
        // inside the shape, is the shadow.
        let source = spread(defs, "SourceAlpha", -s.spread, i);
        let _ = write!(
            defs,
            r#"<feOffset in="{source}" dx="{}" dy="{}" result="o{i}"/><feGaussianBlur in="o{i}" stdDeviation="{}" result="b{i}"/><feComposite in="SourceAlpha" in2="b{i}" operator="arithmetic" k1="-1" k2="1" k3="0" k4="0" result="m{i}"/><feFlood flood-color="{}" flood-opacity="{}"/><feComposite in2="m{i}" operator="in" result="s{i}"/>"#,
            s.dx,
            s.dy,
            s.blur / 2.0,
            s.color.to_rgb().to_css(),
            s.color.opacity()
        );
    }
    defs.push_str(r#"<feMerge><feMergeNode in="SourceGraphic"/>"#);
    for i in 0..shadows.len() {
        let _ = write!(defs, r#"<feMergeNode in="s{i}"/>"#);
    }
    defs.push_str("</feMerge></filter>");
}

/// The input grown (or shrunk) by `spread` pixels, as a filter result name.
fn spread(defs: &mut String, input: &str, spread: f64, i: usize) -> String {
    if spread.abs() < f64::EPSILON {
        return input.to_owned();
    }
    let operator = if spread > 0.0 { "dilate" } else { "erode" };
    let _ = write!(
        defs,
        r#"<feMorphology in="{input}" operator="{operator}" radius="{}" result="r{i}"/>"#,
        spread.abs()
    );
    format!("r{i}")
}

/// Writes filter `id`: a Gaussian blur of `radius` (the design tool's: about twice the standard
/// deviation).
pub fn layer_blur(defs: &mut String, id: &str, radius: f64) {
    let _ = write!(
        defs,
        r#"<filter id="{id}" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="{}"/></filter>"#,
        radius / 2.0
    );
}

/// The CSS `mix-blend-mode` for a blend mode, by any of the names the design tools use
/// (`MULTIPLY`, `multiply`, `color-burn`, `colorBurn`, Sketch's numbers through
/// [`sketch_blend`]); `None` for normal or unknown modes.
#[must_use]
pub fn blend(name: &str) -> Option<&'static str> {
    let key: String = name
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    Some(match key.as_str() {
        "multiply" => "multiply",
        "screen" => "screen",
        "overlay" => "overlay",
        "darken" | "plusdarker" | "lineardarken" | "linearburn" => "darken",
        "lighten" | "pluslighter" | "lineardodge" | "lighter" => "lighten",
        "colordodge" => "color-dodge",
        "colorburn" => "color-burn",
        "hardlight" => "hard-light",
        "softlight" => "soft-light",
        "difference" => "difference",
        "exclusion" => "exclusion",
        "hue" => "hue",
        "saturation" => "saturation",
        "color" => "color",
        "luminosity" => "luminosity",
        _ => return None,
    })
}

/// Sketch stores blend modes as numbers (`contextSettings.blendMode`).
#[must_use]
pub fn sketch_blend(mode: i64) -> Option<&'static str> {
    blend(match mode {
        1 => "darken",
        2 => "multiply",
        3 => "color-burn",
        4 => "lighten",
        5 => "screen",
        6 => "color-dodge",
        7 => "overlay",
        8 => "soft-light",
        9 => "hard-light",
        10 => "difference",
        11 => "exclusion",
        12 => "hue",
        13 => "saturation",
        14 => "color",
        15 => "luminosity",
        16 => "plus-darker",
        17 => "plus-lighter",
        _ => return None,
    })
}

/// A `style` attribute for a blend mode, or nothing.
#[must_use]
pub fn blend_style(mode: Option<&str>) -> String {
    mode.map_or_else(String::new, |m| format!(r#" style="mix-blend-mode:{m}""#))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stops() -> Vec<Stop> {
        vec![
            Stop {
                offset: 0.0,
                color: Rgba::new(255, 0, 0, 255),
            },
            Stop {
                offset: 1.0,
                color: Rgba::new(0, 0, 255, 255),
            },
        ]
    }

    #[test]
    fn colours_between_stops() {
        assert_eq!(color_at(&stops(), 0.0), Rgba::new(255, 0, 0, 255));
        assert_eq!(color_at(&stops(), 1.0), Rgba::new(0, 0, 255, 255));
        assert_eq!(color_at(&stops(), 0.5), Rgba::new(128, 0, 128, 255));
        assert_eq!(color_at(&[], 0.5), Rgba::new(0, 0, 0, 0));
    }

    #[test]
    fn effects_render() {
        let mut defs = String::new();
        let shadow = Shadow {
            dx: 0.0,
            dy: 4.0,
            blur: 8.0,
            spread: 2.0,
            color: Rgba::new(0, 0, 0, 64),
        };
        drop_shadows(&mut defs, "d", &[shadow]);
        inner_shadows(&mut defs, "i", &[shadow]);
        layer_blur(&mut defs, "b", 8.0);
        let a = angular(
            &mut defs,
            "a",
            &stops(),
            [40.0, 0.0, 0.0, 40.0, 0.0, 0.0],
            40.0,
            40.0,
        );
        let d = diamond(
            &mut defs,
            "m",
            &stops(),
            [40.0, 0.0, 0.0, 40.0, 0.0, 0.0],
            40.0,
            40.0,
        );
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="60"><defs>{defs}</defs><rect x="10" y="10" width="40" height="40" fill="red" filter="url(#d)"/><rect x="60" y="10" width="40" height="40" fill="red" filter="url(#i)"/><rect x="110" y="10" width="40" height="40" {a}/><rect width="40" height="40" {d} style="mix-blend-mode:multiply"/></svg>"#
        );
        crate::bake::rasterize(&svg, 1.0).unwrap();
    }

    #[test]
    fn blend_modes_by_any_name() {
        assert_eq!(blend("MULTIPLY"), Some("multiply"));
        assert_eq!(blend("color-burn"), Some("color-burn"));
        assert_eq!(blend("COLOR_DODGE"), Some("color-dodge"));
        assert_eq!(blend("PASS_THROUGH"), None);
        assert_eq!(blend("NORMAL"), None);
        assert_eq!(sketch_blend(2), Some("multiply"));
        assert_eq!(sketch_blend(0), None);
    }
}
