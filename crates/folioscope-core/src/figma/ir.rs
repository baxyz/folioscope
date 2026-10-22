// SPDX-License-Identifier: AGPL-3.0-or-later

//! `.fig` → IR v1 (ADR 0008): pages, flows, top-level frames and their hotspots.

use crate::figma::FigFile;
use crate::figma::tree::{Node, Tree, node_id};
use std::collections::HashMap;

use crate::ir::{
    Action, Document, Easing, Flow, Frame, Hotspot, Interaction, OverlayPosition, OverlaySettings,
    Page, Point, Rect, Scroll, ScrollDirection, Side, SourceFormat, Transition, TransitionKind,
    Trigger,
};
use crate::kiwi::Value;

/// Converts a decoded `.fig` into the IR.
#[must_use]
pub fn to_ir(file: &FigFile) -> Document {
    let tree = Tree::new(file);
    let pages = tree
        .nodes()
        .filter(|n| n.kind == "DOCUMENT")
        .flat_map(|doc| tree.children(doc.id))
        .filter(|n| n.kind == "CANVAS" && !flag(n.value, "internalOnly"))
        .map(|canvas| page(&tree, canvas))
        .collect();
    Document {
        source: SourceFormat::Fig,
        pages,
    }
}

fn page(tree: &Tree<'_>, canvas: &Node<'_>) -> Page {
    let tops: Vec<&Node<'_>> = tree
        .children(canvas.id)
        .filter(|n| matches!(n.kind, "FRAME" | "SYMBOL" | "COMPONENT_SET" | "INSTANCE"))
        .collect();
    let mut flows: Vec<(String, Flow)> = tops
        .iter()
        .filter_map(|frame| {
            let start = frame.value.get("prototypeStartingPoint")?;
            let text = |k: &str| {
                start
                    .get(k)
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned()
            };
            Some((
                text("position"),
                Flow {
                    name: text("name"),
                    description: text("description"),
                    start: frame.id.to_string(),
                },
            ))
        })
        .collect();
    flows.sort_by(|a, b| a.0.cmp(&b.0));
    let mut flows: Vec<Flow> = flows.into_iter().map(|(_, flow)| flow).collect();
    // Files from before named flows have a single starting frame, set on the page.
    if flows.is_empty()
        && let Some(start) = canvas.value.get("prototypeStartNodeID").and_then(node_id)
        && tops.iter().any(|f| f.id == start)
    {
        flows.push(Flow {
            name: canvas.name.to_owned(),
            description: String::new(),
            start: start.to_string(),
        });
    }

    let frames = tops
        .iter()
        .map(|frame| {
            let mut hotspots = Vec::new();
            collect_hotspots(tree, frame, Affine::IDENTITY, false, &mut hotspots);
            resolve_scroll_targets(tree, frame, &mut hotspots);
            let (x, y) = translation(frame.value);
            let (width, height) = size(frame.value);
            Frame {
                id: frame.id.to_string(),
                name: frame.name.to_owned(),
                bounds: Rect {
                    x,
                    y,
                    width,
                    height,
                },
                hotspots,
                overlay: overlay(frame.value),
                scroll: scroll(tree, frame),
            }
        })
        .collect();

    Page {
        id: canvas.id.to_string(),
        name: canvas.name.to_owned(),
        flows,
        frames,
    }
}

/// Walks a frame's subtree; `to_frame` maps the node's own coordinates to the frame's, `fixed`
/// says the node is in a fixed child of a scrolling frame.
fn collect_hotspots(
    tree: &Tree<'_>,
    node: &Node<'_>,
    to_frame: Affine,
    fixed: bool,
    out: &mut Vec<Hotspot>,
) {
    if matches!(node.value.get("visible"), Some(Value::Bool(false))) {
        return;
    }
    let interactions: Vec<Interaction> = node
        .value
        .get("prototypeInteractions")
        .map_or(&[][..], Value::as_array)
        .iter()
        .filter(|i| !flag(i, "isDeleted"))
        .flat_map(|i| {
            let trigger = trigger(i.get("event"));
            i.get("actions")
                .map_or(&[][..], Value::as_array)
                .iter()
                .filter_map(action)
                .map(move |action| Interaction {
                    trigger: trigger.clone(),
                    action,
                })
        })
        .collect();
    if !interactions.is_empty() {
        let (width, height) = size(node.value);
        out.push(Hotspot {
            node: node.id.to_string(),
            name: node.name.to_owned(),
            bounds: to_frame.bounding_box(width, height),
            interactions,
            fixed,
        });
    }
    // Only the top-level frame's direct children can be fixed.
    let top = node
        .value
        .get("parentIndex")
        .and_then(|p| p.get("guid"))
        .and_then(node_id)
        .and_then(|id| tree.get(id))
        .is_some_and(|parent| parent.kind == "CANVAS");
    for child in tree.children(node.id) {
        let to_frame = to_frame.then(Affine::of(child.value));
        collect_hotspots(
            tree,
            child,
            to_frame,
            fixed || (top && is_fixed(child)),
            out,
        );
    }
}

fn is_fixed(node: &Node<'_>) -> bool {
    node.value.get("scrollBehavior").and_then(Value::as_str)
        == Some("FIXED_WHEN_CHILD_OF_SCROLLING_FRAME")
}

/// The frame's scrolling, when its content goes past its bounds in a direction it scrolls.
fn scroll(tree: &Tree<'_>, frame: &Node<'_>) -> Option<Scroll> {
    let direction = match frame.value.get("scrollDirection").and_then(Value::as_str)? {
        "VERTICAL" => ScrollDirection::Vertical,
        "HORIZONTAL" => ScrollDirection::Horizontal,
        "BOTH" => ScrollDirection::Both,
        _ => return None,
    };
    let (width, height) = size(frame.value);
    let (mut right, mut bottom) = (width, height);
    let mut fixed = Vec::new();
    for child in tree.children(frame.id) {
        if matches!(child.value.get("visible"), Some(Value::Bool(false))) {
            continue;
        }
        if is_fixed(child) {
            fixed.push(child.id.to_string());
            continue;
        }
        let (w, h) = size(child.value);
        let bounds = Affine::of(child.value).bounding_box(w, h);
        right = right.max(bounds.x + bounds.width);
        bottom = bottom.max(bounds.y + bounds.height);
    }
    match direction {
        ScrollDirection::Vertical => right = width,
        ScrollDirection::Horizontal => bottom = height,
        ScrollDirection::Both => {}
    }
    (right > width || bottom > height).then_some(Scroll {
        direction,
        content: Rect {
            x: 0.0,
            y: 0.0,
            width: right,
            height: bottom,
        },
        fixed,
    })
}

/// Fills in where each "scroll to" action of the frame's hotspots goes.
fn resolve_scroll_targets(tree: &Tree<'_>, frame: &Node<'_>, hotspots: &mut [Hotspot]) {
    let wanted = |a: &Action| matches!(a, Action::ScrollTo { .. });
    if !hotspots
        .iter()
        .any(|h| h.interactions.iter().any(|i| wanted(&i.action)))
    {
        return;
    }
    let mut bounds = HashMap::new();
    node_bounds(tree, frame, Affine::IDENTITY, &mut bounds);
    for interaction in hotspots.iter_mut().flat_map(|h| &mut h.interactions) {
        if let Action::ScrollTo { to, target, .. } = &mut interaction.action {
            *target = bounds.get(to.as_str()).copied();
        }
    }
}

fn node_bounds(
    tree: &Tree<'_>,
    node: &Node<'_>,
    to_frame: Affine,
    out: &mut HashMap<String, Rect>,
) {
    let (width, height) = size(node.value);
    out.insert(node.id.to_string(), to_frame.bounding_box(width, height));
    for child in tree.children(node.id) {
        node_bounds(tree, child, to_frame.then(Affine::of(child.value)), out);
    }
}

fn trigger(event: Option<&Value>) -> Trigger {
    let event_type = event
        .and_then(|e| e.get("interactionType"))
        .and_then(Value::as_str);
    match event_type {
        // Links from before triggers existed were clicks; migrated ones carry `NONE`.
        None | Some("NONE" | "ON_CLICK") => Trigger::Click,
        Some("ON_HOVER") => Trigger::Hover,
        Some("ON_PRESS") => Trigger::Press,
        Some("MOUSE_ENTER" | "MOUSE_IN") => Trigger::MouseEnter,
        Some("MOUSE_LEAVE" | "MOUSE_OUT") => Trigger::MouseLeave,
        Some("MOUSE_DOWN") => Trigger::MouseDown,
        Some("MOUSE_UP") => Trigger::MouseUp,
        Some("AFTER_TIMEOUT") => Trigger::AfterDelay {
            delay_ms: seconds_to_ms(event.and_then(|e| e.get("transitionTimeout"))),
        },
        Some("DRAG") => Trigger::Drag,
        Some("ON_KEY_DOWN") => Trigger::Key,
        Some(other) => Trigger::Other {
            name: other.to_owned(),
        },
    }
}

/// `None` for an empty placeholder (`connectionType: NONE`).
fn action(action: &Value) -> Option<Action> {
    let text = |k: &str| action.get(k).and_then(Value::as_str);
    Some(match text("connectionType").unwrap_or("NONE") {
        "NONE" => return None,
        "BACK" => Action::Back,
        "CLOSE" => Action::Close,
        "URL" => Action::OpenUrl {
            url: text("connectionURL").unwrap_or("").to_owned(),
            new_tab: flag(action, "openUrlInNewTab"),
        },
        "INTERNAL_NODE" => {
            let Some(to) = action.get("transitionNodeID").and_then(node_id) else {
                return Some(Action::Unsupported {
                    name: "INTERNAL_NODE without target".to_owned(),
                });
            };
            let to = to.to_string();
            let transition = transition(action);
            let offset = action.get("overlayRelativePosition").map(|v| {
                let (x, y) = vector(v);
                Point { x, y }
            });
            match text("navigationType").unwrap_or("NAVIGATE") {
                "NAVIGATE" => Action::Navigate { to, transition },
                "OVERLAY" => Action::Overlay {
                    to,
                    transition,
                    offset,
                },
                "SWAP" => Action::SwapOverlay {
                    to,
                    transition,
                    offset,
                },
                "SCROLL_TO" => Action::ScrollTo {
                    to,
                    transition,
                    target: None,
                    offset: action.get("extraScrollOffset").map(|v| {
                        let (x, y) = vector(v);
                        Point { x, y }
                    }),
                },
                other => Action::Unsupported {
                    name: other.to_owned(),
                },
            }
        }
        other => Action::Unsupported {
            name: other.to_owned(),
        },
    })
}

/// A frame's overlay settings, when the designer set them.
fn overlay(frame: &Value) -> Option<OverlaySettings> {
    let text = |k: &str| frame.get(k).and_then(Value::as_str);
    let position = match text("overlayPositionType")? {
        "TOP_LEFT" => OverlayPosition::TopLeft,
        "TOP_CENTER" => OverlayPosition::TopCenter,
        "TOP_RIGHT" => OverlayPosition::TopRight,
        "BOTTOM_LEFT" => OverlayPosition::BottomLeft,
        "BOTTOM_CENTER" => OverlayPosition::BottomCenter,
        "BOTTOM_RIGHT" => OverlayPosition::BottomRight,
        "MANUAL" => OverlayPosition::Manual,
        _ => OverlayPosition::Center,
    };
    let appearance = frame.get("overlayBackgroundAppearance");
    let backdrop = appearance
        .filter(|a| a.get("backgroundType").and_then(Value::as_str) == Some("SOLID_COLOR"))
        .and_then(|a| a.get("backgroundColor"))
        .map(|c| {
            let ch = |k: &str| c.get(k).and_then(Value::as_f32).unwrap_or(0.0);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let byte = |k: &str| (ch(k).clamp(0.0, 1.0) * 255.0).round() as u8;
            format!(
                "#{:02x}{:02x}{:02x}{:02x}",
                byte("r"),
                byte("g"),
                byte("b"),
                byte("a")
            )
        });
    Some(OverlaySettings {
        position,
        close_on_click_outside: text("overlayBackgroundInteraction")
            == Some("CLOSE_ON_CLICK_OUTSIDE"),
        backdrop,
    })
}

fn transition(action: &Value) -> Transition {
    let text = |k: &str| action.get(k).and_then(Value::as_str);
    let Some(kind) = text("transitionType") else {
        // An absent field takes the schema default: instant.
        return Transition::INSTANT;
    };
    let side = |name: &str| match name.rsplit('_').next() {
        Some("LEFT") => Side::Left,
        Some("RIGHT") => Side::Right,
        Some("TOP") => Side::Top,
        _ => Side::Bottom,
    };
    let kind = match kind {
        "INSTANT_TRANSITION" => TransitionKind::Instant,
        "DISSOLVE" | "FADE" => TransitionKind::Dissolve,
        "SMART_ANIMATE" | "MAGIC_MOVE" => TransitionKind::SmartAnimate,
        k if k.starts_with("SLIDE_FROM_") => TransitionKind::SlideIn { from: side(k) },
        k if k.starts_with("SLIDE_OUT_TO_") => TransitionKind::SlideOut { to: side(k) },
        k if k.starts_with("PUSH_FROM_") => TransitionKind::Push { from: side(k) },
        k if k.starts_with("MOVE_FROM_") => TransitionKind::MoveIn { from: side(k) },
        k if k.starts_with("MOVE_OUT_TO_") => TransitionKind::MoveOut { to: side(k) },
        other => TransitionKind::Other {
            name: other.to_owned(),
        },
    };
    let easing = match text("easingType") {
        Some("LINEAR") => Easing::Linear,
        Some("IN_CUBIC") => Easing::EaseIn,
        // Figma's default for a new transition.
        None | Some("OUT_CUBIC") => Easing::EaseOut,
        Some("INOUT_CUBIC") => Easing::EaseInOut,
        Some("CUSTOM_CUBIC") => match action.get("easingFunction").map(Value::as_array) {
            Some([x1, y1, x2, y2]) => Easing::CubicBezier {
                points: [x1, y1, x2, y2].map(|v| v.as_f32().unwrap_or(0.0)),
            },
            _ => Easing::EaseOut,
        },
        Some(other) => Easing::Other {
            name: other.to_owned(),
        },
    };
    Transition {
        kind,
        duration_ms: seconds_to_ms(action.get("transitionDuration")),
        easing,
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn seconds_to_ms(seconds: Option<&Value>) -> u32 {
    seconds
        .and_then(Value::as_f32)
        .map_or(0, |s| (s * 1000.0).round().max(0.0) as u32)
}

fn flag(value: &Value, name: &str) -> bool {
    matches!(value.get(name), Some(Value::Bool(true)))
}

fn size(v: &Value) -> (f32, f32) {
    v.get("size").map_or((0.0, 0.0), vector)
}

/// A Kiwi `Vector`: `{ x, y }`.
fn vector(v: &Value) -> (f32, f32) {
    let axis = |a: &str| v.get(a).and_then(Value::as_f32).unwrap_or(0.0);
    (axis("x"), axis("y"))
}

fn translation(v: &Value) -> (f32, f32) {
    let m = Affine::of(v);
    (m.e, m.f)
}

/// 2D affine map, column-major like SVG's `matrix(a b c d e f)`.
// TODO(helpers4): move to the helpers4 `geometry` module.
#[derive(Debug, Clone, Copy)]
struct Affine {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
}

impl Affine {
    const IDENTITY: Affine = Affine {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    /// A node's `transform`: from its own coordinates to its parent's.
    fn of(v: &Value) -> Affine {
        let Some(m) = v.get("transform") else {
            return Affine::IDENTITY;
        };
        let k = |name: &str, default: f32| m.get(name).and_then(Value::as_f32).unwrap_or(default);
        Affine {
            a: k("m00", 1.0),
            b: k("m10", 0.0),
            c: k("m01", 0.0),
            d: k("m11", 1.0),
            e: k("m02", 0.0),
            f: k("m12", 0.0),
        }
    }

    /// `self` after `inner`: maps `inner`'s input space to `self`'s output space.
    fn then(self, inner: Affine) -> Affine {
        Affine {
            a: self.a * inner.a + self.c * inner.b,
            b: self.b * inner.a + self.d * inner.b,
            c: self.a * inner.c + self.c * inner.d,
            d: self.b * inner.c + self.d * inner.d,
            e: self.a * inner.e + self.c * inner.f + self.e,
            f: self.b * inner.e + self.d * inner.f + self.f,
        }
    }

    fn bounding_box(self, width: f32, height: f32) -> Rect {
        let corners = [(0.0, 0.0), (width, 0.0), (width, height), (0.0, height)].map(|(x, y)| {
            (
                self.a * x + self.c * y + self.e,
                self.b * x + self.d * y + self.f,
            )
        });
        let xs = corners.map(|p| p.0);
        let ys = corners.map(|p| p.1);
        let min = |v: [f32; 4]| v.into_iter().fold(f32::INFINITY, f32::min);
        let max = |v: [f32; 4]| v.into_iter().fold(f32::NEG_INFINITY, f32::max);
        Rect {
            x: min(xs),
            y: min(ys),
            width: max(xs) - min(xs),
            height: max(ys) - min(ys),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Affine;

    #[test]
    fn composed_translations_add_up() {
        let parent = Affine {
            e: 10.0,
            f: 20.0,
            ..Affine::IDENTITY
        };
        let child = Affine {
            e: 5.0,
            f: 7.0,
            ..Affine::IDENTITY
        };
        let rect = parent.then(child).bounding_box(4.0, 3.0);
        assert_eq!(
            (rect.x, rect.y, rect.width, rect.height),
            (15.0, 27.0, 4.0, 3.0)
        );
    }

    #[test]
    fn rotation_gives_the_bounding_box() {
        // 90° rotation: (x, y) → (-y, x).
        let rotate = Affine {
            a: 0.0,
            b: 1.0,
            c: -1.0,
            d: 0.0,
            ..Affine::IDENTITY
        };
        let rect = rotate.bounding_box(4.0, 2.0);
        assert_eq!(
            (rect.x, rect.y, rect.width, rect.height),
            (-2.0, 0.0, 2.0, 4.0)
        );
    }
}
