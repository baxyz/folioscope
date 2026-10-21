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
    },
    /// Replaces the overlay on top with another one.
    SwapOverlay {
        to: String,
        transition: Transition,
    },
    ScrollTo {
        to: String,
        transition: Transition,
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
