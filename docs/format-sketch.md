# The `.sketch` format

Working notes on the parts of the `.sketch` format Folioscope reads. Unlike `.fig`, the format is
open: Sketch publishes its JSON schema and TypeScript types
([sketch-hq/sketch-file-format](https://github.com/sketch-hq/sketch-file-format), MIT). These
notes come from that schema and from a real file saved by Sketch 69.1 (format version 134).

## Container

A ZIP archive (entries stored or deflated) with:

| Entry                  | Contents                                                        |
| ---------------------- | --------------------------------------------------------------- |
| `document.json`        | Shared styles and symbols, and `pages`: references to the pages |
| `meta.json`            | App and format versions, pages and artboards by id and name     |
| `user.json`            | Editor state (page list, zoom); ignored                         |
| `pages/<uuid>.json`    | One page and its layer tree                                     |
| `images/<hash>.<ext>`  | Bitmaps referenced by image fills and bitmap layers             |
| `previews/preview.png` | Preview of the first page                                       |
| `text-previews/…`      | Optional                                                        |

`document.json.pages` lists `{ _class: "MSJSONFileReference", _ref: "pages/<uuid>" }`; the entry
is `_ref` plus `.json`. Every object has a `_class`, and layers have a `do_objectID` (a UUID).

## Layers

A page's `layers` are artboards, symbol masters and loose layers. An artboard has a `frame`
(`{ x, y, width, height }`, in page coordinates) and its own `layers`; a child's `frame` is relative
to its parent. Rectangles, ovals and shape paths store their outline as `points` (curve points in
0–1 units of the frame), styles as `style.fills`, `style.borders`, `style.shadows`. Text stores
attributed strings, **not** glyph outlines: rendering text needs the fonts.

## Prototype

Much simpler than Figma's:

- **Start points**: an artboard with `isFlowHome: true`. Unnamed.
- **Links**: any layer may carry `flow`:
  `{ _class: "MSImmutableFlowConnection", destinationArtboardID, animationType, maintainScrollPosition? }`.
  - `destinationArtboardID` is an artboard's `do_objectID`, `"back"`, or `"automatic"` (a link
    inside a symbol that follows the instance's own target).
  - `animationType`: `1` slide from left, `2` from right, `3` from bottom, `4` from top; no
    animation is `0` in schema 6.5 but `-1` in real files (Convertify writes both). No duration or
    easing is stored.
- **Hotspots**: `_class: "MSImmutableHotspotLayer"` layers, invisible, that exist only to carry a
  `flow`.
- The only trigger is a click or tap. Schema 6.5 has no overlays, hover, delays or scroll links;
  `artboard.prototypeViewport` gives the device size.

Everything above maps onto the IR as a subset of Figma's model: a click trigger, `NAVIGATE` or
`BACK`, and the `SLIDE_FROM_*` transitions.

## What Lunacy plays

Lunacy (the free Sketch-compatible editor we validate with) played a file converted from our Figma
fixture by the Convertify plugin, and showed no navigation in our first generated files. The
generator now writes what that file has: each artboard's fill as a first, full-size rectangle,
corner radii on the points (`fixedRadius: 0`), uppercase ids, links on the visible layers.
