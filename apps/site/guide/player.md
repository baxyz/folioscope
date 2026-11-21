---
title: Embed the player
description: "Load the player, size it, start it where you want, use it from a framework, listen to it and style it."
---

# Embed the player

## Load it

From a CDN, with no build step:

```html
<script type="module" src="https://cdn.jsdelivr.net/npm/folioscope@0.1/dist/index.mjs"></script>
```

Or from npm, in a bundled app:

```sh
npm install folioscope
```

```ts
import "folioscope"; // defines <folioscope-player>
```

The player is a plain custom element: no framework, no WebAssembly, no font. It works in every
current browser.

## Size and fit

The element is a block: it takes the width of its container and the height that keeps the
prototype's aspect ratio. Give it a width to make it smaller:

```html
<folioscope-player src="/app.folio" style="max-width: 390px; margin: auto"></folioscope-player>
```

- `fit="contain"` (the default) scales the prototype to the element's width. `fit="none"` keeps
  it at its design size, pixel for pixel: give the element that width, or it is cropped.
- In fullscreen (the control bar's button, or `player.requestFullscreen()`), it fits the screen.
- A screen taller than the prototype's device scrolls inside the player, its fixed layers staying
  in place, when the design says so.

## A fast first screen

```html
<link rel="preload" href="/app.folio/index.json" as="fetch" crossorigin />

<folioscope-player src="/app.folio" poster="/app.folio/poster.webp"></folioscope-player>
```

- `poster` shows the first screen as one image as soon as the player's script runs. Without it,
  the player shows a blurred preview stored in `index.json`.
- The `preload` link fetches the description with the page instead of after the script.
- `preload="neighbors"` (the default) fetches the screens one tap away; `all` fetches the whole
  page; `start` and `none` fetch nothing ahead.

CI checks this setup on every change: on a simulated 4G connection with a phone's processor, the
poster shows at about 550 ms and the first playable screen at about 750 ms.

## Where it starts

```html
<!-- The flow named "Checkout", on the page named "Mobile" -->
<folioscope-player src="/app.folio" page="Mobile" flow="Checkout"></folioscope-player>

<!-- The second flow of the first page with a flow -->
<folioscope-player src="/app.folio" flow="1"></folioscope-player>
```

`page` and `flow` take a name or an index (from 0). By default, the player starts on the first
flow of the first page that has one. With `controls`, visitors can switch.

## Clickable areas

Prototypes rarely show what is clickable. `hotspots` decides when the player outlines the links:

- `on-miss` (default): briefly, after a click beside them;
- `always`: all the time, for a guided demo;
- `never`.

Hotspots are buttons: <kbd>Tab</kbd> reaches them, <kbd>Enter</kbd> follows them, <kbd>Escape</kbd>
goes back or closes an overlay. With reduced motion asked by the system, transitions become cuts.

## Drive it from the page

```html
<folioscope-player id="demo" src="/app.folio"></folioscope-player>
<button onclick="demo.restart()">Start over</button>
<button onclick="demo.back()">Back</button>

<script type="module">
  const demo = document.querySelector("#demo");
  demo.addEventListener("folioscope:navigate", (event) => {
    console.log("now on", event.detail.name);
  });
  demo.addEventListener("folioscope:error", (event) => {
    console.error("could not play", event.detail.error);
  });
</script>
```

`goTo(frameId)` jumps to a screen by id (see `player.frame` and `index.json`). Every event and
method is listed in the [API reference](./api).

## In a framework

The player is a custom element: frameworks render it like any HTML tag. Tell yours that
`folioscope-*` tags are not its own components:

::: code-group

```ts [Vue (vite.config.ts)]
import vue from "@vitejs/plugin-vue";

export default {
  plugins: [
    vue({
      template: { compilerOptions: { isCustomElement: (tag) => tag.startsWith("folioscope-") } },
    }),
  ],
};
```

```tsx [React 19]
import "folioscope";

export function Prototype() {
  return <folioscope-player src="/folio/app.folio" controls="" />;
}
```

```svelte [Svelte]
<script>
  import "folioscope";
</script>

<folioscope-player src="/folio/app.folio" controls></folioscope-player>
```

```astro [Astro]
<folioscope-player src="/folio/app.folio" controls></folioscope-player>
<script>
  import "folioscope";
</script>
```

:::

The element only defines itself in the browser: with server rendering, import it where your
framework runs client code.

## Style it

```css
folioscope-player {
  border-radius: 24px;
  overflow: hidden;
  --folioscope-controls-background: #111827;
  --folioscope-controls-color: #f9fafb;
}
folioscope-player::part(controls) {
  font-size: 14px;
}
```

The parts are `viewport` (the prototype) and `controls` (the bar).
