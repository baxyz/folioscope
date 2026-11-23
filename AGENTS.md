# AGENTS.md — baxyz `folioscope`

This repository inherits the [canonical workspace rules](https://github.com/baxyz/.dev/blob/main/AGENTS.md). Only project-specific details are documented here.

## Scope

`<folioscope-player>`: a web component that plays the interactive prototype stored in a `.fig`,
`.sketch`, `.xd` or `.penpot` file, with no Figma, Sketch or Adobe account.

**Status:** the four formats bake and play; what each one supports, and what is left, is on the
site's [Roadmap](https://folioscope.baxyz.dev/roadmap).

## Documentation and feature status

- `docs/architecture.md`: the pipeline and the main choices, with their reasons.
- `docs/spec/`: what the code does and must keep doing: the `.folio` format (`folio.md`), the bake
  (`bake.md`), the player (`player.md`). Change a spec in the same PR as the behaviour it
  describes; a choice and its reasons go there, in a sentence or two, not into a separate record.
- `docs/format-*.md`: what we know of each design format.
- `docs/development.md`: stack, tests, fixtures, website, publishing, licence.
- **`apps/site/data/features.ts`** lists every feature, per format, as works, unverified (tested on
  our own files only), to do, or not in the format, and what each unverified or missing one needs.
  The site's Roadmap and Compatibility pages are generated from it, and
  `packages/folioscope/test/features.test.ts` checks that every feature said to work names a test
  that exists. Update it in the same PR as the feature.

## License

**AGPL-3.0-or-later** for everything (component, Rust core, CLI) but one folder:
`apps/site/public/examples/` holds a real design of the author's, all rights reserved (its
`LICENSE.txt`); never copy it into fixtures, tests or a package. Every source file starts with an
SPDX header:

```ts
// SPDX-License-Identifier: AGPL-3.0-or-later
```

Dependencies must be MIT, Apache-2.0, BSD or LGPL-3.0 (`cargo deny` enforces it on the Rust side).
Credit reference projects, and whatever of others is committed here, in `NOTICE` when first used.
The notices of the dependencies the npm package carries (crates compiled into the WebAssembly,
packages bundled in the player) are generated: `build:wasm` writes `THIRD-PARTY-LICENSES.md`
(`packages/folioscope/scripts/third-party.ts`). `LICENSE` is copied next to each published
manifest (`packages/folioscope/`, `crates/folioscope/`): registries only take files from there.

## Commit Scopes

Defined in `scopes.json`:

| Scope                                       | Covers                                                                   |
| ------------------------------------------- | ------------------------------------------------------------------------ |
| `core`                                      | `crates/folioscope/src`, but `bake/`: IR, importers, instance resolution |
| `bake`                                      | `crates/folioscope/src/bake`: rendering, fonts, the `.folio` writer      |
| `wasm`                                      | `crates/folioscope-wasm`: wasm-bindgen bindings                          |
| `cli`                                       | `crates/folioscope/src/main.rs`: `bake`, `inspect`, `fonts`, `render`    |
| `player`                                    | `packages/folioscope`: custom elements, runtime, lite and full players   |
| `vite`                                      | `folioscope/vite` plugin                                                 |
| `site`                                      | `apps/site`: VitePress site on folioscope.baxyz.dev                      |
| `playground`                                | `apps/playground`                                                        |
| `fixtures`                                  | `fixtures/`                                                              |
| `build`, `test`, `docs`, `deps`, `ci`, `ai` | As usual                                                                 |

## Structure

```text
folioscope/
  crates/
    folioscope/        ← the one published crate library and binary `folioscope`:
                         importers (figma, sketch, xd, penpot), format detection, IR v1, summary;
                         `bake/` (feature `bake`, on by default): rendering and the .folio writer;
                         `main.rs`: inspect, render, bake, fonts
    folioscope-wasm/   ← the bake for JavaScript (folioscope/vite, the full player); not published
  packages/
    folioscope/        ← npm package: <folioscope-player> (src/), folioscope/vite (src/vite/),
                         wasm/ built by `build:wasm`, git-ignored, published
  apps/site/           ← VitePress site on folioscope.baxyz.dev ; public/folio/ holds baked demos
  fixtures/            ← test files we generated, one folder per format;
                         external/ (git-ignored) holds real third-party files for local checks
  tools/fixtures/      ← the generators behind fixtures/
  docs/                ← architecture.md, spec/, format-*.md, development.md (see above)
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
- builds the WebAssembly bake and the player, dry-runs `npm publish`, `jsr publish` and `cargo publish`
  (the `folioscope` crate);
- pushes a signed release commit `chore(release): 🔖 vX.Y.Z` to `main` (by `helpers4-bot`, the
  bypass actor of the ruleset), tags it, publishes to npm, JSR and crates.io with trusted publishing (OIDC) and
  publishes the GitHub release.

What this means when you work here:

- **Never bump `version` or push a `v*` tag by hand.** In `CHANGELOG.md`, write only under
  `## [Unreleased]`: a human summary there becomes the top of the next version's section and of
  its release notes, above the list generated from the commits. So write Conventional Commits that
  read well there: `cliff.toml` lists the `feat`, `fix` and `perf` ones, without their gitmoji, and
  leaves out the scopes `fixtures`, `test`, `ci`, `ai` and `deps`.
- Tags v0.0.3 and v0.0.4 were made by the previous design, on release commits that are on no branch:
  `git fetch --tags` to see them. Later tags are on `main`.
- If a registry step fails after the tag exists, fix the cause and _Re-run failed jobs_ (or run
  **Release** with `bump: resume`). Never delete the tag.
- JSR (`@baxyz/folioscope`) publishes **the player's sources only**, from
  `packages/folioscope/jsr.json`: the lite player (`.`, `src/player.ts`), to play a baked
  `.folio`. Not the Vite plugin, the in-browser bake or the WebAssembly (`wasm/`, 6 MB): they stay
  in the npm package. Two reasons: JSR's maximum file size is documented as 4 MB on one page and 20 MB
  on another, and a JSR publish that fails after the tag cannot be fixed (a published version is
  never replaced), so the release stays on what a dry run proves; and the full player's worker
  (`worker.mjs`) only exists in `dist/`. To add an entry later, check the size limit first (a test
  publish, or quotas@jsr.io), add the file to `exports`, and the `build:wasm` step to the `jsr`
  target in `release.yml`.
- `.` points at `src/player.ts`, not `src/index.ts`: JSR rejects the `declare global` of the tag
  typing, which only the npm entry carries. `player.ts` also defines the element, so it is listed in
  `sideEffects` in `package.json`: without it the bundler drops the `customElements.define` and the
  player never registers. The version is written by the release like `package.json`'s; CI runs
  `jsr publish --dry-run` on every change.
- `release.yml` must keep its file name and the `npm` environment: the npm trusted publisher of the
  `folioscope` package checks both. Change them only together with the setting on npmjs.com.
- Every preflight message links to
  [the action's troubleshooting page](https://github.com/helpers4/action-release/blob/main/docs/troubleshooting.md).
- crates.io gets **one crate**, `folioscope` (the CLI and the library), with the version of
  the release (`- crates: .` in `release.yml` publishes the one publishable crate of the workspace);
  `folioscope-wasm` (`publish = false`) ships in the npm package only. The release writes the version
  in `[workspace.package]` and in the `version` of the path dependency of `[workspace.dependencies]`,
  and refreshes `Cargo.lock`. crates.io accepts a trusted publisher only for a crate that exists: the
  first version (`0.0.0`) was published by hand, then the crate's trusted publisher names this
  repository, `release.yml` and the `npm` environment.

The pnpm version is pinned once, in the `packageManager` field of `package.json`. The workflows
(`pnpm/action-setup` refuses a `version` that differs from it) and Cloudflare Workers Builds read
it: that build starts with pnpm 10, which cannot read this repository's lockfile, and `packageManager`
makes it switch to the pinned version by itself, so the Worker needs no `PNPM_VERSION` build
variable. `engines.pnpm` only says what the repository needs. Do not use `devEngines` instead: npm
refuses to run in a repository whose `devEngines` names another package manager, and the release
runs `npx`.

## Rules

- **helpers4 first.** Before writing a generic helper (geometry, binary reading, easing, FSM, DOM,
  glob…), look in `helpers4` (TS: one dependency, `helpers4`, imported by sub-path such as
  `helpers4/function`; Rust: crate `helpers4` with one feature per module). If it is missing and
  generic, write it here: helpers4 is used as it is (see `docs/development.md`).
- **No UI framework** in the component: vanilla custom elements, no Lit, no React.
- **Performance budgets are features**: the lite player stays under 15 KB of JS gzipped, with no
  WASM and no fonts.
- **Trademarks**: never use "Figma", "Sketch", "Adobe", "XD" or "Penpot" in a package, element or
  logo name. Say "compatible with `.fig`, `.sketch`, `.xd` and `.penpot` files".
- **Fixtures** are files we generate in `tools/fixtures`; edit the generator, never the files.
  Never commit third-party design files: they go into the git-ignored `fixtures/external/`.
- **The design tools are the judges**: a generated fixture counts as valid once it has been
  imported and its prototype played in the real tool. Until then, what it covers is "unverified"
  in `features.ts`, which says what file would validate it.
