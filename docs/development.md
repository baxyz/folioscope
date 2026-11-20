# Development

How the repository is built, tested and published. Commands are listed in
[AGENTS.md](../AGENTS.md#commands).

## Layout

```text
crates/folioscope/        the published crate: library and `folioscope` binary
  src/{figma,sketch,xd,penpot}/   importers: decoding, the IR
  src/bake/               feature `bake`: drawing, fonts, the .folio writer
crates/folioscope-wasm/   the bake for JavaScript (wasm-bindgen); not published
packages/folioscope/      npm `folioscope`: player (src/), Vite plugin (src/vite/), full player (src/full/)
apps/site/                folioscope.baxyz.dev (VitePress); public/folio/ holds the baked demos
fixtures/                 test files we generate, one folder per format; external/ is git-ignored
tools/fixtures/           the generators behind fixtures/
docs/                     architecture, specs, format notes, this file
```

## Stack

- **TypeScript 7** (`strict`, `isolatedDeclarations`) with **Vite+** (`vp`) as the one toolchain:
  `vp check` (Oxfmt, Oxlint, typecheck), `vp test` (Vitest), `vp pack` (tsdown), `vp run`.
- **pnpm**, pinned once in the `packageManager` field of `package.json`; shared versions in the
  `catalog:` of `pnpm-workspace.yaml`; `pnpm-lock.yaml` is the only lockfile. Chosen for its
  workspaces and catalogs, its strict `node_modules`, and because Vite+ delegates installs to it.
  Workers Builds starts with pnpm 10 and switches to the pinned version by itself. Do not use
  `devEngines`: npm refuses to run in a repository whose `devEngines` names another package
  manager, and the release runs `npx`.
- **Rust**, edition 2024, the toolchain pinned in `rust-toolchain.toml`; `wasm32-unknown-unknown`
  with `wasm-bindgen-cli` (the version CI installs). Quality: `cargo fmt`, `cargo clippy -D
  warnings`, `cargo nextest`, `cargo deny` (licences and advisories).
- **One crate on crates.io**, `folioscope`: the importers and the IR always build; the `bake`
  feature (on by default) adds rendering and the CLI. `default-features = false` gives a reader of
  about 33 dependencies; CI builds and lints it.

### helpers4

Folioscope relies on [helpers4](https://helpers4.dev) for everything generic: in TypeScript one
dependency, `helpers4`, imported by sub-path (`helpers4/geometry`); in Rust the crate `helpers4`,
one feature per module. LGPL-3.0, compatible with the AGPL. It is used as it is; what it lacks is
written here:

- **WOFF2** fonts are opened with `wuff` (helpers4's `font` reads collections and WOFF, not WOFF2).
- **`/regex/` font rules**: helpers4's Rust `string::parse_pattern` recognises them without
  validating the regex; the bake validates it.
- **The prototype's state machine** is `runtime.ts`, on helpers4's `createBackStack`; DOM needs are
  covered by `helpers4/browser`.

## Fixtures and tests

- **Fixtures are files we make**, so as not to redistribute third-party files under the AGPL.
  `tools/fixtures` writes them (`pnpm --filter @folioscope/fixtures-tools generate`): edit the
  generator, never the files. `.fig` fixtures are written with `openfig-core`.
- **Real files** stay in the git-ignored `fixtures/external/<format>/`; tests that read them skip
  when they are absent. Real `.penpot` files are free in
  [penpot/penpot-files](https://github.com/penpot/penpot-files) (CC BY 4.0).
- **A fixture is validated once the real tool plays it** (Figma, Sketch, Penpot, Adobe XD). Until
  then, a feature it covers is "unverified" on the site's Roadmap page. Files contributed under a
  free licence (CC BY 4.0, CC0) can join `fixtures/` with their licence next to them.
- **Rust tests**: decoding, the IR (snapshots with `insta`), the bake. **TypeScript tests**: the
  runtime and the Vite plugin (Vitest). **End to end**: `test:e2e` bakes the fixtures and plays
  them in Firefox; `test:budget` checks the first frame on throttled 4G in Chromium.
- **The site's demos** are baked from the fixtures and committed (`apps/site/public/folio/`); CI
  bakes them again and fails when they are out of date.

## Feature status

`apps/site/data/features.ts` lists every feature, per format, with its status: works (tested and
checked in the real tool or on a real file), unverified (tested on our generated fixtures only),
to do, or not in the format, and for what is unverified or to do, what is missing (often a test
file). The site's Roadmap and Compatibility pages are generated from it. Change it in the same
pull request as the feature.

## Website

`folioscope.baxyz.dev`: VitePress 2 in `apps/site/`, served by a Cloudflare Worker with static
assets (`wrangler.jsonc`), built by Workers Builds from this repository: production on `main`, a
preview per branch. Build `pnpm run build`, deploy `pnpm exec wrangler deploy`. A Worker can set
HTTP headers (COOP and COEP, for WebAssembly threads), which GitHub Pages cannot. No tracking.

## Publishing

| Registry        | Contents                                                                        |
| --------------- | ------------------------------------------------------------------------------- |
| npm             | `folioscope`: the player, `folioscope/full`, `folioscope/vite`, the WebAssembly |
| JSR             | `@baxyz/folioscope`: the lite player's sources only                             |
| crates.io       | `folioscope`: the CLI and the library                                           |
| jsDelivr, unpkg | automatic from npm; `dist/folioscope.js` is self-contained                      |

Releases go through `helpers4/action-release` (the **Release** workflow): version and changelog
from the Conventional Commits, trusted publishing (OIDC) everywhere, provenance on npm. Details in
[AGENTS.md](../AGENTS.md#releasing).

## Licence and legal

- **AGPL-3.0-or-later** for everything; an SPDX header in every source file. Dependencies must be
  MIT, Apache-2.0, BSD or LGPL-3.0 (`cargo deny` checks the Rust side). Credits in `NOTICE`.
- **Design files are not covered**: the files the player reads remain their authors'.
- **Reading `.fig` files is reverse engineering for interoperability**: no use of Figma's API or
  code, no third-party file redistributed.
- **Trademarks**: never "Figma", "Sketch", "Adobe", "XD" or "Penpot" in a package, element or logo
  name; say "compatible with `.fig`, `.sketch`, `.xd` and `.penpot` files".
