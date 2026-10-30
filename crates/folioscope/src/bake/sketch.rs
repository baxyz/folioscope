// SPDX-License-Identifier: AGPL-3.0-or-later

//! Draws `.sketch` artboards as SVG, from their JSON layers (see `docs/format-sketch.md`).
//!
//! Handled: groups, shapes from their curve points (rectangles, ovals, paths, boolean groups as
//! one path), colour, gradient and image fills, borders, drop shadows, bitmaps, symbol instances
//! (text overrides included) and text, laid out with the fonts given (ADR 0011). Not yet: masks,
//! blurs, blend modes, inner shadows, per-run text styles (a text takes its first run's style).

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;

use crate::ir::{self, Rect};
use crate::sketch::SketchFile;
use serde_json::Value;

use crate::bake::draw::Draw;
use crate::bake::folio::BakeError;
use crate::bake::fonts::weight_of_style;
use crate::bake::svg::mime;
use crate::bake::text::{Align, Paragraph, Typesetter};

/// Draws the artboards of a decoded `.sketch`.
pub struct SketchDraw<'f> {
    file: &'f SketchFile,
    artboards: HashMap<&'f str, &'f Value>,
    masters: HashMap<&'f str, &'f Value>,
    typesetter: Typesetter,
    missing_fonts: BTreeSet<String>,
}

impl<'f> SketchDraw<'f> {
    /// Fonts embedded in the file (`fonts/…`) join the ones in `typesetter`'s library.
    #[must_use]
    pub fn new(file: &'f SketchFile, typesetter: Typesetter) -> Self {
        let layers = |l: &'f Value| l["layers"].as_array().into_iter().flatten();
        let artboards = file
            .pages
            .iter()
            .flat_map(layers)
            .filter(|l| l["_class"] == "artboard")
            .filter_map(|a| Some((a["do_objectID"].as_str()?, a)))
            .collect();
        let local = file.pages.iter().flat_map(layers);
        let foreign = file.document["foreignSymbols"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|f| &f["symbolMaster"]);
        let masters = local
            .chain(foreign)
            .filter(|l| l["_class"] == "symbolMaster")
            .filter_map(|m| Some((m["symbolID"].as_str()?, m)))
            .collect();
        Self {
            file,
            artboards,
            masters,
            typesetter,
            missing_fonts: BTreeSet::new(),
        }
    }
}

impl Draw for SketchDraw<'_> {
    fn frame(&mut self, frame: &ir::Frame) -> Option<String> {
        let artboard = *self.artboards.get(frame.id.as_str())?;
        let (width, height) = (frame.bounds.width, frame.bounds.height);
        let mut svg = Svg {
            file: self.file,
            masters: &self.masters,
            typesetter: &mut self.typesetter,
            missing_fonts: &mut self.missing_fonts,
            defs: String::new(),
            body: String::new(),
            next_id: 0,
            overrides: Vec::new(),
        };
        // An artboard without a background color shows white, as in Sketch's player.
        let background = if artboard["hasBackgroundColor"] == true {
            color(&artboard["backgroundColor"])
        } else {
            ("rgb(255,255,255)".to_owned(), 1.0)
        };
        let _ = write!(
            svg.body,
            r#"<rect width="{width}" height="{height}" fill="{}" fill-opacity="{}"/>"#,
            background.0, background.1
        );
        svg.children(artboard);
        Some(format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}"><defs>{}</defs>{}</svg>"#,
            svg.defs, svg.body
        ))
    }

    fn layers(
        &mut self,
        frame: &ir::Frame,
        whole: &str,
    ) -> Result<Vec<(String, Rect, bool)>, BakeError> {
        let bounds = Rect {
            x: 0.0,
            y: 0.0,
            width: frame.bounds.width,
            height: frame.bounds.height,
        };
        Ok(vec![(whole.to_owned(), bounds, false)])
    }

    fn state(&mut self, _: &ir::Component, _: &ir::ComponentState) -> Option<String> {
        // Sketch has no interactive components.
        None
    }

    fn missing_fonts(&mut self) -> BTreeSet<String> {
        std::mem::take(&mut self.missing_fonts)
    }
}

/// One artboard being drawn.
struct Svg<'d, 'f> {
    file: &'f SketchFile,
    masters: &'d HashMap<&'f str, &'f Value>,
    typesetter: &'d mut Typesetter,
    missing_fonts: &'d mut BTreeSet<String>,
    defs: String,
    body: String,
    next_id: u32,
    /// Overrides of the symbol instances being drawn, outermost first, with the ids of the
    /// nested instances since each: Sketch keys them `<id>/<id>_<property>`.
    overrides: Vec<(&'f [Value], Vec<String>)>,
}

impl<'f> Svg<'_, 'f> {
    fn id(&mut self, prefix: &str) -> String {
        self.next_id += 1;
        format!("{prefix}{}", self.next_id)
    }

    fn layer(&mut self, layer: &'f Value) {
        if layer["isVisible"] == false {
            return;
        }
        let (width, height) = (n(&layer["frame"], "width"), n(&layer["frame"], "height"));
        let transform = transform(layer);
        let opacity = layer["style"]["contextSettings"]["opacity"]
            .as_f64()
            .unwrap_or(1.0);
        let _ = write!(
            self.body,
            r#"<g transform="{transform}" opacity="{opacity}">"#
        );
        let shadow = self.shadow(&layer["style"]);
        if let Some(filter) = &shadow {
            let _ = write!(self.body, r#"<g filter="url(#{filter})">"#);
        }
        match layer["_class"].as_str().unwrap_or("") {
            "group" | "artboard" => self.children(layer),
            "shapeGroup" => {
                // A boolean group draws as one path of its shapes, with its own style.
                if let Some(path) = outline(layer) {
                    self.paint(&path, &layer["style"], width, height);
                }
            }
            "rectangle" | "oval" | "shapePath" | "triangle" | "star" | "polygon" => {
                if let Some(outline) = shape_path(layer) {
                    self.paint(&outline, &layer["style"], width, height);
                }
            }
            "bitmap" => {
                if let Some(href) = self.image(&layer["image"]) {
                    let _ = write!(
                        self.body,
                        r#"<image href="{href}" width="{width}" height="{height}" preserveAspectRatio="none"/>"#
                    );
                }
            }
            "text" => self.text(layer, width, height),
            "symbolInstance" => self.instance(layer, width, height),
            // Hotspots, slices and anything unknown draw nothing.
            _ => {}
        }
        if shadow.is_some() {
            self.body.push_str("</g>");
        }
        self.body.push_str("</g>");
    }

    /// Draws a group's layers. A layer that is a mask draws, then clips the layers after it to
    /// its outline, up to the end of the group or a layer that breaks the mask chain.
    fn children(&mut self, parent: &'f Value) {
        let mut masked = false;
        for child in children(parent) {
            let mask = child["hasClippingMask"] == true && child["isVisible"] != false;
            if masked && (mask || child["shouldBreakMaskChain"] == true) {
                self.body.push_str("</g>");
                masked = false;
            }
            self.layer(child);
            if mask && let Some(outline) = outline(child) {
                let id = self.id("m");
                let _ = write!(
                    self.defs,
                    r#"<clipPath id="{id}"><path transform="{}" d="{outline}"/></clipPath>"#,
                    transform(child)
                );
                let _ = write!(self.body, r#"<g clip-path="url(#{id})">"#);
                masked = true;
            }
        }
        if masked {
            self.body.push_str("</g>");
        }
    }

    /// A symbol instance draws its master's layers, scaled to its size, with its overrides.
    fn instance(&mut self, layer: &'f Value, w: f32, h: f32) {
        let Some(id) = self
            .override_value(layer, "symbolID")
            .or(layer["symbolID"].as_str())
        else {
            return;
        };
        let Some(master) = self.masters.get(id).copied() else {
            return;
        };
        let (mw, mh) = (n(&master["frame"], "width"), n(&master["frame"], "height"));
        let (sx, sy) = (w / mw.max(1.0), h / mh.max(1.0));
        let _ = write!(self.body, r#"<g transform="scale({sx} {sy})">"#);
        if master["hasBackgroundColor"] == true
            && master["includeBackgroundColorInInstance"] == true
        {
            let (rgb, alpha) = color(&master["backgroundColor"]);
            let _ = write!(
                self.body,
                r#"<rect width="{mw}" height="{mh}" fill="{rgb}" fill-opacity="{alpha}"/>"#
            );
        }
        // Paths of nested instances grow under every instance above.
        let own = layer["do_objectID"].as_str().unwrap_or("").to_owned();
        for (_, path) in &mut self.overrides {
            path.push(own.clone());
        }
        let values = layer["overrideValues"]
            .as_array()
            .map_or(&[][..], Vec::as_slice);
        self.overrides.push((values, Vec::new()));
        self.children(master);
        self.overrides.pop();
        for (_, path) in &mut self.overrides {
            path.pop();
        }
        self.body.push_str("</g>");
    }

    /// The value an instance above overrides `layer`'s `property` with, the outermost first.
    fn override_value(&self, layer: &Value, property: &str) -> Option<&'f str> {
        let id = layer["do_objectID"].as_str()?;
        self.overrides.iter().find_map(|(values, path)| {
            let mut name = path.join("/");
            if !name.is_empty() {
                name.push('/');
            }
            let _ = write!(name, "{id}_{property}");
            values
                .iter()
                .find(|v| v["overrideName"].as_str() == Some(&name))
                .and_then(|v| v["value"].as_str())
        })
    }

    /// Fills, then borders, of a path in a layer of size `w`×`h`.
    fn paint(&mut self, d: &str, style: &Value, w: f32, h: f32) {
        if d.is_empty() {
            return;
        }
        let rule = if style["windingRule"] == 1 {
            "evenodd"
        } else {
            "nonzero"
        };
        for fill in enabled(&style["fills"]) {
            let opacity = fill["contextSettings"]["opacity"].as_f64().unwrap_or(1.0);
            let Some(paint) = self.fill(fill, w, h) else {
                continue;
            };
            let _ = write!(
                self.body,
                r#"<path d="{d}" fill-rule="{rule}" {paint} opacity="{opacity}"/>"#
            );
        }
        for border in enabled(&style["borders"]) {
            let thickness = border["thickness"].as_f64().unwrap_or(1.0);
            let (rgb, alpha) = color(&border["color"]);
            // Inside and outside borders are drawn centred.
            let _ = write!(
                self.body,
                r#"<path d="{d}" fill="none" stroke="{rgb}" stroke-opacity="{alpha}" stroke-width="{thickness}"/>"#
            );
        }
    }

    /// A fill's `fill` attributes: a colour, or a gradient or image defined in `defs`.
    fn fill(&mut self, fill: &Value, w: f32, h: f32) -> Option<String> {
        match fill["fillType"].as_i64().unwrap_or(0) {
            1 => {
                let gradient = &fill["gradient"];
                let id = self.id("g");
                let stops = gradient["stops"].as_array().into_iter().flatten().fold(
                    String::new(),
                    |mut stops, stop| {
                        let (rgb, alpha) = color(&stop["color"]);
                        let _ = write!(
                            stops,
                            r#"<stop offset="{}" stop-color="{rgb}" stop-opacity="{alpha}"/>"#,
                            stop["position"].as_f64().unwrap_or(0.0)
                        );
                        stops
                    },
                );
                let (fx, fy) = point(&gradient["from"]);
                let (tx, ty) = point(&gradient["to"]);
                let (fx, fy, tx, ty) = (fx * w, fy * h, tx * w, ty * h);
                if gradient["gradientType"] == 1 {
                    let r = (tx - fx).hypot(ty - fy);
                    let _ = write!(
                        self.defs,
                        r#"<radialGradient id="{id}" gradientUnits="userSpaceOnUse" cx="{fx}" cy="{fy}" r="{r}">{stops}</radialGradient>"#
                    );
                } else {
                    // Angular gradients fall back to linear.
                    let _ = write!(
                        self.defs,
                        r#"<linearGradient id="{id}" gradientUnits="userSpaceOnUse" x1="{fx}" y1="{fy}" x2="{tx}" y2="{ty}">{stops}</linearGradient>"#
                    );
                }
                Some(format!(r#"fill="url(#{id})""#))
            }
            4 => {
                let href = self.image(&fill["image"])?;
                let aspect = match fill["patternFillType"].as_i64() {
                    Some(2) => "none",
                    Some(3) => "xMidYMid meet",
                    _ => "xMidYMid slice",
                };
                let id = self.id("p");
                let _ = write!(
                    self.defs,
                    r#"<pattern id="{id}" patternUnits="userSpaceOnUse" width="{w}" height="{h}"><image href="{href}" width="{w}" height="{h}" preserveAspectRatio="{aspect}"/></pattern>"#
                );
                Some(format!(r#"fill="url(#{id})""#))
            }
            _ => {
                let (rgb, alpha) = color(&fill["color"]);
                Some(format!(r#"fill="{rgb}" fill-opacity="{alpha}""#))
            }
        }
    }

    /// A `data:` URI for an image the file holds (`{ _ref: "images/<hash>.png" }`).
    fn image(&self, reference: &Value) -> Option<String> {
        let bytes = self.file.files.get(reference["_ref"].as_str()?)?;
        Some(format!(
            "data:{};base64,{}",
            mime(bytes),
            helpers4::hex::base64_encode(bytes)
        ))
    }

    /// The first enabled drop shadow of a style, as a filter in `defs`.
    fn shadow(&mut self, style: &Value) -> Option<String> {
        let shadow = enabled(&style["shadows"]).next()?;
        let id = self.id("s");
        let (rgb, alpha) = color(&shadow["color"]);
        let _ = write!(
            self.defs,
            r#"<filter id="{id}" x="-50%" y="-50%" width="200%" height="200%"><feDropShadow dx="{}" dy="{}" stdDeviation="{}" flood-color="{rgb}" flood-opacity="{alpha}"/></filter>"#,
            n(shadow, "offsetX"),
            n(shadow, "offsetY"),
            n(shadow, "blurRadius") / 2.0
        );
        Some(id)
    }

    /// A text layer, laid out with its first run's style; reported when its font is missing.
    fn text(&mut self, layer: &Value, w: f32, h: f32) {
        let attributed = &layer["attributedString"];
        let string = self
            .override_value(layer, "stringValue")
            .or(attributed["string"].as_str())
            .unwrap_or("");
        if string.trim().is_empty() {
            return;
        }
        let run = &attributed["attributes"][0]["attributes"];
        let font = &run["MSAttributedStringFontAttribute"]["attributes"];
        let name = font["name"].as_str().unwrap_or("");
        let size = n(font, "size").max(1.0);
        let library = self.typesetter.library();
        let Some(face) = library.resolve_postscript(name) else {
            self.missing_fonts.insert(name.to_owned());
            return;
        };
        let family = face.family.clone();
        let italic = face.italic;
        let style = name.split_once('-').map_or("Regular", |(_, s)| s);
        let weight = weight_of_style(style).clamp(face.weight[0], face.weight[1]);
        let paragraph = &run["paragraphStyle"];
        let align = match paragraph["alignment"].as_i64() {
            Some(1) => Align::Right,
            Some(2) => Align::Center,
            Some(3) => Align::Justify,
            _ => Align::Left,
        };
        let line_height = Some(n(paragraph, "maximumLineHeight")).filter(|&h| h > 0.0);
        let Some(set) = self.typesetter.set(&Paragraph {
            text: string,
            family: &family,
            weight,
            italic,
            size,
            line_height,
            letter_spacing: n(run, "kerning"),
            align,
            // Auto width (0) never wraps; fixed width and fixed size wrap at the frame.
            width: (layer["textBehaviour"] != 0).then_some(w),
        }) else {
            return;
        };
        let top = match layer["style"]["textStyle"]["verticalAlignment"].as_i64() {
            Some(1) => (h - set.height) / 2.0,
            Some(2) => h - set.height,
            _ => 0.0,
        };
        let (rgb, alpha) = color(&run["MSAttributedStringColorAttribute"]);
        let _ = write!(
            self.body,
            r#"<path transform="translate(0 {top})" d="{}" fill="{rgb}" fill-opacity="{alpha}"/>"#,
            set.path
        );
    }
}

/// Where a layer sits in its parent: its position, then its rotation (Sketch turns
/// counter-clockwise) and flips, around its centre.
fn transform(layer: &Value) -> String {
    let frame = &layer["frame"];
    let (width, height) = (n(frame, "width"), n(frame, "height"));
    let mut transform = format!("translate({} {})", n(frame, "x"), n(frame, "y"));
    let rotation = n(layer, "rotation");
    if rotation != 0.0 {
        let _ = write!(
            transform,
            " rotate({} {} {})",
            -rotation,
            width / 2.0,
            height / 2.0
        );
    }
    let flip_x = layer["isFlippedHorizontal"] == true;
    let flip_y = layer["isFlippedVertical"] == true;
    if flip_x || flip_y {
        let _ = write!(
            transform,
            " translate({} {}) scale({} {})",
            if flip_x { width } else { 0.0 },
            if flip_y { height } else { 0.0 },
            if flip_x { -1 } else { 1 },
            if flip_y { -1 } else { 1 }
        );
    }
    transform
}

/// A layer's outline in its own coordinates: a shape's path, a boolean group's paths.
fn outline(layer: &Value) -> Option<String> {
    if layer["_class"] == "shapeGroup" {
        let path: String = children(layer)
            .filter(|c| c["isVisible"] != false)
            .filter_map(|c| {
                let shape = shape_path(c)?;
                Some(offset_path(
                    &shape,
                    n(&c["frame"], "x"),
                    n(&c["frame"], "y"),
                ))
            })
            .collect();
        return (!path.is_empty()).then_some(path);
    }
    shape_path(layer)
}

fn children(layer: &Value) -> impl Iterator<Item = &Value> {
    layer["layers"].as_array().into_iter().flatten()
}

fn enabled(list: &Value) -> impl Iterator<Item = &Value> {
    list.as_array()
        .into_iter()
        .flatten()
        .filter(|item| item["isEnabled"] != false)
}

/// A number field; Sketch coordinates fit in f32.
#[allow(clippy::cast_possible_truncation)]
fn n(value: &Value, key: &str) -> f32 {
    value[key].as_f64().unwrap_or(0.0) as f32
}

/// `(css rgb, alpha)` from a Sketch colour with 0–1 channels.
fn color(c: &Value) -> (String, f64) {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let byte = |k: &str| (c[k].as_f64().unwrap_or(0.0).clamp(0.0, 1.0) * 255.0).round() as u8;
    (
        format!("rgb({},{},{})", byte("red"), byte("green"), byte("blue")),
        c["alpha"].as_f64().unwrap_or(1.0),
    )
}

/// A Sketch point string, `"{0.5, 1}"`.
fn point(value: &Value) -> (f32, f32) {
    let text = value.as_str().unwrap_or("{0, 0}");
    let mut numbers = text
        .trim_matches(|c| c == '{' || c == '}')
        .split(',')
        .map(|v| v.trim().parse::<f32>().unwrap_or(0.0));
    (numbers.next().unwrap_or(0.0), numbers.next().unwrap_or(0.0))
}

/// A shape's outline as SVG path data, in its frame's pixels, from its curve points (0–1 units of
/// the frame). Rounded corners are drawn when every point has the same radius on a rectangle.
fn shape_path(layer: &Value) -> Option<String> {
    let frame = &layer["frame"];
    let (width, height) = (n(frame, "width"), n(frame, "height"));
    let points = layer["points"].as_array()?;
    if points.is_empty() {
        return None;
    }
    if layer["_class"] == "rectangle" {
        let radius = n(&points[0], "cornerRadius");
        if radius > 0.0
            && points
                .iter()
                .all(|p| (n(p, "cornerRadius") - radius).abs() < 0.01)
        {
            let r = radius.min(width / 2.0).min(height / 2.0);
            return Some(format!(
                "M{r} 0H{}A{r} {r} 0 0 1 {width} {r}V{}A{r} {r} 0 0 1 {} {height}H{r}A{r} {r} 0 0 1 0 {}V{r}A{r} {r} 0 0 1 {r} 0Z",
                width - r,
                height - r,
                width - r,
                height - r
            ));
        }
    }
    let at = |p: &Value, key: &str| {
        let (x, y) = point(&p[key]);
        (x * width, y * height)
    };
    let mut path = String::new();
    let (x, y) = at(&points[0], "point");
    let _ = write!(path, "M{x} {y}");
    let closed = layer["isClosed"] != false;
    let count = points.len();
    let segments = if closed { count } else { count - 1 };
    for i in 0..segments {
        let from = &points[i];
        let to = &points[(i + 1) % count];
        let (x, y) = at(to, "point");
        if from["hasCurveFrom"] == true || to["hasCurveTo"] == true {
            let (c1x, c1y) = at(from, "curveFrom");
            let (c2x, c2y) = at(to, "curveTo");
            let _ = write!(path, "C{c1x} {c1y} {c2x} {c2y} {x} {y}");
        } else {
            let _ = write!(path, "L{x} {y}");
        }
    }
    if closed {
        path.push('Z');
    }
    Some(path)
}

// TODO(helpers4): path data parsing and transforms (docs/helpers4-requests.md § 4).
/// Moves absolute path data by (`dx`, `dy`): the paths `shape_path` writes only use absolute
/// commands with x, y pairs (and the arcs of rounded rectangles).
fn offset_path(d: &str, dx: f32, dy: f32) -> String {
    let mut out = String::new();
    let mut command = 'M';
    let mut index = 0;
    for token in tokens(d) {
        match token {
            Token::Command(c) => {
                command = c;
                index = 0;
                out.push(c);
            }
            Token::Number(v) => {
                let moved = match command {
                    // Arcs: rx ry rotation large sweep x y.
                    'A' => match index % 7 {
                        5 => v + dx,
                        6 => v + dy,
                        _ => v,
                    },
                    'H' => v + dx,
                    'V' => v + dy,
                    _ if index % 2 == 0 => v + dx,
                    _ => v + dy,
                };
                if !out.ends_with(command) {
                    out.push(' ');
                }
                let _ = write!(out, "{moved}");
                index += 1;
            }
        }
    }
    out
}

enum Token {
    Command(char),
    Number(f32),
}

fn tokens(d: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut number = String::new();
    let flush = |number: &mut String, out: &mut Vec<Token>| {
        if let Ok(v) = number.parse() {
            out.push(Token::Number(v));
        }
        number.clear();
    };
    for c in d.chars() {
        if c.is_ascii_alphabetic() && c != 'e' {
            flush(&mut number, &mut out);
            out.push(Token::Command(c));
        } else if c == ' ' || c == ',' {
            flush(&mut number, &mut out);
        } else if c == '-' && !number.is_empty() && !number.ends_with('e') {
            flush(&mut number, &mut out);
            number.push(c);
        } else {
            number.push(c);
        }
    }
    flush(&mut number, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_follow_curve_points() {
        let square: Value = serde_json::json!({
            "_class": "shapePath",
            "frame": { "x": 0, "y": 0, "width": 10, "height": 20 },
            "isClosed": true,
            "points": [
                { "point": "{0, 0}", "curveFrom": "{0, 0}", "curveTo": "{0, 0}" },
                { "point": "{1, 0}", "curveFrom": "{1, 0}", "curveTo": "{1, 0}" },
                { "point": "{1, 1}", "curveFrom": "{1, 1}", "curveTo": "{1, 1}" }
            ]
        });
        assert_eq!(shape_path(&square).unwrap(), "M0 0L10 0L10 20L0 0Z");
    }

    #[test]
    fn moves_absolute_paths() {
        assert_eq!(offset_path("M0 0L10 -5Z", 2.0, 3.0), "M2 3L12 -2Z");
        assert_eq!(
            offset_path("M4 0H6A4 4 0 0 1 10 4", 1.0, 1.0),
            "M5 1H7A4 4 0 0 1 11 5"
        );
    }
}
