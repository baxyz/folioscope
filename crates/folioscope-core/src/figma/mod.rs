// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reader for `.fig` files: a ZIP archive whose `canvas.fig` entry holds a Kiwi schema and a
//! Kiwi message with every node of the document. See `docs/format-figma.md`.

pub mod instance;
pub mod ir;
pub mod prototype;
pub mod tree;

use std::collections::HashMap;
use std::io::{Cursor, Read};

use helpers4::bytes::{Endian, read_u32};

use crate::kiwi::{KiwiError, Schema, Value};

const ZSTD_MAGIC: [u8; 4] = [0x28, 0xb5, 0x2f, 0xfd];
const PRELUDE_LEN: usize = 8;

#[derive(Debug, thiserror::Error)]
pub enum FigError {
    #[error("not a ZIP archive: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("cannot read the archive: {0}")]
    Io(#[from] std::io::Error),
    #[error("canvas.fig is truncated")]
    Truncated,
    #[error("unknown prelude {0:?}")]
    Prelude(String),
    #[error("canvas.fig has {0} chunk(s), expected at least 2")]
    MissingChunks(usize),
    #[error("cannot decompress chunk {chunk}: {reason}")]
    Decompress { chunk: usize, reason: String },
    #[error(transparent)]
    Kiwi(#[from] KiwiError),
}

/// A decoded `.fig` file, before any interpretation of its nodes.
#[derive(Debug)]
pub struct FigFile {
    /// `fig-kiwi` for design files; `fig-jam.` and `fig-deck` for `FigJam` and slides.
    pub prelude: String,
    pub version: u32,
    pub schema: Schema,
    /// The root `Message`, whose `nodeChanges` array lists every node.
    pub message: Value,
    /// `images/<sha1>` entries of the archive, by hex SHA-1.
    pub images: HashMap<String, Vec<u8>>,
}

/// Decodes a `.fig` archive.
///
/// # Errors
///
/// Fails when the archive has no `canvas.fig`, when a chunk cannot be decompressed, or when the
/// message does not match the schema embedded in the file.
pub fn decode(archive: &[u8]) -> Result<FigFile, FigError> {
    let mut zip = zip::ZipArchive::new(Cursor::new(archive))?;
    let mut canvas = Vec::new();
    zip.by_name("canvas.fig")?.read_to_end(&mut canvas)?;
    let mut file = decode_canvas(&canvas)?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let name = entry.name()?.into_owned();
        if let Some(hash) = name.strip_prefix("images/")
            && !hash.is_empty()
        {
            let hash = hash.to_owned();
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            file.images.insert(hash, bytes);
        }
    }
    Ok(file)
}

/// Decodes the `canvas.fig` entry of a `.fig` archive.
///
/// # Errors
///
/// See [`decode`].
pub fn decode_canvas(canvas: &[u8]) -> Result<FigFile, FigError> {
    let prelude = canvas.get(..PRELUDE_LEN).ok_or(FigError::Truncated)?;
    let prelude = String::from_utf8_lossy(prelude).into_owned();
    if !prelude.starts_with("fig-") {
        return Err(FigError::Prelude(prelude));
    }
    let version = read_u32(canvas, PRELUDE_LEN, Endian::Little).ok_or(FigError::Truncated)?;

    let mut chunks = Vec::new();
    let mut offset = PRELUDE_LEN + 4;
    while offset < canvas.len() {
        let len = read_u32(canvas, offset, Endian::Little).ok_or(FigError::Truncated)? as usize;
        let start = offset + 4;
        let chunk = canvas.get(start..start + len).ok_or(FigError::Truncated)?;
        chunks.push(chunk);
        offset = start + len;
    }
    if chunks.len() < 2 {
        return Err(FigError::MissingChunks(chunks.len()));
    }

    let schema = Schema::decode(&decompress(0, chunks[0])?)?;
    let message = schema.decode_root("Message", &decompress(1, chunks[1])?)?;
    Ok(FigFile {
        prelude,
        version,
        schema,
        message,
        images: HashMap::new(),
    })
}

/// Chunks are zstd-compressed in current files and raw-deflated in older ones.
fn decompress(chunk: usize, data: &[u8]) -> Result<Vec<u8>, FigError> {
    let error = |reason: String| FigError::Decompress { chunk, reason };
    if data.starts_with(&ZSTD_MAGIC) {
        let mut out = Vec::new();
        ruzstd::decoding::StreamingDecoder::new(data)
            .map_err(|e| error(e.to_string()))?
            .read_to_end(&mut out)
            .map_err(|e| error(e.to_string()))?;
        Ok(out)
    } else {
        let mut out = Vec::new();
        flate2::read::DeflateDecoder::new(data)
            .read_to_end(&mut out)
            .map_err(|e| error(e.to_string()))?;
        Ok(out)
    }
}
