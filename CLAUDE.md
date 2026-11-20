# Claude Code — baxyz `folioscope`

Full canonical rules (commit format, restrictions, license) live in the workspace's
[AGENTS.md](https://github.com/baxyz/.dev/blob/main/AGENTS.md); project-specific deltas — scopes,
commands, the helpers4-first rule, SPDX headers — are in [AGENTS.md](AGENTS.md) at this repo's root.

What works and what is left lives in `apps/site/data/features.ts`, published as the site's
Roadmap and Compatibility pages: update it in the same PR as the feature (see "Feature status" in
[AGENTS.md](AGENTS.md)). The design is described in `docs/` (architecture, specs, format notes).

Releases: never bump `version`, edit a released changelog section or push a `v*` tag by hand. The **Release**
workflow (`helpers4/action-release`) computes the version, pushes the release commit to `main`,
tags it, and publishes; see "Releasing" in [AGENTS.md](AGENTS.md). Running it with `dry-run: false`
publishes to npm for real and a version can never be reused: do it only when the owner asks.
