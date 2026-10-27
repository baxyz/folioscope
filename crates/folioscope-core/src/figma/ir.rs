// SPDX-License-Identifier: AGPL-3.0-or-later

//! `.fig` → IR v1 (ADR 0008): pages, flows, top-level frames and their hotspots.

use crate::figma::FigFile;
use helpers4::geometry::{self, Matrix};

use crate::figma::instance::Overrides;
use crate::figma::tree::{Node, NodeId, Tree, node_id};
use std::collections::HashMap;

use crate::ir::{
    Action, Component, ComponentState, Document, Easing, Flow, Frame, Hotspot, Interaction,
    OverlayPosition, OverlaySettings, Page, Point, Rect, Scroll, ScrollDirection, Side,
    SourceFormat, Transition, TransitionKind, Trigger,
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
            let mut walk = Walk::new(tree);
            walk.node(frame, Affine::IDENTITY, false, &mut Vec::new(), false);
            let (mut hotspots, components) = (walk.hotspots, walk.components);
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
                components,
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

/// At most this many variants are baked per interactive component (decision D7); the others
/// stay unreachable.
const MAX_STATES: usize = 16;

/// Hotspots and interactive components of one frame.
struct Walk<'t, 'a> {
    tree: &'t Tree<'a>,
    hotspots: Vec<Hotspot>,
    components: Vec<Component>,
    /// Whether a component's subtree holds a "change to", by component id.
    changes_state: HashMap<NodeId, bool>,
    /// The instances walked through, whose overrides apply below them.
    overrides: Overrides,
}

impl<'t, 'a> Walk<'t, 'a> {
    fn new(tree: &'t Tree<'a>) -> Self {
        Self {
            tree,
            hotspots: Vec::new(),
            components: Vec::new(),
            changes_state: HashMap::new(),
            overrides: Overrides::default(),
        }
    }

    /// Walks `node`'s subtree; `to_frame` maps the node's own coordinates to the frame's (or the
    /// component state's), `fixed` says the node is in a fixed child of a scrolling frame, `path`
    /// lists the instances above it. Instances are walked through their component's children;
    /// interactive ones become components, unless `in_state` (no nesting).
    fn node(
        &mut self,
        node: &Node<'a>,
        to_frame: Affine,
        fixed: bool,
        path: &mut Vec<String>,
        in_state: bool,
    ) {
        // Inside instances, the node as the instances above it show it.
        let resolved = self.overrides.resolve(node);
        let node = &Node {
            id: node.id,
            kind: node.kind,
            name: node.name,
            value: &resolved,
        };
        if matches!(node.value.get("visible"), Some(Value::Bool(false))) {
            return;
        }
        let symbol = symbol_of(node).and_then(|id| self.tree.get(id));
        if let Some(symbol) = symbol
            && !in_state
            && (has_change_to(&interactions(node)) || self.changes_state(symbol))
        {
            let component = self.component(node, symbol, to_frame, fixed, path);
            self.components.push(component);
            return;
        }
        // An instance carries its component's interactions, unless it overrides them.
        let mut own = interactions(node);
        if own.is_empty()
            && let Some(symbol) = symbol
        {
            own = interactions(symbol);
        }
        if !own.is_empty() {
            let (width, height) = size(node.value);
            self.hotspots.push(Hotspot {
                node: node.id.to_string(),
                name: node.name.to_owned(),
                bounds: to_frame.bounding_box(width, height),
                interactions: own,
                fixed,
            });
        }
        // Only the top-level frame's direct children can be fixed.
        let top = parent(self.tree, node).is_some_and(|p| p.kind == "CANVAS");
        let source = symbol.map_or(node.id, |s| s.id);
        if symbol.is_some() {
            path.push(node.id.to_string());
            self.overrides.enter(node, node.value);
        }
        let children: Vec<&Node<'a>> = self.tree.children(source).collect();
        for child in children {
            let to_frame = to_frame.then(Affine::of(&self.overrides.resolve(child)));
            let fixed = fixed || (top && is_fixed(child));
            self.node(child, to_frame, fixed, path, in_state);
        }
        if symbol.is_some() {
            path.pop();
            self.overrides.leave();
        }
    }

    /// An interactive instance: its first variant, then every variant a "change to" reaches.
    fn component(
        &mut self,
        instance: &Node<'_>,
        symbol: &Node<'a>,
        to_frame: Affine,
        fixed: bool,
        path: &[String],
    ) -> Component {
        let (width, height) = size(instance.value);
        let mut states: Vec<ComponentState> = Vec::new();
        let mut queue = vec![symbol.id];
        while let Some(variant) = queue.pop() {
            if states.len() == MAX_STATES || states.iter().any(|s| s.variant == variant.to_string())
            {
                continue;
            }
            let Some(variant) = self.tree.get(variant) else {
                continue;
            };
            let mut walk = Walk::new(self.tree);
            walk.node(variant, Affine::IDENTITY, false, &mut Vec::new(), true);
            let mut hotspots = walk.hotspots;
            // The instance's own interactions replace its first variant's.
            let overrides = interactions(instance);
            if variant.id == symbol.id && !overrides.is_empty() {
                hotspots.retain(|h| h.node != variant.id.to_string());
                let (w, h) = size(variant.value);
                hotspots.insert(
                    0,
                    Hotspot {
                        node: instance.id.to_string(),
                        name: instance.name.to_owned(),
                        bounds: Affine::IDENTITY.bounding_box(w, h),
                        interactions: overrides,
                        fixed: false,
                    },
                );
            }
            for interaction in hotspots.iter().flat_map(|h| &h.interactions) {
                if let Action::ChangeTo { to, .. } = &interaction.action
                    && let Ok(to) = to.parse()
                {
                    queue.insert(0, to);
                }
            }
            // The variant the instance shows has the instance's size, after auto layout, and so
            // has a variant laid out like it (the same size in the component); any other keeps
            // its own.
            let (w, h) = if variant.id == symbol.id || size(variant.value) == size(symbol.value) {
                (width, height)
            } else {
                size(variant.value)
            };
            states.push(ComponentState {
                variant: variant.id.to_string(),
                name: variant.name.to_owned(),
                bounds: Affine::IDENTITY.bounding_box(w, h),
                hotspots,
            });
        }
        let mut id = path.to_vec();
        id.push(instance.id.to_string());
        Component {
            id: id.join(";"),
            name: instance.name.to_owned(),
            bounds: to_frame.bounding_box(width, height),
            initial: symbol.id.to_string(),
            fixed,
            states,
        }
    }

    /// Whether the component (a variant) holds a "change to" in its subtree. Nested instances
    /// do not count: the walk reaches them and decides for each.
    fn changes_state(&mut self, symbol: &Node<'a>) -> bool {
        if let Some(&known) = self.changes_state.get(&symbol.id) {
            return known;
        }
        let mut stack = vec![symbol.id];
        let mut found = false;
        while let Some(id) = stack.pop() {
            let Some(node) = self.tree.get(id) else {
                continue;
            };
            if node.kind == "INSTANCE" {
                continue;
            }
            if has_change_to(&interactions(node)) {
                found = true;
                break;
            }
            stack.extend(self.tree.children(id).map(|c| c.id));
        }
        self.changes_state.insert(symbol.id, found);
        found
    }
}

/// The node's prototype interactions, as stored on it.
fn interactions(node: &Node<'_>) -> Vec<Interaction> {
    node.value
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
        .collect()
}

fn has_change_to(interactions: &[Interaction]) -> bool {
    interactions
        .iter()
        .any(|i| matches!(i.action, Action::ChangeTo { .. }))
}

/// The component an instance draws.
fn symbol_of(node: &Node<'_>) -> Option<NodeId> {
    if node.kind != "INSTANCE" {
        return None;
    }
    node.value
        .get("symbolData")
        .and_then(|s| s.get("symbolID"))
        .and_then(node_id)
}

fn parent<'t, 'a>(tree: &'t Tree<'a>, node: &Node<'_>) -> Option<&'t Node<'a>> {
    node.value
        .get("parentIndex")
        .and_then(|p| p.get("guid"))
        .and_then(node_id)
        .and_then(|id| tree.get(id))
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
                "SWAP_STATE" => Action::ChangeTo { to, transition },
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

#[allow(clippy::cast_possible_truncation)] // design coordinates fit in f32
fn translation(v: &Value) -> (f32, f32) {
    let m = Affine::of(v).0;
    (m.e as f32, m.f as f32)
}

/// A node's transform, as helpers4's [`Matrix`] (the layout of SVG's `matrix(a b c d e f)`).
#[derive(Debug, Clone, Copy)]
struct Affine(Matrix);

impl Affine {
    const IDENTITY: Affine = Affine(Matrix::IDENTITY);

    /// A node's `transform`: from its own coordinates to its parent's.
    fn of(v: &Value) -> Affine {
        let Some(m) = v.get("transform") else {
            return Affine::IDENTITY;
        };
        let k = |name: &str, default: f32| {
            f64::from(m.get(name).and_then(Value::as_f32).unwrap_or(default))
        };
        Affine(Matrix::new(
            k("m00", 1.0),
            k("m10", 0.0),
            k("m01", 0.0),
            k("m11", 1.0),
            k("m02", 0.0),
            k("m12", 0.0),
        ))
    }

    /// `self` after `inner`: maps `inner`'s input space to `self`'s output space.
    fn then(self, inner: Affine) -> Affine {
        Affine(self.0.multiply(inner.0))
    }

    /// Where a `width`×`height` box at the origin lands: its axis-aligned bounding box.
    #[allow(clippy::cast_possible_truncation)] // design coordinates fit in f32
    fn bounding_box(self, width: f32, height: f32) -> Rect {
        let area = self.0.bounding_box(geometry::Rect::new(
            0.0,
            0.0,
            f64::from(width),
            f64::from(height),
        ));
        Rect {
            x: area.x as f32,
            y: area.y as f32,
            width: area.width as f32,
            height: area.height as f32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Affine, Matrix};

    #[test]
    fn composed_translations_add_up() {
        let parent = Affine(Matrix::translate(10.0, 20.0));
        let child = Affine(Matrix::translate(5.0, 7.0));
        let rect = parent.then(child).bounding_box(4.0, 3.0);
        assert_eq!(
            (rect.x, rect.y, rect.width, rect.height),
            (15.0, 27.0, 4.0, 3.0)
        );
    }

    #[test]
    fn rotation_gives_the_bounding_box() {
        // 90° rotation: (x, y) → (-y, x).
        let rotate = Affine(Matrix::new(0.0, 1.0, -1.0, 0.0, 0.0, 0.0));
        let rect = rotate.bounding_box(4.0, 2.0);
        assert_eq!(
            (rect.x, rect.y, rect.width, rect.height),
            (-2.0, 0.0, 2.0, 4.0)
        );
    }
}
