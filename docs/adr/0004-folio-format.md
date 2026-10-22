# ADR 0004 — The `.folio` format

- **Status:** Accepted
- **Date:** 2026-10-20
- **Decision ID:** D4

## Context

`folioscope bake` turns a design file into what the lite player needs to play it instantly: no
WASM, no fonts, a few kilobytes of JS. The format must let the browser fetch only what the current
screen needs, and cache each piece independently across versions.

## Decision

A `.folio` is a **folder**, not an archive:

```text
app.folio/
├─ index.json          # IR version, flows, frames, layers, zones and their states, transitions
├─ poster.webp         # the first frame, for the poster attribute
└─ layers/
   ├─ <hash>@1x.webp   # one file per layer and density, named by content hash
   └─ <hash>@2x.avif   # WebP lossless, or AVIF for photo-like layers (below)
```

- **A frame is a stack of layers** (background, scrolling content, fixed elements, overlays) plus
  **zones**: rectangles with triggers, actions, and the images of their states (`default`,
  `hover`, `press`…). Interactivity lives in the zones and the runtime, never in the pixels.
- **Layers are named by a hash of their content** (SHA-256 of the pixels, 16 hex digits): an
  unchanged layer keeps its URL across bakes and stays in the HTTP cache. Identical layers in two
  frames are stored once.
- **Layers are WebP lossless**, or **AVIF** (quality 80) when the WebP weighs 64 KB or more and
  the AVIF is at least 30 % lighter: flat UI screens stay pixel-exact, photos shrink. The format
  is chosen at the highest density and kept for the others; `index.json` records it per layer.
  The poster is WebP lossless at 1x. `--no-avif` (CLI) and `avif: false` (`folioscope/vite`) keep
  every layer in WebP.
- **One file per density** (1x and 2x by default). The player picks one density per page load.
- **Each frame carries a ThumbHash placeholder** (about 25 bytes, base64) in `index.json`: without
  a `poster`, the player shows it, blurred, from the moment `index.json` arrives.
- `index.json` carries the IR version; a player refuses a newer major version.

## Phase 0 spike (removed once Phase 1's bake and player replaced it)

`fixtures/figma/triggers.fig` baked by hand: a splash that leaves after 1.5 s, then a home screen as
a background plus a button in three states taken from the "while hovering" and "while pressing"
frames. Played by `spikes/prerender/index.html` (~80 lines, no dependency), checked in Firefox
with Playwright: the delay, hover and press all swap images with no re-render.

| Measure                                               | Value                                    |
| ----------------------------------------------------- | ---------------------------------------- |
| 5 layers at 1x + 2x, PNG                              | 30 KB in total                           |
| First frame: HTML + `index.json` + splash@2x          | 12 KB                                    |
| First frame on screen, local, cold / warm             | 110 ms / 29 ms                           |
| Estimate on Lighthouse's mobile 4G (1.6 Mb/s, 150 ms) | ~1 s: three round trips after connection |

## Consequences

- The first frame waits for three sequential requests (page, `index.json`, image). To reach the
  sub-second target on 4G, Phase 1 shows the `poster` straight from the HTML and inlines or
  preloads `index.json`.
- Image formats, measured in Phase 1 on a real 11-frame file with photos, at 1x + 2x:

  | Layers                                  | Weight | Native bake |
  | --------------------------------------- | ------ | ----------- |
  | PNG                                     | 3.9 MB | —           |
  | WebP lossless only                      | 3.2 MB | 3.5 s       |
  | WebP, or AVIF at encoder speed 10       | 932 KB | 8.8 s       |
  | WebP, or AVIF at encoder speed 6 (kept) | 666 KB | 24.8 s      |

  On flat UI screens, WebP lossless takes 4 KB where PNG took 27 KB, and AVIF is never picked.
  Speed 6 is kept: the bake runs once per file at build time and is cached, so a slower bake
  costs little while the lighter layers are downloaded by every visitor. The bake is
  deterministic, so the site demos are checked in CI.

- Still open: SVG layers for simple shapes (D6).
