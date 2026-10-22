# AGENTS.md — baxyz `folioscope`

This repository inherits the [canonical workspace rules](https://github.com/baxyz/.dev/blob/main/AGENTS.md). Only project-specific details are documented here.

## Scope

`<folioscope-player>`: a web component that plays the interactive prototype stored in a `.fig`,
`.sketch`, `.xd` or `.penpot` file, with no Figma, Sketch or Adobe account.

**Status:** Phase 1 under way. `.fig` files bake into `.folio` folders (CLI, and `folioscope/vite`
through WebAssembly) and play in `<folioscope-player>`; the four formats are read into a common
summary. See [PROJECT.md § 8](PROJECT.md#8-roadmap).

## The project plan: `PROJECT.md`

[PROJECT.md](PROJECT.md) holds the design and the roadmap that are not implemented or documented
elsewhere yet: architecture, IR, `.folio` format, fonts, open decisions. Read it before any
structural change. It is a **temporary** document that shrinks as the project advances:

- When something lands, delete the matching text from `PROJECT.md` **in the same PR**: a finished
  roadmap item, a decision now recorded in an ADR, a spec now covered by code, the README or the
  site. Don't tick checkboxes, remove the line.
- Before deleting, check that what is still useful has a permanent home (ADR, README, code docs,
  site). Rationale goes into an ADR, not into a code comment.
- Section numbers stay stable; a deleted section leaves a gap. Update links that point at it.
- Once the file is empty, delete it, and remove the links to it (README, this file, `CLAUDE.md`).

## License

**AGPL-3.0-or-later** for everything (component, Rust core, CLI). Every source file starts with an
SPDX header:

```ts
// SPDX-License-Identifier: AGPL-3.0-or-later
```

Dependencies must be MIT, Apache-2.0, BSD or LGPL-3.0 (`cargo deny` enforces it on the Rust side).
Credit reference projects and Apache-2.0 dependencies in `NOTICE` when they are first used.

## Commit Scopes

Defined in `scopes.json`:

| Scope                                       | Covers                                                                    |
| ------------------------------------------- | ------------------------------------------------------------------------- |
| `core`                                      | `crates/folioscope-core`: IR, importers, instance resolution              |
| `bake`                                      | `crates/folioscope-bake`: layer split, state enumeration, `.folio` writer |
| `wasm`                                      | `crates/folioscope-wasm`: wasm-bindgen bindings                           |
| `cli`                                       | `crates/folioscope-cli`: `folioscope bake`, `inspect`, `fonts`            |
| `player`                                    | `packages/folioscope`: custom elements, runtime, lite and full players    |
| `vite`                                      | `folioscope/vite` plugin                                                  |
| `site`                                      | `apps/site`: VitePress site on folioscope.baxyz.dev                       |
| `playground`                                | `apps/playground`                                                         |
| `fixtures`                                  | `fixtures/`                                                               |
| `build`, `test`, `docs`, `deps`, `ci`, `ai` | As usual                                                                  |

## Structure

```text
folioscope/
  crates/
    folioscope-core/   ← importers (figma, sketch, xd, penpot), format detection, IR v1, summary
    folioscope-bake/   ← rendering (SVG, then PNG with resvg) and the .folio writer
    folioscope-wasm/   ← the bake for JavaScript (folioscope/vite, later the full player)
    folioscope-cli/    ← `folioscope` binary: inspect, render, bake
  packages/
    folioscope/        ← npm package: <folioscope-player> (src/), folioscope/vite (src/vite/),
                         wasm/ built by `build:wasm`, git-ignored, published
  apps/site/           ← VitePress site on folioscope.baxyz.dev (ADR 0010); public/folio/ holds baked demos
  fixtures/            ← test files we generated, one folder per format;
                         external/ (git-ignored) holds real third-party files for local checks
  tools/fixtures/      ← the generators behind fixtures/
  docs/
    adr/               ← decisions D1–D7
    format-*.md        ← what we know of each file format
  PROJECT.md           ← temporary project plan, shrinks to nothing (see above)
```

## Commands

```bash
vp install                                  # pnpm install, via Vite+
vp check                                    # format + lint + typecheck (Oxfmt, Oxlint, tsgo)
vp run -r test                              # Vitest in every package
vp run -r build                             # vp pack in every package
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace --no-tests=pass
cargo deny check
pnpm --filter @folioscope/fixtures-tools generate   # rewrite fixtures/ from tools/fixtures
pnpm --filter @folioscope/site bake-demos            # re-bake the site demos after a fixture or bake change
pnpm run build                                       # player package, then the site (what Workers Builds runs)
pnpm --filter @folioscope/site dev                   # the site on http://localhost:4323 (build the player first)
pnpm --filter folioscope build:wasm                  # WebAssembly bake for folioscope/vite (Rust + wasm-bindgen-cli 0.2.129)
pnpm --filter folioscope test:e2e                    # bakes a fixture and plays it in Firefox
pnpm --filter folioscope test:budget                 # first frame under 1 s on throttled 4G (Chromium)
cargo run -p folioscope-cli -- bake <file.fig> <out.folio>
cargo run -p folioscope-cli -- inspect <file>        # pages, frames, flows and links of any format
cargo run -p folioscope-cli -- render <file.fig> <node-id> <out.png> [scale]
```

## Releasing

Bump `version` in `packages/folioscope/package.json`, merge, then push the tag `vX.Y.Z` on `main`.
`release.yml` builds the WebAssembly bake and the player, publishes to npm through trusted
publishing (OIDC, with provenance, no token), and creates the GitHub release.

The pnpm version is pinned in three places, to change together: `devEngines` in `package.json`,
the `pnpm/action-setup` steps of the workflows, and the `PNPM_VERSION` build variable of the
Cloudflare Worker (production and preview triggers).

## Rules

- **helpers4 first.** Before writing a generic helper (geometry, binary reading, easing, FSM, DOM,
  glob…), look in `helpers4` (TS: one dependency, `helpers4`, imported by sub-path such as
  `helpers4/function`; Rust: crate `helpers4` with one feature per module). If it is missing and
  generic, propose it for helpers4 instead of coding it locally. See [PROJECT.md § 6.4](PROJECT.md#64-helpers4-libraries).
- **No UI framework** in the component: vanilla custom elements, no Lit, no React.
- **Performance budgets are features**: the lite player stays under 15 KB of JS gzipped, with no
  WASM and no fonts.
- **Trademarks**: never use "Figma", "Sketch", "Adobe", "XD" or "Penpot" in a package, element or
  logo name. Say "compatible with `.fig`, `.sketch`, `.xd` and `.penpot` files".
- **Fixtures** are files we generate in `tools/fixtures`; edit the generator, never the files.
  Never commit third-party design files: they go into the git-ignored `fixtures/external/`.
- **Figma, Sketch and Penpot are the judges**: a generated fixture counts as valid once it has
  been imported and its prototype played in the real tool.
- Decisions go into an ADR in `docs/adr/` (`NNNN-short-title.md`, numbered after the design doc's
  D-ids).
