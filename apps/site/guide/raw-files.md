---
title: Play a design file
description: "Give the player a .fig, .sketch, .xd or .penpot file: it bakes it in the visitor's browser. How it works, and what it costs."
---

# Play a design file

The player plays a design file as it is, with no build step: give its URL to `src`.

```html
<folioscope-player src="/designs/app.fig" controls></folioscope-player>
```

1. If a baked `app.folio/` sits next to it, the player plays that: bake ahead when you can, and
   keep the same markup.
2. Otherwise the **full player** bakes the file in the browser: it loads the WebAssembly bake
   (about 2 MB, downloaded once and cached by the browser), bakes in a Web Worker so the page
   stays responsive, and plays the result.
3. The result is kept in the browser's cache: the next visit plays at once, with no WebAssembly.

`folioscope:progress` tells where it is (`download`, `fonts`, `bake`, or `cache` on a second
visit):

```js
player.addEventListener("folioscope:progress", (event) => {
  status.textContent =
    { download: "Downloading…", fonts: "Fonts…", bake: "Preparing…" }[event.detail.stage] ?? "";
});
```

## When to use it

- Files you cannot bake ahead: a visitor's own file (the [Playground](/playground) does this with
  a `blob:` URL), a CMS upload, a design that changes every day.
- Trying Folioscope on a file before setting up a build.

Otherwise bake ahead: visitors then download a few hundred kilobytes instead of 2 MB, and the
first screen shows in under a second.

## Fonts

A `.fig` file stores its glyphs: it needs no font. `.sketch`, `.xd` and `.penpot` files name
their fonts: give the player the files, or their text is left out.

```html
<folioscope-player src="/designs/app.sketch">
  <folioscope-font src="/fonts/Inter.woff2"></folioscope-font>
</folioscope-player>
```

Every way of giving fonts is in [Fonts](./fonts#in-the-browser). Only the fonts the file's text
uses are downloaded.

## From a CDN, from npm

Both work. Loaded from a CDN, the player starts its bake worker through a small module of your
page's own (browsers only run workers from the page's origin) and fetches the WebAssembly from the
CDN. Bundled from npm, your bundler emits the worker and the WebAssembly with your site.
