// SPDX-License-Identifier: AGPL-3.0-or-later

//! Phase 0 spike: lists the pages, top-level frames, flows and prototype interactions of a `.fig`.
//! This is a read-only summary, not the IR.

use std::collections::HashMap;
use std::fmt;

use crate::fig::FigFile;
use crate::kiwi::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId {
    pub session: u32,
    pub local: u32,
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.session, self.local)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    pub prelude: String,
    pub version: u32,
    pub pages: Vec<Page>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub id: NodeId,
    pub name: String,
    pub frames: Vec<Frame>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub id: NodeId,
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
    pub source: NodeId,
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
        target: NodeId,
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

struct Node<'a> {
    id: NodeId,
    kind: &'a str,
    name: &'a str,
    value: &'a Value,
}

struct Tree<'a> {
    nodes: HashMap<NodeId, Node<'a>>,
    children: HashMap<NodeId, Vec<NodeId>>,
}

impl<'a> Tree<'a> {
    fn new(file: &'a FigFile) -> Self {
        let mut nodes = HashMap::new();
        let mut children: HashMap<NodeId, Vec<(&str, NodeId)>> = HashMap::new();
        for value in file
            .message
            .get("nodeChanges")
            .map_or(&[][..], Value::as_array)
        {
            if value.get("phase").and_then(Value::as_str) == Some("REMOVED") {
                continue;
            }
            let Some(id) = value.get("guid").and_then(node_id) else {
                continue;
            };
            if let Some(parent) = value.get("parentIndex") {
                let position = parent.get("position").and_then(Value::as_str).unwrap_or("");
                if let Some(parent_id) = parent.get("guid").and_then(node_id) {
                    children.entry(parent_id).or_default().push((position, id));
                }
            }
            let kind = value.get("type").and_then(Value::as_str).unwrap_or("NONE");
            let name = value.get("name").and_then(Value::as_str).unwrap_or("");
            nodes.insert(
                id,
                Node {
                    id,
                    kind,
                    name,
                    value,
                },
            );
        }
        let children = children
            .into_iter()
            .map(|(parent, mut list)| {
                // Figma orders siblings by comparing their position keys as strings.
                list.sort_by(|a, b| a.0.cmp(b.0));
                (parent, list.into_iter().map(|(_, id)| id).collect())
            })
            .collect();
        Self { nodes, children }
    }

    fn children(&self, id: NodeId) -> impl Iterator<Item = &Node<'a>> {
        self.children
            .get(&id)
            .into_iter()
            .flatten()
            .filter_map(|child| self.nodes.get(child))
    }

    fn collect_interactions(&self, node: &Node<'a>, out: &mut Vec<Interaction>) {
        for interaction in node
            .value
            .get("prototypeInteractions")
            .map_or(&[][..], Value::as_array)
        {
            if bool_field(interaction, "isDeleted") {
                continue;
            }
            let event = interaction.get("event");
            let trigger = event
                .and_then(|e| e.get("interactionType"))
                .and_then(Value::as_str)
                .unwrap_or("ON_CLICK");
            let actions = interaction
                .get("actions")
                .map_or(&[][..], Value::as_array)
                .iter()
                .map(|a| to_action(a, &self.nodes))
                .collect();
            out.push(Interaction {
                source: node.id,
                source_name: node.name.to_owned(),
                trigger: trigger.to_owned(),
                timeout: (trigger == "AFTER_TIMEOUT")
                    .then(|| {
                        event
                            .and_then(|e| e.get("transitionTimeout"))
                            .and_then(Value::as_f32)
                    })
                    .flatten(),
                actions,
            });
        }
        for child in self.children(node.id) {
            self.collect_interactions(child, out);
        }
    }
}

/// Summarizes the prototype of a decoded `.fig`.
#[must_use]
pub fn summarize(file: &FigFile) -> Summary {
    let tree = Tree::new(file);
    let pages = tree
        .nodes
        .values()
        .filter(|n| n.kind == "DOCUMENT")
        .flat_map(|doc| tree.children(doc.id))
        .filter(|n| n.kind == "CANVAS" && !bool_field(n.value, "internalOnly"))
        .map(|canvas| Page {
            id: canvas.id,
            name: canvas.name.to_owned(),
            frames: tree
                .children(canvas.id)
                .filter(|n| matches!(n.kind, "FRAME" | "SYMBOL" | "COMPONENT_SET" | "INSTANCE"))
                .map(|frame| {
                    let mut interactions = Vec::new();
                    tree.collect_interactions(frame, &mut interactions);
                    to_frame(frame, interactions)
                })
                .collect(),
        })
        .collect();

    Summary {
        prelude: file.prelude.clone(),
        version: file.version,
        pages,
    }
}

fn to_action(action: &Value, nodes: &HashMap<NodeId, Node<'_>>) -> Action {
    let connection = action
        .get("connectionType")
        .and_then(Value::as_str)
        .unwrap_or("NONE");
    match connection {
        "INTERNAL_NODE" => {
            let Some(target) = action.get("transitionNodeID").and_then(node_id) else {
                return Action::Unsupported("INTERNAL_NODE without target".to_owned());
            };
            Action::Node {
                navigation: action
                    .get("navigationType")
                    .and_then(Value::as_str)
                    .unwrap_or("NAVIGATE")
                    .to_owned(),
                target,
                target_name: nodes.get(&target).map(|n| n.name.to_owned()),
                transition: action
                    .get("transitionType")
                    .and_then(Value::as_str)
                    .map(|kind| Transition {
                        kind: kind.to_owned(),
                        duration: action
                            .get("transitionDuration")
                            .and_then(Value::as_f32)
                            .unwrap_or(0.0),
                        easing: action
                            .get("easingType")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                    }),
            }
        }
        "BACK" => Action::Back,
        "CLOSE" => Action::Close,
        "URL" => Action::Url(
            action
                .get("connectionURL")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned(),
        ),
        other => Action::Unsupported(other.to_owned()),
    }
}

fn to_frame(node: &Node<'_>, interactions: Vec<Interaction>) -> Frame {
    let vector = |field: &str, axis: &str| {
        node.value
            .get(field)
            .and_then(|v| v.get(axis))
            .and_then(Value::as_f32)
            .unwrap_or(0.0)
    };
    let flow = node.value.get("prototypeStartingPoint").map(|p| Flow {
        name: p
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned(),
        description: p
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned(),
    });
    Frame {
        id: node.id,
        name: node.name.to_owned(),
        kind: node.kind.to_owned(),
        x: vector("transform", "m02"),
        y: vector("transform", "m12"),
        width: vector("size", "x"),
        height: vector("size", "y"),
        flow,
        interactions,
    }
}

fn node_id(guid: &Value) -> Option<NodeId> {
    Some(NodeId {
        session: guid.get("sessionID")?.as_u32()?,
        local: guid.get("localID")?.as_u32()?,
    })
}

fn bool_field(value: &Value, name: &str) -> bool {
    matches!(value.get(name), Some(Value::Bool(true)))
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{} v{}", self.prelude, self.version)?;
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
                    write!(f, " with {} {}ms", t.kind, (t.duration * 1000.0).round())?;
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
