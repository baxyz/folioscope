# ADR 0002 — `.fig` decoding in Rust

- **Status:** Accepted
- **Date:** 2026-10-20
- **Decision ID:** D2

## Context

`.fig` files can be decoded in Rust (the `folioscope` crate, shared by the CLI and the WASM full player)
or in JS with `openfig-core` (MIT), which already parses them.

## Decision

Decode `.fig` **in Rust**, in the `folioscope` crate:

- `kiwi`: a generic Kiwi decoder driven by the schema embedded in each file, so new Figma fields
  never require a code change;
- `fig`: ZIP, `canvas.fig` chunks, zstd (`ruzstd`) or raw deflate (`flate2`, Rust backend).

`openfig-core` is the **test oracle** and the **fixture writer**: `tools/fixtures` builds `.fig`
fixtures with it, and the Rust decoder must read them back identically.

## Consequences

- One decoder for the CLI, the bake and the browser; no JS parser in the lite player.
- The Phase 0 spike reads `fixtures/figma/navigation.fig` and lists its frames, flow and links,
  matching openfig-core's reading of the same file.
- Checked on 29 real files saved by Figma (versions 80 to 106), legacy links included.
- The cursor and varint readers are local for now; they move to helpers4's `binary` module.
