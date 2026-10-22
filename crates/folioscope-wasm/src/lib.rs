// SPDX-License-Identifier: AGPL-3.0-or-later

//! WebAssembly bindings: the same decoding and baking as the CLI, for Node (the Vite plugin) and,
//! later, the browser (the full player).

use folioscope_bake::folio::{BakeOptions, bake_fig_into};
use js_sys::{Map, Uint8Array};
use wasm_bindgen::prelude::*;

/// Bakes a `.fig` file; `avif` lets large, photo-like layers use AVIF. Returns a `Map` from paths
/// inside the `.folio` folder (`index.json`, `poster.webp`, `layers/…`) to their bytes.
///
/// # Errors
///
/// Throws when the file cannot be decoded or a frame cannot be rendered.
#[wasm_bindgen]
pub fn bake(input: &[u8], avif: bool) -> Result<Map, JsError> {
    let file = folioscope_core::figma::decode(input)?;
    let files = Map::new();
    let options = BakeOptions {
        avif,
        ..BakeOptions::default()
    };
    bake_fig_into(&file, &options, &mut |path, bytes| {
        files.set(&path.into(), &Uint8Array::from(bytes).into());
        Ok(())
    })?;
    Ok(files)
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
