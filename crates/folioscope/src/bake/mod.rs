// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bakes prototypes into `.folio` folders (docs/spec/folio.md): renders nodes to SVG, rasterizes them with
//! `resvg` (docs/architecture.md), and writes the layers and `index.json`.

pub mod draw;
pub mod folio;
pub mod fonts;
pub mod fx;
pub mod image;
pub mod penpot;
pub mod sketch;
pub mod svg;
pub mod text;
pub mod xd;

use resvg::{tiny_skia, usvg};

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("invalid SVG: {0}")]
    Svg(#[from] usvg::Error),
    #[error("empty or oversized image")]
    Size,
    #[error("cannot encode the image: {0}")]
    Encode(String),
}

/// How every SVG the bake draws is read: images come inlined (`data:`), never from a path. A
/// design file is not trusted with the machine that bakes it, and `usvg` would otherwise read an
/// image file named by an `href`.
fn options() -> usvg::Options<'static> {
    let mut options = usvg::Options::default();
    options.image_href_resolver.resolve_string = Box::new(|_, _| None);
    options
}

/// A string of a design file, as the value of an XML attribute: what would end the attribute or
/// start markup is escaped, and the control characters XML refuses become spaces.
#[must_use]
pub(crate) fn attr(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            c if c.is_control() && !matches!(c, '\t' | '\n' | '\r') => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// The elements of a layer that may be served as its SVG: shapes, groups, gradients, clips and
/// masks. No script, no foreign content, no image, no filter, no animation.
const SVG_ELEMENTS: [&str; 17] = [
    "svg",
    "defs",
    "g",
    "path",
    "rect",
    "circle",
    "ellipse",
    "line",
    "polyline",
    "polygon",
    "linearGradient",
    "radialGradient",
    "stop",
    "clipPath",
    "mask",
    "pattern",
    "use",
];

/// Whether a layer's SVG may be served as a file (docs/spec/bake.md): it holds only the elements
/// and attributes the bake draws with. An SVG is a document: opened on its own, outside the
/// player's `<img>`, it would run a script it carried, in the site's name. Nothing of a design
/// file may get there, whatever the file says; a layer that fails is served as images, which
/// carry nothing.
#[must_use]
pub fn svg_safe(markup: &str) -> bool {
    let Ok(document) = usvg::roxmltree::Document::parse(markup) else {
        return false;
    };
    document.descendants().all(|node| {
        if node.is_pi() {
            return false;
        }
        if !node.is_element() {
            return true;
        }
        let name = node.tag_name();
        name.namespace() == Some(SVG_NAMESPACE)
            && SVG_ELEMENTS.contains(&name.name())
            && node.attributes().all(|a| {
                let (name, value) = (a.name().to_ascii_lowercase(), a.value());
                // No event handler; links only to the document's own gradients, clips and masks;
                // a style that names no address.
                !name.starts_with("on")
                    && (name != "href" || value.starts_with('#'))
                    && value
                        .to_ascii_lowercase()
                        .split("url(")
                        .skip(1)
                        .all(|rest| rest.starts_with('#'))
                    && (name != "style" || !value.contains(['@', '\\', '/']))
            })
    })
}

const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

/// Rasterizes an SVG document to PNG, at `scale` times its size.
///
/// # Errors
///
/// Fails on invalid SVG, or when the scaled image is empty or too large.
pub fn rasterize(svg: &str, scale: f32) -> Result<Vec<u8>, RenderError> {
    render_pixmap(svg, scale)?
        .encode_png()
        .map_err(|e| RenderError::Encode(e.to_string()))
}

/// A [ThumbHash](https://evanw.github.io/thumbhash/) of the SVG: a blurred placeholder of about
/// 25 bytes, base64-encoded, that the
/// player shows while the real layers load.
///
/// # Errors
///
/// Fails on invalid SVG or an empty image.
pub fn placeholder(svg: &str) -> Result<String, RenderError> {
    let tree = usvg::Tree::from_str(svg, &options())?;
    let size = tree.size();
    // ThumbHash takes images of at most 100x100.
    let scale = (100.0 / size.width().max(size.height())).min(1.0);
    let pixmap = render_pixmap(svg, scale)?;
    let rgba = image::straight_rgba(&pixmap);
    let hash =
        thumbhash::rgba_to_thumb_hash(pixmap.width() as usize, pixmap.height() as usize, &rgba);
    Ok(helpers4::hex::base64_encode(&hash))
}

/// The size an SVG document declares, in CSS pixels; none when it does not parse.
#[must_use]
pub fn svg_size(svg: &str) -> Option<(f32, f32)> {
    let size = usvg::Tree::from_str(svg, &options()).ok()?.size();
    Some((size.width(), size.height()))
}

/// Renders an SVG document at `scale` times its size.
///
/// # Errors
///
/// Fails on invalid SVG, or when the scaled image is empty or too large.
pub fn render_pixmap(svg: &str, scale: f32) -> Result<tiny_skia::Pixmap, RenderError> {
    let tree = usvg::Tree::from_str(svg, &options())?;
    let size = tree
        .size()
        .to_int_size()
        .scale_by(scale)
        .ok_or(RenderError::Size)?;
    let mut pixmap =
        tiny_skia::Pixmap::new(size.width(), size.height()).ok_or(RenderError::Size)?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn svg(inner: &str) -> String {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 10 10">{inner}</svg>"#
        )
    }

    #[test]
    fn attr_keeps_a_string_inside_its_attribute() {
        assert_eq!(attr("#ff0000"), "#ff0000");
        assert_eq!(
            attr("\"/><script>&\u{1}\n"),
            "&quot;/&gt;&lt;script&gt;&amp; \n"
        );
    }

    #[test]
    fn svg_safe_takes_what_the_bake_draws() {
        assert!(svg_safe(&svg(
            r##"<defs><linearGradient id="g1"><stop offset="0" stop-color="#fff"/></linearGradient><clipPath id="c1"><path d="M0 0H10V10Z"/></clipPath></defs><g clip-path="url(#c1)" style="mix-blend-mode:multiply"><rect width="10" height="10" fill="url(#g1)"/><use href="#c1"/></g>"##
        )));
    }

    #[test]
    fn svg_safe_refuses_what_could_run_or_load_anything() {
        for inner in [
            "<script>alert(1)</script>",
            r#"<g onload="alert(1)"/>"#,
            r#"<g ONCLICK="alert(1)"/>"#,
            r#"<a href="javascript:alert(1)"><rect width="1" height="1"/></a>"#,
            r#"<use href="https://example.org/x.svg#a"/>"#,
            r#"<use xmlns:x="http://www.w3.org/1999/xlink" x:href="data:image/svg+xml,x"/>"#,
            r#"<rect fill="url(https://example.org/x.svg#a)"/>"#,
            r#"<rect style="fill:URL(//example.org/x)"/>"#,
            r#"<rect style="background:\75rl(x)"/>"#,
            "<foreignObject><p xmlns=\"http://www.w3.org/1999/xhtml\">x</p></foreignObject>",
            "<style>@import 'https://example.org/x.css';</style>",
            r#"<image href="data:image/png;base64,AAAA"/>"#,
            r#"<animate attributeName="x"/>"#,
            r#"<set attributeName="onload" to="alert(1)"/>"#,
            r#"<script xmlns="http://www.w3.org/1999/xhtml">alert(1)</script>"#,
            r#"<x:rect xmlns:x="http://example.org/other"/>"#,
        ] {
            assert!(!svg_safe(&svg(inner)), "{inner}");
        }
        // Not XML, a stylesheet in front, an entity: nothing a layer is made of.
        assert!(!svg_safe("<svg"));
        assert!(!svg_safe(&format!(
            r#"<?xml-stylesheet href="https://example.org/x.css"?>{}"#,
            svg("")
        )));
        assert!(!svg_safe(&format!(
            r#"<!DOCTYPE svg [<!ENTITY x "y">]>{}"#,
            svg("&x;")
        )));
    }

    #[test]
    fn an_image_is_never_read_from_a_path() {
        // A PNG next to the tests would be drawn by `usvg`'s own resolver.
        let dir = std::env::temp_dir().join(format!("folioscope-href-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("secret.png");
        let red = rasterize(&svg(r##"<rect width="10" height="10" fill="#f00"/>"##), 1.0).unwrap();
        std::fs::write(&png, &red).unwrap();
        let linked = rasterize(
            &svg(&format!(
                r#"<image href="{}" width="10" height="10"/>"#,
                png.display()
            )),
            1.0,
        )
        .unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        let blank = rasterize(&svg(""), 1.0).unwrap();
        assert_eq!(linked, blank);
        assert_ne!(linked, red);
    }
}
