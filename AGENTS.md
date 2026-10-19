# AGENTS.md — baxyz `folioscope`

This repository inherits the [canonical workspace rules](https://github.com/baxyz/.dev/blob/main/AGENTS.md). Only project-specific details are documented here.

## Scope

`<folioscope-player>`: a web component that plays the interactive prototype stored in a `.fig`,
`.sketch`, `.xd` or `.penpot` file, with no Figma, Sketch or Adobe account.

**Status:** design phase, nothing implemented yet. Next up is Phase 0 (spikes), see
[PROJECT.md § 8](PROJECT.md#8-roadmap).

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
    folioscope-core/   ← IR + importers (Rust lib, native and WASM)
    folioscope-cli/    ← `folioscope` binary
                         (folioscope-bake and folioscope-wasm join in Phases 1 and 4)
  packages/
    folioscope/        ← npm package, defines <folioscope-player>
  apps/                ← site and playground (Phase 1)
  fixtures/            ← test files we created ourselves, one folder per format
  docs/
    adr/               ← decisions D1–D7
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
```

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
- **Fixtures** are files we made ourselves. Never commit third-party design files.
- Decisions go into an ADR in `docs/adr/` (`NNNN-short-title.md`, numbered after the design doc's
  D-ids).
