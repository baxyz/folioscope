# ADR 0009 — Baking from Node: WebAssembly

- **Status:** Accepted
- **Date:** 2026-10-21

## Context

`folioscope/vite` bakes design files while a site builds, inside Node. The bake is Rust
(`folioscope-bake`). The plugin must work after a plain `npm install`, on Linux, macOS and Windows,
with no Rust toolchain.

## Options

| Option                                                                                       | For                                                                                                                    | Against                                                                                         |
| -------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| A. Spawn the `folioscope` CLI found on `PATH`                                                | Native speed; trivial plugin                                                                                           | Every user installs a binary first; versions drift from the npm package                         |
| B. Native binaries as optional npm packages per platform (esbuild, Biome, Rolldown)          | Native speed, no install step                                                                                          | A release matrix (6+ targets), cross-compilation, platform packages to publish and keep in sync |
| C. **Compile core and bake to WebAssembly** (`folioscope-wasm`), shipped inside `folioscope` | One package for every platform; no install step; the same module later powers the full player in the browser (Phase 4) | Slower than native; a few hundred KB more in the npm package (never in the site's JS bundle)    |

## Measures (C, prototype in `crates/folioscope-wasm`)

The whole bake (decode, render, rasterize, hash) compiles to `wasm32-unknown-unknown` unchanged:

|                                                     | Native | WASM   | WASM + SIMD             |
| --------------------------------------------------- | ------ | ------ | ----------------------- |
| `navigation.fig` (2 frames)                         | 36 ms  | 83 ms  | 40 ms                   |
| Community file (11 frames, 9 photos, 3.9 MB of PNG) | 3.9 s  | 11.8 s | **8.1 s**               |
| Module size                                         | —      | 2.4 MB | 2.2 MB (790 KB gzipped) |

## Decision

**C, built with SIMD (`+simd128`, supported by Node ≥ 16 and every current browser).** The plugin
bakes on first build and caches each `.folio` by the hash of its source file, so the slower bake is
paid once per change. The CLI stays native for CI and command-line users.

Revisit B if bake times on large real files become a complaint despite the cache.
