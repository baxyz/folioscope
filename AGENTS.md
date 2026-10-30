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
| `cli`                                       | `crates/folioscope-cli` (crate `folioscope`): `bake`, `inspect`, `fonts`  |
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
    folioscope-cli/    ← crate and binary `folioscope`: inspect, render, bake, fonts
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
cargo run -p folioscope -- bake <file.fig> <out.folio> [--fonts <file or folder>]
cargo run -p folioscope -- fonts <folder>        # writes the folioscope-fonts.json manifest
cargo run -p folioscope -- inspect <file>        # pages, frames, flows and links of any format
cargo run -p folioscope -- render <file.fig> <node-id> <out.png> [scale]
```

## Releasing

Releases go through [`helpers4/action-release`](https://github.com/helpers4/action-release): run the
**Release** workflow from the Actions tab (`bump`: `auto`, `patch`, `minor`, `major`, `alpha`,
`beta`, `rc`, or `resume`; tick `dry-run` to build and dry-run without writing anything).
`release.yml` runs the checks of `ci.yml`, then the action (its inputs list the package and its build):

- computes the version from the last `v*` tag and the Conventional Commits (`auto`; before 1.0.0 a
  `feat` is a patch and a breaking change a minor);
- writes it in `packages/folioscope/package.json` and adds the section of the version to
  `CHANGELOG.md`;
- builds the WebAssembly bake and the player, dry-runs `npm publish` and `jsr publish`;
- pushes a signed release commit `chore(release): 🔖 vX.Y.Z` to `main` (by `helpers4-bot`, the
  bypass actor of the ruleset), tags it, publishes to npm and JSR with trusted publishing (OIDC) and
  publishes the GitHub release.

What this means when you work here:

- **Never bump `version` or push a `v*` tag by hand.** In `CHANGELOG.md`, write only under
  `## [Unreleased]`: a human summary there becomes the top of the next version's section and of
  its release notes, above the list generated from the commits. So write Conventional Commits that
  read well there.
- Tags v0.0.3 and v0.0.4 were made by the previous design, on release commits that are on no branch:
  `git fetch --tags` to see them. Later tags are on `main`.
- If a registry step fails after the tag exists, fix the cause and _Re-run failed jobs_ (or run
  **Release** with `bump: resume`). Never delete the tag.
- JSR (`@baxyz/folioscope`) publishes the sources from `packages/folioscope/jsr.json` (its `exports`
  point at `src/player.ts`, not `src/index.ts`: JSR rejects the `declare global` of the tag typing,
  which only the npm entry carries). Its version is written by the release like `package.json`'s.
- `release.yml` must keep its file name and the `npm` environment: the npm trusted publisher of the
  `folioscope` package checks both. Change them only together with the setting on npmjs.com.
- Every preflight message links to
  [the action's troubleshooting page](https://github.com/helpers4/action-release/blob/main/docs/troubleshooting.md).
- The Rust crates are not published by this workflow; only the npm package is released, and the
  workspace crates keep version `0.0.0`.

The pnpm version is pinned in three places, to change together: `devEngines` in `package.json`,
the `pnpm/action-setup` steps of `ci.yml`, and the `PNPM_VERSION` build variable of the
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
