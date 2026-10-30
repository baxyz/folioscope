# ADR 0013 — One crate on crates.io

- **Status:** Accepted
- **Date:** 2026-10-31

## Context

The Rust code was three crates: `folioscope-core` (importers, the IR, 33 dependencies),
`folioscope-bake` (rendering, fonts, the `.folio` writer, about 160 more) and the command-line tool.
Publishing them meant three crates released in dependency order, each with its own trusted
publisher, and a version written into the path dependencies between them.

## Decision

**One crate, `folioscope`**, a library and a binary (`cargo install folioscope`), in
`crates/folioscope/`:

- the importers and the IR (`figma`, `sketch`, `penpot`, `xd`, `ir`, `kiwi`, `summary`) are always
  there;
- rendering and the `.folio` writer live in `folioscope::bake`, behind the **`bake` feature, on by
  default**, which also holds the dependencies they need (resvg, ravif, parley, skrifa…) and the
  command-line tool (`required-features`). `default-features = false` gives a reader of about 33
  dependencies.

`folioscope-wasm` stays a crate of its own, `publish = false`, shipped in the npm package.

## Why

- **A release is one `cargo publish`.** A failure halfway through three interdependent crates leaves
  one published without the next, and a published version is never replaced.
- **The three always had the same version**: separate crates bought no independent versioning.
- **One trusted publisher, one first publication by hand.**
- The library's audience is small: the product is the command-line tool, the player and the
  plugin. The feature still lets someone read files without pulling in the renderer.
- helpers4 is built the same way: one crate, one feature per module.

## Consequences

- The boundary between reading and rendering is a feature flag, not a crate boundary: the compiler
  checks it in CI (`--no-default-features` builds and lints).
- Module paths change: `folioscope_core::x` is `folioscope::x`, `folioscope_bake::x` is
  `folioscope::bake::x`. Nothing was published yet.
- Splitting again later stays possible (for example a `folioscope-core` re-exported by `folioscope`)
  but would be a breaking change after 1.0.
- Commit scopes `core`, `bake`, `cli` and `wasm` stay: they name areas of the code.
