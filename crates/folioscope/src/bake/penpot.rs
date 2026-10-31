// SPDX-License-Identifier: AGPL-3.0-or-later

//! Draws `.penpot` boards as SVG, from their shapes (see `docs/format-penpot.md`). Shapes are in
//! page coordinates; a board is drawn from its own top-left corner.
//!
//! Handled: boards (clipping their content unless they show it), rectangles with their corner
//! radii, ellipses, paths and boolean shapes (as one path), groups, images, colour, gradient and
//! image fills, strokes, drop shadows, rotations and flips (`transform`), and text from the
//! position Penpot computed for each run, drawn with the fonts given (ADR 0011). Not yet: blurs,
//! inner shadows, blend modes, inside and outside strokes (drawn centred), `svg-raw` shapes.

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;

use crate::ir::{self, Rect};
use crate::penpot::{PenpotFile, PenpotPage};
use serde_json::Value;

use crate::bake::draw::Draw;
use crate::bake::folio::BakeError;
use crate::bake::svg::mime;
use crate::bake::text::{Align, Paragraph, Typesetter};

/// Draws the boards of a decoded `.penpot`.
pub struct PenpotDraw<'f> {
    file: &'f PenpotFile,
    /// Each board's page, by board id.
    pages: HashMap<&'f str, &'f PenpotPage>,
    typesetter: Typesetter,
    missing_fonts: BTreeSet<String>,
}

impl<'f> PenpotDraw<'f> {
    #[must_use]
    pub fn new(file: &'f PenpotFile, typesetter: Typesetter) -> Self {
        let pages = file
            .pages
            .iter()
            .flat_map(|page| page.shapes.keys().map(move |id| (id.as_str(), page)))
            .collect();
        Self {
            file,
            pages,
            typesetter,
            missing_fonts: BTreeSet::new(),
        }
    }
}

impl Draw for PenpotDraw<'_> {
    fn frame(&mut self, frame: &ir::Frame) -> Option<String> {
        let page = *self.pages.get(frame.id.as_str())?;
        let board = page.shapes.get(&frame.id)?;
        let (width, height) = (frame.bounds.width, frame.bounds.height);
        let mut svg = Svg {
            file: self.file,
            page,
            typesetter: &mut self.typesetter,
            missing_fonts: &mut self.missing_fonts,
            defs: String::new(),
            body: String::new(),
            next_id: 0,
        };
        let _ = write!(
            svg.body,
            r#"<g transform="translate({} {})">"#,
            -frame.bounds.x, -frame.bounds.y
        );
        svg.shape(board);
        svg.body.push_str("</g>");
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
        // Penpot's component variants are not read yet.
        None
    }

    fn missing_fonts(&mut self) -> BTreeSet<String> {
        std::mem::take(&mut self.missing_fonts)
    }
}

/// One board being drawn, in page coordinates.
struct Svg<'d, 'f> {
    file: &'f PenpotFile,
    page: &'f PenpotPage,
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

    fn shape(&mut self, shape: &Value) {
        if shape["hidden"] == true {
            return;
        }
        let opacity = shape["opacity"].as_f64().unwrap_or(1.0);
        let _ = write!(
            self.body,
            r#"<g transform="{}" opacity="{opacity}">"#,
            transform(shape)
        );
        let shadow = self.shadow(shape);
        if let Some(filter) = &shadow {
            let _ = write!(self.body, r#"<g filter="url(#{filter})">"#);
        }
        match shape["type"].as_str().unwrap_or("") {
            "frame" => {
                let outline = rect_path(shape);
                self.paint(&outline, shape);
                // A board clips its content, unless it is set to show it.
                let clip = (shape["showContent"] != true).then(|| {
                    let id = self.id("c");
                    let _ = write!(
                        self.defs,
                        r#"<clipPath id="{id}"><path d="{outline}"/></clipPath>"#
                    );
                    id
                });
                if let Some(id) = &clip {
                    let _ = write!(self.body, r#"<g clip-path="url(#{id})">"#);
                }
                self.children(shape);
                if clip.is_some() {
                    self.body.push_str("</g>");
                }
            }
            "group" if shape["maskedGroup"] == true => self.masked(shape),
            "group" => self.children(shape),
            "rect" => self.paint(&rect_path(shape), shape),
            "circle" => self.paint(&ellipse_path(shape), shape),
            "path" | "bool" => {
                let path = content_path(&shape["content"]).or_else(|| {
                    // A boolean shape without its computed outline: its shapes' paths.
                    let paths: String = self
                        .page_children(shape)
                        .iter()
                        .filter_map(|c| content_path(&c["content"]).or_else(|| Some(rect_path(c))))
                        .collect();
                    (!paths.is_empty()).then_some(paths)
                });
                if let Some(path) = path {
                    self.paint(&path, shape);
                }
            }
            "image" => {
                let (x, y, w, h) = boxed(shape);
                if let Some(href) = self.media(shape["metadata"]["id"].as_str()) {
                    let _ = write!(
                        self.body,
                        r#"<image href="{href}" x="{x}" y="{y}" width="{w}" height="{h}" preserveAspectRatio="none"/>"#
                    );
                }
            }
            "text" => self.text(shape),
            // svg-raw and anything unknown draw nothing yet.
            _ => {}
        }
        if shadow.is_some() {
            self.body.push_str("</g>");
        }
        self.body.push_str("</g>");
    }

    /// A shape's children, bottom of the layer list first, as they are drawn.
    fn page_children(&self, shape: &Value) -> Vec<&Value> {
        shape["shapes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|id| self.page.shapes.get(id.as_str()?))
            .collect()
    }

    /// A masked group: its first shape (the bottom one) is not drawn, it clips the others.
    fn masked(&mut self, group: &Value) {
        let page = self.page;
        let mut shapes = group["shapes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|id| page.shapes.get(id.as_str()?));
        let Some(mask) = shapes.next() else {
            return;
        };
        let clip = outline(mask).map(|path| {
            let id = self.id("m");
            let _ = write!(
                self.defs,
                r#"<clipPath id="{id}"><path transform="{}" d="{path}"/></clipPath>"#,
                transform(mask)
            );
            id
        });
        if let Some(id) = &clip {
            let _ = write!(self.body, r#"<g clip-path="url(#{id})">"#);
        }
        for shape in shapes {
            self.shape(shape);
        }
        if clip.is_some() {
            self.body.push_str("</g>");
        }
    }

    fn children(&mut self, shape: &Value) {
        let page = self.page;
        for id in shape["shapes"].as_array().into_iter().flatten() {
            if let Some(child) = id.as_str().and_then(|id| page.shapes.get(id)) {
                self.shape(child);
            }
        }
    }

    /// Fills, then strokes, of a path.
    fn paint(&mut self, path: &str, shape: &Value) {
        for fill in list(&shape["fills"]) {
            if let Some(paint) = self.fill(fill, shape) {
                let _ = write!(self.body, r#"<path d="{path}" {paint}/>"#);
            }
        }
        for stroke in list(&shape["strokes"]) {
            let Some(color) = stroke["strokeColor"].as_str() else {
                continue;
            };
            let width = stroke["strokeWidth"].as_f64().unwrap_or(1.0);
            let opacity = stroke["strokeOpacity"].as_f64().unwrap_or(1.0);
            let dashes = match stroke["strokeStyle"].as_str() {
                Some("dashed") => format!(r#" stroke-dasharray="{} {}""#, width * 4.0, width * 2.0),
                Some("dotted") => format!(r#" stroke-dasharray="{width} {width}""#),
                _ => String::new(),
            };
            // Inner and outer strokes are drawn centred.
            let _ = write!(
                self.body,
                r#"<path d="{path}" fill="none" stroke="{color}" stroke-opacity="{opacity}" stroke-width="{width}"{dashes}/>"#
            );
        }
    }

    /// A fill's attributes: a colour, or a gradient or image defined in `defs`.
    fn fill(&mut self, fill: &Value, shape: &Value) -> Option<String> {
        let (x, y, w, h) = boxed(shape);
        if let Some(gradient) = fill.get("fillColorGradient").filter(|g| g.is_object()) {
            let id = self.id("g");
            let stops = list(&gradient["stops"]).fold(String::new(), |mut stops, stop| {
                let _ = write!(
                    stops,
                    r#"<stop offset="{}" stop-color="{}" stop-opacity="{}"/>"#,
                    stop["offset"].as_f64().unwrap_or(0.0),
                    stop["color"].as_str().unwrap_or("#000000"),
                    stop["opacity"].as_f64().unwrap_or(1.0)
                );
                stops
            });
            let at = |kx: &str, ky: &str| (x + n(gradient, kx) * w, y + n(gradient, ky) * h);
            let (sx, sy) = at("startX", "startY");
            let (ex, ey) = at("endX", "endY");
            if gradient["type"] == "radial" {
                let radius = (ex - sx).hypot(ey - sy);
                let _ = write!(
                    self.defs,
                    r#"<radialGradient id="{id}" gradientUnits="userSpaceOnUse" cx="{sx}" cy="{sy}" r="{radius}">{stops}</radialGradient>"#
                );
            } else {
                let _ = write!(
                    self.defs,
                    r#"<linearGradient id="{id}" gradientUnits="userSpaceOnUse" x1="{sx}" y1="{sy}" x2="{ex}" y2="{ey}">{stops}</linearGradient>"#
                );
            }
            return Some(format!(r#"fill="url(#{id})""#));
        }
        if let Some(image) = fill.get("fillImage").filter(|i| i.is_object()) {
            let href = self.media(image["id"].as_str())?;
            let id = self.id("p");
            let _ = write!(
                self.defs,
                r#"<pattern id="{id}" patternUnits="userSpaceOnUse" x="{x}" y="{y}" width="{w}" height="{h}"><image href="{href}" width="{w}" height="{h}" preserveAspectRatio="xMidYMid slice"/></pattern>"#
            );
            return Some(format!(r#"fill="url(#{id})""#));
        }
        let color = fill["fillColor"].as_str()?;
        let opacity = fill["fillOpacity"].as_f64().unwrap_or(1.0);
        Some(format!(r#"fill="{color}" fill-opacity="{opacity}""#))
    }

    /// A `data:` URI for a media the file holds.
    fn media(&self, id: Option<&str>) -> Option<String> {
        let bytes = self.file.media.get(id?)?;
        Some(format!(
            "data:{};base64,{}",
            mime(bytes),
            helpers4::hex::base64_encode(bytes)
        ))
    }

    /// The first visible drop shadow, as a filter in `defs`.
    fn shadow(&mut self, shape: &Value) -> Option<String> {
        let shadow =
            list(&shape["shadow"]).find(|s| s["style"] == "drop-shadow" && s["hidden"] != true)?;
        let id = self.id("s");
        let _ = write!(
            self.defs,
            r#"<filter id="{id}" x="-50%" y="-50%" width="200%" height="200%"><feDropShadow dx="{}" dy="{}" stdDeviation="{}" flood-color="{}" flood-opacity="{}"/></filter>"#,
            n(shadow, "offsetX"),
            n(shadow, "offsetY"),
            n(shadow, "blur") / 2.0,
            shadow["color"]["color"].as_str().unwrap_or("#000000"),
            shadow["color"]["opacity"].as_f64().unwrap_or(0.2)
        );
        Some(id)
    }

    /// Text from the runs Penpot laid out (`positionData`): each run on its own line, in its box.
    fn text(&mut self, shape: &Value) {
        for run in list(&shape["positionData"]) {
            let text = run["text"].as_str().unwrap_or("");
            if text.trim().is_empty() {
                continue;
            }
            let text = match run["textTransform"].as_str() {
                Some("uppercase") => text.to_uppercase(),
                Some("lowercase") => text.to_lowercase(),
                _ => text.to_owned(),
            };
            let family = run["fontFamily"].as_str().unwrap_or("");
            let weight = css_number(&run["fontWeight"]).unwrap_or(400.0);
            let italic = run["fontStyle"] == "italic";
            let size = css_number(&run["fontSize"]).unwrap_or(14.0);
            let height = n(run, "height");
            let Some(set) = self.typesetter.set(&Paragraph {
                text: &text,
                family,
                weight,
                italic,
                size,
                line_height: Some(height),
                letter_spacing: css_number(&run["letterSpacing"]).unwrap_or(0.0),
                align: Align::Left,
                width: None,
            }) else {
                let style = if italic { "Italic" } else { "" };
                self.missing_fonts
                    .insert(format!("{family} {weight}{style}"));
                continue;
            };
            let fill = list(&run["fills"]).next();
            let color = fill
                .and_then(|f| f["fillColor"].as_str())
                .unwrap_or("#000000");
            let opacity = fill.and_then(|f| f["fillOpacity"].as_f64()).unwrap_or(1.0);
            // A run's `y` is the bottom of its box.
            let _ = write!(
                self.body,
                r#"<path transform="translate({} {})" d="{}" fill="{color}" fill-opacity="{opacity}"/>"#,
                n(run, "x"),
                n(run, "y") - height,
                set.path
            );
        }
    }
}

fn list(value: &Value) -> impl Iterator<Item = &Value> {
    value.as_array().into_iter().flatten()
}

/// Penpot coordinates fit in f32.
#[allow(clippy::cast_possible_truncation)]
fn n(value: &Value, key: &str) -> f32 {
    value[key].as_f64().unwrap_or(0.0) as f32
}

/// A CSS number Penpot stores as text or as a number: `"14px"`, `"300"`, `0`; `None` for
/// `"normal"`.
#[allow(clippy::cast_possible_truncation)]
fn css_number(value: &Value) -> Option<f32> {
    match value {
        Value::Number(n) => n.as_f64().map(|n| n as f32),
        Value::String(s) => s.trim_end_matches("px").trim().parse().ok(),
        _ => None,
    }
}

/// `(x, y, width, height)` of a shape, in page coordinates, before its transform.
fn boxed(shape: &Value) -> (f32, f32, f32, f32) {
    (
        n(shape, "x"),
        n(shape, "y"),
        n(shape, "width"),
        n(shape, "height"),
    )
}

/// A shape's `transform` (rotation, flips), applied around its centre; empty when it has none.
fn transform(shape: &Value) -> String {
    let matrix = &shape["transform"];
    let entry = |k: &str, default: f32| {
        matrix[k].as_f64().map_or(default, |v| {
            #[allow(clippy::cast_possible_truncation)]
            let v = v as f32;
            v
        })
    };
    let (a, b, c, d) = (
        entry("a", 1.0),
        entry("b", 0.0),
        entry("c", 0.0),
        entry("d", 1.0),
    );
    if (a - 1.0).abs() < 1e-6 && b.abs() < 1e-6 && c.abs() < 1e-6 && (d - 1.0).abs() < 1e-6 {
        return String::new();
    }
    let (left, top, width, height) = boxed(shape);
    let (cx, cy) = (left + width / 2.0, top + height / 2.0);
    format!(
        "translate({cx} {cy}) matrix({a} {b} {c} {d} {} {}) translate({} {})",
        entry("e", 0.0),
        entry("f", 0.0),
        -cx,
        -cy
    )
}

/// A shape's outline, for a mask: rectangles and boards, ellipses, paths.
fn outline(shape: &Value) -> Option<String> {
    match shape["type"].as_str()? {
        "rect" | "frame" | "image" => Some(rect_path(shape)),
        "circle" => Some(ellipse_path(shape)),
        "path" | "bool" => content_path(&shape["content"]),
        _ => None,
    }
}

fn ellipse_path(shape: &Value) -> String {
    helpers4::geometry::ellipse_path(bounds(shape))
}

/// A rectangle's or board's outline, with its corner radii (`r1` top-left, then clockwise; `rx`
/// for older files).
fn rect_path(shape: &Value) -> String {
    let fallback = n(shape, "rx");
    let radius = |k: &str| {
        if shape[k].as_f64().is_some() {
            f64::from(n(shape, k))
        } else {
            f64::from(fallback)
        }
    };
    helpers4::geometry::rounded_rect_path(
        bounds(shape),
        [radius("r1"), radius("r2"), radius("r3"), radius("r4")],
    )
}

/// A shape's box, in page coordinates, before its transform.
fn bounds(shape: &Value) -> helpers4::geometry::Rect {
    let (x, y, w, h) = boxed(shape);
    helpers4::geometry::Rect::new(x.into(), y.into(), w.into(), h.into())
}

/// A path's `content` as SVG path data: `move-to`, `line-to`, `curve-to` and `close-path`
/// commands, in page coordinates.
fn content_path(content: &Value) -> Option<String> {
    let commands = content.as_array()?;
    let mut path = String::new();
    for command in commands {
        let p = &command["params"];
        let _ = match command["command"].as_str() {
            Some("move-to") => write!(path, "M{} {}", n(p, "x"), n(p, "y")),
            Some("line-to") => write!(path, "L{} {}", n(p, "x"), n(p, "y")),
            Some("curve-to") => write!(
                path,
                "C{} {} {} {} {} {}",
                n(p, "c1x"),
                n(p, "c1y"),
                n(p, "c2x"),
                n(p, "c2y"),
                n(p, "x"),
                n(p, "y")
            ),
            Some("close-path") => write!(path, "Z"),
            _ => Ok(()),
        };
    }
    (!path.is_empty()).then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_css_numbers() {
        assert_eq!(css_number(&serde_json::json!("14px")), Some(14.0));
        assert_eq!(css_number(&serde_json::json!("300")), Some(300.0));
        assert_eq!(css_number(&serde_json::json!(2)), Some(2.0));
        assert_eq!(css_number(&serde_json::json!("normal")), None);
    }

    #[test]
    fn rounds_each_corner() {
        let rect = serde_json::json!({ "x": 0, "y": 0, "width": 10, "height": 10, "r1": 2, "r2": 0, "r3": 0, "r4": 0 });
        assert_eq!(
            rect_path(&rect),
            "M2 0 L10 0 L10 10 L0 10 L0 2 A2 2 0 0 1 2 0 Z"
        );
    }
}
