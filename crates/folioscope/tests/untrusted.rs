// SPDX-License-Identifier: AGPL-3.0-or-later

//! A design file is not trusted: whatever it says, a bake ends, and nothing of the file becomes
//! markup or a script in what the bake writes (docs/spec/bake.md).

use std::collections::BTreeSet;
use std::io::{Cursor, Read, Write};

use folioscope::bake::draw::Draw;
use folioscope::bake::folio::{BakeOptions, bake_file_into};
use folioscope::bake::fonts::FontLibrary;
use folioscope::bake::penpot::PenpotDraw;
use folioscope::bake::sketch::SketchDraw;
use folioscope::bake::svg_safe;
use folioscope::bake::text::Typesetter;
use folioscope::bake::xd::XdDraw;
use folioscope::ir::Document;
use folioscope::{Format, detect, penpot, sketch, summarize, xd};
use serde_json::Value;

/// What a file would write to leave an attribute and start a script.
const PAYLOAD: &str = r#""/><script>alert(document.domain)</script><path d=""#;

fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/../../fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// The archive with every JSON entry changed by `patch` (given the entry's name); `folders` adds
/// an entry for each folder, as some ZIP tools write them.
fn rewrite(archive: &[u8], folders: bool, patch: &mut dyn FnMut(&str, &mut Value)) -> Vec<u8> {
    let mut zip = zip::ZipArchive::new(Cursor::new(archive)).unwrap();
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let mut made = BTreeSet::new();
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).unwrap();
        let name = entry.name().unwrap().into_owned();
        if !entry.is_file() {
            continue;
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        if let Ok(mut json) = serde_json::from_slice::<Value>(&bytes) {
            patch(&name, &mut json);
            bytes = serde_json::to_vec(&json).unwrap();
        }
        if folders
            && let Some((folder, _)) = name.rsplit_once('/')
            && made.insert(folder.to_owned())
        {
            out.add_directory(folder, options).unwrap();
        }
        out.start_file(name, options).unwrap();
        out.write_all(&bytes).unwrap();
    }
    out.finish().unwrap().into_inner()
}

/// Calls `visit` on every object of `value`, depth first.
fn objects(value: &mut Value, visit: &mut dyn FnMut(&mut serde_json::Map<String, Value>)) {
    match value {
        Value::Object(map) => {
            visit(map);
            map.values_mut().for_each(|v| objects(v, visit));
        }
        Value::Array(list) => list.iter_mut().for_each(|v| objects(v, visit)),
        _ => {}
    }
}

/// Everything the bake would draw of the file, without rasterizing it: each frame, its layers,
/// what scrolls in it and its parts. `None` when the file no longer decodes.
fn markups(archive: &[u8]) -> Option<Vec<String>> {
    fn drawn(document: &Document, draw: &mut dyn Draw) -> Vec<String> {
        let mut out = Vec::new();
        for frame in document.pages.iter().flat_map(|p| &p.frames) {
            let Some(whole) = draw.frame(frame) else {
                continue;
            };
            if let Ok(layers) = draw.layers(frame, &whole) {
                out.extend(layers.into_iter().map(|(markup, _, _)| markup));
            }
            if let Ok(layers) = draw.scroll_layers(frame) {
                out.extend(layers.into_iter().map(|layer| layer.markup));
            }
            out.extend(draw.parts(frame).into_iter().map(|part| part.markup));
            out.push(whole);
        }
        out
    }
    let typesetter = || Typesetter::new(FontLibrary::default());
    Some(match detect(archive)? {
        Format::Penpot => {
            let file = penpot::decode(archive).ok()?;
            drawn(
                &penpot::ir::to_ir(&file),
                &mut PenpotDraw::new(&file, typesetter()),
            )
        }
        Format::Sketch => {
            let file = sketch::decode(archive).ok()?;
            drawn(
                &sketch::ir::to_ir(&file),
                &mut SketchDraw::new(&file, typesetter()),
            )
        }
        Format::Xd => {
            let file = xd::decode(archive).ok()?;
            drawn(&xd::ir::to_ir(&file), &mut XdDraw::new(&file, typesetter()))
        }
        Format::Fig => return None,
    })
}

/// Every key some entry gives a string to.
fn string_keys(archive: &[u8]) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    rewrite(archive, false, &mut |_, json| {
        objects(json, &mut |map| {
            keys.extend(
                map.iter()
                    .filter(|(_, v)| v.is_string())
                    .map(|(k, _)| k.clone()),
            );
        });
    });
    keys
}

#[test]
fn no_string_of_a_file_becomes_markup() {
    for name in [
        "penpot/features.penpot",
        "sketch/features.sketch",
        "xd/features.xd",
    ] {
        let archive = fixture(name);
        let keys = string_keys(&archive);
        let mut drawn = 0;
        for key in &keys {
            // Every string under `key` tries to close its attribute and start a script.
            let tainted = rewrite(&archive, false, &mut |_, json| {
                objects(json, &mut |map| {
                    if let Some(Value::String(text)) = map.get_mut(key) {
                        text.push_str(PAYLOAD);
                    }
                });
            });
            // A key the file is read by (an id, a type) may make it unreadable: nothing drawn.
            let Some(markups) = markups(&tainted) else {
                continue;
            };
            for markup in &markups {
                assert!(
                    !markup.contains("<script"),
                    "{name}: a string under {key:?} became markup"
                );
            }
            drawn += usize::from(!markups.is_empty());
        }
        assert!(
            drawn * 2 > keys.len(),
            "{name}: only {drawn} of {} keys still drew something",
            keys.len()
        );
    }
}

#[test]
fn a_script_in_a_colour_is_never_served() {
    let archive = rewrite(
        &fixture("penpot/navigation.penpot"),
        false,
        &mut |_, json| {
            objects(json, &mut |map| {
                for key in ["fillColor", "strokeColor"] {
                    if let Some(Value::String(color)) = map.get_mut(key) {
                        color.push_str(PAYLOAD);
                    }
                }
            });
        },
    );
    let mut files = Vec::new();
    bake_file_into(&archive, &BakeOptions::default(), &mut |path, bytes| {
        files.push((path.to_owned(), bytes.to_vec()));
        Ok(())
    })
    .unwrap();
    assert!(files.iter().any(|(path, _)| path == "index.json"));
    for (path, bytes) in &files {
        let text = String::from_utf8_lossy(bytes);
        assert!(!text.contains("<script"), "{path} carries a script");
        // A layer in one file for every density is an SVG.
        if path.starts_with("layers/") && !path.contains('@') {
            assert!(svg_safe(&text), "{path} is served and not safe");
        }
    }
}

#[test]
fn a_shape_that_lists_itself_ends() {
    // Every board and group lists itself and its parent among its children.
    let archive = rewrite(&fixture("penpot/features.penpot"), false, &mut |_, json| {
        let (Some(id), Some(parent)) = (
            json["id"].as_str().map(str::to_owned),
            json["parentId"].as_str().map(str::to_owned),
        ) else {
            return;
        };
        if let Some(children) = json.get_mut("shapes").and_then(Value::as_array_mut)
            && !children.is_empty()
        {
            children.push(Value::String(id));
            children.push(Value::String(parent));
        }
    });
    let summary = summarize(&archive).unwrap();
    assert!(summary.pages.iter().any(|page| !page.frames.is_empty()));
    assert!(markups(&archive).is_some_and(|markups| !markups.is_empty()));
}

#[test]
fn a_symbol_that_holds_itself_ends() {
    // A symbol whose master holds two instances of itself, and an instance of it in every
    // artboard: drawn without a guard, it never ends.
    let instance = serde_json::json!({
        "_class": "symbolInstance",
        "do_objectID": "LOOP-INSTANCE",
        "symbolID": "LOOP",
        "name": "Loop",
        "isVisible": true,
        "frame": { "x": 0, "y": 0, "width": 40, "height": 40 },
        "flow": { "destinationArtboardID": "back" },
        "overrideValues": [],
    });
    let master = serde_json::json!({
        "_class": "symbolMaster",
        "do_objectID": "LOOP-MASTER",
        "symbolID": "LOOP",
        "name": "Loop",
        "isVisible": true,
        "frame": { "x": 0, "y": 0, "width": 40, "height": 40 },
        "layers": [instance, instance],
    });
    let mut artboards = 0;
    let archive = rewrite(
        &fixture("sketch/navigation.sketch"),
        false,
        &mut |name, json| {
            if !name.starts_with("pages/") {
                return;
            }
            let Some(layers) = json.get_mut("layers").and_then(Value::as_array_mut) else {
                return;
            };
            for artboard in layers.iter_mut().filter(|l| l["_class"] == "artboard") {
                if let Some(inner) = artboard.get_mut("layers").and_then(Value::as_array_mut) {
                    inner.push(instance.clone());
                    artboards += 1;
                }
            }
            layers.push(master.clone());
        },
    );
    assert!(artboards > 0, "the fixture has no artboard");
    assert!(summarize(&archive).is_ok());
    assert!(markups(&archive).is_some_and(|markups| !markups.is_empty()));
}

#[test]
fn an_archive_that_lists_its_folders_reads_the_same() {
    for name in [
        "penpot/navigation.penpot",
        "sketch/navigation.sketch",
        "xd/navigation.xd",
    ] {
        let archive = fixture(name);
        let with_folders = rewrite(&archive, true, &mut |_, _| {});
        assert_eq!(
            summarize(&with_folders).unwrap().to_string(),
            summarize(&archive).unwrap().to_string(),
            "{name}"
        );
    }
}
