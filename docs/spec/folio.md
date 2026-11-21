# The `.folio` format

`folioscope bake` (the CLI, the Vite plugin, the full player) turns a design file into a `.folio`
folder: what the lite player needs to play the prototype at once, with no WebAssembly, no font and
a few kilobytes of JavaScript. The browser fetches only what the current screen needs, and each
file is cached on its own across versions.

```text
app.folio/
├─ index.json          # the prototype graph (the IR), plus each frame's layers
├─ poster.webp         # the first frame, lossless WebP at 1x, for the poster attribute
└─ layers/
   ├─ <hash>@1x.webp   # one file per layer and density: lossless WebP,
   ├─ <hash>@2x.avif   #   or AVIF for photo-like layers,
   └─ <hash>.svg       #   or one SVG for every density, for flat layers
```

The TypeScript types of `index.json` are in `packages/folioscope/src/folio.ts`; the Rust ones in
`crates/folioscope/src/ir.rs` (the IR) and `crates/folioscope/src/bake/folio.rs` (layers).

## Versions

`index.json` carries `folio` (the layout, now `1`) and `irVersion` (the prototype graph, now `1`).
A player refuses a newer major version of either.

## `index.json`

```jsonc
{
  "folio": 1,
  "irVersion": 1,
  "densities": [1, 2],
  "source": "fig", // fig, sketch, xd or penpot
  "pages": [{ "id": "…", "name": "…", "flows": [/* Flow */], "frames": [/* Frame */] }],
}
```

JSON is camelCase; enums are objects tagged by `type`. Ids are strings, each format keeping its own
(`12:34` for Figma, UUIDs elsewhere). Coordinates are CSS pixels at 1x.

- **Flow**: `name`, `description`, `start` (the id of its first frame). A page without a flow is
  left out of the bake when another page has one (a component library is not part of the
  prototype), unless `--all-pages`.
- **Frame**: `id`, `name`, `bounds` (on the canvas), `hotspots`, `layers`, `placeholder`, and
  when they apply:
  - `overlay`: how it shows when opened as an overlay: `position` (`center`, a corner or side,
    or `manual` at the opening action's `offset` from the hotspot), `backdrop` (`#rrggbbaa`),
    `closeOnClickOutside`;
  - `scroll`: `direction` (`vertical`, `horizontal`, `both`), `content` (what scrolls, from the
    frame's corner), `fixed` (ids of the children that stay in place), `sticky` (ids of the
    children that scroll, then hold at the top edge);
  - `scrollers`: frames inside the screen whose content scrolls: `id`, `name`, `bounds` (in the
    frame), `direction`, `content`, `sticky`. One level; hotspots inside one carry its id in
    `scroller`, with bounds in its content;
  - `components`: its interactive components (below);
  - `parts`: for smart animate (below).
- **Hotspot**: the node carrying interactions, its `bounds` in frame coordinates (bounding box
  after transforms), its `interactions`, and `fixed` when it sits in a fixed child. A frame's own
  interactions ("after delay") use a hotspot covering the frame.
- **Interaction**: a `trigger` and an `action`.
  - Triggers: `click`, `hover`, `press`, `mouseEnter`, `mouseLeave`, `mouseDown`, `mouseUp`,
    `afterDelay` (`delayMs`), `drag`, `key`, `other` (`name`).
  - Actions: `navigate`, `changeTo` (another variant of the hotspot's component), `scrollTo`
    (`target` bounds, `offset`), `overlay` (`offset`, `toggle`), `swapOverlay`, `back`, `close`,
    `openUrl` (`url`, `newTab`), `unsupported` (`name`). All but the last three carry `to` and a
    `transition`.
- **Transition**: `kind` (`instant`, `dissolve`, `smartAnimate`, `slideIn`/`push`/`moveIn` with
  `from`, `slideOut`/`moveOut` with `to`, `other`), `durationMs`, `easing` (`linear`, `easeIn`,
  `easeOut`, `easeInOut`, `cubicBezier` with four `points`, `spring` with `mass`, `stiffness`,
  `damping`, `other`).

## Layers

A frame is a **stack of layers** plus **hotspots**: interactivity lives in the hotspots and the
runtime, never in the pixels. A layer is `{ src, format, bounds, scrolls? }`, drawn at `bounds`
(frame coordinates).

- **Any frame is one layer**, except:
- **a frame that scrolls is three**: its background (its own fills, at the frame's size), its
  content (every child but the fixed ones, at the content's full size, `scrolls: true`), and its
  fixed children on top. The player puts the content in a native scroller between the two.
- **A frame that scrolls inside the screen** (a scroller) is left out of the screen's layers but
  for its own fills, and its content is a layer of its own with `scroller` set; a sticky child is
  a layer of its own, `sticky`, in its scroller's content (or the frame's, with `scrolls`).
- **Overlays** are frames of their own, drawn over the screen.
- **Smart animate**: a frame a smart animate leaves or reaches (a navigation, an overlay, a swap)
  also has `parts`: its background (named `""`), then its layers by path (`Card/Title`), three
  levels deep, at most 80, each drawn without its own parts. A part has its `layer` (unrotated,
  at its position), its `matrix` in the frame when it is turned or flipped, and a `shape`
  (`fill`, `radius`) when it is a plain rounded rectangle. A component state changed with smart
  animate has `parts` too, in the state's coordinates.
- **Interactive components** are left out of their frame's layers, and have one layer per state.

### Files

- **Named by content**: `src` is the first 16 hex digits of the SHA-256 of the layer's SVG. An
  unchanged layer keeps its URL across bakes and stays in the HTTP cache; identical layers in two
  frames are stored once. The bake is deterministic.
- **Formats** (`format`), chosen by the bake per layer ([rules](bake.md#layer-formats)):
  - `webp`: lossless, at every density: `layers/<src>@<density>x.webp`;
  - `avif`: quality 80, for photo-like layers: `layers/<src>@<density>x.avif`;
  - `svg`: one file for every density: `layers/<src>.svg`.
- **Densities**: `densities` lists them (1x and 2x by default). The player picks one per page
  load, from `devicePixelRatio`.
- **Placeholder**: each frame carries a [ThumbHash](https://evanw.github.io/thumbhash/) (about 25
  bytes, base64), shown blurred until the frame's layers are decoded.

## Interactive components

An instance whose variant holds a "change to" is a **component** of its frame:

```jsonc
{
  "id": "1:2;3:4", // the instances from the frame down to this one, joined by ";"
  "name": "Toggle",
  "bounds": {/* in the frame */},
  "initial": "5:6", // the variant shown first
  "fixed": false, // in a fixed child of a scrolling frame
  "states": [
    {
      "variant": "5:6",
      "name": "Off",
      "bounds": {/* from the component's corner */},
      "hotspots": [/* in the state's coordinates */],
      "layer": {/* Layer */},
    },
  ],
}
```

- One state per variant reachable from the initial one through "change to", the initial one
  first, each drawn through the overrides of the instances it sits in. The player swaps the state
  in place; every state is fetched with the frame, so a change shows at once.
- **At most 16 states per component** by default (`--max-states`, `maxStates`). Past it, the bake
  leaves the others out and warns, naming the component and its frame; a "change to" a state left
  out does nothing. Real variant sets cross design properties (size, theme, type: up to 81
  variants in Penpot's examples), but a prototype changes one of them, the state: 2 to 8 values
  there, 11 for a carousel.
- **Nested interactive components**: a component inside a state is not drawn into it. The state
  lists it in its own `components` (ids from the state's variant down to the instance), the state's
  layer is drawn without it, and it gets its layers like any component, drawn over the state: N + M
  images rather than N × M. Up to three levels deep. Each nested component is capped on its own.
- **"After delay" inside a state** counts from when the state shows.

## Loading speed

The first frame waits for chained requests: the page, the player, `index.json`, then the image.
The `poster` attribute shows `poster.webp` as soon as the player runs, and a
`<link rel="preload">` can fetch `index.json` along with the player. `test:budget` checks it in CI
with Chromium throttled to WebPageTest's "4G" (170 ms, 9 Mb/s) and a CPU four times slower,
connection setup excluded: the first frame shows in about 750 ms, under the 1 s budget (about
900 ms with neither poster nor preload).
