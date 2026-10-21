// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reader for `.xd` files: a ZIP whose `manifest` lists the artboards, each with a JSON graphics
//! tree (`artwork/artboard-<id>/graphics/graphicContent.agc`), and whose
//! `interactions/interactions.json` holds the prototype. Adobe does not document it; see
//! `docs/format-xd.md`.

use std::collections::HashMap;
use std::io::{Cursor, Read};

use serde_json::Value;

use crate::summary::{Action, Flow, Frame, Interaction, Page, Summary, Transition};

#[derive(Debug, thiserror::Error)]
pub enum XdError {
    #[error("not a ZIP archive: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("cannot read the archive: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid JSON in {entry}: {source}")]
    Json {
        entry: String,
        source: serde_json::Error,
    },
}

/// A decoded `.xd` file.
#[derive(Debug)]
pub struct XdFile {
    pub manifest: Value,
    /// Empty when the file has no prototype.
    pub interactions: Value,
    /// Graphics tree of each artboard, by artboard id.
    pub artboards: HashMap<String, Value>,
}

/// Decodes an `.xd` archive.
///
/// # Errors
///
/// Fails when the manifest or an artboard is missing or is not valid JSON.
pub fn decode(archive: &[u8]) -> Result<XdFile, XdError> {
    let mut zip = zip::ZipArchive::new(Cursor::new(archive))?;
    let mut json = |entry: &str| -> Result<Value, XdError> {
        let mut text = String::new();
        zip.by_name(entry)?.read_to_string(&mut text)?;
        serde_json::from_str(&text).map_err(|source| XdError::Json {
            entry: entry.to_owned(),
            source,
        })
    };
    let manifest = json("manifest")?;
    let interactions = json("interactions/interactions.json").unwrap_or(Value::Null);
    let mut artboards = HashMap::new();
    for entry in artboard_entries(&manifest) {
        let id = entry["path"]
            .as_str()
            .and_then(|p| p.strip_prefix("artboard-"))
            .unwrap_or("")
            .to_owned();
        let path = format!(
            "artwork/{}/graphics/graphicContent.agc",
            entry["path"].as_str().unwrap_or("")
        );
        artboards.insert(id, json(&path)?);
    }
    Ok(XdFile {
        manifest,
        interactions,
        artboards,
    })
}

/// Summarizes the prototype of a decoded `.xd`. XD has a single canvas: one page.
#[must_use]
pub fn summarize(file: &XdFile) -> Summary {
    let home = file.interactions["homeArtboard"].as_str().unwrap_or("");
    let names: HashMap<&str, &str> = artboard_entries(&file.manifest)
        .filter_map(|a| {
            Some((
                a["path"].as_str()?.strip_prefix("artboard-")?,
                a["name"].as_str()?,
            ))
        })
        .collect();
    let empty = serde_json::Map::new();
    let interactions = file.interactions["interactions"]
        .as_object()
        .unwrap_or(&empty);

    let frames = artboard_entries(&file.manifest)
        .map(|entry| {
            let id = entry["path"]
                .as_str()
                .and_then(|p| p.strip_prefix("artboard-"))
                .unwrap_or("");
            // Sources in layer order: the artboard itself, then every node of its tree.
            let mut nodes = vec![(
                id.to_owned(),
                names.get(id).copied().unwrap_or("").to_owned(),
            )];
            if let Some(tree) = file.artboards.get(id) {
                collect_nodes(tree, &mut nodes);
            }
            // The tree repeats the artboard as its root node.
            let mut seen = std::collections::HashSet::new();
            nodes.retain(|(node, _)| seen.insert(node.clone()));
            let frame_interactions = nodes
                .iter()
                .filter_map(|(node, name)| Some((node, name, interactions.get(node)?.as_array()?)))
                .flat_map(|(node, name, list)| {
                    list.iter().map(|i| to_interaction(node, name, i, &names))
                })
                .collect();
            // Bounds are in the canvas' coordinates; XD rounds them to whole pixels.
            #[allow(clippy::cast_possible_truncation)]
            let bound = |k: &str| entry["uxdesign#bounds"][k].as_f64().unwrap_or(0.0) as f32;
            Frame {
                id: id.to_owned(),
                name: entry["name"].as_str().unwrap_or("").to_owned(),
                kind: "ARTBOARD".to_owned(),
                x: bound("x"),
                y: bound("y"),
                width: bound("width"),
                height: bound("height"),
                flow: (id == home).then(|| Flow {
                    name: String::new(),
                    description: String::new(),
                }),
                interactions: frame_interactions,
            }
        })
        .collect();

    Summary {
        format: format!(
            "xd interactions v{} (manifest v{})",
            file.interactions["version"].as_str().unwrap_or("?"),
            file.manifest["manifest-format-version"]
        ),
        pages: vec![Page {
            id: file.manifest["id"].as_str().unwrap_or("").to_owned(),
            name: file.manifest["uxdesign#specName"]
                .as_str()
                .or_else(|| file.manifest["name"].as_str())
                .unwrap_or("")
                .to_owned(),
            frames,
        }],
    }
}

fn to_interaction(
    source: &str,
    name: &str,
    interaction: &Value,
    artboards: &HashMap<&str, &str>,
) -> Interaction {
    let properties = &interaction["properties"];
    let trigger = match interaction["triggerEvent"].as_str().unwrap_or("") {
        "tap" => "ON_CLICK",
        "time" => "AFTER_TIMEOUT",
        "hover" => "ON_HOVER",
        "drag" => "DRAG",
        "keys-gamepad" => "ON_KEY_DOWN",
        "voice" => "ON_VOICE",
        other => other,
    };
    #[allow(clippy::cast_possible_truncation)]
    let seconds = |v: &Value| v.as_f64().map(|s| s as f32);
    let navigation = |navigation: &str| {
        let target = properties["destination"].as_str().unwrap_or("").to_owned();
        Action::Node {
            navigation: navigation.to_owned(),
            target_name: artboards.get(target.as_str()).map(|n| (*n).to_owned()),
            target,
            transition: Some(Transition {
                kind: if interaction["action"] == "auto-animate" {
                    "SMART_ANIMATE".to_owned()
                } else {
                    properties["transition"]
                        .as_str()
                        .unwrap_or("none")
                        .replace('-', "_")
                        .to_uppercase()
                },
                duration: seconds(&properties["duration"]).unwrap_or(0.0),
                easing: properties["easing"].as_str().map(str::to_owned),
            }),
        }
    };
    let action = match interaction["action"].as_str().unwrap_or("") {
        "artboard-transition" | "auto-animate" => navigation("NAVIGATE"),
        "overlay" => navigation("OVERLAY"),
        "previous-artboard" | "goToPreviousArtboard" => Action::Back,
        other => Action::Unsupported(other.to_owned()),
    };
    Interaction {
        source: source.to_owned(),
        source_name: name.to_owned(),
        trigger: trigger.to_owned(),
        timeout: (trigger == "AFTER_TIMEOUT")
            .then(|| seconds(&properties["delay"]))
            .flatten(),
        actions: vec![action],
    }
}

/// Every node with an `id` in a graphics tree, in document order, with its name.
fn collect_nodes(value: &Value, out: &mut Vec<(String, String)>) {
    match value {
        Value::Object(object) => {
            if let (Some(id), Some(_)) =
                (object.get("id").and_then(Value::as_str), object.get("type"))
            {
                out.push((
                    id.to_owned(),
                    object
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_owned(),
                ));
            }
            for (key, child) in object {
                // Styles and metadata hold no nodes.
                if key != "style" && key != "meta" {
                    collect_nodes(child, out);
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|item| collect_nodes(item, out)),
        _ => {}
    }
}

/// The manifest's artboard entries: children of `artwork` whose path starts with `artboard-`.
fn artboard_entries(manifest: &Value) -> impl Iterator<Item = &Value> {
    manifest["children"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|c| c["path"] == "artwork")
        .flat_map(|artwork| artwork["children"].as_array().into_iter().flatten())
        .filter(|c| {
            c["path"]
                .as_str()
                .is_some_and(|p| p.starts_with("artboard-"))
        })
}
