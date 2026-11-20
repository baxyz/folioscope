# Architecture

Folioscope plays the interactive prototype stored in a `.fig`, `.sketch`, `.xd` or `.penpot` file,
in the browser, with one HTML element. It is a **player**, not a viewer: visitors click through the
screens the way the designer linked them. It is made for archiving and showcasing (portfolios, case
studies), with no third-party service.

Out of scope: editing, inspecting layers or code, collaboration, writing design files, and 100 %
rendering fidelity.

## Two paths

The visitor's display must be as instant as possible, so the heavy work happens **ahead of time**:

```text
                     AT BUILD TIME (CLI or Vite plugin, once, cached)
 .fig / .sketch / .xd / .penpot ──► importer ──► IR ──► layer split ──► SVG ──► resvg ──► app.folio/
                                                                                             index.json
                                                                                             layers/…
                     IN THE BROWSER
 app.folio/ ──► lite player (TypeScript, no WebAssembly, no fonts): <img> layers, DOM hotspots, CSS
 app.fig    ──► full player: the same bake in WebAssembly, in a worker, then the lite player
```

- **The lite player** plays a `.folio` folder ([spec](spec/folio.md)): images, hotspots and a state
  machine. Under 15 KB of JavaScript gzipped, no WebAssembly, no font.
- **The full player** plays a raw design file with no build step. It runs the very same bake in a
  Web Worker (WebAssembly), produces a `.folio` in memory, and hands it to the lite player
  ([spec](spec/player.md#full-player)). One code path renders every format, for the CLI, the Vite
  plugin and the browser: what bakes plays, and the other way round.

## Rust and TypeScript

| Layer                                                                | Language             | Where                                    |
| -------------------------------------------------------------------- | -------------------- | ---------------------------------------- |
| Unzipping, Kiwi decoding (`.fig`), JSON (Sketch, XD, Penpot), the IR | Rust                 | `crates/folioscope` (`figma`, `sketch`…) |
| Instances, overrides, variants, layer split, rendering, `.folio`     | Rust, feature `bake` | `crates/folioscope/src/bake`             |
| Command-line tool (`bake`, `inspect`, `render`, `fonts`)             | Rust                 | `crates/folioscope/src/main.rs`          |
| The bake for JavaScript (Vite plugin, full player)                   | Rust → WebAssembly   | `crates/folioscope-wasm`                 |
| Prototype runtime, players, custom elements, controls, Vite plugin   | TypeScript           | `packages/folioscope`                    |

## Choices, and why

- **Rendering: the IR is drawn as SVG and rasterized by `resvg`.** Pure Rust, native and
  WebAssembly alike, small, and the same SVG can be served as is ([SVG layers](spec/bake.md#layer-formats)).
  Grida's engine (`@grida/refig`) was tried first: faithful on `.fig`, but WebAssembly only
  (6.4 MB gzipped), and it reads `.fig` or Figma's REST JSON, not our IR. It stays a test oracle.
- **`.fig` files are decoded in Rust**, with the schema-driven Kiwi decoder of helpers4
  (`helpers4::kiwi`): new Figma fields need no code change. `openfig-core` (JavaScript) writes
  the `.fig` fixtures and serves as a decoding oracle. Checked on 29 real files saved by Figma
  versions 80 to 106.
- **Text: Figma stores every glyph's outline**, so a `.fig` bakes with no font. For the other
  formats the bake lays text out itself with `parley` and `skrifa`, from fonts embedded in the file
  or given by the user, never the system's ([fonts](spec/bake.md#fonts-and-text)).
- **Transitions are CSS on baked images** (Web Animations): light, smooth, any easing. Smart animate
  moves and fades parts paired by name ([transitions](spec/player.md#transitions)). Baking every
  intermediate image would weigh dozens of images per transition.
- **The Vite plugin bakes with WebAssembly**, not a native binary: one npm package for every
  platform, no install step, and the module the full player uses. Built with SIMD; about twice as
  slow as native (8.1 s against 3.9 s on an 11-frame file with 9 photos), paid once per change
  thanks to the cache.
- **One crate on crates.io**, `folioscope`, library and binary. Reading files (importers, IR) is
  always built; rendering and the `.folio` writer sit behind the `bake` feature, on by default.
  A release is one `cargo publish`. `folioscope-wasm` is not published: it ships in the npm package.

## Prior art

There is no standalone, open-source interactive prototype player. The building blocks exist:
`openfig-core` and Evan Wallace's `kiwi` (`.fig` decoding), Grida and OpenPencil (`.fig` rendering
and editing), `fig2sketch` (the mapping between `.fig` and Sketch), the documented Sketch and
`.penpot` v3 formats, `xd2svg` (the `.xd` structure). Folioscope's own part is the prototype layer:
interactions, states and transitions, played from pre-rendered images.

What we know of each format: [Figma](format-figma.md), [Sketch](format-sketch.md),
[Adobe XD](format-xd.md), [Penpot](format-penpot.md).
