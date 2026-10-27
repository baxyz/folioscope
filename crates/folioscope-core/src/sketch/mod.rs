// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reader for `.sketch` files: a ZIP of JSON documents (`document.json`, `meta.json`, one
//! `pages/<id>.json` per page), described by Sketch's open file format schema. See
//! `docs/format-sketch.md`.

pub mod ir;

use std::collections::HashMap;
use std::io::{Cursor, Read};

use serde_json::Value;

use crate::summary::{Action, Flow, Frame, Interaction, Page, Summary, Transition};

#[derive(Debug, thiserror::Error)]
pub enum SketchError {
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

/// A decoded `.sketch` file: its JSON documents, pages in document order.
#[derive(Debug)]
pub struct SketchFile {
    pub meta: Value,
    pub document: Value,
    pub pages: Vec<Value>,
}

/// Decodes a `.sketch` archive.
///
/// # Errors
///
/// Fails when an entry is missing or is not valid JSON.
pub fn decode(archive: &[u8]) -> Result<SketchFile, SketchError> {
    let mut zip = zip::ZipArchive::new(Cursor::new(archive))?;
    let mut json = |entry: &str| -> Result<Value, SketchError> {
        let mut text = String::new();
        zip.by_name(entry)?.read_to_string(&mut text)?;
        serde_json::from_str(&text).map_err(|source| SketchError::Json {
            entry: entry.to_owned(),
            source,
        })
    };
    let meta = json("meta.json")?;
    let document = json("document.json")?;
    let pages = document["pages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|page| page["_ref"].as_str())
        .map(|reference| json(&format!("{reference}.json")))
        .collect::<Result<_, _>>()?;
    Ok(SketchFile {
        meta,
        document,
        pages,
    })
}

/// Summarizes the prototype of a decoded `.sketch`: artboards, start points and links.
#[must_use]
pub fn summarize(file: &SketchFile) -> Summary {
    let artboard_names: HashMap<&str, &str> = file
        .pages
        .iter()
        .flat_map(layers)
        .filter(|layer| layer["_class"] == "artboard")
        .filter_map(|a| Some((a["do_objectID"].as_str()?, a["name"].as_str()?)))
        .collect();

    let pages = file
        .pages
        .iter()
        .map(|page| Page {
            id: string(page, "do_objectID"),
            name: string(page, "name"),
            frames: layers(page)
                .filter(|layer| layer["_class"] == "artboard")
                .map(|artboard| {
                    let mut interactions = Vec::new();
                    collect_interactions(artboard, &artboard_names, &mut interactions);
                    // Sketch coordinates fit in f32, the summary's precision.
                    #[allow(clippy::cast_possible_truncation)]
                    let rect = |k: &str| artboard["frame"][k].as_f64().unwrap_or(0.0) as f32;
                    Frame {
                        id: string(artboard, "do_objectID"),
                        name: string(artboard, "name"),
                        kind: "ARTBOARD".to_owned(),
                        x: rect("x"),
                        y: rect("y"),
                        width: rect("width"),
                        height: rect("height"),
                        // Sketch has one unnamed start point per artboard flagged as a flow home.
                        flow: (artboard["isFlowHome"] == true).then(|| Flow {
                            name: String::new(),
                            description: String::new(),
                        }),
                        interactions,
                    }
                })
                .collect(),
        })
        .collect();

    Summary {
        format: format!(
            "sketch v{} ({} {})",
            file.meta["version"],
            file.meta["app"].as_str().unwrap_or("?"),
            file.meta["appVersion"].as_str().unwrap_or("?")
        ),
        pages,
    }
}

/// Every layer with a `flow` is a link, clicked or tapped: hotspots, and any other layer.
fn collect_interactions(
    layer: &Value,
    artboards: &HashMap<&str, &str>,
    out: &mut Vec<Interaction>,
) {
    if let Some(flow) = layer.get("flow").filter(|f| f.is_object()) {
        let destination = flow["destinationArtboardID"].as_str().unwrap_or("");
        let action = match destination {
            "back" => Action::Back,
            // Links inside a symbol that follow the instance's own target.
            "automatic" => Action::Unsupported("automatic".to_owned()),
            target => Action::Node {
                navigation: "NAVIGATE".to_owned(),
                target: target.to_owned(),
                target_name: artboards.get(target).map(|n| (*n).to_owned()),
                transition: Some(Transition {
                    kind: match flow["animationType"].as_i64().unwrap_or(0) {
                        1 => "SLIDE_FROM_LEFT",
                        2 => "SLIDE_FROM_RIGHT",
                        3 => "SLIDE_FROM_BOTTOM",
                        4 => "SLIDE_FROM_TOP",
                        _ => "INSTANT_TRANSITION",
                    }
                    .to_owned(),
                    // Sketch does not store a duration.
                    duration: 0.0,
                    easing: None,
                }),
            },
        };
        out.push(Interaction {
            source: string(layer, "do_objectID"),
            source_name: string(layer, "name"),
            trigger: "ON_CLICK".to_owned(),
            timeout: None,
            actions: vec![action],
        });
    }
    for child in layers(layer) {
        collect_interactions(child, artboards, out);
    }
}

fn layers(layer: &Value) -> impl Iterator<Item = &Value> {
    layer["layers"].as_array().into_iter().flatten()
}

fn string(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap_or("").to_owned()
}
