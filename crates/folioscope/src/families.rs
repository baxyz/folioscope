// SPDX-License-Identifier: AGPL-3.0-or-later

//! The font families a design file's text uses, so that a player fetches only the fonts it needs
//! before baking (docs/spec/player.md, fonts in the browser).

use std::collections::BTreeSet;

use serde_json::Value;

use crate::{Error, Format, detect};

/// The families the text of a `.fig`, `.sketch`, `.xd` or `.penpot` file uses, as the file names
/// them: a `.sketch` gives PostScript names (`Inter-Bold`), cut at the dash (`Inter`), and the
/// whole name too, for rules written against it.
///
/// # Errors
///
/// Fails on an unknown format or a file that cannot be decoded.
pub fn families(bytes: &[u8]) -> Result<BTreeSet<String>, Error> {
    let mut out = BTreeSet::new();
    match detect(bytes).ok_or(Error::UnknownFormat)? {
        Format::Fig => {
            let file = crate::figma::decode(bytes)?;
            for node in file
                .message
                .get("nodeChanges")
                .map_or(&[][..], crate::kiwi::Value::as_array)
            {
                if let Some(family) = node
                    .get("fontName")
                    .and_then(|f| f.get("family"))
                    .and_then(crate::kiwi::Value::as_str)
                {
                    out.insert(family.to_owned());
                }
            }
        }
        Format::Sketch => {
            let file = crate::sketch::decode(bytes)?;
            for value in file.pages.iter().chain(std::iter::once(&file.document)) {
                walk(value, &mut |key, value| {
                    if key == "MSAttributedStringFontAttribute"
                        && let Some(name) = value["attributes"]["name"].as_str()
                    {
                        out.insert(name.to_owned());
                        if let Some((family, _)) = name.split_once('-') {
                            out.insert(family.to_owned());
                        }
                    }
                });
            }
        }
        Format::Xd => {
            let file = crate::xd::decode(bytes)?;
            for tree in file.artboards.values() {
                walk(tree, &mut |key, value| {
                    if key == "fontFamily"
                        && let Some(family) = value.as_str()
                    {
                        out.insert(family.to_owned());
                    }
                    if key == "font"
                        && let Some(family) = value["family"].as_str()
                    {
                        out.insert(family.to_owned());
                    }
                });
            }
        }
        Format::Penpot => {
            let file = crate::penpot::decode(bytes)?;
            for page in &file.pages {
                for shape in page.shapes.values() {
                    walk(shape, &mut |key, value| {
                        if key == "fontFamily"
                            && let Some(family) = value.as_str()
                        {
                            let first = family.split(',').next().unwrap_or("").trim();
                            out.insert(first.trim_matches(['"', '\'']).to_owned());
                        }
                    });
                }
            }
        }
    }
    out.remove("");
    Ok(out)
}

/// Calls `visit` with every key and value of a JSON tree.
fn walk(value: &Value, visit: &mut impl FnMut(&str, &Value)) {
    match value {
        Value::Object(fields) => {
            for (key, field) in fields {
                visit(key, field);
                walk(field, visit);
            }
        }
        Value::Array(items) => {
            for item in items {
                walk(item, visit);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(path: &str) -> Vec<u8> {
        std::fs::read(format!(
            "{}/../../fixtures/{path}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    #[test]
    fn lists_the_families_text_uses() {
        let sketch = families(&fixture("sketch/text.sketch")).unwrap();
        assert!(
            sketch.contains("Inter") && sketch.contains("Brand"),
            "{sketch:?}"
        );
        let xd = families(&fixture("xd/text.xd")).unwrap();
        assert!(xd.contains("Inter"), "{xd:?}");
        let penpot = families(&fixture("penpot/features.penpot")).unwrap();
        assert!(penpot.contains("Inter"), "{penpot:?}");
        assert!(
            families(&fixture("figma/navigation.fig"))
                .unwrap()
                .is_empty()
        );
    }
}
