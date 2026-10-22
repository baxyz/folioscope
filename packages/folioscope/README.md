# folioscope

**Flip through your prototypes.** `<folioscope-player>` plays the interactive prototype of a
`.fig` file in the browser, with no account and no design tool: screens, clicks and timed links,
transitions. `.sketch`, `.xd` and `.penpot` files follow. Not a viewer for the design file: a player
for its clickable prototype.

> Early development: see the [roadmap](https://github.com/baxyz/folioscope/blob/main/PROJECT.md#8-roadmap).

## Bake at build time, with Vite

```ts
// vite.config.ts
import { folioscope } from "folioscope/vite";

export default { plugins: [folioscope({ include: "designs/*.fig" })] };
```

`designs/app.fig` is served as `/folio/app.folio/`: layered images (lossless WebP, or AVIF for
photos), a description of the screens and links, and a poster. The bake runs as WebAssembly, so
nothing else is installed, and is cached between builds.

## Play

```html
<script type="module" src="https://cdn.jsdelivr.net/npm/folioscope"></script>
<link rel="preload" href="/folio/app.folio/index.json" as="fetch" crossorigin />

<folioscope-player src="/folio/app.folio" poster="/folio/app.folio/poster.webp" controls>
</folioscope-player>
```

Or `import "folioscope";` from a bundled app. The player is a vanilla custom element, under 15 KB
of JS gzipped, with no WebAssembly and no fonts.

Attributes: `src`, `poster`, `preload`, `flow`, `page`, `fit`, `hotspots`, `controls`. Methods: `restart()`,
`back()`, `goTo(frameId)`. Events: `folioscope:load`, `folioscope:navigate`, `folioscope:error`.
Details on [folioscope.baxyz.dev](https://folioscope.baxyz.dev).

## License

[AGPL-3.0-or-later](https://github.com/baxyz/folioscope/blob/main/LICENSE). The design files you
play remain yours; the license covers the player, not your mockups.

Folioscope is an independent project, not affiliated with, endorsed by or sponsored by Figma,
Sketch, Adobe or Penpot. Their names are used only to describe file compatibility.
