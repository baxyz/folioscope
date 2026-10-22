// SPDX-License-Identifier: AGPL-3.0-or-later

//! The intermediate model, v1: the prototype graph of a document, whatever its source format
//! (ADR 0008). Serialized as camelCase JSON into every `.folio`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceFormat {
    Fig,
    Sketch,
    Xd,
    Penpot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    pub source: SourceFormat,
    pub pages: Vec<Page>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub id: String,
    pub name: String,
    /// Starting points, in the order the designer set them.
    pub flows: Vec<Flow>,
    pub frames: Vec<Frame>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Flow {
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Id of the frame the flow starts on.
    pub start: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    pub id: String,
    pub name: String,
    /// Position and size on the canvas.
    pub bounds: Rect,
    /// Clickable areas, in layer order (later ones on top).
    pub hotspots: Vec<Hotspot>,
    /// How the frame shows when an action opens it as an overlay; the player's defaults
    /// (centered, no backdrop, no close on click outside) when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlay: Option<OverlaySettings>,
    /// Set when the frame's content scrolls past its bounds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scroll: Option<Scroll>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scroll {
    pub direction: ScrollDirection,
    /// What scrolls, in the frame's coordinates: from its top-left corner, at least its size.
    pub content: Rect,
    /// Ids of the direct children that stay in place while the rest scrolls.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fixed: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScrollDirection {
    Vertical,
    Horizontal,
    Both,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlaySettings {
    pub position: OverlayPosition,
    /// A click on the backdrop, outside the overlay, closes it.
    pub close_on_click_outside: bool,
    /// Color drawn over the frame below, as CSS `#rrggbbaa`; none when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backdrop: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OverlayPosition {
    Center,
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
    /// At the opening action's `offset`, from the top-left corner of the hotspot that opened it.
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// A node that carries interactions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hotspot {
    pub node: String,
    pub name: String,
    /// Bounding box in the frame's coordinates, after every transform.
    pub bounds: Rect,
    pub interactions: Vec<Interaction>,
    /// In a fixed child of a scrolling frame: stays in place while the content scrolls.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fixed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Interaction {
    pub trigger: Trigger,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Trigger {
    Click,
    /// While hovering: the action is undone when the pointer leaves.
    Hover,
    /// While pressing: undone on release.
    Press,
    MouseEnter,
    MouseLeave,
    MouseDown,
    MouseUp,
    AfterDelay {
        delay_ms: u32,
    },
    Drag,
    Key,
    /// A trigger the player does not handle, by its source name.
    Other {
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Action {
    Navigate {
        to: String,
        transition: Transition,
    },
    Overlay {
        to: String,
        transition: Transition,
        /// For a `Manual` overlay: its top-left corner, from the hotspot's.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        offset: Option<Point>,
    },
    /// Replaces the overlay on top with another one.
    SwapOverlay {
        to: String,
        transition: Transition,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        offset: Option<Point>,
    },
    ScrollTo {
        to: String,
        transition: Transition,
        /// Bounds of the node scrolled to, in the frame's coordinates.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<Rect>,
        /// Stops this far before the target (room for a fixed header, say).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        offset: Option<Point>,
    },
    Back,
    Close,
    OpenUrl {
        url: String,
        #[serde(default)]
        new_tab: bool,
    },
    /// An action the player does not handle, by its source name.
    Unsupported {
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transition {
    pub kind: TransitionKind,
    pub duration_ms: u32,
    pub easing: Easing,
}

impl Transition {
    pub const INSTANT: Transition = Transition {
        kind: TransitionKind::Instant,
        duration_ms: 0,
        easing: Easing::Linear,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TransitionKind {
    Instant,
    Dissolve,
    /// The new screen slides in over the old one, from `from`.
    SlideIn {
        from: Side,
    },
    /// The old screen slides out towards `to`, revealing the new one.
    SlideOut {
        to: Side,
    },
    /// Both screens move together, the new one entering from `from`.
    Push {
        from: Side,
    },
    MoveIn {
        from: Side,
    },
    MoveOut {
        to: Side,
    },
    /// Matching layers animate between the two screens (Figma's smart animate, XD's
    /// auto-animate).
    SmartAnimate,
    /// A transition the player does not handle, by its source name: played as a dissolve.
    Other {
        name: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Easing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
    /// CSS `cubic-bezier(x1, y1, x2, y2)`.
    CubicBezier {
        points: [f32; 4],
    },
    /// A spring or another named curve, by its source name.
    Other {
        name: String,
    },
}
