# AGENTS.md — baxyz `folioscope`

This repository inherits the [canonical workspace rules](https://github.com/baxyz/.dev/blob/main/AGENTS.md). Only project-specific details are documented here.

## Scope

`<folioscope-player>`: a web component that plays the interactive prototype stored in a `.fig`,
`.sketch`, `.xd` or `.penpot` file, with no Figma, Sketch or Adobe account. The full design —
architecture, IR, `.folio` format, roadmap — is in [docs/design.md](docs/design.md). Read it before
any structural change.

**Status:** design phase, nothing implemented yet. Next up is Phase 0 (spikes), see
[docs/design.md § 8](docs/design.md#8-roadmap).

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
    folioscope-core/   ← IR + importers (Rust lib, native and WASM)
    folioscope-cli/    ← `folioscope` binary
                         (folioscope-bake and folioscope-wasm join in Phases 1 and 4)
  packages/
    folioscope/        ← npm package, defines <folioscope-player>
  apps/                ← site and playground (Phase 1)
  fixtures/            ← test files we created ourselves, one folder per format
  docs/
    design.md          ← the design document (source of truth)
    adr/               ← decisions D1–D7
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
```

## Rules

- **helpers4 first.** Before writing a generic helper (geometry, binary reading, easing, FSM, DOM,
  glob…), look in `helpers4` (TS: one dependency, `helpers4`, imported by sub-path such as
  `helpers4/function`; Rust: crate `helpers4` with one feature per module). If it is missing and
  generic, propose it for helpers4 instead of coding it locally. See [docs/design.md § 6.4](docs/design.md#64-helpers4-libraries).
- **No UI framework** in the component: vanilla custom elements, no Lit, no React.
- **Performance budgets are features**: the lite player stays under 15 KB of JS gzipped, with no
  WASM and no fonts.
- **Trademarks**: never use "Figma", "Sketch", "Adobe", "XD" or "Penpot" in a package, element or
  logo name. Say "compatible with `.fig`, `.sketch`, `.xd` and `.penpot` files".
- **Fixtures** are files we made ourselves. Never commit third-party design files.
- Decisions go into an ADR in `docs/adr/` (`NNNN-short-title.md`, numbered after the design doc's
  D-ids).
