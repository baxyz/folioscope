# ADR 0006 — SVG layers

- **Status:** Accepted
- **Date:** 2026-11-20
- **Decision ID:** D6

## Context

The bake draws every layer as an SVG document, then rasterizes it with `resvg` (ADR 0001) into
lossless WebP or AVIF, at 1x and 2x (ADR 0004). The SVG is there anyway: serving it as is costs no
rendering code. It is one file for every density, sharp at any zoom (fullscreen, 3x phones), and a
flat screen of shapes and text compresses well as text. Against it:

- **Images.** An SVG shown by `<img>` cannot load anything else: a photo has to be inlined in
  base64 (a third heavier), and loses AVIF and its own HTTP cache entry.
- **Filters.** Blurs and shadows are slow for the browser to draw, and drawn a little differently
  from `resvg`.
- **Fidelity.** The browser draws the SVG, not `resvg`: anti-aliasing differs at the edges.
- **Weight.** Text is glyph outlines: a text-heavy layer can weigh more than its image.

## Options

| Option                                         | For                                                    | Against                                                    |
| ---------------------------------------------- | ------------------------------------------------------ | ---------------------------------------------------------- |
| A. Raster only (as before)                     | The same pixels everywhere                             | Blurry past 2x; two files per layer; heavy on flat screens |
| B. SVG only                                    | Sharp everywhere; one file per layer                   | Photos inlined, filters slow, heavier text layers          |
| C. **SVG per layer, when allowed and lighter** | The best of each, layer by layer; nothing to configure | Two renderers draw the prototype (the browser and `resvg`) |

## Decision

**C.** A layer is served as its SVG (`layers/<hash>.svg`, `"format": "svg"` in `index.json`) when:

1. it holds **no `<image>` and no `<filter>`**, and
2. its SVG, **gzipped** as a server sends it, is **lighter than its lightest image** (the 1x one):
   whatever their screen, no visitor downloads more than before.

Otherwise it stays WebP or AVIF, at every density, as before. The poster stays WebP. `--no-svg`
(CLI) and `svg: false` (`folioscope/vite`) keep every layer a raster image. The colours in the SVG
are written by helpers4's `Rgba::to_css` (`rgb(r g b)`), which changed every layer's hash once.

The first version compared the SVG with the densest image (2x). On real files, 16 of its 57 SVG
layers were then heavier than their 1x image: 56 KB more for 1x visitors, against 39 KB less for
2x visitors on the same layers. It was changed for the rule above.

## Measures

Eleven real `.penpot` files from [penpot/penpot-files](https://github.com/penpot/penpot-files)
(CC BY 4.0, kept in the git-ignored `fixtures/external/`), baked with `--all-pages` and their
Google Fonts (OFL): every text drawn but three fonts used by a few runs (DM Mono, Plus
Jakarta Sans, Trocchi). These are the bytes a visitor downloads for every layer: SVG
gzipped, images as they are.

| Kind of file                                             | Layers as SVG          | 1x          | 2x           |
| -------------------------------------------------------- | ---------------------- | ----------- | ------------ |
| App screens with photos, templates, wireframes (8 files) | 0 to 11 of 6–40        | 0 to −6 %   | 0 to −8 %    |
| Component sheets and kits (3 files)                      | 4 of 8, 2 of 3, 3 of 3 | −9 to −92 % | −10 to −96 % |
| All eleven (9.9 MB at 2x)                                | 41 of 184              | −3 %        | −4 %         |

- **Text is what keeps a layer an image**: glyph outlines weigh more than the pixels of small text.
  A first pass without the files' fonts (their text left out) made 83 layers of 136 SVG; with the
  text drawn, 41 of 184. The gain is real but modest on app screens; it is large on flat, shape-heavy
  layers (component sheets, icons, diagrams).
- **Fidelity**, each SVG layer drawn by the browser against `resvg`'s 2x image, share of pixels
  differing by more than 32 of 255 on a channel: median 0.015 % in Chromium and 0.002 % in
  Firefox; at worst 1.3 % and 1.0 %, on the edges of small icons and buttons (anti-aliasing). The
  fixtures' 42 SVG layers give the same picture (at worst 1.7 % and 0.3 %).
- The site's ten demos, flat screens, become SVG only: 35 files instead of 70, 2 to 5 KB per
  demo.
- No `.fig`, `.sketch` or `.xd` file under a free licence was found to measure; the bake draws all
  four formats to the same SVG, so the rule applies the same way, but their numbers are not known.

## Consequences

- The lite player builds a layer's URL from its format: `layers/<src>.svg`, or
  `layers/<src>@<density>x.<format>`. No other change: `<img>` shows both.
- Visual regression tests compare `resvg`'s pixels; an SVG layer is drawn by the browser, within
  the differences above.
- Serving an SVG layer _inline_ (in the page, not through `<img>`) would let it point at its photos
  by URL and toggle groups: ADR 0007 uses that for nested interactive components.
