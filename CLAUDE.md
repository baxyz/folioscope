# Claude Code — baxyz `folioscope`

Full canonical rules (commit format, restrictions, license) live in the workspace's
[AGENTS.md](https://github.com/baxyz/.dev/blob/main/AGENTS.md); project-specific deltas — scopes,
commands, the helpers4-first rule, SPDX headers — are in [AGENTS.md](AGENTS.md) at this repo's root.

The project plan is [PROJECT.md](PROJECT.md), a temporary document. Work one roadmap phase per
session, using its § 8 as the task list, and delete from `PROJECT.md` whatever each PR completes or
moves to a permanent home (see "The project plan" in [AGENTS.md](AGENTS.md)).

Releases: never bump `version`, edit a released changelog section or push a `v*` tag by hand. The **Release**
workflow (`helpers4/action-release`) computes the version, pushes the release commit to `main`,
tags it, and publishes; see "Releasing" in [AGENTS.md](AGENTS.md). Running it with `dry-run: false`
publishes to npm for real and a version can never be reused: do it only when the owner asks.
