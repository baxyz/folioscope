// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reader for `.penpot` files, format v3: a ZIP with `manifest.json`, then per file
//! `files/<file>/pages/<page>.json` and one JSON per shape in `files/<file>/pages/<page>/`.
//! Penpot documents it; see `docs/format-penpot.md`. The binary v1 format is not supported.

pub mod ir;

use std::collections::HashMap;
use std::io::{Cursor, Read};

use serde_json::Value;

use crate::summary::{Action, Flow, Frame, Interaction, Page, Summary, Transition};

/// The root frame of every page, parent of its top-level shapes.
pub(crate) const ROOT: &str = "00000000-0000-0000-0000-000000000000";

#[derive(Debug, thiserror::Error)]
pub enum PenpotError {
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

/// A decoded `.penpot` export: its pages, each with its shapes by id. An export can hold several
/// files; their pages follow each other.
#[derive(Debug)]
pub struct PenpotFile {
    pub manifest: Value,
    pub pages: Vec<PenpotPage>,
    /// Bitmaps by the media id shapes refer to (`metadata.id`, `fillImage.id`).
    pub media: HashMap<String, Vec<u8>>,
}

#[derive(Debug)]
pub struct PenpotPage {
    pub page: Value,
    pub shapes: HashMap<String, Value>,
}

/// Decodes a `.penpot` v3 archive.
///
/// # Errors
///
/// Fails when an entry is missing or is not valid JSON.
pub fn decode(archive: &[u8]) -> Result<PenpotFile, PenpotError> {
    let mut zip = zip::ZipArchive::new(Cursor::new(archive))?;
    let names: Vec<String> = zip
        .file_names()
        .filter_map(|name| name.ok().map(std::borrow::Cow::into_owned))
        .collect();
    let manifest = read_json(&mut zip, "manifest.json")?;
    let mut pages = Vec::new();
    for file in manifest["files"].as_array().into_iter().flatten() {
        let prefix = format!("files/{}/pages/", file["id"].as_str().unwrap_or(""));
        let mut file_pages = Vec::new();
        for name in names.iter().filter(|n| n.starts_with(&prefix)) {
            let rest = &name[prefix.len()..];
            // `<page>.json` is a page; `<page>/<shape>.json` one of its shapes.
            if !rest.contains('/')
                && std::path::Path::new(rest)
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("json"))
            {
                let page = read_json(&mut zip, name)?;
                let dir = format!("{prefix}{}/", rest.trim_end_matches(".json"));
                let mut shapes = HashMap::new();
                for shape_name in names.iter().filter(|n| n.starts_with(&dir)) {
                    let shape = read_json(&mut zip, shape_name)?;
                    if let Some(id) = shape["id"].as_str() {
                        shapes.insert(id.to_owned(), shape);
                    }
                }
                file_pages.push(PenpotPage { page, shapes });
            }
        }
        file_pages.sort_by_key(|p| p.page["index"].as_u64().unwrap_or(0));
        pages.extend(file_pages);
    }
    // `files/<file>/media/<id>.json` points a media id at its bytes, `objects/<mediaId>.<ext>`.
    let mut media = HashMap::new();
    let is_json = |name: &str| {
        std::path::Path::new(name)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("json"))
    };
    for name in names.iter().filter(|n| n.contains("/media/") && is_json(n)) {
        let entry = read_json(&mut zip, name)?;
        let (Some(id), Some(object)) = (entry["id"].as_str(), entry["mediaId"].as_str()) else {
            continue;
        };
        let stored = names
            .iter()
            .find(|n| n.starts_with(&format!("objects/{object}.")) && !is_json(n));
        if let Some(stored) = stored {
            let mut bytes = Vec::new();
            zip.by_name(stored)?.read_to_end(&mut bytes)?;
            media.insert(id.to_owned(), bytes);
        }
    }
    Ok(PenpotFile {
        manifest,
        pages,
        media,
    })
}

fn read_json(zip: &mut zip::ZipArchive<Cursor<&[u8]>>, entry: &str) -> Result<Value, PenpotError> {
    let mut text = String::new();
    zip.by_name(entry)?.read_to_string(&mut text)?;
    serde_json::from_str(&text).map_err(|source| PenpotError::Json {
        entry: entry.to_owned(),
        source,
    })
}

/// Summarizes the prototype of a decoded `.penpot`: boards, named flows and interactions.
#[must_use]
pub fn summarize(file: &PenpotFile) -> Summary {
    let names: HashMap<&str, &str> = file
        .pages
        .iter()
        .flat_map(|p| p.shapes.values())
        .filter_map(|s| Some((s["id"].as_str()?, s["name"].as_str()?)))
        .collect();

    let pages = file
        .pages
        .iter()
        .map(|page| {
            let flows: HashMap<&str, &str> = page.page["flows"]
                .as_object()
                .into_iter()
                .flat_map(|flows| flows.values())
                .filter_map(|f| Some((f["startingFrame"].as_str()?, f["name"].as_str()?)))
                .collect();
            let top: Vec<&Value> = page.shapes.get(ROOT).map_or(Vec::new(), |root| {
                root["shapes"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    // Bottom to top in the file; listed top first, as Penpot's layer panel does.
                    .rev()
                    .filter_map(|id| page.shapes.get(id.as_str()?))
                    .filter(|s| s["type"] == "frame")
                    .collect()
            });
            let frames = top
                .into_iter()
                .map(|board| {
                    let id = board["id"].as_str().unwrap_or("");
                    let mut interactions = Vec::new();
                    collect_interactions(board, &page.shapes, &names, &mut interactions);
                    #[allow(clippy::cast_possible_truncation)]
                    let number = |k: &str| board[k].as_f64().unwrap_or(0.0) as f32;
                    Frame {
                        id: id.to_owned(),
                        name: board["name"].as_str().unwrap_or("").to_owned(),
                        kind: "BOARD".to_owned(),
                        x: number("x"),
                        y: number("y"),
                        width: number("width"),
                        height: number("height"),
                        flow: flows.get(id).map(|name| Flow {
                            name: (*name).to_owned(),
                            description: String::new(),
                        }),
                        interactions,
                    }
                })
                .collect();
            Page {
                id: page.page["id"].as_str().unwrap_or("").to_owned(),
                name: page.page["name"].as_str().unwrap_or("").to_owned(),
                frames,
            }
        })
        .collect();

    Summary {
        format: format!(
            "penpot export v{} ({})",
            file.manifest["version"],
            file.manifest["generatedBy"].as_str().unwrap_or("?")
        ),
        pages,
    }
}

/// Interactions of a shape and of its children, depth first in layer order.
fn collect_interactions(
    shape: &Value,
    shapes: &HashMap<String, Value>,
    names: &HashMap<&str, &str>,
    out: &mut Vec<Interaction>,
) {
    for interaction in shape["interactions"].as_array().into_iter().flatten() {
        out.push(to_interaction(shape, interaction, names));
    }
    for child in shape["shapes"]
        .as_array()
        .into_iter()
        .flatten()
        .rev()
        .filter_map(|id| shapes.get(id.as_str()?))
    {
        collect_interactions(child, shapes, names, out);
    }
}

fn to_interaction(shape: &Value, interaction: &Value, names: &HashMap<&str, &str>) -> Interaction {
    let trigger = match interaction["eventType"].as_str().unwrap_or("") {
        "click" => "ON_CLICK",
        "mouse-press" => "ON_PRESS",
        "mouse-enter" => "MOUSE_ENTER",
        "mouse-leave" => "MOUSE_LEAVE",
        "after-delay" => "AFTER_TIMEOUT",
        other => other,
    };
    #[allow(clippy::cast_possible_truncation)]
    let seconds = |v: &Value| v.as_f64().map(|ms| (ms / 1000.0) as f32);
    let node = |navigation: &str| {
        let target = interaction["destination"].as_str().unwrap_or("").to_owned();
        let animation = &interaction["animation"];
        Action::Node {
            navigation: navigation.to_owned(),
            target_name: names.get(target.as_str()).map(|n| (*n).to_owned()),
            target,
            transition: animation["animationType"].as_str().map(|kind| Transition {
                // `slide` and `push` also carry a direction and a way (in or out).
                kind: [
                    Some(kind),
                    animation["way"].as_str(),
                    animation["direction"].as_str(),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join("_")
                .replace('-', "_")
                .to_uppercase(),
                duration: seconds(&animation["duration"]).unwrap_or(0.0),
                easing: animation["easing"].as_str().map(str::to_owned),
            }),
        }
    };
    let action = match interaction["actionType"].as_str().unwrap_or("") {
        "navigate" => node("NAVIGATE"),
        "open-overlay" => node("OVERLAY"),
        "toggle-overlay" => node("TOGGLE_OVERLAY"),
        "close-overlay" => Action::Close,
        "prev-screen" => Action::Back,
        "open-url" => Action::Url(interaction["url"].as_str().unwrap_or("").to_owned()),
        other => Action::Unsupported(other.to_owned()),
    };
    Interaction {
        source: shape["id"].as_str().unwrap_or("").to_owned(),
        source_name: shape["name"].as_str().unwrap_or("").to_owned(),
        trigger: trigger.to_owned(),
        timeout: (trigger == "AFTER_TIMEOUT")
            .then(|| seconds(&interaction["delay"]))
            .flatten(),
        actions: vec![action],
    }
}
