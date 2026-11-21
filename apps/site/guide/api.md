---
title: API reference
description: "Every attribute, property, method, event and export of the folioscope package."
---

# API reference

What the `folioscope` package offers, element by element. The types are those of
`folioscope` (`dist/index.d.mts`); a test checks that this page names every attribute, method,
event and export of the code.

## `<folioscope-player>`

### Examples

::: code-group

```html [Baked]
<!-- A baked .folio folder, its first screen as an image until the player is ready -->
<folioscope-player src="/app.folio" poster="/app.folio/poster.webp" controls></folioscope-player>
```

```html [Fast start]
<!-- The description fetched with the page; the whole page's screens fetched ahead -->
<link rel="preload" href="/app.folio/index.json" as="fetch" crossorigin />
<folioscope-player
  src="/app.folio"
  poster="/app.folio/poster.webp"
  preload="all"
></folioscope-player>
```

```html [A given flow]
<!-- Starts on the "Checkout" flow of the "Mobile" page; links always outlined -->
<folioscope-player
  src="/app.folio"
  page="Mobile"
  flow="Checkout"
  hotspots="always"
></folioscope-player>
```

```html [Design size]
<!-- Pixel for pixel, no scaling: the element takes the design's width -->
<folioscope-player src="/app.folio" fit="none" style="width: 390px"></folioscope-player>
```

```html [Raw file]
<!-- Baked in the browser by the full player; a .fig needs no font -->
<folioscope-player src="/designs/app.fig" controls></folioscope-player>
```

```html [Raw file, fonts]
<!-- A .sketch names its fonts: give them; only those its text uses are fetched -->
<folioscope-player src="/designs/app.sketch">
  <folioscope-font src="/fonts/Inter.var.woff2"></folioscope-font>
  <folioscope-font src="/fonts/Inter.var.woff2" match="SF Pro*, Helvetica*"></folioscope-font>
</folioscope-player>
```

:::

### Attributes

| Attribute  | Values                                        | What it does                                                                                                   |
| ---------- | --------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `src`      | URL                                           | A `.folio` folder (or its `index.json`), a `.fig`, `.sketch`, `.xd` or `.penpot` file, or a `blob:` URL of one |
| `poster`   | URL                                           | An image shown until the first frame is decoded, usually `app.folio/poster.webp`                               |
| `flow`     | name or index                                 | The flow to start from; the first one by default                                                               |
| `page`     | name or index                                 | The page; the first one with a flow by default                                                                 |
| `controls` | boolean                                       | Shows the control bar: restart, back, page and flow pickers, fullscreen                                        |
| `fit`      | `contain` (default), `none`                   | Scales the prototype to the element's width; in fullscreen, to the screen                                      |
| `hotspots` | `on-miss` (default), `always`, `never`        | Shows the clickable areas: after a click beside them, always, or never                                         |
| `preload`  | `neighbors` (default), `all`, `start`, `none` | Which frames are fetched ahead: those one interaction away, the whole page, or none                            |
| `fonts`    | id                                            | A `<folioscope-fonts id="…">` whose fonts the player uses too                                                  |

### Properties

| Property | Type                             | What it is                                                         |
| -------- | -------------------------------- | ------------------------------------------------------------------ |
| `src`    | `string`                         | The `src` attribute                                                |
| `frame`  | `FolioFrame \| undefined`        | The frame on screen                                                |
| `fonts`  | `FolioscopeFontSet \| undefined` | A shared font set given in JavaScript; a string sets the attribute |

### Methods

| Method                                            | What it does                                                                |
| ------------------------------------------------- | --------------------------------------------------------------------------- |
| `restart()`                                       | Back to the flow's first frame, history cleared                             |
| `back()`                                          | Back to the previous frame, or closes the top overlay                       |
| `goTo(frameId)`                                   | Shows a frame of the page, by id                                            |
| `registerFont(font)`, `registerFont(family, src)` | Gives a font for a raw file's text (see `FontRegistration`); bakes it again |

### Events

Every event bubbles and crosses the shadow root (`composed`).

| Event                     | `detail`          | When                                             |
| ------------------------- | ----------------- | ------------------------------------------------ |
| `folioscope:load`         | `{ pages }`       | The prototype is loaded                          |
| `folioscope:navigate`     | `{ frame, name }` | A frame shows (on top: an overlay's frame)       |
| `folioscope:progress`     | `{ stage }`       | A raw file: `download`, `fonts`, `cache`, `bake` |
| `folioscope:font-missing` | `{ font }`        | A raw file's text needed a font it was not given |
| `folioscope:error`        | `{ error }`       | The prototype could not be loaded                |

### Styling

CSS parts: `viewport`, `controls`. Custom properties: `--folioscope-controls-background`,
`--folioscope-controls-color`.

## `<folioscope-font>` and `<folioscope-fonts>`

Fonts for raw design files baked in the browser (a `.folio` folder needs none). Children of a
player, or of a `<folioscope-fonts id="…">` shared by `fonts="…"`. Close them explicitly: `/>`
does not close an element in HTML. When to use which: [Fonts](/guide/fonts).

### Examples

::: code-group

```html [One file]
<!-- The file says its family: it serves the text set in that family -->
<folioscope-font src="/fonts/Inter.var.woff2"></folioscope-font>
```

```html [A family]
<!-- Fetched only if the design uses "Brand", for its bold italic -->
<folioscope-font
  src="/fonts/Brand-BoldItalic.woff2"
  family="Brand"
  weight="700"
  style="italic"
></folioscope-font>
```

```html [Stand-ins]
<!-- A free font for fonts you may not serve: globs, or one regular expression -->
<folioscope-font src="/fonts/Inter.var.woff2" match="SF Pro*, -apple-system"></folioscope-font>
<folioscope-font src="/fonts/Arimo.var.woff2" match="/^Helvetica( Neue)?$/i"></folioscope-font>
```

```html [A folder]
<!-- folioscope-fonts.json, written by `folioscope fonts public/fonts` -->
<folioscope-font base="/fonts/"></folioscope-font>
```

```html [A stylesheet]
<!-- The @font-face rules of a stylesheet -->
<folioscope-font css="/styles/fonts.css"></folioscope-font>
```

```html [Shared]
<folioscope-fonts id="brand">
  <folioscope-font base="/fonts/"></folioscope-font>
</folioscope-fonts>
<folioscope-player src="/designs/home.sketch" fonts="brand"></folioscope-player>
<folioscope-player src="/designs/cart.sketch" fonts="brand"></folioscope-player>
```

:::

### Attributes

| Attribute | What it says                                                                         |
| --------- | ------------------------------------------------------------------------------------ |
| `src`     | The font file (TTF, OTF, TTC, WOFF, WOFF2)                                           |
| `family`  | A family the font also answers to                                                    |
| `match`   | Families it stands in for: globs (`SF Pro*`), comma-separated, or one `/regex/flags` |
| `weight`  | Its weight, instead of the file's                                                    |
| `style`   | `italic` or `oblique`, instead of the file's                                         |
| `base`    | A folder with the `folioscope-fonts.json` manifest `folioscope fonts` writes         |
| `css`     | A stylesheet whose `@font-face` rules give fonts                                     |

Only the fonts the design's text uses are fetched.

## JavaScript exports

From `folioscope`:

| Export                      | What it is                                                                                                                                              |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `FolioscopePlayer`          | The element's class; `TAG_NAME` is `"folioscope-player"`                                                                                                |
| `FolioscopeFontSet`         | Fonts shared by players: `add(font)` (as `registerFont()`), `declarations`, `watch(listener)`                                                           |
| `FontRegistration`          | What `registerFont()` and `add()` take: `src`, `data`, `family`, `match`, `weight`, `style`, `base`, `css`                                              |
| `Runtime`, `Step`           | The prototype's state machine, without the DOM: `frame`, `top`, `canGoBack`, `run()`, `goTo()`, `hold()`, `release()`, `close()`, `back()`, `restart()` |
| `FolioIndex`, `FolioFrame`… | The types of a `.folio` folder's `index.json` ([format](https://github.com/baxyz/folioscope/blob/main/docs/spec/folio.md))                              |

```js
import { FolioscopeFontSet } from "folioscope";

const player = document.querySelector("folioscope-player");
player.addEventListener("folioscope:load", (event) => console.log(event.detail.pages));
player.addEventListener("folioscope:navigate", (event) => console.log("on", event.detail.name));
player.goTo("1:2"); // a screen, by its id in index.json
player.restart(); // back to the flow's first screen

// Fonts shared by every player of the page, for raw files
const fonts = new FolioscopeFontSet()
  .add({ base: "/fonts/" })
  .add({ src: "/fonts/Inter.woff2", match: "SF Pro*" });
for (const p of document.querySelectorAll("folioscope-player")) p.fonts = fonts;
```

From `folioscope/full`: `bakeInBrowser(url, { density, fonts, onProgress })`, which bakes a raw
design file into the files of a `.folio` folder, in a worker; the player calls it itself.

From `folioscope/vite`: `folioscope(options)`, the Vite plugin (see
[Bake with Vite](/guide/vite)): `include`, `base`, `cacheDir`, `avif`, `svg`,
`maxStates`, `fonts`.

## Command line

```bash
folioscope bake <file> <out.folio> [--config <file>] [--no-avif] [--no-svg] [--max-states <n>]
    [--all-pages] [--fonts <path> [--family <name>] [--match <pattern>]... [--weight <n>] [--italic]]...
folioscope inspect <file>
folioscope render <file.fig> <node> <out.png> [scale] [--fonts <path>]...
folioscope fonts <folder>
```

`folioscope.config.json` gives `bake` the options of the Vite plugin, as JSON.
