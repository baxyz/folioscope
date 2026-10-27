# The `.penpot` format

Working notes on the parts of the `.penpot` format Folioscope reads: the **v3** export (a ZIP of
JSON), which Penpot documents
([penpot file format](https://help.penpot.app/technical-guide/developer/data-model/penpot-file-format/)).
The binary v1 export is obsolete and not supported. Checked on two public files from
[penpot/penpot-files](https://github.com/penpot/penpot-files) (exported by Penpot 2.6 and 2.18),
kept locally in the git-ignored `fixtures/external/penpot/`.

## Container

| Entry                                    | Contents                                                                                    |
| ---------------------------------------- | ------------------------------------------------------------------------------------------- |
| `manifest.json`                          | `{ type: "penpot/export-files", version: 1, generatedBy, files: [{ id, name, features }] }` |
| `files/<file>.json`                      | File metadata: features, options, `migrations` already applied                              |
| `files/<file>/pages/<page>.json`         | `{ id, name, background, index, flows }`                                                    |
| `files/<file>/pages/<page>/<shape>.json` | One shape per file                                                                          |
| `files/<file>/components/<id>.json`      | Components                                                                                  |
| `objects/<id>.json`, `objects/<id>.png`  | Media (images, fonts) and their metadata                                                    |

An export can hold several files; pages are ordered by `index`.

## Shapes

Every page has a root frame, id `00000000-0000-0000-0000-000000000000`, whose `shapes` lists the
top-level shapes in order. A **board** is a shape of `type: "frame"`; top-level boards are the
screens. Each shape has `id`, `name`, `type` (`frame`, `rect`, `circle`, `path`, `text`, `group`,
`bool`, `image`, `svg-raw`), `parentId`, `frameId` (nearest board), `shapes` (children ids), and
its geometry in **absolute page coordinates**, stored several times over: `x`, `y`, `width`,
`height`, `selrect`, the four `points`, `transform` and `transformInverse`. Fills are
`fills: [{ fillColor: "#RRGGBB", fillOpacity }]`.

## Prototype

- **Flows**: named, on the page: `flows: { <id>: { id, name, startingFrame } }`. Several per page.
- **Interactions** on any shape: `interactions: [{ eventType, actionType, destination, … }]`.
  - `eventType`: `click`, `mouse-press`, `mouse-enter`, `mouse-leave`, `after-delay` (with
    `delay` in milliseconds).
  - `actionType`: `navigate`, `open-overlay`, `toggle-overlay`, `close-overlay`, `prev-screen`,
    `open-url` (with `url`).
  - Overlays: `overlayPosType` (`center`, `top-left`… `bottom-right`, `manual` with
    `overlayPosition`), `closeClickOutside`, `backgroundOverlay`, `positionRelativeTo`.
  - `animation`: `{ animationType: "dissolve" | "slide" | "push", duration (ms), easing, way
    ("in" | "out"), direction ("right" | "left" | "up" | "down") }`.
  - `preserveScroll`.

The model maps onto the IR with no structural change: named flows, the usual triggers, navigate,
overlays, back, URL, and dissolve, slide and push transitions.

## Rendering

`folioscope-bake`'s `penpot` module draws a board from its shapes, in page coordinates, moved to
the board's top-left corner:

- `transform` (`{ a, b, c, d, e, f }`, rotation and flips) applies around the shape's centre
  (`x`, `y`, `width`, `height` are the shape before it); `opacity`; `hidden` shapes are left out.
- `frame` (a board): its fills and strokes on its outline, then its shapes, clipped to it unless
  `showContent` is true. `rect`: `r1` to `r4` are the corner radii from the top left, clockwise
  (`rx` in older files). `circle`: an ellipse in its box. `path` and `bool`: `content`, a list of
  `{ command: "move-to" | "line-to" | "curve-to" | "close-path", params }` in page coordinates
  (`c1x`, `c1y`, `c2x`, `c2y` for curves); a `bool` without `content` draws its shapes' outlines
  as one path. `group`: its shapes.
- **Masks**: a group with `maskedGroup: true` clips its other shapes to its first one (the bottom
  of the layer list), which is not drawn.
- `fills`: `fillColor` and `fillOpacity`; `fillColorGradient` (`type` linear or radial, `startX`,
  `startY`, `endX`, `endY` in 0–1 units of the shape's box, `stops` of `{ color, opacity,
  offset }`); `fillImage` (`{ id }`, a media). `strokes`: `strokeColor`, `strokeOpacity`,
  `strokeWidth`, `strokeStyle` (`solid`, `dashed`, `dotted`), `strokeAlignment`. `shadow`: the
  first visible `drop-shadow` (`offsetX`, `offsetY`, `blur`, `color: { color, opacity }`).
- **Images**: an `image` shape's `metadata.id`, or a `fillImage.id`, is a media:
  `files/<file>/media/<id>.json` gives its `mediaId`, whose bytes are `objects/<mediaId>.<ext>`.
- **Text**: `positionData` holds every run as Penpot laid it out: `text`, `x`, `y` (the **bottom**
  of the run's box), `height` (its line height), `fontFamily`, `fontWeight` and `fontSize` (CSS
  strings such as `"300"`, `"14px"`), `fontStyle`, `letterSpacing`, `textTransform`, `fills`. Each
  run is drawn on its own line with the fonts given (ADR 0011): no line breaking is needed.
