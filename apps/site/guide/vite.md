---
title: Bake with Vite
description: "The folioscope/vite plugin bakes design files while the site builds, and serves them in dev."
---

# Bake with Vite

Baking turns a design file into a [`.folio`](./folio) folder, the fast and private way to publish
a prototype.

`folioscope/vite` bakes the design files of your project while Vite builds the site, and on the
fly in `vite dev`. It runs the bake as WebAssembly: nothing to install besides the npm package.

```sh
npm install folioscope
```

```ts
// vite.config.ts
import { defineConfig } from "vite";
import { folioscope } from "folioscope/vite";

export default defineConfig({
  plugins: [folioscope({ include: "designs/*.{fig,penpot}" })],
});
```

`designs/app.fig` becomes `/folio/app.folio/` in the built site, and is served at the same URL by
the dev server:

```html
<folioscope-player src="/folio/app.folio" poster="/folio/app.folio/poster.webp"></folioscope-player>
```

## Options

| Option      | Default                                                          | What it does                                                                                      |
| ----------- | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `include`   | every `.fig`, `.sketch`, `.xd`, `.penpot` outside `node_modules` | Globs of the design files, relative to the Vite root                                              |
| `base`      | `"folio"`                                                        | The URL folder the `.folio` folders are served under                                              |
| `cacheDir`  | `node_modules/.vite/folioscope`                                  | Where bakes are kept, by the hash of their file: an unchanged file is not baked again             |
| `fonts`     | none                                                             | Font files, folders or globs, for text the files store no glyphs for (see [Fonts](./fonts))       |
| `avif`      | `true`                                                           | Photo-heavy screens become AVIF when that is much lighter; `false` keeps lossless WebP everywhere |
| `svg`       | `true`                                                           | A flat layer (no photo, blur or shadow) is served as SVG when lighter: sharp at any zoom          |
| `maxStates` | `16`                                                             | The most states drawn per interactive component; the build warns about the rest                   |

```ts
folioscope({
  include: ["designs/**/*.fig", "mockups/*.sketch"],
  base: "prototypes",
  fonts: [
    "fonts", // a folder: every font file in it
    { src: "fonts/Inter.ttf", match: ["SF Pro*", "Helvetica"] }, // stands in for these
  ],
});
```

A font given as an object says what its file does not: `family` (another name it answers to),
`match` (names it stands in for), `weight`, `italic`.

## What the build does

- Each design file is baked once per content: the result is cached in `cacheDir`, so a build with
  no changed design file bakes nothing.
- The bake reports what it could not do as build warnings: missing fonts, frames too large for an
  image, pages without a flow, components with more states than `maxStates`.
- The `.folio` folders are emitted as static assets, served with your site: no server code.
