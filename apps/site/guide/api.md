---
title: API reference
description: Every attribute, property, method, event and export of the folioscope package.
---

# API reference

What the `folioscope` package offers, element by element. The types are those of
`folioscope` (`dist/index.d.mts`); a test checks that this page names every attribute, method,
event and export of the code.

## `<folioscope-player>`

```html
<folioscope-player src="/app.folio" poster="/app.folio/poster.webp" controls></folioscope-player>
```

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

Children of a player, or of a `<folioscope-fonts id="…">` shared by `fonts="…"`. Close them
explicitly: `/>` does not close an element in HTML.

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

From `folioscope/full`: `bakeInBrowser(url, { density, fonts, onProgress })`, which bakes a raw
design file into the files of a `.folio` folder, in a worker; the player calls it itself.

From `folioscope/vite`: `folioscope(options)`, the Vite plugin (see
[Getting started](/guide/getting-started)): `include`, `base`, `cacheDir`, `avif`, `svg`,
`maxStates`, `fonts`.

## Command line

```bash
folioscope bake <file> <out.folio> [--config <file>] [--no-avif] [--no-svg] [--max-states <n>]
    [--all-pages] [--fonts <path> [--family <name>] [--match <pattern>]... [--weight <n>] [--italic]]...
folioscope inspect <file>
folioscope render <file.fig> <node> <out.png> [scale]
folioscope fonts <folder>
```

`folioscope.config.json` gives `bake` the options of the Vite plugin, as JSON.
