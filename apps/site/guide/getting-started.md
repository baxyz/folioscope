---
title: Getting started
description: "Put a prototype on your page in two steps: add the player, point it at your design file. Then bake it for production."
---

# Getting started

Folioscope plays the clickable prototype of a design file on your web page: its screens, and the
links the designer drew between them. It reads `.fig` (Figma), `.sketch`, `.xd` and `.penpot`
files. Your visitors need no account and no design tool.

## 1. Add the player

The player is a web component, `<folioscope-player>`. Load it from a CDN:

```html
<script type="module" src="https://cdn.jsdelivr.net/npm/folioscope@1/dist/index.mjs"></script>
```

Or install it with npm, if your site has a build step:

```sh
npm install folioscope
```

```ts
import "folioscope"; // defines <folioscope-player>
```

## 2. Point it at your design file

Put your design file with your site's files (`public/designs/app.fig` here), and add the tag
where the prototype goes:

```html
<folioscope-player src="/designs/app.fig" controls></folioscope-player>
```

That's it: open the page, the first screen of the first flow shows, and its links work.
`controls` adds a bar to restart, go back, switch flows and go fullscreen.

What happens: the player reads the file in the visitor's browser and turns it into screens it can
play (it **bakes** it). The first visit downloads the bake engine (about 2 MB) and waits a few
seconds; the next visits are instant, from the browser's cache.

::: tip Not sure your file has a prototype?
It needs at least one flow (a starting screen) and some links. See
[Your design file](./design-files), or drop it in the [Playground](/playground) to try it.
:::

### `.sketch`, `.xd` and `.penpot` files need their fonts

A `.fig` stores the shape of every letter. The other formats only name their fonts: give the
player the font files, or the text is left out.

```html
<folioscope-player src="/designs/app.sketch" controls>
  <folioscope-font src="/fonts/Inter.woff2"></folioscope-font>
</folioscope-player>
```

All the ways to give fonts: [Fonts](./fonts).

## 3. For production: bake it ahead

Playing the design file directly is the quickest way to start. For a public site, **bake it
ahead** into a [`.folio`](./folio) folder and publish that instead:

- **faster**: the first screen shows in under a second on a phone, with no bake engine to download;
- **lighter**: visitors download only the screens they see;
- **private**: your design file is not published. Its layers, hidden pages and comments stay
  with you; pictures of the prototype's screens go online.

```html
<folioscope-player src="/app.folio" poster="/app.folio/poster.webp" controls></folioscope-player>
```

Bake it while your site builds with [Vite](./vite), or with the [command line](./cli):

```sh
folioscope bake app.fig public/app.folio
```

Keep the same tag if you like: when `app.folio/` sits next to `app.fig`, the player plays the
`.folio`.

## Next

- [The `.folio` format](./folio): what a baked prototype is.
- [Embed the player](./player): size, start screen, events, frameworks, styling.
- [Demos](/demos): what plays, from each format.
