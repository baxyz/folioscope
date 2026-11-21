// SPDX-License-Identifier: AGPL-3.0-or-later

//! Draws `.sketch` artboards as SVG, from their JSON layers (see `docs/format-sketch.md`).
//!
//! Handled: groups, shapes from their curve points (rectangles, ovals, paths, boolean groups as
//! one path), colour, gradient and image fills, borders, drop shadows, bitmaps, symbol instances
//! (text overrides included) and text, laid out with the fonts given (docs/spec/bake.md). Not yet: masks,
//! blurs, blend modes, inner shadows, per-run text styles (a text takes its first run's style).

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;

use crate::ir::{self, Rect};
use crate::sketch::SketchFile;
use helpers4::color::Rgba;
use serde_json::Value;

use crate::bake::fx;

use crate::bake::draw::Draw;
use crate::bake::folio::BakeError;
use crate::bake::fonts::weight_of_style;
use crate::bake::svg::{image_size, mime};
use crate::bake::text::{Align, Rich, Span, Typesetter};
use helpers4::geometry::{Matrix, transform_path};

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

/// What of an artboard to draw.
#[derive(Clone, Copy)]
enum Part {
    Whole,
    /// Its background, at the viewport's size.
    Background,
    /// Its layers but the fixed ones, at the artboard's whole size.
    Content,
    /// Its fixed layers only, at the viewport's size.
    Fixed,
}

impl SketchDraw<'_> {
    fn draw(&mut self, frame: &ir::Frame, part: Part) -> Option<String> {
        let artboard = *self.artboards.get(frame.id.as_str())?;
        let (width, height) = match (part, &frame.scroll) {
            (Part::Content | Part::Whole, Some(scroll)) => {
                (scroll.content.width, scroll.content.height)
            }
            _ => (frame.bounds.width, frame.bounds.height),
        };
        let fixed: &[String] = frame.scroll.as_ref().map_or(&[], |s| &s.fixed);
        let mut svg = Svg {
            file: self.file,
            masters: &self.masters,
            typesetter: &mut self.typesetter,
            missing_fonts: &mut self.missing_fonts,
            defs: String::new(),
            body: String::new(),
            next_id: 0,
            overrides: Vec::new(),
            artboard: Some(artboard),
            backdrop: fx::Backdrop::default(),
            top: match part {
                Part::Whole => None,
                Part::Background => Some((true, Vec::new())),
                Part::Content => Some((false, fixed.to_vec())),
                Part::Fixed => Some((true, fixed.to_vec())),
            },
        };
        // An artboard without a background color shows white, as in Sketch's player.
        let background = if artboard["hasBackgroundColor"] == true {
            color(&artboard["backgroundColor"])
        } else {
            ("rgb(255 255 255)".to_owned(), 1.0)
        };
        if matches!(part, Part::Whole | Part::Background) {
            let _ = write!(
                svg.body,
                r#"<rect width="{width}" height="{height}" fill="{}" fill-opacity="{}"/>"#,
                background.0, background.1
            );
        }
        svg.children(artboard);
        Some(format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}"><defs>{}</defs>{}</svg>"#,
            svg.defs, svg.body
        ))
    }
}

impl Draw for SketchDraw<'_> {
    fn frame(&mut self, frame: &ir::Frame) -> Option<String> {
        self.draw(frame, Part::Whole)
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
        let Some(scroll) = &frame.scroll else {
            return Ok(vec![(whole.to_owned(), bounds, false)]);
        };
        let missing = || BakeError::MissingFrame(frame.id.clone());
        let mut layers = vec![
            (
                self.draw(frame, Part::Background).ok_or_else(missing)?,
                bounds,
                false,
            ),
            (
                self.draw(frame, Part::Content).ok_or_else(missing)?,
                scroll.content,
                true,
            ),
        ];
        if !scroll.fixed.is_empty() {
            layers.push((
                self.draw(frame, Part::Fixed).ok_or_else(missing)?,
                bounds,
                false,
            ));
        }
        Ok(layers)
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
    /// The artboard drawn, for background blurs (none in a second drawing).
    artboard: Option<&'f Value>,
    backdrop: fx::Backdrop,
    /// Which of the artboard's own layers to draw: `(true, ids)` only those, `(false, ids)` all
    /// but those; all when absent.
    top: Option<(bool, Vec<String>)>,
}

impl<'f> Svg<'_, 'f> {
    fn id(&mut self, prefix: &str) -> String {
        self.next_id += 1;
        format!("{}{prefix}{}", self.backdrop.prefix, self.next_id)
    }

    fn layer(&mut self, layer: &'f Value) {
        if layer["isVisible"] == false
            || self
                .backdrop
                .stops_at(layer["do_objectID"].as_str().unwrap_or(""))
        {
            return;
        }
        let (width, height) = (n(&layer["frame"], "width"), n(&layer["frame"], "height"));
        let local = matrix(layer);
        let style = &layer["style"];
        let opacity = style["contextSettings"]["opacity"].as_f64().unwrap_or(1.0);
        let blend = fx::blend_style(
            style["contextSettings"]["blendMode"]
                .as_i64()
                .and_then(fx::sketch_blend),
        );
        let _ = write!(
            self.body,
            r#"<g transform="{}" opacity="{opacity}"{blend}>"#,
            svg_matrix(local)
        );
        self.backdrop.enter(local);
        self.background_blur(layer, style);
        let open = self.effects(style);
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
        for _ in 0..open {
            self.body.push_str("</g>");
        }
        self.body.push_str("</g>");
        self.backdrop.leave();
    }

    /// Opens the groups that draw a style's effects: every enabled shadow, inner shadow, and a
    /// layer blur (Gaussian, motion or zoom, all drawn Gaussian). Returns how many it opened.
    fn effects(&mut self, style: &Value) -> usize {
        let shadows = |list: &Value| -> Vec<fx::Shadow> {
            enabled(list)
                .map(|s| fx::Shadow {
                    dx: f64::from(n(s, "offsetX")),
                    dy: f64::from(n(s, "offsetY")),
                    blur: f64::from(n(s, "blurRadius")),
                    spread: f64::from(n(s, "spread")),
                    color: rgba(&s["color"]),
                })
                .collect()
        };
        let mut open = 0;
        let drops = shadows(&style["shadows"]);
        if !drops.is_empty() {
            let id = self.id("s");
            fx::drop_shadows(&mut self.defs, &id, &drops);
            let _ = write!(self.body, r#"<g filter="url(#{id})">"#);
            open += 1;
        }
        let blur = &style["blur"];
        if blur["isEnabled"] == true && blur["type"].as_i64() != Some(3) {
            let id = self.id("b");
            fx::layer_blur(&mut self.defs, &id, f64::from(n(blur, "radius")) * 2.0);
            let _ = write!(self.body, r#"<g filter="url(#{id})">"#);
            open += 1;
        }
        let inner = shadows(&style["innerShadows"]);
        if !inner.is_empty() {
            let id = self.id("i");
            fx::inner_shadows(&mut self.defs, &id, &inner);
            let _ = write!(self.body, r#"<g filter="url(#{id})">"#);
            open += 1;
        }
        open
    }

    /// A background blur (blur type 3): what is drawn behind the layer, blurred, in its outline.
    fn background_blur(&mut self, layer: &'f Value, style: &Value) {
        let blur = &style["blur"];
        if blur["isEnabled"] != true || blur["type"].as_i64() != Some(3) {
            return;
        }
        let (Some(artboard), Some(outline), Some(to_node)) =
            (self.artboard, outline(layer), self.backdrop.to_node())
        else {
            return;
        };
        self.next_id += 1;
        let prefix = format!("{}b{}_", self.backdrop.prefix, self.next_id);
        let mut typesetter = Typesetter::new(self.typesetter.library().clone());
        let mut missing = BTreeSet::new();
        let mut behind = Svg {
            file: self.file,
            masters: self.masters,
            typesetter: &mut typesetter,
            missing_fonts: &mut missing,
            defs: String::new(),
            body: String::new(),
            next_id: 0,
            overrides: Vec::new(),
            artboard: None,
            top: self.top.clone(),
            backdrop: fx::Backdrop {
                prefix: prefix.clone(),
                stop_at: layer["do_objectID"].as_str().map(str::to_owned),
                ..fx::Backdrop::default()
            },
        };
        behind.children(artboard);
        let (clip, filter) = (self.id("c"), self.id("f"));
        fx::backdrop(
            &mut self.defs,
            &mut self.body,
            &prefix,
            (&behind.defs, &behind.body),
            &format!(r#"<path d="{outline}"/>"#),
            (&clip, &filter),
            f64::from(n(blur, "radius")) * 2.0,
            to_node,
        );
    }

    /// Draws a group's layers. A layer that is a mask draws, then clips the layers after it to
    /// its outline, up to the end of the group or a layer that breaks the mask chain.
    fn children(&mut self, parent: &'f Value) {
        let mut masked = false;
        let top = if parent["_class"] == "artboard" {
            self.top.clone()
        } else {
            None
        };
        for child in children(parent) {
            if let Some((only, ids)) = &top {
                let listed = ids.iter().any(|id| child["do_objectID"] == id.as_str());
                if listed != *only {
                    continue;
                }
            }
            let mask = child["hasClippingMask"] == true && child["isVisible"] != false;
            if masked && (mask || child["shouldBreakMaskChain"] == true) {
                self.body.push_str("</g>");
                masked = false;
            }
            if mask && child["clippingMaskMode"].as_i64() == Some(1) {
                // An alpha mask: the layer is drawn, and its transparency masks what follows.
                let start = self.body.len();
                self.layer(child);
                let drawn = self.body[start..].to_owned();
                let id = self.id("m");
                let _ = write!(
                    self.defs,
                    r#"<mask id="{id}" mask-type="alpha" maskUnits="userSpaceOnUse" x="-100000" y="-100000" width="200000" height="200000">{drawn}</mask>"#
                );
                let _ = write!(self.body, r#"<g mask="url(#{id})">"#);
                masked = true;
                continue;
            }
            self.layer(child);
            if mask && let Some(outline) = outline(child) {
                let id = self.id("m");
                let _ = write!(
                    self.defs,
                    r#"<clipPath id="{id}"><path transform="{}" d="{outline}"/></clipPath>"#,
                    svg_matrix(matrix(child))
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
        self.backdrop.enter(Matrix::new(
            f64::from(sx),
            0.0,
            0.0,
            f64::from(sy),
            0.0,
            0.0,
        ));
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
        self.backdrop.leave();
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
            let mut attributes = format!(r#"stroke="{rgb}" stroke-opacity="{alpha}""#);
            let dashes: Vec<String> = style["borderOptions"]["dashPattern"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_f64)
                .map(|d| d.to_string())
                .collect();
            if !dashes.is_empty() {
                let _ = write!(attributes, r#" stroke-dasharray="{}""#, dashes.join(" "));
            }
            let align = match border["position"].as_i64() {
                Some(1) => fx::StrokeAlign::Inside,
                Some(2) => fx::StrokeAlign::Outside,
                _ => fx::StrokeAlign::Center,
            };
            let clip = self.id("k");
            fx::stroke(
                &mut self.defs,
                &mut self.body,
                &clip,
                d,
                &attributes,
                thickness,
                align,
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
                } else if gradient["gradientType"] == 2 {
                    // Angular: around the layer's centre, from its right, clockwise.
                    let stops: Vec<fx::Stop> = gradient["stops"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|stop| fx::Stop {
                            offset: stop["position"].as_f64().unwrap_or(0.0),
                            color: rgba(&stop["color"]),
                        })
                        .collect();
                    let (w, h) = (f64::from(w), f64::from(h));
                    return Some(fx::angular(
                        &mut self.defs,
                        &id,
                        &stops,
                        [w, 0.0, 0.0, h, 0.0, 0.0],
                        w,
                        h,
                    ));
                } else {
                    let _ = write!(
                        self.defs,
                        r#"<linearGradient id="{id}" gradientUnits="userSpaceOnUse" x1="{fx}" y1="{fy}" x2="{tx}" y2="{ty}">{stops}</linearGradient>"#
                    );
                }
                Some(format!(r#"fill="url(#{id})""#))
            }
            4 => {
                let href = self.image(&fill["image"])?;
                let id = self.id("p");
                // Tiles: the image at its own size, times the tile scale.
                if fill["patternFillType"].as_i64() == Some(0) {
                    let bytes = self.file.files.get(fill["image"]["_ref"].as_str()?)?;
                    let (iw, ih) = image_size(bytes)?;
                    #[allow(clippy::cast_possible_truncation)]
                    let scale = fill["patternTileScale"].as_f64().unwrap_or(1.0) as f32;
                    let (tw, th) = (iw * scale, ih * scale);
                    let _ = write!(
                        self.defs,
                        r#"<pattern id="{id}" patternUnits="userSpaceOnUse" width="{tw}" height="{th}"><image href="{href}" width="{tw}" height="{th}" preserveAspectRatio="none"/></pattern>"#
                    );
                    return Some(format!(r#"fill="url(#{id})""#));
                }
                let aspect = match fill["patternFillType"].as_i64() {
                    Some(2) => "none",
                    Some(3) => "xMidYMid meet",
                    _ => "xMidYMid slice",
                };
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

    /// A text layer, each run in its own font, size, colour and decorations; a run whose font is
    /// missing is reported and left out.
    #[allow(clippy::too_many_lines)] // one pass over the runs, then one layout
    fn text(&mut self, layer: &Value, w: f32, h: f32) {
        let attributed = &layer["attributedString"];
        let overridden = self.override_value(layer, "stringValue");
        let string = overridden.or(attributed["string"].as_str()).unwrap_or("");
        if string.trim().is_empty() {
            return;
        }
        let runs: Vec<&Value> = attributed["attributes"]
            .as_array()
            .into_iter()
            .flatten()
            .collect();
        let Some(first) = runs.first().map(|r| &r["attributes"]) else {
            return;
        };
        // Sketch counts UTF-16 units; an overridden text takes the first run's style throughout.
        let utf16_to_byte = |unit: usize| utf16_offset(string, unit);
        let mut spans = Vec::new();
        let mut colors = Vec::new();
        let library = self.typesetter.library().clone();
        let mut families = Vec::new();
        for run in &runs {
            let style = &run["attributes"];
            let font = &style["MSAttributedStringFontAttribute"]["attributes"];
            let name = font["name"].as_str().unwrap_or("");
            let Some(face) = library.resolve_postscript(name) else {
                self.missing_fonts.insert(name.to_owned());
                continue;
            };
            let weight = weight_of_style(name.split_once('-').map_or("Regular", |(_, s)| s))
                .clamp(face.weight[0], face.weight[1]);
            let index = |key: &str| usize::try_from(run[key].as_u64().unwrap_or(0)).unwrap_or(0);
            let range = if overridden.is_some() {
                0..string.len()
            } else {
                utf16_to_byte(index("location"))..utf16_to_byte(index("location") + index("length"))
            };
            families.push(face.family.clone());
            spans.push((range, weight, face.italic, n(font, "size").max(1.0), style));
            colors.push(color(&style["MSAttributedStringColorAttribute"]));
            if overridden.is_some() {
                break;
            }
        }
        if spans.is_empty() {
            return;
        }
        let paragraph = &first["paragraphStyle"];
        let align = match paragraph["alignment"].as_i64() {
            Some(1) => Align::Right,
            Some(2) => Align::Center,
            Some(3) => Align::Justify,
            _ => Align::Left,
        };
        let rich = Rich {
            text: string,
            spans: spans
                .iter()
                .zip(&families)
                .map(|((range, weight, italic, size, attributes), family)| Span {
                    range: range.clone(),
                    family,
                    weight: *weight,
                    italic: *italic,
                    size: *size,
                    letter_spacing: n(attributes, "kerning"),
                    underline: attributes["underlineStyle"].as_i64().unwrap_or(0) != 0,
                    strikethrough: attributes["strikethroughStyle"].as_i64().unwrap_or(0) != 0,
                })
                .collect(),
            line_height: Some(n(paragraph, "maximumLineHeight")).filter(|&h| h > 0.0),
            align,
            // Auto width (0) never wraps; fixed width and fixed size wrap at the frame.
            width: (layer["textBehaviour"] != 0).then_some(w),
        };
        let set = self.typesetter.set_rich(&rich);
        let top = match layer["style"]["textStyle"]["verticalAlignment"].as_i64() {
            Some(1) => (h - set.height) / 2.0,
            Some(2) => h - set.height,
            _ => 0.0,
        };
        for (path, (rgb, alpha)) in set.paths.iter().zip(&colors) {
            if path.is_empty() {
                continue;
            }
            let _ = write!(
                self.body,
                r#"<path transform="translate(0 {top})" d="{path}" fill="{rgb}" fill-opacity="{alpha}"/>"#
            );
        }
    }
}

/// The byte offset of the UTF-16 unit `unit` of `text` (Sketch counts in UTF-16).
fn utf16_offset(text: &str, unit: usize) -> usize {
    let mut units = 0;
    for (byte, c) in text.char_indices() {
        if units >= unit {
            return byte;
        }
        units += c.len_utf16();
    }
    text.len()
}

/// A layer's transform from its parent: its position, then its rotation around its centre and its
/// flips.
fn matrix(layer: &Value) -> Matrix {
    let frame = &layer["frame"];
    let (width, height) = (f64::from(n(frame, "width")), f64::from(n(frame, "height")));
    let mut m = Matrix::new(
        1.0,
        0.0,
        0.0,
        1.0,
        f64::from(n(frame, "x")),
        f64::from(n(frame, "y")),
    );
    let rotation = f64::from(n(layer, "rotation"));
    if rotation != 0.0 {
        // Sketch turns counterclockwise; SVG and the matrices turn clockwise.
        let (sin, cos) = (-rotation).to_radians().sin_cos();
        let (cx, cy) = (width / 2.0, height / 2.0);
        m = m.multiply(Matrix::new(
            cos,
            sin,
            -sin,
            cos,
            cx - cos * cx + sin * cy,
            cy - sin * cx - cos * cy,
        ));
    }
    let flip_x = layer["isFlippedHorizontal"] == true;
    let flip_y = layer["isFlippedVertical"] == true;
    if flip_x || flip_y {
        m = m.multiply(Matrix::new(
            if flip_x { -1.0 } else { 1.0 },
            0.0,
            0.0,
            if flip_y { -1.0 } else { 1.0 },
            if flip_x { width } else { 0.0 },
            if flip_y { height } else { 0.0 },
        ));
    }
    m
}

/// An SVG `matrix(…)`.
fn svg_matrix(m: Matrix) -> String {
    format!("matrix({} {} {} {} {} {})", m.a, m.b, m.c, m.d, m.e, m.f)
}

/// A Sketch colour (`red`, `green`, `blue`, `alpha` from 0 to 1).
fn rgba(c: &Value) -> Rgba {
    let unit = |k: &str| c[k].as_f64().unwrap_or(0.0);
    Rgba::from_unit(
        unit("red"),
        unit("green"),
        unit("blue"),
        c["alpha"].as_f64().unwrap_or(1.0),
    )
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
    let unit = |k: &str| c[k].as_f64().unwrap_or(0.0);
    (
        Rgba::from_unit(unit("red"), unit("green"), unit("blue"), 1.0)
            .to_rgb()
            .to_css(),
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

/// Moves path data by (`dx`, `dy`), as absolute commands.
fn offset_path(d: &str, dx: f32, dy: f32) -> String {
    transform_path(d, Matrix::translate(dx.into(), dy.into()), 6).unwrap_or_else(|_| d.to_owned())
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
        assert_eq!(offset_path("M0 0L10 -5Z", 2.0, 3.0), "M2 3 L12 -2 Z");
        assert_eq!(offset_path("M4 0H6", 1.0, 1.0), "M5 1 L7 1");
    }
}
