# The `.fig` format

Working notes on the parts of the `.fig` format Folioscope reads. Figma does not document it; these
notes come from [openfig-core](https://github.com/OpenFig-org/openfig-core)'s documentation and
from decoding real files saved by Figma (versions 80 to 106) with `folioscope inspect`. Anything
marked _unverified_ has not been checked against such a file.

## Container

A ZIP archive (entries stored, not deflated) with:

| Entry           | Contents                                       |
| --------------- | ---------------------------------------------- |
| `canvas.fig`    | The document, see below                        |
| `meta.json`     | File name, background color, export metadata   |
| `thumbnail.png` | Optional preview                               |
| `images/<sha1>` | Image fills, named by the SHA-1 of their bytes |

## `canvas.fig`

```text
[8 B prelude][u32 LE version][u32 LE len][chunk 0][u32 LE len][chunk 1][…more chunks]
```

- **Prelude**: `fig-kiwi` (design), `fig-jam.` (FigJam), `fig-deck` (slides).
- **Version**: stamped at creation and never upgraded; `101` and `106` are common. Accept any.
- **Chunk 0**: the Kiwi binary schema, raw deflate (RFC 1951, no zlib header).
- **Chunk 1**: the Kiwi message, zstd (magic `28 B5 2F FD`) in current files, raw deflate in older
  ones. Detect by the magic bytes.
- **Chunk 2+**: opaque; ignored.

The schema travels with each file, so the decoder (`folioscope_core::kiwi`) is generic: it decodes
the schema, then the root definition `Message`.

## Nodes

`Message.nodeChanges` is a flat array of every node. Each one has a `guid`
(`{ sessionID, localID }`, written `s:l`), a `type`, a `name`, and a `parentIndex`
(`{ guid, position }`). Siblings are ordered by comparing `position` **as strings**.

- Nodes with `phase: REMOVED` are deleted; skip them.
- `DOCUMENT` (`0:0`) → `CANVAS` (pages) → top-level `FRAME`, `SYMBOL`, `COMPONENT_SET`, `INSTANCE`.
- A canvas with `internalOnly: true` holds internal data (e.g. components from libraries), not a
  page.
- Geometry: `size` `{ x, y }` and `transform` (2×3 affine: `m02` = x, `m12` = y, relative to the
  parent).

## Prototype

### Flows

A top-level frame that starts a flow carries `prototypeStartingPoint`:
`{ name, description, position }`, where `position` orders the flows like sibling positions.
Files from before named flows have no `prototypeStartingPoint`; their single starting frame is
`CANVAS.prototypeStartNodeID`.

### Interactions

Any node can carry `prototypeInteractions: PrototypeInteraction[]`:

| Field                         | Meaning                                                                                                                             |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `id`                          | GUID of the interaction itself                                                                                                      |
| `isDeleted`                   | Skip when true                                                                                                                      |
| `event.interactionType`       | `ON_CLICK`, `ON_HOVER`, `ON_PRESS`, `AFTER_TIMEOUT`, `MOUSE_ENTER`, `MOUSE_LEAVE`, `MOUSE_DOWN`, `MOUSE_UP`, `ON_KEY_DOWN`, `DRAG`… |
| `event.transitionTimeout`     | Seconds, for `AFTER_TIMEOUT`                                                                                                        |
| `event.interactionMaintained` | "While hovering / pressing" variants                                                                                                |
| `actions[]`                   | `PrototypeAction`, see below                                                                                                        |

`PrototypeAction`:

| Field                                           | Meaning                                                                                          |
| ----------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| `connectionType`                                | `INTERNAL_NODE`, `BACK`, `CLOSE`, `URL`, `SET_VARIABLE`, `CONDITIONAL`…                          |
| `transitionNodeID`                              | Destination, for `INTERNAL_NODE`                                                                 |
| `navigationType`                                | `NAVIGATE`, `OVERLAY`, `SWAP`, `SWAP_STATE` (change to), `SCROLL_TO`                             |
| `transitionType`                                | `INSTANT_TRANSITION`, `DISSOLVE`, `SLIDE_FROM_*`, `PUSH_FROM_*`, `MOVE_FROM_*`, `SMART_ANIMATE`… |
| `transitionDuration`                            | Seconds                                                                                          |
| `easingType`, `easingFunction`                  | Named easing, or cubic Bézier control points for `CUSTOM_CUBIC`                                  |
| `transitionPreserveScroll`, `extraScrollOffset` | Scroll handling                                                                                  |
| `overlayRelativePosition`                       | Manual overlay position                                                                          |
| `connectionURL`, `openUrlInNewTab`              | For `URL`                                                                                        |

Overlay settings (`overlayPositionType`, `overlayBackgroundInteraction`,
`overlayBackgroundAppearance`) live on the **destination** frame or component, not on the action.

An absent field takes its schema default, so a missing `transitionType` means
`INSTANT_TRANSITION`. `easingFunction` holds the four cubic Bézier control points of a
`CUSTOM_CUBIC` easing.

The GUID `0xFFFFFFFF:0xFFFFFFFF` means "no node": an `INTERNAL_NODE` action pointing at it has no
destination.

### Legacy links

Before `prototypeInteractions`, a node held a single link in its own fields (`transitionNodeID`,
`connectionType`, `interactionType`, `transitionType`…). Figma migrated such files: the node keeps
those fields **and** gets a `prototypeInteractions` copy, with an `id` of `0:0`. In the copy:

- `event.interactionType` is often `NONE`, or `event` is empty: the link was a click, the only
  trigger at the time;
- a node without a link gets a placeholder interaction whose only action has `connectionType: NONE`;
  skip it.
