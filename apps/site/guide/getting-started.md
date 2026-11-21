---
title: Getting started
description: "What Folioscope does, the three ways to use it, and a first prototype on a page."
---

# Getting started

Folioscope plays the clickable prototype of a design file on a web page: screens, clicks,
hovers, overlays, scrolling and transitions, from a `.fig` (Figma), `.sketch`, `.xd` or
`.penpot` file. No account, no design tool and no third-party server for your visitors.

It has two parts:

- **The bake** turns a design file into a `.folio` folder: one image per screen (or per layer)
  and a small `index.json` that says where the links go. It runs once, ahead of time.
- **The player**, `<folioscope-player>`, is a web component of about 11 KB (gzipped) that plays a
  `.folio` folder. Your visitors download only the screens they see.

## What you need

- A **design file** with a prototype in it: at least one flow (a starting screen) and some links.
  See [Your design file](./design-files) for how to save one from each tool.
- A **web page** to put it on: any site, static or not.

## Pick a way

| Your site                                            | Bake with                                 | Guide                           |
| ---------------------------------------------------- | ----------------------------------------- | ------------------------------- |
| Built with **Vite** (Vue, React, Svelte, Astro, …)   | the Vite plugin, while the site builds    | [Bake with Vite](./vite)        |
| Built another way, or plain HTML                     | the `folioscope` command, then copy files | [Bake with the CLI](./cli)      |
| No build at all, or files you cannot bake (uploads…) | nothing: the player bakes in the browser  | [Raw design files](./raw-files) |

Baking ahead of time is the fast path: the first screen shows in under a second on a phone on 4G.
Baking in the browser costs each visitor about 2 MB of WebAssembly and a few seconds, once.

## A first prototype, with the CLI

The command line tool is a Rust program, published on crates.io. With
[Rust](https://rustup.rs) installed:

```sh
cargo install folioscope
```

Bake your file into the folder your site publishes (`public/` here):

```sh
folioscope bake app.fig public/app.folio
```

It prints what it made (screens, layers, weight) and warns about what it could not do. Then put
the player on a page:

```html
<script type="module" src="https://cdn.jsdelivr.net/npm/folioscope@0.1/dist/index.mjs"></script>

<folioscope-player src="/app.folio" poster="/app.folio/poster.webp" controls></folioscope-player>
```

Open the page: the first screen of the first flow shows, and its links work. `controls` adds a bar
to restart, go back, pick another flow or page, and go fullscreen.

## A first prototype, with Vite

```sh
npm install folioscope
```

```ts
// vite.config.ts
import { defineConfig } from "vite";
import { folioscope } from "folioscope/vite";

export default defineConfig({
  plugins: [folioscope({ include: "designs/*.fig" })],
});
```

```ts
// main.ts: defines <folioscope-player>
import "folioscope";
```

```html
<folioscope-player src="/folio/app.folio" controls></folioscope-player>
```

`designs/app.fig` is baked while the site builds (and on the fly in `vite dev`) and served as
`/folio/app.folio/`. Nothing else to install: the plugin runs the bake as WebAssembly.

## Next

- [Embed the player](./player): size, poster, flows and pages, frameworks, events, styling.
- [Fonts](./fonts): when a design file needs them, and how to give them.
- [API reference](./api): every attribute, property, method, event and export.
- [Demos](/demos) to see what plays, and the [Playground](/playground) to try your own file.
