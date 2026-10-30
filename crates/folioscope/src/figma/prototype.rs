// SPDX-License-Identifier: AGPL-3.0-or-later

//! Summarizes the prototype of a decoded `.fig`: see [`crate::summary`].

use crate::figma::FigFile;
use crate::figma::tree::{Node, Tree, node_id};
use crate::kiwi::Value;
use crate::summary::{Action, Flow, Frame, Interaction, Page, Summary, Transition};

fn collect_interactions<'a>(tree: &Tree<'a>, node: &Node<'a>, out: &mut Vec<Interaction>) {
    for interaction in node
        .value
        .get("prototypeInteractions")
        .map_or(&[][..], Value::as_array)
    {
        if bool_field(interaction, "isDeleted") {
            continue;
        }
        let actions: Vec<Action> = interaction
            .get("actions")
            .map_or(&[][..], Value::as_array)
            .iter()
            .filter_map(|a| to_action(a, tree))
            .collect();
        // Migrated legacy links keep an empty placeholder when the node had no connection.
        if actions.is_empty() {
            continue;
        }
        let event = interaction.get("event");
        // Before triggers existed, every link was a click: migrated ones carry `NONE`.
        let trigger = match event
            .and_then(|e| e.get("interactionType"))
            .and_then(Value::as_str)
        {
            None | Some("NONE") => "ON_CLICK",
            Some(trigger) => trigger,
        };
        out.push(Interaction {
            source: node.id.to_string(),
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
    for child in tree.children(node.id) {
        collect_interactions(tree, child, out);
    }
}

/// Summarizes the prototype of a decoded `.fig`.
#[must_use]
pub fn summarize(file: &FigFile) -> Summary {
    let tree = Tree::new(file);
    let pages = tree
        .nodes()
        .filter(|n| n.kind == "DOCUMENT")
        .flat_map(|doc| tree.children(doc.id))
        .filter(|n| n.kind == "CANVAS" && !bool_field(n.value, "internalOnly"))
        .map(|canvas| {
            let mut frames: Vec<Frame> = tree
                .children(canvas.id)
                .filter(|n| matches!(n.kind, "FRAME" | "SYMBOL" | "COMPONENT_SET" | "INSTANCE"))
                .map(|frame| {
                    let mut interactions = Vec::new();
                    collect_interactions(&tree, frame, &mut interactions);
                    to_frame(frame, interactions)
                })
                .collect();
            // Files from before named flows only have one starting frame, set on the page.
            if frames.iter().all(|f| f.flow.is_none())
                && let Some(start) = canvas.value.get("prototypeStartNodeID").and_then(node_id)
                && let Some(frame) = frames.iter_mut().find(|f| f.id == start.to_string())
            {
                frame.flow = Some(Flow {
                    name: String::new(),
                    description: String::new(),
                });
            }
            Page {
                id: canvas.id.to_string(),
                name: canvas.name.to_owned(),
                frames,
            }
        })
        .collect();

    Summary {
        format: format!("{} v{}", file.prelude, file.version),
        pages,
    }
}

/// `None` for an empty placeholder action (`connectionType: NONE`).
fn to_action(action: &Value, tree: &Tree<'_>) -> Option<Action> {
    let text = |k: &str| action.get(k).and_then(Value::as_str);
    match text("connectionType").unwrap_or("NONE") {
        "NONE" => None,
        "INTERNAL_NODE" => Some(to_node_action(action, tree)),
        "BACK" => Some(Action::Back),
        "CLOSE" => Some(Action::Close),
        "URL" => Some(Action::Url(text("connectionURL").unwrap_or("").to_owned())),
        other => Some(Action::Unsupported(other.to_owned())),
    }
}

/// An `INTERNAL_NODE` action: navigate, overlay, swap or scroll towards another node.
fn to_node_action(action: &Value, tree: &Tree<'_>) -> Action {
    let text = |k: &str| action.get(k).and_then(Value::as_str);
    let Some(target) = action.get("transitionNodeID").and_then(node_id) else {
        return Action::Unsupported("INTERNAL_NODE without target".to_owned());
    };
    Action::Node {
        navigation: text("navigationType").unwrap_or("NAVIGATE").to_owned(),
        target: target.to_string(),
        target_name: tree.get(target).map(|n| n.name.to_owned()),
        transition: text("transitionType").map(|kind| Transition {
            kind: kind.to_owned(),
            duration: action
                .get("transitionDuration")
                .and_then(Value::as_f32)
                .unwrap_or(0.0),
            easing: text("easingType").map(str::to_owned),
        }),
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
        id: node.id.to_string(),
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

fn bool_field(value: &Value, name: &str) -> bool {
    matches!(value.get(name), Some(Value::Bool(true)))
}
