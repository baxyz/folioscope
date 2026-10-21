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

## 2. Add the player

```html
<script type="module" src="https://cdn.jsdelivr.net/npm/folioscope"></script>

<folioscope-player src="/app.folio" poster="/app.folio/poster.png" controls></folioscope-player>
```

The poster shows straight from the HTML, before any JavaScript runs.

## Attributes

| Attribute  | Values            | Role                                                            |
| ---------- | ----------------- | --------------------------------------------------------------- |
| `src`      | URL               | The `.folio` folder                                             |
| `poster`   | URL               | Image shown until the first screen is ready                     |
| `flow`     | name or index     | Starting point; the page's first flow by default                |
| `page`     | name or index     | Page; the first page with a flow by default                     |
| `fit`      | `contain`, `none` | `contain` scales the prototype to the element's width           |
| `controls` | boolean           | Shows the control bar: restart, back, page and flow, fullscreen |

## Methods and events

- `restart()`, `back()`, `goTo(frameId)`
- `folioscope:load`, `folioscope:navigate` (`detail.frame`, `detail.name`), `folioscope:error` (`detail.error`)

## Keyboard

Hotspots are buttons: <kbd>Tab</kbd> to reach them, <kbd>Enter</kbd> to follow them, <kbd>Escape</kbd> to go back.

## Styling

The element is a block that takes its width from the page and its height from the prototype's aspect ratio. The control bar is `::part(controls)`, coloured with `--folioscope-controls-background` and `--folioscope-controls-color`.
