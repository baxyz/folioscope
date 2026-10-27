# ADR 0012 — The full player bakes in the browser

- **Status:** Accepted
- **Date:** 2026-10-28

## Context

`<folioscope-player src="app.fig">` must play a raw design file with no build step (Phase 4). The
lite player only plays `.folio` folders: images, a description and no rendering. The plan
foresaw a WebAssembly core per format and a WebAssembly renderer, drawing in the browser.

## Decision

The full player **runs the bake itself** in the browser: the same WebAssembly module as
`folioscope/vite` (ADR 0009), in a Web Worker. It turns the raw file into the files of a `.folio`
folder in memory, which the lite player plays as any other, through `blob:` URLs.

- **A `.folio` first**: for `src="app.fig"`, the player looks for `app.folio/index.json` next to
  it, and plays it when it is there: no WebAssembly is downloaded.
- **Loaded on demand**: the lite player imports `folioscope/full` (`dist/full/`) only for a raw
  file without its `.folio`; the worker loads the WebAssembly (`wasm/`, 2.0 MB gzipped) itself.
  The lite player stays under its 15 KB budget; CI caps the WebAssembly at 2.5 MB gzipped.
- **One density, no AVIF**: the screen's density (1x or 2x) and lossless WebP, to bake fast.
- **Cached**: the files go into the Cache API, keyed by the SHA-256 of the design file (and the
  density and fonts); a second visit reads them back without baking.
- **Progress**: `folioscope:progress` events (`download`, `cache`, `bake`); fonts the bake missed
  become `folioscope:font-missing` events.

## Consequences

- One code path renders every format, for the CLI, the Vite plugin and the browser: what bakes
  plays, and the other way round.
- One module for every format, rather than one per format: simpler, heavier for a `.fig` alone.
  Splitting it stays possible if its weight matters.
- No dynamic layers yet (variables, conditions): the bake draws what the file shows.
