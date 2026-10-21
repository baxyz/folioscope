// SPDX-License-Identifier: AGPL-3.0-or-later

//! Phase 0 spike: a format-neutral, read-only summary of a prototype: pages, top-level frames,
//! flows and interactions. Every importer produces one; it is not the IR.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    /// Format and version, e.g. `fig-kiwi v101` or `sketch v134`.
    pub format: String,
    pub pages: Vec<Page>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub id: String,
    pub name: String,
    pub frames: Vec<Frame>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub flow: Option<Flow>,
    /// Interactions on the frame and on every node inside it, in layer order.
    pub interactions: Vec<Interaction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Flow {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Interaction {
    pub source: String,
    pub source_name: String,
    pub trigger: String,
    /// Seconds, for `AFTER_TIMEOUT`.
    pub timeout: Option<f32>,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// `NAVIGATE`, `OVERLAY`, `SWAP`, `SCROLL_TO`… towards another node.
    Node {
        navigation: String,
        target: String,
        target_name: Option<String>,
        transition: Option<Transition>,
    },
    Back,
    Close,
    Url(String),
    /// Anything else (variables, media, conditionals…), by connection type.
    Unsupported(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    pub kind: String,
    /// Seconds.
    pub duration: f32,
    pub easing: Option<String>,
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.format)?;
        for page in &self.pages {
            writeln!(f, "page {:?} ({})", page.name, page.id)?;
            for frame in &page.frames {
                write!(
                    f,
                    "  {} {:?} ({}) {}x{} at {},{}",
                    frame.kind.to_lowercase(),
                    frame.name,
                    frame.id,
                    frame.width,
                    frame.height,
                    frame.x,
                    frame.y
                )?;
                if let Some(flow) = &frame.flow {
                    write!(f, ", starts flow {:?}", flow.name)?;
                    if !flow.description.is_empty() {
                        write!(f, " ({})", flow.description)?;
                    }
                }
                writeln!(f)?;
                for interaction in &frame.interactions {
                    write!(
                        f,
                        "    {:?} ({}) {}",
                        interaction.source_name, interaction.source, interaction.trigger
                    )?;
                    if let Some(timeout) = interaction.timeout {
                        write!(f, " {}ms", (timeout * 1000.0).round())?;
                    }
                    let actions: Vec<String> = interaction
                        .actions
                        .iter()
                        .map(ToString::to_string)
                        .collect();
                    writeln!(f, " -> {}", actions.join(", "))?;
                }
            }
        }
        Ok(())
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Node {
                navigation,
                target,
                target_name,
                transition,
            } => {
                write!(
                    f,
                    "{} {:?} ({target})",
                    navigation.to_lowercase(),
                    target_name.as_deref().unwrap_or("?")
                )?;
                if let Some(t) = transition {
                    write!(f, " with {}", t.kind)?;
                    if t.duration > 0.0 {
                        write!(f, " {}ms", (t.duration * 1000.0).round())?;
                    }
                    if let Some(easing) = &t.easing {
                        write!(f, " {easing}")?;
                    }
                }
                Ok(())
            }
            Action::Back => f.write_str("back"),
            Action::Close => f.write_str("close overlay"),
            Action::Url(url) => write!(f, "open {url:?}"),
            Action::Unsupported(kind) => write!(f, "unsupported {kind}"),
        }
    }
}
