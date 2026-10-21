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
