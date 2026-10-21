# ADR 0010 — Website: VitePress on Cloudflare Workers

- **Status:** Accepted
- **Date:** 2026-10-21

## Context

The plan put the site on GitHub Pages, built with VitePress. Two things changed: the owner's other
sites (`berenger.arnaud.work`, `baxyz.dev`) are served by Cloudflare Workers, built by Workers
Builds from their repositories; and the toolchain is Vite+ on Vite 8, which VitePress 1 does not
support.

## Decision

- **Hosting: a Cloudflare Worker with static assets**, `site-dev-baxyz-folioscope`, built by
  Workers Builds from this repository (`wrangler.jsonc` at the root): production on
  `folioscope.baxyz.dev`, a preview per branch. Unlike GitHub Pages, a Worker can set HTTP headers,
  which the full player's WASM threads will need (COOP and COEP, Phase 4).
- **Generator: VitePress 2** (alpha), in `apps/site/`. VitePress 1.6 builds under Vite 8 only with
  esbuild added, and still loses its page chunks (`*.lean.js`) because Rolldown ignores how it
  emits them; VitePress 2 targets Vite 7+ and builds cleanly.
- **Demos are baked from the fixtures and committed** (`apps/site/public/folio/`, `pnpm --filter
  @folioscope/site bake-demos`). The Workers Builds image has Node but no Rust, so the site build
  cannot run the CLI. CI re-bakes them and fails when they are out of date. Once the `folioscope`
  package ships its WASM bake (ADR 0009), the site switches to `folioscope/vite`.

## Consequences

- Workers Builds must use pnpm: build `pnpm run build`, deploy `pnpm exec wrangler deploy`,
  previews `pnpm exec wrangler preview`; `npm` refuses to run in this repository, which pins pnpm.
- Move to VitePress 2 stable when it ships.
