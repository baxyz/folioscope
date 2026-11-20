# The bake

`folioscope::bake` (Rust, feature `bake`) turns a design file into a [`.folio`](folio.md) folder. It
runs natively in the CLI and as WebAssembly in the Vite plugin and the full player, with the same
output byte for byte.

```text
design file ──► importer ──► IR (prototype graph) ──┐
            └─► the format's own tree ──► SVG per layer ──► resvg ──► WebP / AVIF, or the SVG
```

## Command line

```bash
folioscope bake <file> <out.folio> [--no-avif] [--no-svg] [--max-states <n>] [--all-pages]
    [--fonts <file or folder> [--family <name>] [--match <pattern>]... [--weight <n>] [--italic]]...
folioscope inspect <file>                  # pages, frames, flows and links, any format
folioscope render <file.fig> <node> <out.png> [scale]   # one node, and its SVG
folioscope fonts <folder>                  # writes folder/folioscope-fonts.json
```

The `folioscope/vite` plugin takes the same options: `include`, `base`, `cacheDir`, `avif`, `svg`,
`maxStates`, `fonts` (see the site's Getting started).

The bake reports what it could not do, as warnings: fonts it lacked, frames too large to be an
image (over 16,383 px at the highest density, the most WebP holds), pages left out for having no
flow, components with states left out.

## Drawing

Each format is drawn from its own tree (`bake/svg` for `.fig`, `bake/sketch.rs`, `bake/xd.rs`,
`bake/penpot.rs`) into one SVG document per layer, rasterized by `resvg` (built without its text
and system-font features).

- **Shapes**: a `.fig` stores each node's outline (`fillGeometry`, `strokeGeometry`) as path
  commands, which the SVG fills as they are. The other formats give shapes and paths the bake
  writes with helpers4's `geometry`.
- **Colours** are written by helpers4's `Rgba::to_css` (`rgb(r g b)`).
- What each format draws, and what it does not yet, is listed on the site's Roadmap page; the
  format notes ([Figma](../format-figma.md), [Sketch](../format-sketch.md),
  [XD](../format-xd.md), [Penpot](../format-penpot.md)) say how each feature is stored.

## Layer formats

Each layer is encoded in the lightest format that keeps it right:

1. **Rasterized** at every density, highest first. **Lossless WebP**, or **AVIF** (quality 80,
   encoder speed 6) when the WebP weighs 64 KB or more and the AVIF is at least 30 % lighter: flat
   UI stays pixel-exact, photos shrink. The format chosen at the highest density is kept for the
   others. `--no-avif` / `avif: false` keep WebP.
2. **Served as its SVG** instead, when the layer holds **no `<image>` and no `<filter>`** and its
   SVG, **gzipped** as a server sends it, is **lighter than its lightest image** (the 1x one): no
   visitor downloads more than with images, the layer is one file for every density, sharp at any
   zoom. `--no-svg` / `svg: false` keep images. Photos stay images (an SVG in `<img>` can only
   inline them, a third heavier, without AVIF or a cache of their own); blurs and shadows too (slow
   for the browser to draw, and not quite as `resvg` draws them).

The poster is lossless WebP at 1x.

### Measures

- **Image formats**, on an 11-frame `.fig` with photos, 1x and 2x: PNG 3.9 MB; lossless WebP
  3.2 MB (3.5 s); WebP or AVIF at speed 10, 932 KB (8.8 s); at speed 6, kept, 666 KB (24.8 s). On
  flat UI, lossless WebP takes 4 KB where PNG took 27 KB. The bake runs once per file and is
  cached, so the slower encoder costs little while every visitor gets lighter layers.
- **SVG layers**, on eleven real `.penpot` files from
  [penpot/penpot-files](https://github.com/penpot/penpot-files) (CC BY 4.0, git-ignored in
  `fixtures/external/`), every text drawn with its Google Font: 41 layers of 184 become SVG; the
  bytes a visitor downloads drop by 3 % at 1x and 4 % at 2x overall, 0 to 8 % on app screens with
  photos, 9 to 96 % on component sheets and kits. Text keeps most layers images: glyph outlines
  weigh more than the pixels of small text. Comparing with the densest image instead made 16
  layers heavier for 1x visitors (+56 KB) than they saved at 2x (−39 KB).
- **Fidelity of SVG layers**, the browser against `resvg`'s 2x image, share of pixels differing
  by more than 32 of 255 on a channel: median 0.015 % in Chromium and 0.002 % in Firefox; at worst
  1.3 % and 1.0 %, on the edges of small icons (anti-aliasing).

## Fonts and text

A `.fig` file stores the outline of every laid-out glyph: its text is drawn as paths, with no font.
Fonts are needed where the file stored no glyphs: `.sketch`, `.xd` and `.penpot` files, and a
`.fig` variant other than the one on screen.

- **Layout: `parley`** (shaping through HarfRust, line breaking and alignment in the box), glyphs
  read with **`skrifa`** and drawn as SVG paths, so text goes through the same rasterization as
  everything else and no font reaches the `.folio`. Pure Rust: the same in WebAssembly.
- **Never the system's fonts**, so the bake is the same on every machine. Fonts come from, in this
  order: the design file (a `.sketch` may embed its fonts, a `.penpot` its custom ones), then the
  user's (`--fonts`, the `fonts` option, `<folioscope-font>` in the browser).
- **Font files**: TTF, OTF, collections (`.ttc`), WOFF and WOFF2 (`wuff` for WOFF2, helpers4 for
  the rest). Each face is read for its family (typographic family first), weight, style and, for a
  variable font, its weight range.
- **A font may stand in for other names**: its own family, plus the names and patterns it is told
  to answer to (`--family`, `--match`; `*` and `?`, case-insensitive, or `/regex/flags`), with a
  weight and style that override the file's. This is how a free font replaces one you may not serve
  (`SF Pro*`).
- **Matching**, for each family a text asks for: the same family, case-insensitive, with the
  closest weight and the same style (CSS font matching); otherwise the first matching rule;
  otherwise the text is left out and the bake reports the family and style missing.
- `folioscope fonts <folder>` writes `folioscope-fonts.json`, each file's family, weight and
  style, for the player's `base` mode.
- Test fonts live in `fixtures/fonts/`: Inter cut down to Latin (SIL OFL 1.1).

## WebAssembly

`crates/folioscope-wasm` exposes `bake(input, avif, fonts, densities?, hints?, svg?, maxStates?)`
and `inspect(input)` through `wasm-bindgen`. It is built with SIMD (`+simd128`, Node 16 and every
current browser), about 6 MB (2 MB gzipped, capped at 2.5 MB in CI), and ships in the npm package,
never in a site's JavaScript.
