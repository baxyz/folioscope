// SPDX-License-Identifier: AGPL-3.0-or-later

//! `.xd` → IR v1 (ADR 0008): artboards as frames, the home artboard as the flow's start, and
//! every node with an interaction as a hotspot. See `docs/format-xd.md`.

use std::collections::HashSet;

use helpers4::geometry::{Matrix, Rect as Box64};
use serde_json::Value;

use crate::ir::{
    Action, Document, Easing, Flow, Frame, Hotspot, Interaction, Page, Rect, Side, SourceFormat,
    Transition, TransitionKind, Trigger,
};
use crate::xd::XdFile;
use crate::xd::geometry::{children, local_box, matrix};

/// Converts a decoded `.xd` into the IR. XD has a single canvas: one page.
#[must_use]
pub fn to_ir(file: &XdFile) -> Document {
    let entries = artboard_entries(file);
    let home = home(file, &entries);
    let interactions = file.interactions["interactions"].as_object();
    let frames: Vec<Frame> = entries
        .iter()
        .map(|entry| {
            let mut hotspots = Vec::new();
            let origin = (entry.bounds.x, entry.bounds.y);
            if let Some(list) = interactions.and_then(|all| all.get(&entry.id)) {
                hotspots.push(Hotspot {
                    node: entry.id.clone(),
                    name: entry.name.clone(),
                    bounds: Rect {
                        x: 0.0,
                        y: 0.0,
                        width: entry.bounds.width,
                        height: entry.bounds.height,
                    },
                    interactions: interactions_of(list),
                    fixed: false,
                });
            }
            if let (Some(tree), Some(all)) = (file.artboards.get(&entry.id), interactions) {
                let mut seen = HashSet::new();
                for artboard in tree["children"].as_array().into_iter().flatten() {
                    for child in children(artboard) {
                        collect(
                            child,
                            Matrix::IDENTITY,
                            origin,
                            all,
                            &mut seen,
                            &mut hotspots,
                        );
                    }
                }
            }
            Frame {
                id: entry.id.clone(),
                name: entry.name.clone(),
                bounds: entry.bounds,
                hotspots,
                overlay: None,
                scroll: None,
                components: Vec::new(),
            }
        })
        .collect();
    let flows = frames
        .iter()
        .filter(|f| Some(f.id.as_str()) == home.as_deref())
        .map(|f| Flow {
            name: f.name.clone(),
            description: String::new(),
            start: f.id.clone(),
        })
        .collect();
    Document {
        source: SourceFormat::Xd,
        pages: vec![Page {
            id: file.manifest["id"].as_str().unwrap_or("").to_owned(),
            name: file.manifest["uxdesign#specName"]
                .as_str()
                .or_else(|| file.manifest["name"].as_str())
                .unwrap_or("")
                .to_owned(),
            flows,
            frames,
        }],
    }
}

/// An artboard as the manifest lists it.
pub(crate) struct Entry {
    pub id: String,
    pub name: String,
    /// In the canvas' coordinates.
    pub bounds: Rect,
}

pub(crate) fn artboard_entries(file: &XdFile) -> Vec<Entry> {
    file.manifest["children"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| c["path"] == "artwork")
        .flat_map(|artwork| artwork["children"].as_array().into_iter().flatten())
        .filter_map(|entry| {
            let id = entry["path"].as_str()?.strip_prefix("artboard-")?;
            #[allow(clippy::cast_possible_truncation)]
            let bound = |k: &str| entry["uxdesign#bounds"][k].as_f64().unwrap_or(0.0) as f32;
            Some(Entry {
                id: id.to_owned(),
                name: entry["name"].as_str().unwrap_or("").to_owned(),
                bounds: Rect {
                    x: bound("x"),
                    y: bound("y"),
                    width: bound("width"),
                    height: bound("height"),
                },
            })
        })
        .collect()
}

/// The artboard the prototype starts on: the one XD marks, else the top-left one, as XD does.
fn home(file: &XdFile, entries: &[Entry]) -> Option<String> {
    if let Some(id) = file.interactions["homeArtboard"].as_str()
        && entries.iter().any(|e| e.id == id)
    {
        return Some(id.to_owned());
    }
    entries
        .iter()
        .min_by(|a, b| {
            (a.bounds.y, a.bounds.x)
                .partial_cmp(&(b.bounds.y, b.bounds.x))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|e| e.id.clone())
}

/// Walks a node; `parent` takes its coordinates to the canvas'.
fn collect(
    node: &Value,
    parent: Matrix,
    origin: (f32, f32),
    all: &serde_json::Map<String, Value>,
    seen: &mut HashSet<String>,
    out: &mut Vec<Hotspot>,
) {
    if node["visible"] == false {
        return;
    }
    let to_canvas = parent.multiply(matrix(node));
    let id = node["id"].as_str().unwrap_or("");
    if let Some(list) = all.get(id)
        && seen.insert(id.to_owned())
        && let Some(local) = local_box(node)
    {
        let b: Box64 = to_canvas.bounding_box(local);
        #[allow(clippy::cast_possible_truncation)]
        let hotspot = Hotspot {
            node: id.to_owned(),
            name: node["name"].as_str().unwrap_or("").to_owned(),
            bounds: Rect {
                x: b.x as f32 - origin.0,
                y: b.y as f32 - origin.1,
                width: b.width as f32,
                height: b.height as f32,
            },
            interactions: interactions_of(list),
            fixed: false,
        };
        out.push(hotspot);
    }
    for child in children(node) {
        collect(child, to_canvas, origin, all, seen, out);
    }
}

fn interactions_of(list: &Value) -> Vec<Interaction> {
    list.as_array()
        .into_iter()
        .flatten()
        .map(|interaction| Interaction {
            trigger: trigger(interaction),
            action: action(interaction),
        })
        .collect()
}

fn trigger(interaction: &Value) -> Trigger {
    match interaction["triggerEvent"].as_str().unwrap_or("") {
        "tap" => Trigger::Click,
        "time" => Trigger::AfterDelay {
            delay_ms: millis(&interaction["properties"]["delay"]),
        },
        "drag" => Trigger::Drag,
        "keys-gamepad" => Trigger::Key,
        other => Trigger::Other {
            name: other.to_owned(),
        },
    }
}

fn action(interaction: &Value) -> Action {
    let properties = &interaction["properties"];
    let to = || properties["destination"].as_str().unwrap_or("").to_owned();
    let kind = interaction["action"].as_str().unwrap_or("");
    match kind {
        "artboard-transition" | "auto-animate" => Action::Navigate {
            to: to(),
            transition: transition(kind == "auto-animate", properties),
        },
        "previous-artboard" | "goToPreviousArtboard" => Action::Back,
        other => Action::Unsupported {
            name: other.to_owned(),
        },
    }
}

fn transition(smart: bool, properties: &Value) -> Transition {
    let name = properties["transition"].as_str().unwrap_or("none");
    // XD names the direction the content moves in: the new artboard enters from the opposite side.
    let entering = |direction: &str| match direction {
        "left" => Some(Side::Right),
        "right" => Some(Side::Left),
        "up" => Some(Side::Bottom),
        "down" => Some(Side::Top),
        _ => None,
    };
    let kind = if smart {
        TransitionKind::SmartAnimate
    } else if name == "dissolve" {
        TransitionKind::Dissolve
    } else if let Some(from) = name.strip_prefix("slide-").and_then(entering) {
        TransitionKind::SlideIn { from }
    } else if let Some(from) = name.strip_prefix("push-").and_then(entering) {
        TransitionKind::Push { from }
    } else if name == "none" {
        TransitionKind::Instant
    } else {
        TransitionKind::Other {
            name: name.to_owned(),
        }
    };
    if kind == TransitionKind::Instant {
        return Transition::INSTANT;
    }
    Transition {
        kind,
        duration_ms: millis(&properties["duration"]),
        easing: easing(properties["easing"].as_str().unwrap_or("linear")),
    }
}

fn easing(name: &str) -> Easing {
    match name {
        "linear" => Easing::Linear,
        "ease-in" => Easing::EaseIn,
        "ease-out" => Easing::EaseOut,
        "ease-in-out" => Easing::EaseInOut,
        // XD's wind-up pulls back before it goes: CSS's ease-in-back.
        "wind-up" => Easing::CubicBezier {
            points: [0.6, -0.28, 0.735, 0.045],
        },
        other => Easing::Other {
            name: other.to_owned(),
        },
    }
}

/// Seconds, as XD writes durations and delays, in whole milliseconds.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn millis(seconds: &Value) -> u32 {
    (seconds.as_f64().unwrap_or(0.0).max(0.0) * 1000.0).round() as u32
}
