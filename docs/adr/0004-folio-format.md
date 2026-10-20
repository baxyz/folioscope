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
├─ poster.png          # the first frame, for the poster attribute
└─ layers/
   ├─ <hash>@1x.png    # one file per layer and density, named by content hash
   └─ <hash>@2x.png
```

- **A frame is a stack of layers** (background, scrolling content, fixed elements, overlays) plus
  **zones**: rectangles with triggers, actions, and the images of their states (`default`,
  `hover`, `press`…). Interactivity lives in the zones and the runtime, never in the pixels.
- **Layers are named by a hash of their content**: an unchanged layer keeps its URL across bakes
  and stays in the HTTP cache. Identical layers in two frames are stored once.
- **One file per density** (1x and 2x by default). The player picks one density per page load.
- `index.json` carries the IR version; a player refuses a newer major version.

## Phase 0 spike (`spikes/prerender/`)

`fixtures/fig/triggers.fig` baked by hand: a splash that leaves after 1.5 s, then a home screen as
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
- Still open, to measure in Phase 1: AVIF or WebP instead of PNG (photos will dominate the
  weight), the hash function (the spike uses Rust's `DefaultHasher`, which is not stable across
  releases: use a content hash such as SHA-256, truncated), and SVG layers for simple shapes (D6).
