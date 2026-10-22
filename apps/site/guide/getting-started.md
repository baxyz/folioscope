# Getting started

::: warning Not published yet
The `folioscope` npm package and CLI are not released yet. Until then, build them from the [repository](https://github.com/baxyz/folioscope).
:::

## 1. Bake your design file

Baking turns a design file into a `.folio` folder: a small `index.json` and one image per screen, at 1x and 2x. It runs once, ahead of time, so visitors never wait for a renderer.

```sh
folioscope bake app.fig public/app.folio
```

Fonts are not needed for `.fig` files: Figma stores the outline of every letter, and the bake draws them.

### Or let Vite bake them

With Vite, `folioscope/vite` bakes your design files while the site builds, and serves them in
dev. It runs the bake as WebAssembly, so nothing else needs installing, and caches each result by
the hash of its source file.

```ts
// vite.config.ts
import { folioscope } from "folioscope/vite";

export default {
  plugins: [folioscope({ include: "designs/*.fig" })],
};
```

`designs/app.fig` is then served as `/folio/app.folio/`. Options: `include` (globs, relative to
the Vite root), `base` (URL folder, `folio` by default), `cacheDir`, `avif` (`false` keeps every
screen in lossless WebP; by default, photo-heavy screens become AVIF when it is much lighter).

## 2. Add the player

```html
<script type="module" src="https://cdn.jsdelivr.net/npm/folioscope"></script>

<folioscope-player src="/app.folio" poster="/app.folio/poster.webp" controls></folioscope-player>
```

The poster shows as soon as the player's script runs, before the prototype's description
arrives.

### A first screen in under a second

To have the first screen up in under a second on a phone on 4G, give a `poster` and have the
browser fetch the prototype's `index.json` along with the player:

```html
<link rel="preload" href="/app.folio/index.json" as="fetch" crossorigin />
```

CI checks this setup on every change: on a simulated 4G connection with a phone's CPU, the poster
shows at about 550 ms and the first playable screen at about 750 ms (900 ms without the poster and
the preload).

## Attributes

| Attribute  | Values                              | Role                                                                                               |
| ---------- | ----------------------------------- | -------------------------------------------------------------------------------------------------- |
| `src`      | URL                                 | The `.folio` folder                                                                                |
| `poster`   | URL                                 | Image shown until the first screen is ready; without it, a blurred preview baked into the `.folio` |
| `preload`  | `neighbors`, `all`, `start`, `none` | Screens to fetch ahead: those the current one links to (default), the whole page once, or none     |
| `flow`     | name or index                       | Starting point; the page's first flow by default                                                   |
| `page`     | name or index                       | Page; the first page with a flow by default                                                        |
| `fit`      | `contain`, `none`                   | `contain` scales the prototype to the element's width                                              |
| `hotspots` | `on-miss`, `always`, `never`        | Shows the clickable areas: flashed after a click beside them (default), always, or never           |
| `controls` | boolean                             | Shows the control bar: restart, back, page and flow, fullscreen                                    |

## Methods and events

- `restart()`, `back()`, `goTo(frameId)`
- `folioscope:load`, `folioscope:navigate` (`detail.frame`, `detail.name`), `folioscope:error` (`detail.error`)

## Keyboard

Hotspots are buttons: <kbd>Tab</kbd> to reach them, <kbd>Enter</kbd> to follow them, <kbd>Escape</kbd> to go back.

## Styling

The element is a block that takes its width from the page and its height from the prototype's aspect ratio. The control bar is `::part(controls)`, coloured with `--folioscope-controls-background` and `--folioscope-controls-color`.
