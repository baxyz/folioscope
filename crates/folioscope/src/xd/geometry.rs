// SPDX-License-Identifier: AGPL-3.0-or-later

//! Where the nodes of an `.xd` graphics tree are: transforms, children and bounding boxes,
//! shared by the IR (hotspots) and the bake (drawing).

use helpers4::geometry::{Matrix, Rect, parse_path, transform_path};
use serde_json::Value;

/// The node's transform in its parent's space; the identity when it has none.
#[must_use]
pub fn matrix(node: &Value) -> Matrix {
    let t = &node["transform"];
    let n = |k: &str, default: f64| t[k].as_f64().unwrap_or(default);
    Matrix::new(
        n("a", 1.0),
        n("b", 0.0),
        n("c", 0.0),
        n("d", 1.0),
        n("tx", 0.0),
        n("ty", 0.0),
    )
}

/// The children of an artboard or a group.
#[must_use]
pub fn children(node: &Value) -> &[Value] {
    let list = match node["type"].as_str() {
        Some("artboard") => &node["artboard"]["children"],
        Some("group") => &node["group"]["children"],
        _ => return &[],
    };
    list.as_array().map_or(&[], Vec::as_slice)
}

/// The box of a node in its own coordinates (before its transform), `None` when it draws nothing
/// with an extent. Curves count with their control points, so a curved path may be a little
/// larger than what it paints.
#[must_use]
pub fn local_box(node: &Value) -> Option<Rect> {
    match node["type"].as_str()? {
        "group" | "artboard" => union(children(node).iter().filter(|c| c["visible"] != false)),
        "text" => Some(text_box(node)),
        "shape" => shape_box(&node["shape"]),
        _ => None,
    }
}

/// The box of a node in its parent's coordinates.
#[must_use]
pub fn bounds(node: &Value) -> Option<Rect> {
    local_box(node).map(|rect| matrix(node).bounding_box(rect))
}

fn union<'a>(nodes: impl Iterator<Item = &'a Value>) -> Option<Rect> {
    let rects: Vec<Rect> = nodes.filter_map(bounds).collect();
    Rect::bounding(&rects)
}

fn shape_box(shape: &Value) -> Option<Rect> {
    let n = |k: &str| shape[k].as_f64().unwrap_or(0.0);
    Some(match shape["type"].as_str()? {
        "rect" => Rect::new(n("x"), n("y"), n("width"), n("height")),
        "circle" => Rect::new(
            n("cx") - n("r"),
            n("cy") - n("r"),
            2.0 * n("r"),
            2.0 * n("r"),
        ),
        "ellipse" => Rect::new(
            n("cx") - n("rx"),
            n("cy") - n("ry"),
            2.0 * n("rx"),
            2.0 * n("ry"),
        ),
        "line" => {
            let (x1, y1, x2, y2) = (n("x1"), n("y1"), n("x2"), n("y2"));
            Rect::new(x1.min(x2), y1.min(y2), (x2 - x1).abs(), (y2 - y1).abs())
        }
        "polygon" => {
            let points: Vec<(f64, f64)> = shape["points"]
                .as_array()?
                .iter()
                .map(|p| {
                    (
                        p["x"].as_f64().unwrap_or(0.0),
                        p["y"].as_f64().unwrap_or(0.0),
                    )
                })
                .collect();
            around(points.into_iter())?
        }
        "path" | "compound" => path_box(shape["path"].as_str()?)?,
        _ => return None,
    })
}

fn text_box(node: &Value) -> Rect {
    let frame = &node["text"]["frame"];
    if frame["type"] == "area" {
        return Rect::new(
            0.0,
            0.0,
            frame["width"].as_f64().unwrap_or(0.0),
            frame["height"].as_f64().unwrap_or(0.0),
        );
    }
    // Point text has no size of its own: estimate it from the font size and the lines.
    let size = node["style"]["font"]["size"].as_f64().unwrap_or(12.0);
    let chars = node["text"]["rawText"].as_str().map_or(0, |t| {
        t.lines().map(|l| l.chars().count()).max().unwrap_or(0)
    });
    let lines = node["text"]["rawText"]
        .as_str()
        .map_or(1, |t| t.lines().count().max(1));
    #[allow(clippy::cast_precision_loss)]
    Rect::new(
        0.0,
        0.0,
        chars as f64 * size * 0.55,
        lines as f64 * size * 1.2,
    )
}

/// The box around the coordinates of path data. XD writes absolute `M`, `L`, `C` and `Z`; the
/// path is normalized first, so any other command is covered too.
fn path_box(d: &str) -> Option<Rect> {
    let absolute = transform_path(d, Matrix::IDENTITY, 6).ok()?;
    let mut points = Vec::new();
    for command in parse_path(&absolute).ok()? {
        let (pairs, _) = command.values.as_chunks::<2>();
        points.extend(pairs.iter().map(|&[x, y]| (x, y)));
    }
    around(points.into_iter())
}

fn around(points: impl Iterator<Item = (f64, f64)>) -> Option<Rect> {
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    let mut any = false;
    for (x, y) in points {
        any = true;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    any.then(|| Rect::new(min_x, min_y, max_x - min_x, max_y - min_y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_group_spans_its_transformed_children() {
        let group = json!({
            "type": "group",
            "transform": { "a": 1, "b": 0, "c": 0, "d": 1, "tx": 100, "ty": 50 },
            "group": { "children": [
                { "type": "shape", "shape": { "type": "rect", "x": 0, "y": 0, "width": 10, "height": 20 } },
                { "type": "shape",
                  "transform": { "a": 1, "b": 0, "c": 0, "d": 1, "tx": 30, "ty": 0 },
                  "shape": { "type": "circle", "cx": 5, "cy": 5, "r": 5 } }
            ] }
        });
        let rect = bounds(&group).unwrap();
        assert_eq!(
            (rect.x, rect.y, rect.width, rect.height),
            (100.0, 50.0, 40.0, 20.0)
        );
    }

    #[test]
    fn paths_and_polygons_are_boxed_by_their_points() {
        let path = json!({ "type": "shape", "shape": { "type": "path", "path": "M 2 3 L 12 3 L 12 9 Z" } });
        let rect = local_box(&path).unwrap();
        assert_eq!(
            (rect.x, rect.y, rect.width, rect.height),
            (2.0, 3.0, 10.0, 6.0)
        );
        let polygon = json!({ "type": "shape", "shape": { "type": "polygon",
            "points": [{ "x": 7.5, "y": 0 }, { "x": 15, "y": 7 }, { "x": 0, "y": 7 }] } });
        let rect = local_box(&polygon).unwrap();
        assert_eq!((rect.width, rect.height), (15.0, 7.0));
    }

    #[test]
    fn a_rotated_node_is_boxed_after_its_transform() {
        let square = json!({
            "type": "shape",
            "transform": { "a": 0, "b": 1, "c": -1, "d": 0, "tx": 10, "ty": 0 },
            "shape": { "type": "rect", "x": 0, "y": 0, "width": 4, "height": 2 }
        });
        let rect = bounds(&square).unwrap();
        assert_eq!(
            (rect.x, rect.y, rect.width, rect.height),
            (8.0, 0.0, 2.0, 4.0)
        );
    }
}
