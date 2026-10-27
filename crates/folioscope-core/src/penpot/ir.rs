// SPDX-License-Identifier: AGPL-3.0-or-later

//! `.penpot` → IR v1 (ADR 0008): pages, top-level boards as frames, named flows, and the
//! interactions of every shape. Penpot keeps geometry in page coordinates; the IR wants frame
//! coordinates. Overlay settings live on Penpot's interactions, and on the IR's destination frame.

use std::collections::HashMap;

use serde_json::Value;

use crate::ir::{
    Action, Document, Easing, Flow, Frame, Hotspot, Interaction, OverlayPosition, OverlaySettings,
    Page, Point, Rect, Side, SourceFormat, Transition, TransitionKind, Trigger,
};
use crate::penpot::{PenpotFile, PenpotPage, ROOT};

/// Penpot's "background overlay": a dark veil over the screen below.
const BACKDROP: &str = "#00000033";

/// Converts a decoded `.penpot` into the IR.
#[must_use]
pub fn to_ir(file: &PenpotFile) -> Document {
    Document {
        source: SourceFormat::Penpot,
        pages: file.pages.iter().map(page).collect(),
    }
}

fn page(page: &PenpotPage) -> Page {
    let boards = boards(page);
    let mut frames: Vec<Frame> = boards
        .iter()
        .map(|board| {
            let origin = (number(board, "x"), number(board, "y"));
            let mut hotspots = Vec::new();
            collect(board, origin, page, &mut hotspots);
            Frame {
                id: string(board, "id"),
                name: string(board, "name"),
                bounds: Rect {
                    x: origin.0,
                    y: origin.1,
                    width: number(board, "width"),
                    height: number(board, "height"),
                },
                hotspots,
                overlay: None,
                scroll: None,
                components: Vec::new(),
            }
        })
        .collect();
    // An overlay's settings are on the interactions that open it: the first one decides.
    let mut settings: HashMap<String, OverlaySettings> = HashMap::new();
    for board in &boards {
        overlay_settings(board, page, &mut settings);
    }
    for frame in &mut frames {
        frame.overlay = settings.remove(&frame.id);
    }
    let mut flows: Vec<(u64, Flow)> = page.page["flows"]
        .as_object()
        .into_iter()
        .flat_map(|flows| flows.values())
        .filter_map(|f| {
            Some((
                f["index"].as_u64().unwrap_or(0),
                Flow {
                    name: string(f, "name"),
                    description: String::new(),
                    start: f["startingFrame"].as_str()?.to_owned(),
                },
            ))
        })
        .collect();
    flows.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.name.cmp(&b.1.name)));
    Page {
        id: string(&page.page, "id"),
        name: string(&page.page, "name"),
        flows: flows.into_iter().map(|(_, flow)| flow).collect(),
        frames,
    }
}

/// The page's top-level boards, top of the layer list first, as Penpot's layer panel lists them.
pub(crate) fn boards(page: &PenpotPage) -> Vec<&Value> {
    page.shapes.get(ROOT).map_or(Vec::new(), |root| {
        children(root, page)
            .into_iter()
            .filter(|s| s["type"] == "frame")
            .collect()
    })
}

/// A shape's children, top of the layer list first (the file lists them bottom first).
pub(crate) fn children<'p>(shape: &Value, page: &'p PenpotPage) -> Vec<&'p Value> {
    shape["shapes"]
        .as_array()
        .into_iter()
        .flatten()
        .rev()
        .filter_map(|id| page.shapes.get(id.as_str()?))
        .collect()
}

/// Hotspots of a shape and of its children; `origin` is the board's top-left on the page.
fn collect(shape: &Value, origin: (f32, f32), page: &PenpotPage, out: &mut Vec<Hotspot>) {
    if shape["hidden"] == true {
        return;
    }
    let bounds = bounds_in(shape, origin);
    let interactions: Vec<Interaction> = shape["interactions"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|i| interaction(i, bounds, origin, page))
        .collect();
    if !interactions.is_empty() {
        out.push(Hotspot {
            node: string(shape, "id"),
            name: string(shape, "name"),
            bounds,
            interactions,
            fixed: false,
        });
    }
    for child in children(shape, page) {
        collect(child, origin, page, out);
    }
}

fn interaction(
    interaction: &Value,
    hotspot: Rect,
    origin: (f32, f32),
    page: &PenpotPage,
) -> Option<Interaction> {
    let trigger = match interaction["eventType"].as_str()? {
        "click" => Trigger::Click,
        // Penpot's "mouse press" lasts while the button is down, like Figma's "while pressing".
        "mouse-press" => Trigger::Press,
        "mouse-enter" => Trigger::MouseEnter,
        "mouse-leave" => Trigger::MouseLeave,
        "after-delay" => Trigger::AfterDelay {
            delay_ms: millis(&interaction["delay"]),
        },
        other => Trigger::Other {
            name: other.to_owned(),
        },
    };
    let to = || string(interaction, "destination");
    let transition = || transition(&interaction["animation"]);
    let action = match interaction["actionType"].as_str()? {
        "navigate" => Action::Navigate {
            to: to(),
            transition: transition(),
        },
        // Toggling opens the overlay; the player closes it on the next toggle as on a close.
        "open-overlay" | "toggle-overlay" => Action::Overlay {
            to: to(),
            transition: transition(),
            offset: manual_offset(interaction, hotspot, origin, page),
        },
        "close-overlay" => Action::Close,
        "prev-screen" => Action::Back,
        "open-url" => Action::OpenUrl {
            url: string(interaction, "url"),
            new_tab: true,
        },
        other => Action::Unsupported {
            name: other.to_owned(),
        },
    };
    Some(Interaction { trigger, action })
}

/// A manual overlay's top-left corner, from the hotspot's. Penpot places it from the board, or
/// from the shape in `positionRelativeTo`.
fn manual_offset(
    interaction: &Value,
    hotspot: Rect,
    origin: (f32, f32),
    page: &PenpotPage,
) -> Option<Point> {
    if interaction["overlayPosType"] != "manual" {
        return None;
    }
    let position = &interaction["overlayPosition"];
    let base = interaction["positionRelativeTo"]
        .as_str()
        .and_then(|id| page.shapes.get(id))
        .filter(|shape| shape["type"] != "frame" || shape["parentId"] != ROOT)
        .map_or(Rect::default(), |shape| bounds_in(shape, origin));
    Some(Point {
        x: base.x + number(position, "x") - hotspot.x,
        y: base.y + number(position, "y") - hotspot.y,
    })
}

/// Overlay settings of the boards the interactions under `shape` open.
fn overlay_settings(shape: &Value, page: &PenpotPage, out: &mut HashMap<String, OverlaySettings>) {
    for interaction in shape["interactions"].as_array().into_iter().flatten() {
        let opens = matches!(
            interaction["actionType"].as_str(),
            Some("open-overlay" | "toggle-overlay")
        );
        let Some(to) = interaction["destination"].as_str().filter(|_| opens) else {
            continue;
        };
        out.entry(to.to_owned()).or_insert_with(|| OverlaySettings {
            position: match interaction["overlayPosType"].as_str() {
                Some("top-left") => OverlayPosition::TopLeft,
                Some("top-center") => OverlayPosition::TopCenter,
                Some("top-right") => OverlayPosition::TopRight,
                Some("bottom-left") => OverlayPosition::BottomLeft,
                Some("bottom-center") => OverlayPosition::BottomCenter,
                Some("bottom-right") => OverlayPosition::BottomRight,
                Some("manual") => OverlayPosition::Manual,
                _ => OverlayPosition::Center,
            },
            close_on_click_outside: interaction["closeClickOutside"] == true,
            backdrop: (interaction["backgroundOverlay"] == true).then(|| BACKDROP.to_owned()),
        });
    }
    for child in children(shape, page) {
        overlay_settings(child, page, out);
    }
}

fn transition(animation: &Value) -> Transition {
    let Some(kind) = animation["animationType"].as_str() else {
        return Transition::INSTANT;
    };
    // Penpot names the direction things move in: "right" comes in from the left.
    let side = match animation["direction"].as_str() {
        Some("left") => Side::Right,
        Some("up") => Side::Bottom,
        Some("down") => Side::Top,
        _ => Side::Left,
    };
    let opposite = match side {
        Side::Left => Side::Right,
        Side::Right => Side::Left,
        Side::Top => Side::Bottom,
        Side::Bottom => Side::Top,
    };
    let kind = match kind {
        "dissolve" => TransitionKind::Dissolve,
        "slide" if animation["way"] == "out" => TransitionKind::SlideOut { to: opposite },
        "slide" => TransitionKind::SlideIn { from: side },
        "push" => TransitionKind::Push { from: side },
        other => TransitionKind::Other {
            name: other.to_owned(),
        },
    };
    Transition {
        kind,
        duration_ms: millis(&animation["duration"]),
        easing: match animation["easing"].as_str() {
            Some("linear") => Easing::Linear,
            Some("ease-in") => Easing::EaseIn,
            Some("ease-out") => Easing::EaseOut,
            Some("ease-in-out") => Easing::EaseInOut,
            // CSS `ease`, Penpot's default.
            _ => Easing::CubicBezier {
                points: [0.25, 0.1, 0.25, 1.0],
            },
        },
    }
}

/// A shape's bounding box (`selrect`, after rotation), from the board's top-left.
pub(crate) fn bounds_in(shape: &Value, origin: (f32, f32)) -> Rect {
    let rect = if shape["selrect"].is_object() {
        &shape["selrect"]
    } else {
        shape
    };
    Rect {
        x: number(rect, "x") - origin.0,
        y: number(rect, "y") - origin.1,
        width: number(rect, "width"),
        height: number(rect, "height"),
    }
}

fn millis(value: &Value) -> u32 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let ms = value.as_f64().unwrap_or(0.0).max(0.0).round() as u32;
    ms
}

/// Penpot coordinates fit in f32.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn number(value: &Value, key: &str) -> f32 {
    value[key].as_f64().unwrap_or(0.0) as f32
}

fn string(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap_or("").to_owned()
}
