# Folioscope

[![CI](https://github.com/baxyz/folioscope/actions/workflows/ci.yml/badge.svg)](https://github.com/baxyz/folioscope/actions/workflows/ci.yml)
[![License: AGPL-3.0-or-later](https://img.shields.io/badge/license-AGPL--3.0--or--later-blue.svg)](LICENSE)

**Flip through your prototypes.**

`<folioscope-player>` is a web component that plays the interactive prototype stored in a `.fig`,
`.sketch`, `.xd` or `.penpot` file — clicks, hovers, overlays, navigation between screens — right
in the browser, with no account and no Figma, Sketch or Adobe licence.

It is **not a viewer** for design files: no canvas to pan, no layers to inspect, no specs. It is a
**player**: visitors click through the screens the way the designer linked them.

> **Why "Folioscope"?**
> _Folioscope_ is the French word for a flip book: a stack of still pictures that come to life when
> you thumb through them. That is what this player does with your design files: it takes static
> screens from a `.fig`, `.sketch`, `.xd` or `.penpot` file and flips through them following the
> prototype links you designed, so the mockup moves again. And yes, the "folio" is a nod to
> _portfolio_, where it all started.

What plays for each format, and what is left, is on the
[Roadmap](https://folioscope.baxyz.dev/roadmap) and
[Compatibility](https://folioscope.baxyz.dev/compatibility) pages.

## Usage

```html
<script type="module" src="https://cdn.jsdelivr.net/npm/folioscope@1/dist/index.mjs"></script>

<folioscope-player src="/mockups/app.folio" poster="/mockups/app.folio/poster.webp" controls>
</folioscope-player>
```

- **Fast path:** pre-render your design file at build time (`folioscope bake app.fig
  public/app.folio`, or the `folioscope/vite` plugin). Visitors get layered images and DOM hotspots:
  no WASM, no fonts, under 15 KB of JS.
- **Zero-setup path:** point `src` straight at a `.fig`, `.sketch`, `.xd` or `.penpot` file. The full
  player decodes and renders it in the browser (WASM, loaded on demand). Slower, but no build step.

Everything stays on your site: no third-party service, no network call other than your own files.
Made for archiving and showcasing work — portfolios, case studies — in plain files that keep
playing whatever happens to the design tool that made them.

## Packages

| Registry  | Name                                                     | Contents                                                     |
| --------- | -------------------------------------------------------- | ------------------------------------------------------------ |
| npm       | [`folioscope`](https://www.npmjs.com/package/folioscope) | `<folioscope-player>`, `folioscope/full`, `folioscope/vite`  |
| crates.io | [`folioscope`](https://crates.io/crates/folioscope)      | The `folioscope` CLI, and the importers, IR and bake it uses |

## Documentation

- Website: <https://folioscope.baxyz.dev>: getting started, demos, compatibility, roadmap
- [Architecture](docs/architecture.md): the pipeline and its choices
- Specs: [the `.folio` format](docs/spec/folio.md), [the bake](docs/spec/bake.md),
  [the player](docs/spec/player.md)
- [Development](docs/development.md): stack, tests, fixtures, publishing

## Versions

From 1.0, Folioscope follows [semantic versioning](https://semver.org) for what you build on: the
`<folioscope-player>` element (attributes, methods, events), the `.folio` format (what a 1.x
release baked plays in every later 1.x player), the options of `folioscope/vite` and the
`folioscope` command line. The Rust library behind the command line is still settling: see
[its README](crates/folioscope/README.md).

## License

[AGPL-3.0-or-later](LICENSE). The design files you play with Folioscope remain yours; the license
covers the player, not your mockups.

One folder of this repository is not under that license: the example design the website plays
(`apps/site/public/examples/`) is its author's, all rights reserved; see
[its notice](apps/site/public/examples/LICENSE.txt).

## Trademarks

Folioscope is an independent project. It is not affiliated with, endorsed by or sponsored by Figma,
Sketch, Adobe or Penpot. Their names are used only to describe file compatibility.
