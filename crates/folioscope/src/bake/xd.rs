// SPDX-License-Identifier: AGPL-3.0-or-later

//! Draws `.xd` artboards as SVG, from their JSON graphics trees (see `docs/format-xd.md`).
//!
//! Handled: groups and clip paths, rectangles (with corner radii), circles, ellipses, lines,
//! polygons, paths and compound shapes, colour and image fills, borders (centred, inside and
//! outside), drop shadows, opacity, and text, laid out with the fonts given (ADR 0011) at the
//! positions XD stored for each line. Not yet: gradients, blurs, per-artboard scrolling, text
//! underline and strikethrough.

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;

use helpers4::font::is_italic;
use helpers4::geometry::{Rect as Box64, ellipse_path, rounded_rect_path};
use serde_json::Value;

use crate::bake::draw::Draw;
use crate::bake::folio::BakeError;
use crate::bake::fonts::weight_of_style;
use crate::bake::svg::mime;
use crate::bake::text::{Align, Paragraph, Typesetter};
use crate::ir::{self, Rect};
use crate::xd::XdFile;
use crate::xd::geometry::{children, local_box, matrix};

/// What of an artboard to draw.
#[derive(Clone, Copy)]
enum What<'a> {
    Whole,
    Background,
    Node(&'a Value),
}

/// Draws the artboards of a decoded `.xd`.
pub struct XdDraw<'f> {
    file: &'f XdFile,
    /// The artboard node of each artboard id.
    artboards: HashMap<&'f str, &'f Value>,
    typesetter: Typesetter,
    missing_fonts: BTreeSet<String>,
}

impl<'f> XdDraw<'f> {
    #[must_use]
    pub fn new(file: &'f XdFile, typesetter: Typesetter) -> Self {
        let artboards = file
            .artboards
            .iter()
            .filter_map(|(id, tree)| Some((id.as_str(), tree["children"].as_array()?.first()?)))
            .collect();
        Self {
            file,
            artboards,
            typesetter,
            missing_fonts: BTreeSet::new(),
        }
    }

    /// The artboard's SVG: all of it, only its background, or only one of its children (the parts
    /// smart animate pairs).
    fn svg(&mut self, frame: &ir::Frame, what: What<'_>) -> Option<String> {
        let artboard = *self.artboards.get(frame.id.as_str())?;
        let (width, height) = (frame.bounds.width, frame.bounds.height);
        let mut svg = Svg {
            file: self.file,
            typesetter: &mut self.typesetter,
            missing_fonts: &mut self.missing_fonts,
            defs: String::new(),
            body: String::new(),
            next_id: 0,
        };
        if !matches!(what, What::Node(_)) {
            svg.background(artboard, width, height);
        }
        // The children are placed in the canvas: the artboard's corner is the frame's origin.
        let _ = write!(
            svg.body,
            r#"<g transform="translate({} {})">"#,
            -frame.bounds.x, -frame.bounds.y
        );
        match what {
            What::Node(node) => svg.node(node),
            What::Whole => children(artboard).iter().for_each(|c| svg.node(c)),
            What::Background => {}
        }
        svg.body.push_str("</g>");
        Some(format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}"><defs>{}</defs>{}</svg>"#,
            svg.defs, svg.body
        ))
    }
}

impl Draw for XdDraw<'_> {
    fn frame(&mut self, frame: &ir::Frame) -> Option<String> {
        self.svg(frame, What::Whole)
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
        // XD's interactive components (states) are not read yet.
        None
    }

    fn missing_fonts(&mut self) -> BTreeSet<String> {
        std::mem::take(&mut self.missing_fonts)
    }

    /// XD's auto-animate pairs layers by name: the background, then each child of the artboard.
    fn parts(&mut self, frame: &ir::Frame) -> Vec<(String, String, Rect)> {
        let Some(artboard) = self.artboards.get(frame.id.as_str()).copied() else {
            return Vec::new();
        };
        let whole = Rect {
            x: 0.0,
            y: 0.0,
            width: frame.bounds.width,
            height: frame.bounds.height,
        };
        let mut parts = Vec::new();
        if let Some(markup) = self.svg(frame, What::Background) {
            parts.push((String::new(), markup, whole));
        }
        for child in children(artboard) {
            if child["visible"] == false {
                continue;
            }
            let Some(canvas) = crate::xd::geometry::bounds(child) else {
                continue;
            };
            let Some(markup) = self.svg(frame, What::Node(child)) else {
                continue;
            };
            #[allow(clippy::cast_possible_truncation)]
            let bounds = Rect {
                x: canvas.x as f32 - frame.bounds.x,
                y: canvas.y as f32 - frame.bounds.y,
                width: canvas.width as f32,
                height: canvas.height as f32,
            };
            parts.push((
                child["name"].as_str().unwrap_or("").to_owned(),
                markup,
                bounds,
            ));
        }
        parts
    }
}

/// One artboard being drawn.
struct Svg<'d, 'f> {
    file: &'f XdFile,
    typesetter: &'d mut Typesetter,
    missing_fonts: &'d mut BTreeSet<String>,
    defs: String,
    body: String,
    next_id: u32,
}

impl Svg<'_, '_> {
    fn id(&mut self, prefix: &str) -> String {
        self.next_id += 1;
        format!("{prefix}{}", self.next_id)
    }

    /// The artboard's own fill; white when it has none, as in XD's player.
    fn background(&mut self, artboard: &Value, width: f32, height: f32) {
        let fill = &artboard["style"]["fill"];
        let (color, opacity) = match fill["type"].as_str() {
            Some("solid") => color(&fill["color"]),
            Some("none") => return,
            _ => ("rgb(255,255,255)".to_owned(), 1.0),
        };
        let _ = write!(
            self.body,
            r#"<rect width="{width}" height="{height}" fill="{color}" fill-opacity="{opacity}"/>"#
        );
    }

    fn node(&mut self, node: &Value) {
        if node["visible"] == false {
            return;
        }
        let m = matrix(node);
        let style = &node["style"];
        let opacity = style["opacity"].as_f64().unwrap_or(1.0);
        let _ = write!(
            self.body,
            r#"<g transform="matrix({} {} {} {} {} {})" opacity="{opacity}">"#,
            m.a, m.b, m.c, m.d, m.e, m.f
        );
        let shadow = self.shadow(style);
        if let Some(filter) = &shadow {
            let _ = write!(self.body, r#"<g filter="url(#{filter})">"#);
        }
        match node["type"].as_str().unwrap_or("") {
            "group" => self.group(node),
            "shape" => self.shape(node),
            "text" => self.text(node),
            _ => {}
        }
        if shadow.is_some() {
            self.body.push_str("</g>");
        }
        self.body.push_str("</g>");
    }

    fn group(&mut self, node: &Value) {
        let clip = self.clip(&node["style"]["clipPath"]);
        if let Some(id) = &clip {
            let _ = write!(self.body, r#"<g clip-path="url(#{id})">"#);
        }
        for child in children(node) {
            self.node(child);
        }
        if clip.is_some() {
            self.body.push_str("</g>");
        }
    }

    /// A group's clip path, from the shared resources: its shapes, in the group's coordinates.
    fn clip(&mut self, reference: &Value) -> Option<String> {
        let shapes =
            self.file.resources["clipPaths"][reference["ref"].as_str()?]["children"].as_array()?;
        let mut inner = String::new();
        for node in shapes {
            let Some(d) = node["shape"]
                .as_object()
                .and_then(|_| shape_path(&node["shape"]))
            else {
                continue;
            };
            let m = matrix(node);
            let _ = write!(
                inner,
                r#"<path transform="matrix({} {} {} {} {} {})" d="{d}"/>"#,
                m.a, m.b, m.c, m.d, m.e, m.f
            );
        }
        if inner.is_empty() {
            return None;
        }
        let id = self.id("c");
        let _ = write!(self.defs, r#"<clipPath id="{id}">{inner}</clipPath>"#);
        Some(id)
    }

    fn shape(&mut self, node: &Value) {
        let Some(path) = shape_path(&node["shape"]) else {
            return;
        };
        let style = &node["style"];
        let rule = if node["shape"]["winding"] == "evenodd" {
            r#" fill-rule="evenodd""#
        } else {
            ""
        };
        if let Some(paint) = self.fill(&style["fill"], node) {
            let _ = write!(self.body, r#"<path d="{path}" {paint}{rule}/>"#);
        }
        self.stroke(&style["stroke"], &path);
    }

    /// A fill's attributes: a colour, or an image defined in `defs`; `None` when it paints nothing.
    fn fill(&mut self, fill: &Value, node: &Value) -> Option<String> {
        match fill["type"].as_str()? {
            "solid" => {
                let (color, opacity) = color(&fill["color"]);
                Some(format!(r#"fill="{color}" fill-opacity="{opacity}""#))
            }
            "pattern" => {
                let uid = fill["pattern"]["meta"]["ux"]["uid"].as_str()?;
                let bytes = self.file.images.get(uid)?;
                let b: Box64 = local_box(node)?;
                let id = self.id("p");
                let _ = write!(
                    self.defs,
                    r#"<pattern id="{id}" patternUnits="userSpaceOnUse" x="{}" y="{}" width="{}" height="{}"><image href="data:{};base64,{}" width="{}" height="{}" preserveAspectRatio="xMidYMid slice"/></pattern>"#,
                    b.x,
                    b.y,
                    b.width,
                    b.height,
                    mime(bytes),
                    helpers4::hex::base64_encode(bytes),
                    b.width,
                    b.height
                );
                Some(format!(r#"fill="url(#{id})""#))
            }
            // Gradients and the rest are not drawn yet.
            _ => None,
        }
    }

    /// A border. XD centres it, or keeps it inside or outside the shape: those two are drawn twice
    /// as wide and clipped to the shape's inside or outside.
    fn stroke(&mut self, stroke: &Value, path: &str) {
        if stroke["type"] != "solid" {
            return;
        }
        let width = stroke["width"].as_f64().unwrap_or(1.0);
        let (color, opacity) = color(&stroke["color"]);
        let mut attributes = format!(r#"fill="none" stroke="{color}" stroke-opacity="{opacity}""#);
        if let Some(cap) = stroke["cap"].as_str() {
            let _ = write!(attributes, r#" stroke-linecap="{cap}""#);
        }
        if let Some(join) = stroke["join"].as_str() {
            let _ = write!(attributes, r#" stroke-linejoin="{join}""#);
        }
        if let Some(limit) = stroke["miterLimit"].as_f64() {
            let _ = write!(attributes, r#" stroke-miterlimit="{limit}""#);
        }
        if let Some(dash) = stroke["dash"].as_array().filter(|d| !d.is_empty()) {
            let dash: Vec<String> = dash
                .iter()
                .filter_map(Value::as_f64)
                .map(|d| d.to_string())
                .collect();
            let _ = write!(attributes, r#" stroke-dasharray="{}""#, dash.join(" "));
        }
        let (width, clip) = match stroke["align"].as_str() {
            Some("inside") => {
                let id = self.id("c");
                let _ = write!(
                    self.defs,
                    r#"<clipPath id="{id}"><path d="{path}"/></clipPath>"#
                );
                (width * 2.0, Some(id))
            }
            Some("outside") => {
                let id = self.id("c");
                let _ = write!(
                    self.defs,
                    r#"<clipPath id="{id}"><path clip-rule="evenodd" d="M-100000 -100000H100000V100000H-100000Z{path}"/></clipPath>"#
                );
                (width * 2.0, Some(id))
            }
            _ => (width, None),
        };
        match clip {
            Some(id) => {
                let _ = write!(
                    self.body,
                    r#"<g clip-path="url(#{id})"><path d="{path}" {attributes} stroke-width="{width}"/></g>"#
                );
            }
            None => {
                let _ = write!(
                    self.body,
                    r#"<path d="{path}" {attributes} stroke-width="{width}"/>"#
                );
            }
        }
    }

    /// The first visible drop shadow of a style, as a filter in `defs`.
    fn shadow(&mut self, style: &Value) -> Option<String> {
        let filter = style["filters"]
            .as_array()?
            .iter()
            .find(|f| f["type"] == "dropShadow" && f["visible"] != false)?;
        let shadow = filter["params"]["dropShadows"].as_array()?.first()?;
        let (color, opacity) = color(&shadow["color"]);
        let id = self.id("s");
        let _ = write!(
            self.defs,
            r#"<filter id="{id}" x="-50%" y="-50%" width="200%" height="200%"><feDropShadow dx="{}" dy="{}" stdDeviation="{}" flood-color="{color}" flood-opacity="{opacity}"/></filter>"#,
            shadow["dx"].as_f64().unwrap_or(0.0),
            shadow["dy"].as_f64().unwrap_or(0.0),
            shadow["r"].as_f64().unwrap_or(0.0) / 2.0,
        );
        Some(id)
    }

    /// Text at the positions XD laid out: each line from its baseline, in the style of each run.
    #[allow(clippy::cast_possible_truncation)]
    fn text(&mut self, node: &Value) {
        let Some(raw) = node["text"]["rawText"].as_str() else {
            return;
        };
        let chars: Vec<char> = raw.chars().collect();
        let runs = runs(node);
        let default_font = &node["style"]["font"];
        let default_fill = &node["style"]["fill"];
        for paragraph in node["text"]["paragraphs"].as_array().into_iter().flatten() {
            for line in paragraph["lines"].as_array().into_iter().flatten() {
                // A line is a list of segments (XD splits on style changes and tabs).
                for segment in line.as_array().into_iter().flatten() {
                    let from = index(&segment["from"]);
                    let to = index(&segment["to"]).min(chars.len());
                    if from >= to {
                        continue;
                    }
                    let mut x = number(&segment["x"]);
                    let y = number(&segment["y"]);
                    // The runs this segment crosses, each laid out after the one before.
                    for run in &runs {
                        let (start, end) = (from.max(run.start), to.min(run.end));
                        if start >= end {
                            continue;
                        }
                        let text: String = chars[start..end].iter().collect();
                        let text = match run.transform.as_str() {
                            "uppercase" => text.to_uppercase(),
                            "lowercase" => text.to_lowercase(),
                            _ => text,
                        };
                        let font = run.font.as_ref().unwrap_or(&segment["style"]["font"]);
                        let font = if font.is_null() { default_font } else { font };
                        let family = run.family.as_deref().or(font["family"].as_str());
                        let style = run.style.as_deref().or(font["style"].as_str());
                        let family = family.unwrap_or("");
                        let style = style.unwrap_or("Regular");
                        let size = run
                            .size
                            .or(font["size"].as_f64())
                            .map_or(12.0, |s| s as f32);
                        let weight = weight_of_style(style);
                        let italic = is_italic(style);
                        let Some(set) = self.typesetter.set(&Paragraph {
                            text: &text,
                            family,
                            weight,
                            italic,
                            size,
                            line_height: None,
                            letter_spacing: run.spacing * size / 1000.0,
                            align: Align::Left,
                            width: None,
                        }) else {
                            self.missing_fonts.insert(format!("{family} {style}"));
                            continue;
                        };
                        let (color, opacity) = run
                            .fill
                            .clone()
                            .or_else(|| {
                                (default_fill["type"] == "solid")
                                    .then(|| color(&default_fill["color"]))
                            })
                            .unwrap_or_else(|| ("rgb(0,0,0)".to_owned(), 1.0));
                        let _ = write!(
                            self.body,
                            r#"<path transform="translate({x} {})" d="{}" fill="{color}" fill-opacity="{opacity}"/>"#,
                            y - set.baseline,
                            set.path
                        );
                        x += set.width;
                    }
                }
            }
        }
    }
}

/// A run of text in one style, over `start..end` characters of the node's text.
struct Run {
    start: usize,
    end: usize,
    family: Option<String>,
    style: Option<String>,
    font: Option<Value>,
    size: Option<f64>,
    /// In thousandths of an em.
    spacing: f32,
    fill: Option<(String, f64)>,
    transform: String,
}

/// The style runs of a text node (`rangedStyles`), or one run over all of it.
fn runs(node: &Value) -> Vec<Run> {
    let total = node["text"]["rawText"]
        .as_str()
        .map_or(0, |t| t.chars().count());
    let ranged = node["meta"]["ux"]["rangedStyles"].as_array();
    let Some(ranged) = ranged.filter(|r| !r.is_empty()) else {
        return vec![Run {
            start: 0,
            end: total,
            family: None,
            style: None,
            font: None,
            size: None,
            spacing: 0.0,
            fill: None,
            transform: String::new(),
        }];
    };
    let mut start = 0;
    ranged
        .iter()
        .map(|style| {
            let end = (start + index(&style["length"])).min(total);
            let run = Run {
                start,
                end,
                family: style["fontFamily"].as_str().map(str::to_owned),
                style: style["fontStyle"].as_str().map(str::to_owned),
                font: None,
                size: style["fontSize"].as_f64(),
                #[allow(clippy::cast_possible_truncation)]
                spacing: style["charSpacing"].as_f64().unwrap_or(0.0) as f32,
                fill: style["fill"]["value"].as_u64().map(argb),
                transform: style["textTransform"].as_str().unwrap_or("none").to_owned(),
            };
            start = end;
            run
        })
        .collect()
}

/// A coordinate; XD's fit in f32.
#[allow(clippy::cast_possible_truncation)]
fn number(value: &Value) -> f32 {
    value.as_f64().unwrap_or(0.0) as f32
}

/// A character index XD wrote as a number.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn index(value: &Value) -> usize {
    value.as_f64().unwrap_or(0.0).max(0.0) as usize
}

/// `(css rgb, alpha)` from XD's `0xAARRGGBB` integer.
fn argb(value: u64) -> (String, f64) {
    let channel = |shift: u32| (value >> shift) & 0xff;
    (
        format!("rgb({},{},{})", channel(16), channel(8), channel(0)),
        #[allow(clippy::cast_precision_loss)]
        {
            channel(24) as f64 / 255.0
        },
    )
}

/// `(css rgb, alpha)` from an XD colour (`{ mode: "RGB", value: { r, g, b }, alpha }`).
fn color(color: &Value) -> (String, f64) {
    let channel = |k: &str| {
        color["value"][k]
            .as_f64()
            .unwrap_or(0.0)
            .clamp(0.0, 255.0)
            .round()
    };
    (
        format!("rgb({},{},{})", channel("r"), channel("g"), channel("b")),
        color["alpha"].as_f64().unwrap_or(1.0),
    )
}

/// A shape's outline as SVG path data, in its own coordinates.
fn shape_path(shape: &Value) -> Option<String> {
    let n = |k: &str| shape[k].as_f64().unwrap_or(0.0);
    Some(match shape["type"].as_str()? {
        "rect" => {
            let radii = shape["r"].as_array().map_or([0.0; 4], |r| {
                let at = |i: usize| r.get(i).and_then(Value::as_f64).unwrap_or(0.0);
                [at(0), at(1), at(2), at(3)]
            });
            let radii = if let Some(r) = shape["r"].as_f64() {
                [r; 4]
            } else {
                radii
            };
            rounded_rect_path(Box64::new(n("x"), n("y"), n("width"), n("height")), radii)
        }
        "circle" => ellipse_path(Box64::new(
            n("cx") - n("r"),
            n("cy") - n("r"),
            2.0 * n("r"),
            2.0 * n("r"),
        )),
        "ellipse" => ellipse_path(Box64::new(
            n("cx") - n("rx"),
            n("cy") - n("ry"),
            2.0 * n("rx"),
            2.0 * n("ry"),
        )),
        "line" => format!("M{} {}L{} {}", n("x1"), n("y1"), n("x2"), n("y2")),
        "polygon" => {
            let mut d = String::new();
            for (i, p) in shape["points"].as_array()?.iter().enumerate() {
                let _ = write!(
                    d,
                    "{}{} {}",
                    if i == 0 { 'M' } else { 'L' },
                    p["x"].as_f64().unwrap_or(0.0),
                    p["y"].as_f64().unwrap_or(0.0)
                );
            }
            d.push('Z');
            d
        }
        "path" | "compound" => shape["path"].as_str()?.to_owned(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn colours_come_from_both_encodings() {
        assert_eq!(argb(0xFFD2_0808), ("rgb(210,8,8)".to_owned(), 1.0));
        let c = json!({ "mode": "RGB", "value": { "r": 1, "g": 2, "b": 3 }, "alpha": 0.5 });
        assert_eq!(color(&c), ("rgb(1,2,3)".to_owned(), 0.5));
    }

    #[test]
    fn rectangles_take_a_radius_per_corner() {
        let rect =
            json!({ "type": "rect", "x": 0, "y": 0, "width": 10, "height": 10, "r": [2, 0, 0, 0] });
        assert_eq!(
            shape_path(&rect).unwrap(),
            "M2 0 L10 0 L10 10 L0 10 L0 2 A2 2 0 0 1 2 0 Z"
        );
    }

    #[test]
    fn text_is_cut_into_runs_by_style() {
        let node = json!({
            "text": { "rawText": "Hello world" },
            "meta": { "ux": { "rangedStyles": [
                { "length": 6, "fontSize": 20 }, { "length": 5, "fontSize": 12 }
            ] } }
        });
        let runs = runs(&node);
        assert_eq!(
            runs.iter().map(|r| (r.start, r.end)).collect::<Vec<_>>(),
            [(0, 6), (6, 11)]
        );
    }
}
