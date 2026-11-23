// SPDX-License-Identifier: AGPL-3.0-or-later

//! WebAssembly bindings: the same decoding and baking as the CLI, for Node (the Vite plugin) and
//! the browser (the full player).

use folioscope::bake::folio::{BakeOptions, bake_file_into};
use folioscope::bake::fonts::{FontHints, FontLibrary};
use js_sys::{Array, Map, Object, Reflect, Uint8Array};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

/// What a page says about one of the fonts it gives: the name of its entry in `fonts`, and the
/// family names, weight and style to use instead of what the file says.
#[derive(Deserialize)]
struct DeclaredFont {
    name: String,
    #[serde(flatten)]
    hints: FontHints,
}

/// Bakes a `.fig`, `.sketch`, `.xd` or `.penpot` file; `avif` lets large, photo-like layers use AVIF.
/// `fonts` maps font file names to their bytes, for text the file stores no glyphs for; `hints`
/// is a JSON array of `{ name, family?, match?, weight?, italic? }` describing some of them.
/// Returns `{ files, missingFonts }`: a `Map` from paths inside the `.folio` folder (`index.json`,
/// `poster.webp`, `layers/…`) to their bytes, and the fonts some text needed and was not given
/// (`Family Style`), plus the given files that are not fonts. `densities` (1x and 2x when absent)
/// lets the browser bake only the one its screen needs. `svg` (true when absent) serves a layer as
/// its SVG when it holds no image or filter and is lighter (docs/spec/bake.md); `max_states` (16 when
/// absent) caps the states baked per interactive component (docs/spec/folio.md), and the result's
/// `cappedComponents` lists those it cut, as `Component (frame): 16 of 40 states`.
///
/// # Errors
///
/// Throws when the file cannot be decoded or a frame cannot be rendered, or `hints` is not valid.
#[wasm_bindgen]
#[allow(clippy::needless_pass_by_value)] // wasm-bindgen passes an optional string by value
pub fn bake(
    input: &[u8],
    avif: bool,
    fonts: &Map,
    densities: Option<Vec<u8>>,
    hints: Option<String>,
    svg: Option<bool>,
    max_states: Option<u32>,
) -> Result<Object, JsError> {
    let declared: Vec<DeclaredFont> = match hints.as_deref().filter(|h| !h.is_empty()) {
        Some(json) => {
            serde_json::from_str(json).map_err(|e| JsError::new(&format!("hints: {e}")))?
        }
        None => Vec::new(),
    };
    let mut library = FontLibrary::default();
    let mut unreadable = Vec::new();
    fonts.for_each(&mut |bytes, name| {
        let name = name.as_string().unwrap_or_default();
        let hints = declared
            .iter()
            .find(|d| d.name == name)
            .map(|d| d.hints.clone())
            .unwrap_or_default();
        let bytes = Uint8Array::new(&bytes).to_vec();
        // A file that is not a font must not stop the bake: its text is reported missing.
        if library.add_with(&name, &bytes, &hints).is_err() {
            unreadable.push(format!("{name} (not a font)"));
        }
    });
    let files = Map::new();
    let mut options = BakeOptions {
        avif,
        svg: svg.unwrap_or(true),
        fonts: library,
        ..BakeOptions::default()
    };
    if let Some(max) = max_states.filter(|&m| m > 0) {
        options.max_states = max as usize;
    }
    if let Some(densities) = densities.filter(|d| !d.is_empty()) {
        options.densities = densities;
    }
    let report = bake_file_into(input, &options, &mut |path, bytes| {
        files.set(&path.into(), &Uint8Array::from(bytes).into());
        Ok(())
    })?;
    let missing: Array = unreadable
        .iter()
        .chain(&report.missing_fonts)
        .map(|font| JsValue::from(font.as_str()))
        .collect();
    let capped: Array = report
        .capped_components
        .iter()
        .map(|c| {
            JsValue::from(format!(
                "{} ({}): {} of {} states",
                c.name, c.frame, c.kept, c.states
            ))
        })
        .collect();
    let result = Object::new();
    Reflect::set(&result, &"cappedComponents".into(), &capped)
        .map_err(|_| JsError::new("result"))?;
    Reflect::set(&result, &"files".into(), &files).map_err(|_| JsError::new("result"))?;
    Reflect::set(&result, &"missingFonts".into(), &missing).map_err(|_| JsError::new("result"))?;
    Ok(result)
}

/// The font families the text of a design file uses (a `.sketch` also gives its PostScript
/// names), so a page fetches only the fonts it needs.
///
/// # Errors
///
/// Throws on an unknown format or a file that cannot be decoded.
#[wasm_bindgen]
pub fn families(input: &[u8]) -> Result<Array, JsError> {
    Ok(folioscope::families::families(input)?
        .iter()
        .map(|family| JsValue::from(family.as_str()))
        .collect())
}

/// Summarizes the prototype of any supported design file, as `folioscope inspect` prints it.
///
/// # Errors
///
/// Throws on an unknown format or a file that cannot be decoded.
#[wasm_bindgen]
pub fn inspect(input: &[u8]) -> Result<String, JsError> {
    Ok(folioscope::summarize(input)?.to_string())
}
