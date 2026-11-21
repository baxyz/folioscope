// SPDX-License-Identifier: AGPL-3.0-or-later

//! `.sketch` → IR v1 (docs/spec/folio.md): pages, artboards as frames, their start points and links.
//! Sketch's prototype is a subset of Figma's: clicks only, navigate or back, slides.

use std::collections::HashMap;

use serde_json::Value;

use crate::ir::{
    Action, Document, Easing, Flow, Frame, Hotspot, Interaction, Page, Rect, Scroll,
    ScrollDirection, Side, SourceFormat, Transition, TransitionKind, Trigger,
};
use crate::sketch::SketchFile;

/// Sketch stores no duration for its animations; this is how long they look in Sketch's player.
const ANIMATION_MS: u32 = 300;

/// Converts a decoded `.sketch` into the IR.
#[must_use]
pub fn to_ir(file: &SketchFile) -> Document {
    let masters = symbol_masters(file);
    let pages = file
        .pages
        .iter()
        .map(|page| {
            let artboards: Vec<&Value> = layers(page)
                .filter(|layer| layer["_class"] == "artboard")
                .collect();
            Page {
                id: string(page, "do_objectID"),
                name: string(page, "name"),
                // Each artboard flagged as a flow home starts a flow, named after it.
                flows: artboards
                    .iter()
                    .filter(|a| a["isFlowHome"] == true)
                    .map(|a| Flow {
                        name: string(a, "name"),
                        description: String::new(),
                        start: string(a, "do_objectID"),
                    })
                    .collect(),
                frames: artboards
                    .iter()
                    .map(|artboard| {
                        let mut hotspots = Vec::new();
                        for child in layers(artboard) {
                            let fixed = child["isFixedToViewport"] == true;
                            collect(child, (0.0, 0.0), fixed, &masters, &mut hotspots);
                        }
                        let whole = rect(&artboard["frame"]);
                        // An artboard taller than its prototype's viewport scrolls under it.
                        let visible =
                            viewport_height(artboard).filter(|&h| h > 0.0 && h < whole.height);
                        Frame {
                            id: string(artboard, "do_objectID"),
                            name: string(artboard, "name"),
                            bounds: Rect {
                                height: visible.unwrap_or(whole.height),
                                ..whole
                            },
                            hotspots,
                            overlay: None,
                            scroll: visible.map(|_| Scroll {
                                direction: ScrollDirection::Vertical,
                                content: Rect {
                                    x: 0.0,
                                    y: 0.0,
                                    width: whole.width,
                                    height: whole.height,
                                },
                                fixed: layers(artboard)
                                    .filter(|l| l["isFixedToViewport"] == true)
                                    .map(|l| string(l, "do_objectID"))
                                    .collect(),
                                sticky: Vec::new(),
                            }),
                            components: Vec::new(),
                            scrollers: Vec::new(),
                        }
                    })
                    .collect(),
            }
        })
        .collect();
    Document {
        source: SourceFormat::Sketch,
        pages,
    }
}

/// The height of an artboard's prototype viewport (`prototypeViewport.size`, `"{375, 812}"`).
fn viewport_height(artboard: &Value) -> Option<f32> {
    let size = artboard["prototypeViewport"]["size"].as_str()?;
    let (_, height) = size.trim_matches(['{', '}']).split_once(',')?;
    height.trim().parse().ok()
}

/// Symbol masters by `symbolID`, from the pages and from libraries (`foreignSymbols`).
pub(crate) fn symbol_masters(file: &SketchFile) -> HashMap<&str, &Value> {
    let local = file.pages.iter().flat_map(layers);
    let foreign = file.document["foreignSymbols"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|f| &f["symbolMaster"]);
    local
        .chain(foreign)
        .filter(|layer| layer["_class"] == "symbolMaster")
        .filter_map(|master| Some((master["symbolID"].as_str()?, master)))
        .collect()
}

/// Walks a layer; `origin` is its parent's top-left corner in the artboard's coordinates.
fn collect(
    layer: &Value,
    origin: (f32, f32),
    fixed: bool,
    masters: &HashMap<&str, &Value>,
    out: &mut Vec<Hotspot>,
) {
    if layer["isVisible"] == false {
        return;
    }
    let mut bounds = rect(&layer["frame"]);
    bounds.x += origin.0;
    bounds.y += origin.1;
    if let Some(action) = layer.get("flow").and_then(action) {
        out.push(Hotspot {
            node: string(layer, "do_objectID"),
            name: string(layer, "name"),
            bounds,
            interactions: vec![Interaction {
                trigger: Trigger::Click,
                action,
            }],
            fixed,
            scroller: None,
        });
    }
    // An instance draws its master's layers, which may carry links of their own.
    let children = if layer["_class"] == "symbolInstance" {
        layer["symbolID"]
            .as_str()
            .and_then(|id| masters.get(id))
            .map_or(&Value::Null, |master| &master["layers"])
    } else {
        &layer["layers"]
    };
    for child in children.as_array().into_iter().flatten() {
        collect(child, (bounds.x, bounds.y), fixed, masters, out);
    }
}

fn action(flow: &Value) -> Option<Action> {
    Some(match flow["destinationArtboardID"].as_str()? {
        "back" => Action::Back,
        // A link inside a symbol that follows the instance's own: the instance carries it.
        "automatic" => return None,
        to => Action::Navigate {
            to: to.to_owned(),
            transition: transition(flow["animationType"].as_i64().unwrap_or(-1)),
        },
    })
}

fn transition(animation: i64) -> Transition {
    let from = match animation {
        1 => Side::Left,
        2 => Side::Right,
        3 => Side::Bottom,
        4 => Side::Top,
        // -1 in real files, 0 in the schema: no animation.
        _ => return Transition::INSTANT,
    };
    Transition {
        kind: TransitionKind::SlideIn { from },
        duration_ms: ANIMATION_MS,
        easing: Easing::EaseOut,
    }
}

pub(crate) fn layers(layer: &Value) -> impl Iterator<Item = &Value> {
    layer["layers"].as_array().into_iter().flatten()
}

fn string(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap_or("").to_owned()
}

/// A Sketch `rect` (`{ x, y, width, height }`); Sketch coordinates fit in f32.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn rect(frame: &Value) -> Rect {
    let n = |k: &str| frame[k].as_f64().unwrap_or(0.0) as f32;
    Rect {
        x: n("x"),
        y: n("y"),
        width: n("width"),
        height: n("height"),
    }
}
