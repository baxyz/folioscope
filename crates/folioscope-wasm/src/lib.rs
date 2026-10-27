// SPDX-License-Identifier: AGPL-3.0-or-later

//! WebAssembly bindings: the same decoding and baking as the CLI, for Node (the Vite plugin) and,
//! later, the browser (the full player).

use folioscope_bake::folio::{BakeOptions, bake_file_into};
use folioscope_bake::fonts::FontLibrary;
use js_sys::{Array, Map, Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;

/// Bakes a `.fig` or `.sketch` file; `avif` lets large, photo-like layers use AVIF; `fonts` maps font file
/// names to their bytes, for text the file stores no glyphs for. Returns `{ files, missingFonts }`:
/// a `Map` from paths inside the `.folio` folder (`index.json`, `poster.webp`, `layers/…`) to
/// their bytes, and the fonts some text needed and was not given (`Family Style`).
///
/// # Errors
///
/// Throws when the file cannot be decoded or a frame cannot be rendered.
#[wasm_bindgen]
pub fn bake(input: &[u8], avif: bool, fonts: &Map) -> Result<Object, JsError> {
    let mut library = FontLibrary::default();
    let mut font_error = None;
    fonts.for_each(&mut |bytes, name| {
        let name = name.as_string().unwrap_or_default();
        if let Err(e) = library.add(&name, &Uint8Array::new(&bytes).to_vec()) {
            font_error.get_or_insert(e);
        }
    });
    if let Some(e) = font_error {
        return Err(e.into());
    }
    let files = Map::new();
    let options = BakeOptions {
        avif,
        fonts: library,
        ..BakeOptions::default()
    };
    let report = bake_file_into(input, &options, &mut |path, bytes| {
        files.set(&path.into(), &Uint8Array::from(bytes).into());
        Ok(())
    })?;
    let missing: Array = report.missing_fonts.iter().map(JsValue::from).collect();
    let result = Object::new();
    Reflect::set(&result, &"files".into(), &files).map_err(|_| JsError::new("result"))?;
    Reflect::set(&result, &"missingFonts".into(), &missing).map_err(|_| JsError::new("result"))?;
    Ok(result)
}

/// Summarizes the prototype of any supported design file, as `folioscope inspect` prints it.
///
/// # Errors
///
/// Throws on an unknown format or a file that cannot be decoded.
#[wasm_bindgen]
pub fn inspect(input: &[u8]) -> Result<String, JsError> {
    Ok(folioscope_core::summarize(input)?.to_string())
}
