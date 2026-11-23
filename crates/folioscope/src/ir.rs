// SPDX-License-Identifier: AGPL-3.0-or-later

//! The intermediate model, v1: the prototype graph of a document, whatever its source format
//! (docs/spec/folio.md). Serialized as camelCase JSON into every `.folio`.

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
    /// Interactive component instances: their variants change in place ("change to").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<Component>,
    /// Frames inside the screen whose content scrolls in place.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scrollers: Vec<Scroller>,
}

/// A frame inside a screen whose content scrolls past its bounds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scroller {
    pub id: String,
    pub name: String,
    /// Where it shows, in the screen's coordinates.
    pub bounds: Rect,
    pub direction: ScrollDirection,
    /// What scrolls, from the frame's corner: at least its size.
    pub content: Rect,
    /// Ids of its direct children that stick at its top (or left) edge once scrolled to it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sticky: Vec<String>,
}

/// An instance whose variants the prototype switches between, each drawn as a state of its own.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    /// Ids of the instances from the frame down to this one, joined by `;` (instances nest).
    pub id: String,
    pub name: String,
    /// Where the instance sits, in the frame's coordinates.
    pub bounds: Rect,
    /// The variant shown first.
    pub initial: String,
    /// In a fixed child of a scrolling frame.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fixed: bool,
    /// Every variant reachable from the first one through "change to" actions.
    pub states: Vec<ComponentState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentState {
    /// Id of the variant.
    pub variant: String,
    pub name: String,
    /// The variant's size, from the instance's top-left corner.
    pub bounds: Rect,
    /// In the state's coordinates.
    pub hotspots: Vec<Hotspot>,
    /// Interactive components inside this state, in its coordinates; their ids are the instances
    /// from the state's variant down to them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<Component>,
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
    /// Ids of the direct children that scroll, then stick at the top (or left) edge.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sticky: Vec<String>,
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
    /// In a frame that scrolls inside the screen (one of [`Frame::scrollers`]), by its id; then
    /// `bounds` are in that frame's content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scroller: Option<String>,
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
    /// A drag (a swipe) across the hotspot.
    Drag,
    /// A key, or keys pressed together: browser key codes (`13` Enter, `37` to `40` arrows,
    /// `16` Shift…); any key when empty.
    Key {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        codes: Vec<u32>,
    },
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
        /// Closes the overlay instead when it is already open (Penpot's "toggle overlay").
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        toggle: bool,
    },
    /// Replaces the overlay on top with another one.
    SwapOverlay {
        to: String,
        transition: Transition,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        offset: Option<Point>,
    },
    /// Switches the interactive component the hotspot belongs to to another of its variants.
    ChangeTo {
        to: String,
        transition: Transition,
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

impl Action {
    /// "Open a link": a web address, an e-mail or a phone link, or an address with no scheme
    /// (the player resolves it against its page). A design file is not trusted with the page
    /// that plays it: any other scheme (`javascript:`, `data:`, `file:`…) is no link, and comes
    /// out as [`Action::Unsupported`]. The player checks again (docs/spec/player.md).
    #[must_use]
    pub fn open_url(url: &str, new_tab: bool) -> Self {
        if link_scheme(url).is_none_or(|scheme| LINK_SCHEMES.contains(&scheme.as_str())) {
            Action::OpenUrl {
                url: url.to_owned(),
                new_tab,
            }
        } else {
            Action::Unsupported {
                name: "URL".to_owned(),
            }
        }
    }
}

/// The schemes a link may have.
const LINK_SCHEMES: [&str; 4] = ["http", "https", "mailto", "tel"];

/// The scheme of `url` as a browser reads it, in lower case: it drops the tabs and line breaks
/// anywhere in it and the spaces and control characters around it first. `None` for an address
/// with no scheme.
fn link_scheme(url: &str) -> Option<String> {
    let cleaned: String = url
        .trim_matches(|c: char| c <= ' ')
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    let (scheme, _) = cleaned.split_once(':')?;
    let mut chars = scheme.chars();
    (chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')))
    .then(|| scheme.to_ascii_lowercase())
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
    /// A damped spring: it lasts as long as it takes to settle, whatever the duration says.
    Spring {
        mass: f32,
        stiffness: f32,
        damping: f32,
    },
    /// A spring or another named curve, by its source name.
    Other {
        name: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_url_keeps_web_mail_and_phone_links() {
        for url in [
            "https://folioscope.baxyz.dev",
            "HTTP://example.com/a?b#c",
            "mailto:hello@example.org",
            "tel:+33123456789",
            "case-study/page:2",
            "//example.com/page",
            "",
        ] {
            assert!(
                matches!(Action::open_url(url, true), Action::OpenUrl { url: kept, new_tab: true } if kept == url),
                "{url}"
            );
        }
    }

    #[test]
    fn open_url_refuses_schemes_that_run_code_or_read_files() {
        for url in [
            "javascript:alert(document.domain)",
            "JavaScript:alert(1)",
            "  javascript:alert(1)",
            "java\tscript:alert(1)",
            "java\nscript:alert(1)",
            "\u{1}javascript:alert(1)",
            "data:text/html,<script>alert(1)</script>",
            "vbscript:msgbox(1)",
            "file:///etc/passwd",
            "blob:https://example.org/0d5f3a0e",
        ] {
            assert!(
                matches!(Action::open_url(url, false), Action::Unsupported { .. }),
                "{url}"
            );
        }
    }
}
