# Folioscope — design document

> Web player for interactive `.fig`, `.sketch`, `.xd` and `.penpot` prototypes, with no Figma,
> Sketch or Adobe account or licence.
> A web component that loads a design file from the current site and plays it: clicks, hovers,
> navigation between screens.

- **Repository**: `github.com/baxyz/folioscope`
- **Website**: `folioscope.baxyz.dev` (GitHub Pages)
- **npm package**: `folioscope`, which defines the `<folioscope-player>` element
- **Rust crate**: `folioscope-core`, on crates.io
- **License**: `AGPL-3.0-or-later`
- **Status**: design phase, nothing implemented yet

---

## 1. The name

### Why "Folioscope"

A **folioscope** is the French word for a _flip book_: a small booklet of still pictures that come
to life when you thumb through it. It is one of the ancestors of cinema.

That is exactly what the project does. A Figma or Sketch mockup is a sequence of still screens.
Folioscope "flips" through them following the prototype links the designer defined, and the mockup
comes to life.

The name also carries the word **"folio"**, as in **portfolio**: the use case the project was born
for — showing archived mockups on one's own site.

The README explains the name on its first screen, because the word is little known, especially to
English speakers.

Short tagline: **"Flip through your prototypes."**

### Why `<folioscope-player>`

- **The name stays whole.** `<folio-scope>` would cut the word artificially.
- **"player" rather than "viewer" or "reader".** A viewer is looked at, a player is played: you
  start, you click, it reacts, you go back to the start.
- **`folioscope-` is a prefix** for possible future elements, such as `<folioscope-thumbnail>`.

### Trademarks

- Never use "Figma", "Sketch", "Adobe", "XD" or "Penpot" in the name of a package, element or logo.
- Wording to use: "compatible with `.fig`, `.sketch`, `.xd` and `.penpot` files".
- The README carries a non-affiliation notice.

---

## 2. Goal and scope

### Goal

Display **the interactive prototype** stored in a `.fig`, `.sketch`, `.xd` or `.penpot` file,
directly in a browser, with a single HTML element:

```html
<script type="module" src="https://cdn.jsdelivr.net/npm/folioscope"></script>

<folioscope-player src="/mockups/app-mobile.fig"></folioscope-player>
```

It is designed for **archiving and showcasing** (portfolios, case studies), with no third-party
service.

### In scope

- Load a `.fig`, `.sketch`, `.xd` or `.penpot` from a URL of the site. The format is detected
  automatically.
- Show the prototype starting point of the first page.
- Play the interactions described in the file: click, hover, press, delay, navigation, overlays,
  back, scroll.
- Offer a few controls:
  - restart;
  - pick the starting point when there are several;
  - pick the page;
  - fullscreen;
  - show the clickable areas (_hotspots_), as InVision does.
- Work offline: no network call other than the file itself and the provided fonts.

### Out of scope

- Editing, in any form.
- Inspecting layers or code, in the manner of Figma's Dev Mode.
- 100 % rendering fidelity.
- Figma's prototype variables, expressions and conditions, at least until v1.
- Collaboration, comments and writing files.

---

## 3. State of the art (October 2026)

**There is no standalone, open-source interactive prototype player.** The building blocks do exist:

| Building block                       | Role                                                                                                                                                                                                                                                               | License                           | Use in Folioscope                                                                                          |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `openfig-core`                       | Isomorphic `.fig` parser (ZIP, Kiwi, zstd)                                                                                                                                                                                                                         | MIT                               | Reference, and failing that a decoding dependency                                                          |
| `kiwi` (Evan Wallace)                | Binary format of `.fig`                                                                                                                                                                                                                                            | MIT                               | Schema embedded in every file                                                                              |
| `@grida/refig`, `@grida/canvas-wasm` | `.fig` rendering in Rust, Skia and WASM, in Node and in the browser                                                                                                                                                                                                | Apache-2.0                        | Rendering engine candidate (decision D1)                                                                   |
| OpenPencil                           | `.fig`-compatible editor                                                                                                                                                                                                                                           | MIT                               | Code reference for rendering and the format                                                                |
| `fig2sketch` (Sketch)                | `.fig` to `.sketch` conversion                                                                                                                                                                                                                                     | MIT                               | Reference for the mapping between the two models. **Not** used as a pipeline, since it loses interactivity |
| Sketch file format                   | ZIP of documented JSON, with an official schema                                                                                                                                                                                                                    | MIT                               | Basis of the `.sketch` importer                                                                            |
| `sketch-web-viewer` (Anima)          | Static `.sketch` viewer                                                                                                                                                                                                                                            | MIT                               | Counter-example: flattened, single page, not interactive                                                   |
| `.penpot` v3 format (official docs)  | ZIP of documented JSON: `manifest.json`, `files/` (pages, one shape per JSON file, components, typographies), `objects/` (images, fonts). `flows` on pages, `interactions` on shapes                                                                               | Open format, documented by Penpot | Basis of the `.penpot` importer. The v1 (binary) format is obsolete and will not be supported              |
| `xd2svg`                             | `.xd` to SVG conversion. Documents the structure: ZIP with `manifest`, `artwork/artboard-*/graphics/graphicContent.agc` (JSON; the `.agc` or `.ags` extension varies between sources, to be checked on real files), `resources/`, `interactions/interactions.json` | MIT                               | Reference for the `.xd` importer. Format not documented by Adobe                                           |

**Adobe XD** has been in maintenance mode since 2023: many designers have `.xd` archives they can no
longer open easily. It is the format where archiving is most valuable.

**Penpot** is free software and its format is open and documented. It is the simplest to support,
and a showcase consistent with the project's license.

Folioscope's added value is **the prototype layer**: interactions, states, transitions. None of the
building blocks above provides it.

---

## 4. Architecture

**Priority: the most instant display possible for the visitor.** The heavy lifting therefore happens
**ahead of time**, at pre-render (`folioscope bake`), and the browser receives a ready-to-play
`.folio`. There are two paths:

```
                      PRE-RENDER (CLI or Vite plugin, once, no time limit)
 .fig / .sketch / .xd / .penpot ──► folioscope-core ──► IR ──► layer split ──► render ──► app.folio/
                                     (native Rust)          + component states   (Skia)    index.json + layers

                      BROWSER
 app.folio/ ──► lite player (TS, no WASM, no fonts) ──► layers as <img>, interactive zones, CSS
 app.fig    ──► full player (TS + WASM, loaded on demand) ──► same runtime, rendered on the fly (slower)
```

The second path, reading a raw file directly, stays available: it is the "works right away" option
with no build step. It is simply slower, and also serves as a fallback for layers that cannot be
pre-rendered.

Full player in detail:

```
            ┌───────────────────────────────────────────────┐
 .fig  ───► │  FIG importer      ─┐                         │
            │  (Kiwi + zstd)      │                         │
 .sketch ─► │  SKETCH importer   ─┤                         │
            │  (ZIP + JSON)       ├─► Intermediate model    │  folioscope-core
 .xd   ───► │  XD importer       ─┤   ("prototype" IR)      │  (Rust → WASM)
            │  (ZIP + AGC JSON)   │                         │
 .penpot ─► │  PENPOT importer   ─┘                         │
            │  (ZIP + JSON v3)                              │
            └───────────────────────────┬───────────────────┘
                                        │ IR (frames, render tree, interactions)
            ┌───────────────────────────▼───────────────────┐
            │  Prototype runtime (state machine)            │
            │  - current frame, history stack               │  folioscope
            │  - overlays, variant states                   │  (TypeScript)
            │  - hit-testing, triggers, transitions         │
            ├───────────────────────────────────────────────┤
            │  Rendering engine (Skia WASM, see D1)         │
            ├───────────────────────────────────────────────┤
            │  <folioscope-player>: Custom Element,         │
            │  Shadow DOM, attributes, events, controls     │
            └───────────────────────────────────────────────┘
```

### Rust / TypeScript split

| Layer                                                                                                 | Language                                                                       | Reason                                                          |
| ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ | --------------------------------------------------------------- |
| Decompression (ZIP, zstd, deflate), Kiwi decoding, JSON parsing (Sketch, XD, Penpot), IR construction | **Rust** compiled to WASM                                                      | Compute-heavy on large files, also runs in the CLI              |
| Resolution of instances, overrides and variants                                                       | **Rust**                                                                       | Complex logic, to be tested on fixtures                         |
| Layer split and state enumeration, for pre-rendering                                                  | Native **Rust**                                                                | Done once, outside the browser                                  |
| Prototype runtime (state machine), shared by both players                                             | **TypeScript**                                                                 | Lightweight, tightly bound to the DOM and events, easy to debug |
| Rendering                                                                                             | Skia (see D1): native in the CLI, WASM in the browser only for the full player | Fidelity: blurs, blend modes, gradients, text                   |
| Web component, controls, accessibility                                                                | **TypeScript**                                                                 | It is the public API                                            |
| Pre-compilation CLI (`folioscope bake`)                                                               | Native **Rust**                                                                | Same core, no browser                                           |

### The intermediate model (IR)

- It is **modelled on Figma**, the richest format. Sketch, XD and Penpot project onto it easily:
  their prototype models are subsets (click, hover, delay; navigate, overlay, back, scroll; dissolve,
  slide, push, and auto-animate for XD). Component states in XD and Penpot map to Figma variants.
- Each importer declares what it supports, so the compatibility matrix is generated automatically.
- It is **serializable**: it is also the output format of `folioscope bake`.
- Main contents:
  - `Document`: pages and starting points (_flows_);
  - `Frame`: size, scrolling, render tree;
  - `Node`: already-resolved geometry, styles, text;
  - `Interaction`: trigger, action, transition;
  - `Component` and `Variant`: for hover and press states, and the "change to" action.
- The IR schema is versioned, and the pre-compiled file carries that version number.

### Instant display: layered pre-rendering

Pre-rendering to images does not break interactivity: it comes from the interactive zones and the
state machine, which are kept. What would break it is flattening **each frame into a single image**.
Pre-rendering therefore splits each frame **into layers**, like video-game compositing:

| Prototype element                              | Pre-render treatment                                         | In the browser                                                                            |
| ---------------------------------------------- | ------------------------------------------------------------ | ----------------------------------------------------------------------------------------- |
| Static frame background                        | One image                                                    | `<img>`                                                                                   |
| Interactive component (hover, press, variants) | One image **per state**, over the component's area only      | Image swapped by state, no re-render                                                      |
| Scrolling content                              | Separate layer, the size of the content                      | Real `overflow: scroll` container, native scrolling                                       |
| Elements fixed on scroll                       | Separate layers                                              | Positioned above the scrolling content                                                    |
| Overlays                                       | Separate layers                                              | Positioned above the frame                                                                |
| Transitions (dissolve, slide, push…)           | Parameters and curves                                        | Web Animations or CSS, GPU-accelerated                                                    |
| Smart animate and auto-animate (XD)            | Layers paired between the two frames, and computed keyframes | CSS transforms (position, size, opacity, rotation)                                        |
| Interactive zones                              | Rectangles, triggers and actions                             | Transparent, focusable DOM elements: keyboard-accessible, no canvas                       |
| High-density screens                           | Several densities per layer (1x, 2x, 3x)                     | `srcset`, the browser picks                                                               |
| Variables, conditions, typed text              | **Not pre-renderable**                                       | Layer flagged "dynamic": rendered on the fly by the full player, loaded only in that case |

**Limits:**

- **Combinatorial explosion.** If interactive components are nested or depend on each other, states
  multiply. As long as they are independent, they only add up. Past a configurable threshold, the
  layer switches to dynamic mode, with a warning at pre-render time.
- **Dynamic content** goes through the full player, as above.

**Why it is fast:**

- **No fonts to download**: text is rasterized at pre-render. Fonts are only needed at bake time.
- **No WASM** in the lite player: target under 15 KB of JS gzipped.
- **Display before the JS even runs**: the `poster` attribute (an image) shows straight from the
  HTML; a blurred placeholder (_ThumbHash_), included in `index.json`, takes over.
- **Loading in reachability order**: the starting frame first, then those reachable in one
  interaction, then the rest.
- **Preloading**: on hovering an interactive zone, and during idle time (`requestIdleCallback`), the
  player loads the destination frame's layers.
- **Decoding off the main thread**: `img.decode()` before display, and AVIF or WebP images,
  compressed at pre-render.
- **Effective HTTP caching**: one file per layer, with a name containing its hash. A layer that does
  not change between versions stays cached.

**The `.folio` format** (decision D4) is a **folder**, not an archive: each layer is a separate
file, loadable on demand and cached independently.

```
app.folio/
├─ index.json          # IR version, flows, frames, layers, zones, interactions, transitions, ThumbHash
├─ poster.avif         # first frame, for the poster attribute
└─ layers/
   ├─ 3f9a…@2x.avif    # layers, named by hash, per density
   └─ …
```

**Build integration**:

- **CLI**: `folioscope bake app.fig -o public/app.folio`.
- **Vite plugin**: `folioscope/vite`, a sub-path of the same package. It pre-renders files
  automatically when the site is built, and caches by hash of the source file. For a portfolio,
  this is the recommended path: drop the `.fig` into the project and the build takes care of the
  rest.

### Loading (direct reading of a raw file)

1. `fetch(src)`, streamed, with progress shown.
2. Format detection, by ZIP contents and not by extension:
   - `canvas.fig` → `.fig`;
   - `document.json` → `.sketch`;
   - a `mimetype` file containing `application/vnd.adobe.sparkler.project+dcxucf` → `.xd`;
   - `manifest.json` and a `files/` folder → `.penpot` v3.
     Each importer is a module loaded on demand: one `.wasm` per format, built with Cargo features. Only
     the code for the format in use is downloaded.
3. Decoding in a Web Worker, so as not to block the page.
4. Lazy rendering: only the current frame and its direct neighbours.
5. If a `.folio` exists next to the source file (`app.folio/` for `app.fig`), the player uses it
   instead, without loading the WASM.
6. Caching of the result in the browser (Cache API), so the next visit is fast.

### Fonts

This is the first practical problem:

| Format    | Embedded fonts                                                                    |
| --------- | --------------------------------------------------------------------------------- |
| `.fig`    | No                                                                                |
| `.sketch` | Only if the designer ticked the option                                            |
| `.xd`     | No                                                                                |
| `.penpot` | Yes for custom fonts (in `objects/`). No for Google Fonts, which must be provided |

Fonts embedded in the file take priority.

**With a `.folio`, the visitor downloads no font**: text is rasterized at pre-render. Fonts are then
given to the CLI or the Vite plugin, with the same five modes as below, in a `folioscope.config.ts`
file or as options (`--fonts ./fonts`).

The HTML declarations below are therefore for **direct reading of a raw file**, and for dynamic
layers.

#### Declaring in HTML: `<folioscope-font>`

```html
<folioscope-player src="/mockups/app.fig">
  <!-- 1. Auto-detection: family, weight and style read from the file -->
  <folioscope-font src="/fonts/Inter.var.woff2"></folioscope-font>

  <!-- 2. Forced family; weight and style optional (otherwise read from the file) -->
  <folioscope-font
    family="Roboto"
    weight="700"
    style="italic"
    src="/fonts/Roboto-BoldItalic.woff2"
  ></folioscope-font>

  <!-- 3. Rule-based substitution: glob, or regex between slashes -->
  <folioscope-font match="SF Pro*" src="/fonts/Inter.var.woff2"></folioscope-font>
  <folioscope-font match="/^Helvetica( Neue)?$/i" src="/fonts/Arimo.var.woff2"></folioscope-font>

  <!-- 4. Directory, via a manifest generated by the CLI -->
  <folioscope-font base="/fonts/"></folioscope-font>

  <!-- 5. Reuse of existing @font-face rules -->
  <folioscope-font css="/fonts/fonts.css"></folioscope-font>
</folioscope-player>
```

**Why `<folioscope-font>` and not `<font>`**: `<font>` is a real HTML element, obsolete but still
recognised by the parser. And `/>` is not self-closing in HTML: the following elements would end up
nested inside it. A custom element must be closed explicitly (`</folioscope-font>`).

**The five modes:**

| Mode           | Attributes                                       | Loading                                                                              | Use                                                                                                                                                                                                                                |
| -------------- | ------------------------------------------------ | ------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Auto-detection | `src` alone                                      | **Eager**: the file must be downloaded to know its family (`name` and `OS/2` tables) | Simplest, for a few fonts                                                                                                                                                                                                          |
| Forced family  | `family`, `src`, optionally `weight` and `style` | **Lazy**: only if the mockup uses that family                                        | Recommended as soon as the family is known                                                                                                                                                                                         |
| Substitution   | `match`, `src`                                   | Lazy                                                                                 | Replace a proprietary font (SF Pro, Helvetica…) you may not serve with a free font of similar metrics                                                                                                                              |
| Directory      | `base`                                           | Lazy                                                                                 | Drop fonts into a folder. Since a browser cannot list a folder over HTTP, the CLI generates a manifest: `folioscope fonts ./public/fonts` writes `./public/fonts/folioscope-fonts.json`, with each file's family, weight and style |
| CSS            | `css`                                            | Lazy                                                                                 | Reuse an existing site's `@font-face` rules: the player reads the stylesheet and downloads the `src` URLs it needs                                                                                                                 |

- `match` rules use a glob by default (`*`, `?`, case-insensitive). A value between slashes is a
  regex (`/…/flags`).
- Variable fonts are recognised: a single declaration covers every weight of the `wght` axis.
- Accepted formats: WOFF2, WOFF, TTF, OTF. WOFF2 is decompressed in the Rust core before reading the
  tables (`ttf-parser`).
- No naming convention such as `/fonts/Inter-Bold.woff2`: every wrong guess would show a 404 in the
  console.

**Resolution order**, for each family used by the mockup:

1. Font embedded in the design file.
2. Exact `family` (with the closest weight and style).
3. First matching `match` rule, in HTML order.
4. Auto-detected fonts.
5. `base` manifest, then `css` stylesheets.
6. Generic fallback with similar metrics, and a `folioscope:font-missing` event (with the missing
   family, weight and style).

#### Shared font set: `<folioscope-fonts>`

For a portfolio with several players on the same page, fonts are declared once and downloaded once:

```html
<folioscope-fonts id="portfolio">
  <folioscope-font base="/fonts/"></folioscope-font>
  <folioscope-font match="SF Pro*" src="/fonts/Inter.var.woff2"></folioscope-font>
</folioscope-fonts>

<folioscope-player src="/a.fig" fonts="portfolio"></folioscope-player>
<folioscope-player src="/b.xd" fonts="portfolio"></folioscope-player>
```

`<folioscope-font>` children of a `<folioscope-player>` are added to the set referenced by `fonts`,
and take precedence over it.

#### JavaScript API

HTML is only one way to use the API. Each `<folioscope-font>` calls the same methods, and the player
watches its children (`MutationObserver`) to react to additions and removals.

```ts
player.registerFont({ src: "/fonts/Inter.var.woff2" }); // auto-detection
player.registerFont({ family: "Roboto", weight: 700, src: "/fonts/Roboto-Bold.woff2" });
player.registerFont({ match: "SF Pro*", src: "/fonts/Inter.var.woff2" });
player.registerFont({ base: "/fonts/" });
player.registerFont({ css: "/fonts/fonts.css" });
player.registerFont({ family: "Brand", data: arrayBuffer }); // bytes already loaded

// Shorthand:
player.registerFont("Roboto", "/fonts/Roboto-Regular.woff2");

// Shared set, in JS:
const fonts = new FolioscopeFontSet().add({ base: "/fonts/" });
player.fonts = fonts;
```

### Component interface

```html
<folioscope-player
  src="/mockups/app.fig"
  flow="Onboarding"
  page="1"
  fit="contain"
  hotspots="on-miss"
  controls
>
</folioscope-player>
```

| Attribute  | Values                              | Role                                                                       |
| ---------- | ----------------------------------- | -------------------------------------------------------------------------- |
| `src`      | URL                                 | `.folio` folder (recommended), or `.fig`, `.sketch`, `.xd`, `.penpot` file |
| `poster`   | URL                                 | Image shown before the JS even loads; usually `app.folio/poster.avif`      |
| `preload`  | `none`, `start`, `neighbors`, `all` | Layer preloading strategy; `neighbors` by default                          |
| `flow`     | name or index                       | Starting point; the first one by default                                   |
| `page`     | name or index                       | Page; the first one by default                                             |
| `fit`      | `contain`, `width`, `none`          | Scaling                                                                    |
| `hotspots` | `never`, `on-miss`, `always`        | Clickable-area display. `on-miss` flashes them when clicking beside one    |
| `controls` | boolean                             | Control bar                                                                |
| `fonts`    | id                                  | Shared font set (`<folioscope-fonts id="…">`); see "Fonts"                 |

- **Elements**: `<folioscope-player>`, `<folioscope-font>`, `<folioscope-fonts>`.
- **Methods**: `restart()`, `goTo(frameId)`, `back()`, `setFlow(name)`, `registerFont(…)`.
- **Events**: `folioscope:load`, `folioscope:navigate`, `folioscope:error`, `folioscope:font-missing`.
- **Accessibility**:
  - hotspots are keyboard-focusable;
  - Escape goes back;
  - each frame's `aria-label` comes from the frame name;
  - `prefers-reduced-motion` disables transitions.

---

## 5. Open decisions

These are settled with ADRs, in [`docs/adr/`](adr/).

| ID     | Question                                                                                          | Starting recommendation                                                                                                                                                                                                                                                    | How to decide                                                                      |
| ------ | ------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| **D1** | Rendering engine, used mostly at pre-render (native), and in the browser only for the full player | Reuse Grida (Apache-2.0): its engine is Rust and Skia, so presumably usable natively in the CLI (to check: is it published as a crate?) and as WASM via `@grida/canvas-wasm`. Alternatives: CanvasKit (BSD), or an in-house Rust engine with `tiny-skia` and a text module | Phase 0 spike: can it take our IR, or only `.fig` files? WASM weight, text quality |
| **D2** | `.fig` decoding: in-house in Rust, or `openfig-core` on the JS side                               | In-house in Rust, with `openfig-core` as a test oracle                                                                                                                                                                                                                     | Compare outputs on the fixtures                                                    |
| **D3** | Transitions: CSS or frame-by-frame rendering                                                      | Instant and dissolve first, then slide and push by compositing two bitmaps. Smart animate later                                                                                                                                                                            | Phase 3                                                                            |
| **D4** | Pre-compiled `.folio` format                                                                      | A folder: `index.json`, `poster.avif`, and layers named by hash, in AVIF or WebP and at several densities (see "Instant display")                                                                                                                                          | Phase 1: measure weight and display time on the fixtures                           |
| **D5** | Package manager                                                                                   | **pnpm**, see 6.2 — [ADR 0005](adr/0005-package-manager-pnpm.md)                                                                                                                                                                                                           | Accepted                                                                           |
| **D6** | Raster or SVG layers                                                                              | Raster by default. SVG as an option for simple layers (shapes and vectorized text), crisp at every zoom                                                                                                                                                                    | Compare weight and effect rendering (blurs, blend modes)                           |
| **D7** | Combinatorial explosion threshold                                                                 | Maximum number of pre-rendered states per frame, configurable; beyond it, dynamic layer                                                                                                                                                                                    | Measure on real files                                                              |

---

## 6. Tech stack

### 6.1 TypeScript

- **TypeScript 7** (the native version, in Go; `typescript@7.0.x` on npm).
  - `strict` configuration, with `isolatedDeclarations` for fast `.d.ts` generation.
  - Before pinning the version, check that every tool in the chain supports it.
- **Vite+** (`vite-plus`, `vp` command, MIT) as the single toolchain:

| Need                    | Command                                                 |
| ----------------------- | ------------------------------------------------------- |
| Playground dev server   | `vp dev`                                                |
| Format, lint, typecheck | `vp check` (Oxfmt, Oxlint, TS)                          |
| Tests                   | `vp test` (Vitest, with browser mode for the component) |
| Library build           | `vp pack` (tsdown)                                      |
| Cached tasks, monorepo  | `vp run`                                                |

- No UI framework: the component is vanilla, without Lit or React, to stay light.

### 6.2 Package manager: pnpm

- **pnpm** for workspaces and the lockfile. Vite+ keeps the existing package manager, and
  `vp install` delegates installation to it.
- **nub** remains possible as a script runner, since it reads pnpm lockfiles without imposing a
  format.
  - Not recommended as a base until its maturity is established.
  - It also partly overlaps with Vite+ (Node management, script execution).
- Pin the version in `package.json` (`devEngines.packageManager`).

### 6.3 Rust

- Edition 2024, stable version pinned in `rust-toolchain.toml`.
- Target `wasm32-unknown-unknown`, with `wasm-bindgen` (and `wasm-bindgen-cli`) for bindings, and
  `wasm-opt` (Binaryen) to reduce size.
  - Check `wasm-pack`'s maintenance status before adopting it. Otherwise, use `wasm-bindgen-cli`
    directly.
- Candidate crates, all MIT or Apache-2.0, hence AGPL-compatible:
  - `zip` or `rc-zip` for archives;
  - `ruzstd` (pure-Rust zstd, compiles to WASM without C);
  - `ttf-parser` to read fonts' `name` and `OS/2` tables, and a pure-Rust WOFF2 decoder (Brotli via
    `brotli-decompressor`), to be chosen during phase 2;
  - `miniz_oxide` for deflate;
  - `serde` and `serde_json` for Sketch;
  - `thiserror` for errors.
- Quality: `cargo clippy -D warnings`, `cargo fmt`, `cargo deny` (licenses and vulnerabilities), and
  `cargo nextest` for tests.

### 6.4 helpers4 libraries

The idea is to rely on **helpers4** as much as possible, in TypeScript (package `helpers4`, v3.1.x)
as in Rust (crate `helpers4`, one feature per module).

- LGPL-3.0 license, compatible with integration into an AGPL-3.0 project.
- In TypeScript, **a single dependency: `helpers4`**, not the separate `@helpers4/*` packages. This
  keeps dependencies simple. Imports go through sub-paths, which preserves tree-shaking:
  ```ts
  import { debounce, once } from "helpers4/function";
  import { isNonEmpty } from "helpers4/array";
  ```
- The candidates proposed below become new sub-paths of the same package (`helpers4/geometry`, etc.).

**Planned uses:**

| Folioscope need                            | helpers4 TS                                                     | helpers4 Rust                 |
| ------------------------------------------ | --------------------------------------------------------------- | ----------------------------- |
| Resize debounce, `once` for initialization | `helpers4/function` (`debounce`, `throttle`, `once`, `memoize`) | —                             |
| Type guards on the IR received from WASM   | `helpers4/guard`, `helpers4/type`                               | —                             |
| Colors (RGBA, hex, conversions)            | `helpers4/color`                                                | feature `color`               |
| Bounds, interpolation, rounding            | `helpers4/math`, `helpers4/number`                              | feature `number`              |
| URL resolution (`src`, fonts, images)      | `helpers4/url`                                                  | feature `url`                 |
| Delays (`after delay`), load cancellation  | `helpers4/promise`                                              | feature `future`              |
| Node and frame identifiers                 | `helpers4/id`                                                   | —                             |
| Structures (history stack, index by id)    | `helpers4/structure`, `helpers4/map`, `helpers4/set`            | features `map`, `set`, `iter` |
| Reading bytes and binary buffers           | —                                                               | features `bytes`, `hex`       |
| IR version and `.folio` compatibility      | `helpers4/version`                                              | feature `version`             |
| Durations (transitions, delays)            | `helpers4/date` (to check)                                      | features `duration`, `time`   |
| Rendered-frame cache                       | —                                                               | feature `cache`               |
| CI: badges, build status table             | `helpers4/ci`                                                   | feature `ci`                  |
| License header and `NOTICE` generation     | `helpers4/license`                                              | feature `license`             |
| Conventional commit messages               | `helpers4/commit`                                               | feature `commit`              |

**Candidates to add to helpers4** when something is missing. Each function is written in helpers4
rather than locally in Folioscope, with tests:

| Candidate               | TS                   | Rust               | Contents                                                                                                                                                                                          | Why it is generic                                                      |
| ----------------------- | -------------------- | ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| **`geometry`**          | `helpers4/geometry`  | feature `geometry` | `Rect`, `Point`, `Size`, 2D affine matrices (`compose`, `invert`, `apply`), `intersects`, `contains`, `fitContain`, `fitCover`                                                                    | Hit-testing, scaling and transforms are useful in any graphics project |
| **`easing`**            | `helpers4/easing`    | feature `easing`   | Cubic Bézier curves (`cubicBezier(x1, y1, x2, y2)`), springs, standard easings (`ease-in-out`, etc.), `lerp`                                                                                      | Prototype transitions; useful for any animation                        |
| **`binary`**            | `helpers4/binary`    | merge into `bytes` | `DataView` reader and writer with a cursor, varint and zigzag (LEB128), little-endian `f32` and `u32`                                                                                             | Kiwi uses varints; useful for any binary format                        |
| **`color` (extension)** | `helpers4/color`     | feature `color`    | Blend modes (multiply, screen, overlay…), premultiplied alpha, HSL and HSB conversion, as in Figma and Sketch                                                                                     | Style rendering requires it; likely not covered yet                    |
| **`dom` or `web`**      | `helpers4/dom`       | —                  | Custom Element helpers (typed, reflected attributes, `observedAttributes`), debounced `ResizeObserver`, pointer handling (hover, press, and the tap / drag distinction), `prefers-reduced-motion` | Every web component needs it                                           |
| **`fsm`**               | `helpers4/fsm`       | feature `fsm`      | Small typed state machine, with a history stack (_back_)                                                                                                                                          | The prototype runtime, and many UIs                                    |
| **`font`**              | `helpers4/font`      | —                  | Font loading with `FontFace`, fallback, missing-font detection, closest weight and style selection                                                                                                | Any text rendering on canvas                                           |
| **`glob`**              | in `helpers4/string` | feature `string`   | `globToRegExp`, `matchGlob` (with `*`, `?`, case-insensitive), and `parsePatternOrRegExp` for the `/…/flags` syntax                                                                               | Font `match` rules; useful wherever names are filtered                 |

Suggested order: `geometry` and `binary` first (phase 0), then `fsm` and `dom` (phase 1), then
`easing` (phase 3).

---

## 7. Repository layout

```
folioscope/
├─ crates/
│  ├─ folioscope-core/        # IR, importers (fig, sketch, xd, penpot), instance resolution (Rust lib)
│  ├─ folioscope-bake/        # layer split, state enumeration, native rendering, .folio writer
│  ├─ folioscope-wasm/        # wasm-bindgen bindings to folioscope-core (full player)
│  └─ folioscope-cli/         # `folioscope bake`, `folioscope inspect`, `folioscope fonts`
├─ packages/
│  └─ folioscope/             # npm: folioscope
│     ├─ src/
│     │  ├─ element.ts        # Custom Elements <folioscope-player>, <folioscope-font(s)>
│     │  ├─ runtime/          # prototype state machine, shared by both players
│     │  ├─ lite/             # lite player: <img> layers, DOM zones, preloading (no WASM)
│     │  ├─ full/             # full player: WASM, on-the-fly rendering, fonts (loaded on demand)
│     │  ├─ worker.ts         # off-main-thread decoding (full player)
│     │  └─ vite/             # Vite plugin (folioscope/vite): bake at build time
│     └─ test/
├─ apps/
│  ├─ site/                   # VitePress site → folioscope.baxyz.dev (GitHub Pages)
│  └─ playground/             # demo: drop a file, play it (vp dev), embedded in the site
├─ fixtures/                  # test files we created (see § 10)
│  ├─ fig/  ├─ sketch/  ├─ xd/  └─ penpot/
├─ docs/
│  ├─ adr/                    # D1 to D7
│  ├─ format-fig.md           # reverse-engineering notes on .fig prototyping
│  └─ format-sketch.md
├─ Cargo.toml                 # Rust workspace
├─ pnpm-workspace.yaml
├─ package.json
├─ vite.config.ts             # Vite+ config (check, test, pack)
├─ rust-toolchain.toml
├─ deny.toml
├─ LICENSE                    # AGPL-3.0-or-later (full text)
├─ NOTICE                     # credits for dependencies and reference projects
└─ README.md
```

---

## 8. Roadmap

Each phase ends with a demo in the playground and a published release.

### Phase 0 — Spikes (v0.0.x)

Goal: remove the risks before writing the real code.

- [ ] Rendering spike (D1): render to PNG, **natively** from Rust (or failing that with
      `@grida/refig` under Node), a frame of a `.fig`, then a single node of that frame in isolation,
      which conditions the layer split.
- [ ] Pre-render spike: by hand, split a fixture frame into layers (background, a button with its two
      states), write a minimal `index.json`, and display it with a few lines of HTML. Measure the time
      to display the first frame.
- [ ] Decoding spike: in Rust, decode a `.fig` (ZIP, zstd, Kiwi) and list the frames and the
      `prototypeInteractions`.
- [ ] Sketch spike: list the artboards and `flow`s of a `.sketch`.
- [ ] XD and Penpot spikes: list the artboards and interactions of an `.xd`
      (`interactions/interactions.json`) and of a `.penpot` v3 (`flows` and `interactions`). Check that
      IR v1 covers them without structural change.
- [ ] Create 5 to 10 fixtures covering the basic cases (§ 10).
- [ ] Add `geometry` and `binary` to helpers4.
- [ ] Write ADRs D1, D2 and D4 (D5 is done).

**Exit criterion**: a `.fig` frame rendered natively, the list of navigation links printed to the
console, and the pre-render spike displaying almost instantly.

### Phase 1 — Pre-rendering and lite player (v0.1)

The fast path comes first: it is the one portfolio visitors will see.

- [ ] IR v1 and `.fig` importer: frames, nodes, navigation interactions.
- [ ] `folioscope bake` (Rust CLI): one layer per frame to start with, plus interactive zones;
      `index.json`, `poster.avif`, layers named by hash, several densities.
- [ ] `folioscope/vite` Vite plugin, with a cache keyed on the source file's hash.
- [ ] Lite player, no WASM: layers as `<img>`, interactive zones as DOM, neighbour preloading,
      ThumbHash and `poster`.
- [ ] Runtime: click trigger, _navigate_ and _back_ actions, starting points.
- [ ] `<folioscope-player>`: `src`, `poster`, `preload`, `flow`, `page`, `fit`, `restart()`.
- [ ] Minimal control bar: restart, flow picker.
- [ ] Budgets measured in CI: under 15 KB of JS gzipped for the lite player, first frame displayed
      in under a second on an average mobile on 4G (Lighthouse CI; values to adjust after phase 0).
- [ ] Add `fsm` and `dom` to helpers4.
- [ ] Publish to npm from CI, with provenance.
- [ ] Minimal site on `folioscope.baxyz.dev`: home, the name explained, a playable demo, a "Getting
      started" page.

**Exit criterion**: a simple `.fig` prototype, pre-rendered at build time, clickable end to end and
displayed almost instantly on a portfolio page.

### Phase 2 — Sketch, layer split and rich interactions (v0.2 to v0.4)

- [ ] `.sketch` importer to the IR, with the same pre-rendering.
- [ ] Layer split at pre-render: background, scrolling content, fixed elements, overlays, interactive
      components.
- [ ] Overlays: position, backdrop, close on outside click.
- [ ] Frame scrolling (native scrolling), fixed elements, _scroll to_ action.
- [ ] Hover, press and _after delay_ triggers.
- [ ] Interactive components: instance and variant resolution, one image per state, _change to_
      action. Combinatorial explosion threshold (D7).
- [ ] Visible hotspots (`hotspots="on-miss"`).
- [ ] Fonts for pre-rendering: `folioscope.config.ts` and `--fonts`, with the five modes; bake-time
      warning for any missing font.
- [ ] `folioscope fonts <folder>` CLI: generates the `folioscope-fonts.json` manifest.

**Exit criterion**: the same test scenarios work from a `.fig` and from a `.sketch`.

### Phase 2b — Penpot and Adobe XD (v0.4 to v0.5)

- [ ] `.penpot` v3 importer to the IR:
  - pages, shapes, components, typographies;
  - `flows` and `interactions`;
  - embedded images and fonts (`objects/`).
- [ ] `.xd` importer to the IR:
  - artboards (`graphicContent.agc`), shared resources (`resources/`);
  - interactions (`interactions/interactions.json`): tap, hover, delay, navigate, overlay, back,
    scroll;
  - component states, mapped to IR variants.
- [ ] XD auto-animate: mapped to the IR's smart animate (implemented in phase 3).
- [ ] Fixtures and tests for both formats, with the same scenarios as for `.fig` and `.sketch`.

**Exit criterion**: the compatibility matrix covers all four formats, with the same basic scenarios
passing everywhere.

**Points of attention:**

- **XD**: format not documented by Adobe and frozen since 2023. It will not move any more, which is an
  advantage for stability. Creating fixtures requires the XD app: use personal archive files, or
  create them while the app can still be installed.
- **Penpot**: only the v3 format (ZIP of JSON) is supported; the v1 (binary) format is obsolete.
  Fixtures can be created for free with Penpot, online or self-hosted.

### Phase 3 — Transitions (v0.5 to v0.6)

- [ ] Dissolve, slide, push, move in and move out, with easings from `helpers4/easing`.
- [ ] Honour `prefers-reduced-motion`.
- [ ] Smart animate and auto-animate, basic version: layer pairing by name and path at pre-render,
      computed keyframes, interpolation of position, size and opacity played in CSS.

### Phase 4 — Full player: raw files in the browser (v0.7 to v0.9)

The "works right away" path with no build step, and the fallback for dynamic layers.

- [ ] WASM build of `folioscope-core`: one `.wasm` per format, loaded on demand.
- [ ] WASM rendering engine (D1), loaded only when needed.
- [ ] The component accepts a raw file in `src`, and automatically uses the sibling `.folio` if it
      exists.
- [ ] Decoding in a Web Worker, with progress; result caching (Cache API).
- [ ] Fonts in the browser: `<folioscope-font>` (the five modes), `<folioscope-fonts>`,
      `registerFont()`, resolution order, fallback, `folioscope:font-missing` event.
- [ ] Dynamic layers of a `.folio` (variables, conditions) rendered on the fly.
- [ ] Full player budget: WASM weight measured and tracked in CI.
- [ ] Publish `folioscope-cli` and `folioscope-core` to crates.io.

### v1.0

- [ ] Stable, documented API. Complete site: demo gallery, playground, compatibility matrix, API
      reference.
- [ ] Published compatibility matrix (what is supported for each format).
- [ ] Verified accessibility: keyboard, screen readers, reduced motion.

### After v1 (to be evaluated)

- Figma prototype variables, conditions and expressions.
- Drag and swipe, mobile gestures, device rotation.
- Device frame around the mockup.

---

## 9. Publishing

| Registry            | Contents                                                                                                                                                                                                                  | Note                                                            |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| **npm**             | `folioscope` (ESM, `.d.ts`, `.wasm` included) with three entry points: `folioscope` (lite player, which loads the full one only when needed), `folioscope/full` (full player loaded up front), `folioscope/vite` (plugin) | Published from CI with OIDC and `--provenance`, as for helpers4 |
| **JSR**             | `@baxyz/folioscope`                                                                                                                                                                                                       | Optional, from the same sources                                 |
| **CDN**             | jsDelivr and unpkg, automatic from npm                                                                                                                                                                                    | The `dist/folioscope.js` bundle is self-contained               |
| **crates.io**       | `folioscope-core`, `folioscope-cli`                                                                                                                                                                                       | From phase 4                                                    |
| **GitHub Releases** | CLI binaries (Linux, macOS, Windows)                                                                                                                                                                                      | Built with cargo-dist or equivalent                             |

- SemVer versioning, with Changesets (or equivalent) for the changelog.
- Conventional commits, checked with `helpers4/commit`.
- The npm package must work with no configuration: the `.wasm` is loaded through a URL relative to
  `import.meta.url`.

---

## 9b. Website: `folioscope.baxyz.dev`

### Hosting: GitHub Pages, on a baxyz.dev subdomain

Both options combined: the site is **hosted on GitHub Pages** and **served on
`folioscope.baxyz.dev`**.

- **GitHub Pages**: free, deployed from the same repository by GitHub Actions, no extra account.
- **`baxyz.dev`**: DNS is already at Cloudflare. A `CNAME folioscope → baxyz.github.io` record is
  enough.
  - Set it to **DNS only** (grey cloud), at least while GitHub issues the HTTPS certificate.
  - Then tick "Enforce HTTPS" in the repository's Pages settings.
  - Verify the `baxyz.dev` domain at the GitHub account level, so no other repository can claim it.
- **The subdomain gives a stable URL**: if hosting changes one day, links do not break.

**When moving to Cloudflare (Worker or Pages) becomes necessary**: GitHub Pages cannot set HTTP
headers. WASM threads (`SharedArrayBuffer`) need the COOP and COEP headers. Two options:

- move to Cloudflare, on the account that holds the domains, to avoid cross-account domain binding
  problems;
- use a workaround service worker (`coi-serviceworker`).

To be decided in phase 4, when performance is measured. GitHub Pages does serve `.wasm` as
`application/wasm`, so nothing blocks until then.

### Contents

| Section             | Contents                                                                                                                         |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| **Home**            | Tagline ("Flip through your prototypes."), the name explained (§ 1), a prototype playable right in the page                      |
| **Demos**           | Gallery of playable fixtures (`.fig`, `.sketch`, `.xd`, `.penpot`), each with the feature it illustrates and the HTML to copy    |
| **Playground**      | Drop your own file to play it. **Everything stays in the browser, nothing is uploaded**: say so clearly, it is a strong argument |
| **Getting started** | Installation (npm, CDN), minimal example, attributes, methods and events                                                         |
| **Compatibility**   | Matrix of what is supported for each format, generated from the tests                                                            |
| **API**             | Reference generated from the types (TypeDoc or equivalent)                                                                       |
| **About**           | AGPL license, non-affiliation notice, credits (`NOTICE`)                                                                         |

### Technical

- **Generator**: VitePress, built on Vite and integrated into the Vite+ toolchain. The component is
  used there exactly as published, which doubles as an integration test.
- **Location**: `apps/site/`. The playground (`apps/playground/`) is embedded in the site as a page.
- **Deployment**: `site.yml` workflow on every push to `main`, via `actions/deploy-pages`. PR
  previews optional.
- **The site uses the `folioscope/vite` plugin**: demos are pre-rendered at build time, which shows
  the fast path in real conditions. The playground uses the full player, since the visitor drops a
  raw file there.
- **Fixtures**: the site serves the fixtures from `fixtures/`. GitHub Pages limits: 1 GB per site,
  100 MB per file. Plenty.
- **No tracking**: analytics disabled, or a privacy-respecting, cookie-less solution if needed.

---

## 10. Tests and fixtures

- **Fixtures must be created by us**, so as not to redistribute third-party files under the AGPL.
  For each of the four formats, one fixture per feature (when the format supports it):
  - simple navigation;
  - overlay;
  - scrolling and fixed elements;
  - hover on a component;
  - variants;
  - several flows;
  - several pages;
  - missing fonts;
  - images;
  - effects (blurs, shadows, blend modes).
- **Rust tests**: decoding and IR, with snapshots via `insta`.
- **TS tests**: runtime (click scenarios, expected state) with Vitest, and the component in browser
  mode.
- **Visual regression**: capture of each frame, compared with a reference (the PNG export from Figma,
  Sketch, XD or Penpot, made once).
- **Real files, outside the repository**: locally only, to validate on real mockups, including the
  portfolio's.
- **CI** (GitHub Actions): `vp check`, `vp test`, `cargo clippy`, `cargo nextest`, `cargo deny`, WASM
  build, and bundle size check.

---

## 11. License and legal aspects

- **Project license**: `AGPL-3.0-or-later`, for the component, the Rust core and the CLI.
  - SPDX header in every source file: `// SPDX-License-Identifier: AGPL-3.0-or-later`.
- **Design files are not covered**: the `.fig`, `.sketch`, `.xd` and `.penpot` files the player reads
  remain their authors' property.
- **Dependency compatibility**: MIT, Apache-2.0, BSD and LGPL-3.0 (helpers4) can all be integrated
  into an AGPL-3.0 project. `cargo deny` and an npm license check in CI block any incompatible
  dependency.
- **`NOTICE` file**: credits for Grida (Apache-2.0, mandatory if used), kiwi, openfig-core,
  fig2sketch and the Sketch schema.
- **`.fig` reverse engineering**:
  - for interoperability purposes only;
  - no use of Figma's API or code;
  - no third-party file redistributed.
  - Other projects do it publicly (Sketch, Grida, OpenPencil). To be checked with a lawyer if the
    project grows.
- **Trademarks**: see § 1.

---

## 12. Using Claude on the project

Models available today: Claude Fable 5.1, Opus 5.5, Sonnet 5.5 and Haiku 5.5. The effort level is set
per tool (Claude Code, the app, the API).

| Step                                                                  | Model                                             | Effort        | Why                                                          |
| --------------------------------------------------------------------- | ------------------------------------------------- | ------------- | ------------------------------------------------------------ |
| Architecture, ADRs, IR schema                                         | **Opus 5.5**                                      | high          | Structural decisions are expensive to change                 |
| `.fig` prototyping reverse engineering (decoding, Kiwi field mapping) | **Opus 5.5**                                      | high          | Reasoning about an undocumented format, hypotheses to verify |
| Instance and variant resolution, smart animate                        | **Opus 5.5**, or **Fable 5.1** if Opus gets stuck | high          | The hardest part of the project                              |
| Day-to-day implementation (importers, runtime, component, CLI)        | **Sonnet 5.5**                                    | medium        | Good quality / speed / cost balance for well-specified code  |
| Tests, fixtures, CI, Vite+ and Cargo configuration                    | **Sonnet 5.5**                                    | medium to low | Well-scoped tasks                                            |
| helpers4 contributions (`geometry`, `easing`, `binary`…)              | **Sonnet 5.5**                                    | medium        | Isolated functions, easy to test                             |
| Docs, README, changelog, commit messages, small fixes                 | **Haiku 5.5**                                     | low           | Fast and cheap                                               |
| Code review before a release                                          | **Opus 5.5**                                      | medium        | A second look at security and edge cases                     |

One session per roadmap phase, with the § 8 checkboxes as the task list.

---

## References

- openfig-core: https://github.com/OpenFig-org/openfig-core
- Grida (canvas-wasm, refig): https://github.com/gridaco/grida
- OpenPencil: https://github.com/open-pencil/open-pencil
- fig2sketch: https://github.com/sketch-hq/fig2sketch
- Sketch file format: https://github.com/sketch-hq/sketch-file-format
- `.penpot` format: https://help.penpot.app/technical-guide/developer/data-model/penpot-file-format/
- xd2svg (`.xd` structure): https://www.npmjs.com/package/xd2svg
- Vite+: https://voidzero.dev
- nub: https://www.npmjs.com/package/@nubjs/nub
- helpers4: https://helpers4.dev (TS: https://github.com/helpers4/typescript, Rust: https://github.com/helpers4/rust)
