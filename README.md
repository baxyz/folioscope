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

> [!WARNING]
> **Status: early development.** `.fig` prototypes bake and play, with click, timed, hover and
> press links, overlays, scrolling and interactive components; `.sketch` prototypes too, with
> their links and slides. `.xd` and `.penpot` follow. See the [project plan](PROJECT.md) and its
> roadmap.

## Usage

```html
<script type="module" src="https://cdn.jsdelivr.net/npm/folioscope"></script>

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

| Registry  | Name                                                     | Contents                                                    |
| --------- | -------------------------------------------------------- | ----------------------------------------------------------- |
| npm       | [`folioscope`](https://www.npmjs.com/package/folioscope) | `<folioscope-player>`, `folioscope/full`, `folioscope/vite` |
| crates.io | `folioscope-core`, `folioscope-cli`                      | Importers, IR, the `folioscope` CLI                         |

## Documentation

- Website: <https://folioscope.baxyz.dev> (coming with v0.1)
- [Project plan](PROJECT.md) — architecture, `.folio` format, fonts, roadmap
- [Architecture decision records](docs/adr/)

## License

[AGPL-3.0-or-later](LICENSE). The design files you play with Folioscope remain yours; the license
covers the player, not your mockups.

## Trademarks

Folioscope is an independent project. It is not affiliated with, endorsed by or sponsored by Figma,
Sketch, Adobe or Penpot. Their names are used only to describe file compatibility.
