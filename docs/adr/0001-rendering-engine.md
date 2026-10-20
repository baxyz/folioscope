# ADR 0001 — Rendering engine

- **Status:** Proposed
- **Date:** 2026-10-20
- **Decision ID:** D1

## Context

Rendering happens mostly at bake time (`folioscope bake`, the Vite plugin), and in the browser
only for the full player (raw files, dynamic layers). The engine must render a whole frame **and a
single node in isolation**, since baking splits frames into layers. It must eventually serve all
four formats, which meet in the IR.

The starting recommendation was Grida's engine, "presumably usable natively". Phase 0 checked it.

## Findings (Phase 0 spike, `fixtures/fig/navigation.fig`)

**`@grida/refig` 0.0.7** (MIT) on `@grida/canvas-wasm` 0.91.0-canary.22 (Apache-2.0), under Node:

- Renders frames and isolated nodes to PNG correctly (rounded corners, fills, 2x scale).
- 1.5 s for the first render (WASM start-up), then 3–50 ms per node.
- `grida_canvas_wasm.wasm` is **17.6 MB, 6.4 MB gzipped**.
- Input is a `.fig` or a Figma REST API JSON, **not our IR**. Other formats would need an
  IR → REST-JSON adapter.
- **No native path.** Grida's Rust workspace is `publish = false`; the canvas engine's sources are
  not in the public `gridaco/grida` repository any more, only the npm WASM binary is. A native Rust
  CLI cannot link it.

Alternatives looked at (not spiked yet):

| Option                                   | Licence          | Native            | Browser                  | Input                  | Notes                                                                                                    |
| ---------------------------------------- | ---------------- | ----------------- | ------------------------ | ---------------------- | -------------------------------------------------------------------------------------------------------- |
| A. refig / canvas-wasm, driven from Node | MIT / Apache-2.0 | No (WASM in Node) | 6.4 MB gz                | `.fig`, REST JSON      | Best fidelity on `.fig` today; opaque binary; bake moves to Node                                         |
| B. CanvasKit (Skia WASM) driven from TS  | BSD-3-Clause     | No                | ~3 MB gz                 | Our IR, via draw calls | Skia fidelity, text via Paragraph; we write the IR → Skia layer                                          |
| C. **IR → SVG, rasterized by `resvg`**   | MIT / Apache-2.0 | **Yes**           | Small (pure Rust → WASM) | Our IR                 | One emitter yields both raster and SVG layers (D6); blurs, blend modes, gradients, text with given fonts |
| D. Own engine on `tiny-skia` + `parley`  | BSD / MIT        | Yes               | Small                    | Our IR                 | Most control, most work                                                                                  |

## Proposed decision

**C: the IR is rendered by emitting SVG, rasterized natively by `resvg`.**

- Keeps the planned architecture: a native Rust `folioscope bake`, a small WASM for the full
  player, one renderer for all four formats.
- Answers D6 at the same time: a layer is an SVG first, rasterized or not.
- **refig stays as a test oracle** for `.fig` (pixel comparison in visual regression), the same
  role `openfig-core` plays for decoding (ADR 0002).
- Fallback if fidelity falls short on real files (Figma-specific effects such as background blur,
  angular gradients, smoothed corners): B for the offending layers.

## To confirm before accepting

- Spike C on the fixtures: frame and isolated node, compare with refig's output.
- Text: line breaking and metrics against Figma's on a real file.
- WASM size of `resvg` with the features we need.
