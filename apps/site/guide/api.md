---
title: API reference
description: "Every tag, attribute, property, method, event and export of the folioscope package, with examples."
outline: [2, 3]
---

# API reference

Everything the `folioscope` package offers. For how to use it step by step, see the
[guides](/guide/getting-started). A test checks that this page lists every attribute, method,
event and export of the code.

## The player

`<folioscope-player>` plays a prototype. Point `src` at a baked `.folio` folder, or at a design
file that the player bakes itself.

### Examples

::: code-group

```html [Design file]
<!-- The simplest: the design file itself, baked in the browser on the first visit -->
<folioscope-player src="/designs/app.fig" controls></folioscope-player>
```

```html [Baked .folio]
<!-- A baked folder: the first screen shows as an image until the player is ready -->
<folioscope-player src="/app.folio" poster="/app.folio/poster.webp" controls></folioscope-player>
```

```html [Fast start]
<!-- index.json is fetched with the page; every screen of the page is fetched ahead -->
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
<!-- Pixel for pixel, not scaled: give the element the design's width -->
<folioscope-player src="/app.folio" fit="none" style="width: 390px"></folioscope-player>
```

```html [With fonts]
<!-- A .sketch, .xd or .penpot file names its fonts: give the font files -->
<folioscope-player src="/designs/app.sketch">
  <folioscope-font src="/fonts/Inter.var.woff2"></folioscope-font>
</folioscope-player>
```

:::

### Attributes

| Attribute  | Values                                        | What it does                                                                                      |
| ---------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `src`      | URL                                           | What to play: a `.folio` folder, or a `.fig`, `.sketch`, `.xd` or `.penpot` file (or `blob:` URL) |
| `poster`   | URL                                           | An image to show until the first screen is ready, usually `app.folio/poster.webp`                 |
| `flow`     | name or index                                 | The flow to start with. Default: the page's first flow                                            |
| `page`     | name or index                                 | The page to start on. Default: the first page that has a flow                                     |
| `controls` | boolean                                       | Adds a bar: restart, back, page and flow pickers, fullscreen                                      |
| `fit`      | `contain` (default), `none`                   | `contain` scales the prototype to the element's width; `none` keeps the design size               |
| `hotspots` | `on-miss` (default), `always`, `never`        | When to outline what is clickable: after a click that misses, always, or never                    |
| `preload`  | `neighbors` (default), `all`, `start`, `none` | Which screens to fetch ahead: those one click away, all of the page, or none                      |
| `fonts`    | id                                            | The id of a `<folioscope-fonts>` set to use too                                                   |

### Properties

| Property | Type                             | What it is                                                      |
| -------- | -------------------------------- | --------------------------------------------------------------- |
| `src`    | `string`                         | Same as the `src` attribute                                     |
| `frame`  | `FolioFrame \| undefined`        | The screen shown now                                            |
| `fonts`  | `FolioscopeFontSet \| undefined` | A font set shared from JavaScript (a string sets the attribute) |

### Methods

| Method                                            | What it does                                                   |
| ------------------------------------------------- | -------------------------------------------------------------- |
| `restart()`                                       | Goes back to the flow's first screen and forgets the history   |
| `back()`                                          | Goes back one screen, or closes the overlay on top             |
| `goTo(frameId)`                                   | Shows a screen of the page, by its id                          |
| `registerFont(font)`, `registerFont(family, src)` | Adds a font for a design file's text, and bakes the file again |

### Events

All events bubble and cross the shadow root (`composed`), so you can listen on any ancestor.

| Event                     | `detail`          | Fired when                                                         |
| ------------------------- | ----------------- | ------------------------------------------------------------------ |
| `folioscope:load`         | `{ pages }`       | The prototype is loaded                                            |
| `folioscope:navigate`     | `{ frame, name }` | A screen shows (for an overlay: the overlay's screen)              |
| `folioscope:progress`     | `{ stage }`       | A design file is being baked: `download`, `fonts`, `cache`, `bake` |
| `folioscope:font-missing` | `{ font }`        | A design file's text needs a font that was not given               |
| `folioscope:error`        | `{ error }`       | The prototype could not be loaded                                  |

### Styling

Style the bar with the CSS parts `viewport` (the prototype) and `controls` (the bar), and the
custom properties `--folioscope-controls-background` and `--folioscope-controls-color`.

## Fonts

`<folioscope-font>` gives a font to a player that bakes a `.sketch`, `.xd` or `.penpot` file. A
`.fig` stores its letters and needs none; a baked `.folio` needs none either. Put the tag inside
the player, or inside a `<folioscope-fonts id="…">` set that several players share with
`fonts="…"`. Only the fonts the file's text uses are downloaded. More in [Fonts](/guide/fonts).

Close each tag with `</folioscope-font>`: `/>` does not close an element in HTML.

### Examples

::: code-group

```html [One font]
<!-- The font file says its family (Inter): used for the text set in Inter -->
<folioscope-player src="/designs/app.sketch">
  <folioscope-font src="/fonts/Inter.var.woff2"></folioscope-font>
</folioscope-player>
```

```html [A family]
<!-- Used for the bold italic of "Brand", if the design uses it -->
<folioscope-player src="/designs/app.sketch">
  <folioscope-font
    src="/fonts/Brand-BoldItalic.woff2"
    family="Brand"
    weight="700"
    style="italic"
  ></folioscope-font>
</folioscope-player>
```

```html [Stand-ins]
<!-- Free fonts in place of fonts you may not serve: globs, or one regular expression -->
<folioscope-player src="/designs/app.xd">
  <folioscope-font src="/fonts/Inter.var.woff2" match="SF Pro*, -apple-system"></folioscope-font>
  <folioscope-font src="/fonts/Arimo.var.woff2" match="/^Helvetica( Neue)?$/i"></folioscope-font>
</folioscope-player>
```

```html [A folder]
<!-- Every font of a folder, described by folioscope-fonts.json (`folioscope fonts public/fonts`) -->
<folioscope-player src="/designs/app.penpot">
  <folioscope-font base="/fonts/"></folioscope-font>
</folioscope-player>
```

```html [A stylesheet]
<!-- The fonts of your site's @font-face rules -->
<folioscope-player src="/designs/app.sketch">
  <folioscope-font css="/styles/fonts.css"></folioscope-font>
</folioscope-player>
```

```html [Shared]
<!-- One set of fonts for several players -->
<folioscope-fonts id="brand">
  <folioscope-font base="/fonts/"></folioscope-font>
</folioscope-fonts>
<folioscope-player src="/designs/home.sketch" fonts="brand"></folioscope-player>
<folioscope-player src="/designs/cart.sketch" fonts="brand"></folioscope-player>
```

:::

### Attributes

| Attribute | What it says                                                                            |
| --------- | --------------------------------------------------------------------------------------- |
| `src`     | The font file: TTF, OTF, TTC, WOFF or WOFF2                                             |
| `family`  | Another family name this font answers to                                                |
| `match`   | Families it replaces: globs (`SF Pro*`), separated by commas, or one `/regex/flags`     |
| `weight`  | Its weight (100 to 900), when the file does not say it                                  |
| `style`   | `italic` or `oblique`, when the file does not say it                                    |
| `base`    | A folder of fonts, described by `folioscope-fonts.json` (written by `folioscope fonts`) |
| `css`     | A stylesheet whose `@font-face` rules list fonts                                        |

## JavaScript

### Example

```js
import { FolioscopeFontSet } from "folioscope";

const player = document.querySelector("folioscope-player");
player.addEventListener("folioscope:load", (event) => console.log(event.detail.pages));
player.addEventListener("folioscope:navigate", (event) => console.log("on", event.detail.name));
player.goTo("1:2"); // a screen, by its id in index.json
player.restart(); // back to the flow's first screen

// One set of fonts for every player of the page
const fonts = new FolioscopeFontSet()
  .add({ base: "/fonts/" })
  .add({ src: "/fonts/Inter.woff2", match: "SF Pro*" });
for (const p of document.querySelectorAll("folioscope-player")) p.fonts = fonts;
```

### Exports

From `folioscope`:

| Export                      | What it is                                                                                                                                      |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `FolioscopePlayer`          | The element's class; its `TAG_NAME` is `"folioscope-player"`                                                                                    |
| `FolioscopeFontSet`         | A set of fonts shared by players: `add(font)` (takes what `registerFont()` takes), `declarations`, `watch(listener)`                            |
| `FontRegistration`          | A font, as `registerFont()` and `add()` take it: `src`, `data`, `family`, `match`, `weight`, `style`, `base`, `css`                             |
| `Runtime`, `Step`           | The prototype's logic without the page: `frame`, `top`, `canGoBack`, `run()`, `goTo()`, `hold()`, `release()`, `close()`, `back()`, `restart()` |
| `FolioIndex`, `FolioFrame`… | The types of a `.folio` folder's `index.json` ([the format](/guide/folio))                                                                      |

From `folioscope/full`: `bakeInBrowser(url, { density, fonts, onProgress })` bakes a design file
into the files of a `.folio` folder, in a worker. The player calls it for you.

From `folioscope/vite`: `folioscope(options)`, the Vite plugin, with the options `include`, `base`,
`cacheDir`, `avif`, `svg`, `maxStates` and `fonts` (see [Bake with Vite](/guide/vite)).

## Command line

```bash
folioscope bake <file> <out.folio> [--config <file>] [--no-avif] [--no-svg] [--max-states <n>]
    [--all-pages] [--fonts <path> [--family <name>] [--match <pattern>]... [--weight <n>] [--italic]]...
folioscope inspect <file>
folioscope render <file.fig> <node> <out.png> [scale] [--fonts <path>]...
folioscope fonts <folder>
```

| Command   | What it does                                                                   |
| --------- | ------------------------------------------------------------------------------ |
| `bake`    | Turns a design file into a `.folio` folder                                     |
| `inspect` | Lists the pages, screens, flows and links of a design file                     |
| `render`  | Draws one node of a `.fig` as a PNG (and its SVG)                              |
| `fonts`   | Writes `folioscope-fonts.json` for a folder of fonts, for the `base` attribute |

`folioscope.config.json` in the current folder gives `bake` the same options as the Vite plugin,
in JSON. Details in [Bake with the CLI](/guide/cli).
