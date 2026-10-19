# ADR 0005 — Package manager: pnpm

- **Status:** Accepted
- **Date:** 2026-10-19
- **Design reference:** D5, [design.md § 6.2](../design.md#62-package-manager-pnpm)

## Context

The repository is a monorepo: one npm package (`packages/folioscope`) today, then the VitePress
site and the playground (`apps/*`). The JS toolchain is Vite+ (`vp`), which does not replace the
package manager: `vp install` delegates to the one the project declares.

Options considered:

| Option | For | Against |
| --- | --- | --- |
| **pnpm** | Workspaces and catalogs, strict `node_modules` (no phantom dependencies), shared content-addressed store already mounted in the devcontainer, used by every other baxyz repo, first-class in Vite+ templates | — |
| npm | No setup | Weaker workspaces, no catalogs, hoisting hides missing dependencies |
| Bun | Fast installs | Second runtime to track; its lockfile is not read by the rest of the chain |
| nub | Reads pnpm lockfiles, fast script runner | Young project, maturity not established; overlaps Vite+ (Node management, script running) |

## Decision

Use **pnpm**, pinned through `devEngines.packageManager` in the root `package.json` (with
`onFail: download`, so a mismatched local pnpm fetches the pinned one). Shared versions
(`typescript`, `vite-plus`, `@types/node`) live in the `catalog:` of `pnpm-workspace.yaml`.

nub stays allowed as a local script runner, since it reads the pnpm lockfile, but nothing in the
repository or CI depends on it.

## Consequences

- `pnpm-lock.yaml` is the only lockfile; CI installs with `--frozen-lockfile`.
- Contributors run `vp install` (or `pnpm install`); Corepack is not required.
- Revisit nub once it has a stable release and a clear edge over `vp run`.
