---
title: The .folio format
description: "A .folio folder is a prototype ready to play, the way a PDF is a document ready to print: what it holds, and why bake one."
---

# The `.folio` format

A `.folio` folder is a prototype ready to play, the way a PDF is a document ready to print. It
comes from a design file, it no longer needs the tool that made it, and it plays the same
everywhere.

```text
app.folio/
├── index.json        the screens, flows and links, and where each picture goes
├── poster.webp       the first screen, as one picture
└── layers/
    ├── 3f9a…@1x.webp  a screen, or a part of one, for normal screens
    ├── 3f9a…@2x.webp  the same, for sharp screens
    └── 8c21….svg      a flat layer, sharp at any zoom
```

## Every prototype goes through it

Whatever the format, Folioscope first turns the design file into the same model: screens, the
layers that move, and links (click, hover, delay, key, drag) with their transitions. Then it draws
each screen into pictures. That model and those pictures are the `.folio`.

It is made in every case:

- **baked ahead** with the [CLI](./cli) or [Vite](./vite): written to a folder you publish;
- **baked in the browser**, when the player is given a design file: kept in the visitor's
  browser.

The player only ever plays a `.folio`. It does not know about Figma, Sketch, XD or Penpot.

## Why bake ahead

| What matters           | Design file in the page                          | Baked `.folio`                                   |
| ---------------------- | ------------------------------------------------ | ------------------------------------------------ |
| First visit            | about 2 MB of bake engine, a few seconds         | a few hundred KB, first screen in under a second |
| What visitors download | the whole design file, and its fonts             | the screens they see, as light images            |
| What is published      | your design file: layers, pages, comments        | pictures of the prototype's screens              |
| Fonts                  | served to visitors (`.sketch`, `.xd`, `.penpot`) | not needed: the text is drawn into the pictures  |

We recommend baking ahead for any public site. Playing the design file directly is for trying
Folioscope, for files you cannot bake (a visitor's upload), and for files that change every day.

## What it is good at

- **Light**: lossless WebP for interfaces, AVIF for photos, SVG for flat layers when lighter;
  only the screens one click away are fetched ahead.
- **Durable**: plain files and JSON, readable without Folioscope's code; no server, no account.
  A prototype baked today plays in ten years, whatever happened to the tool that made it.
- **Private**: no design file, no hidden page, no comment; only the names of the layers that
  are clickable or animated (the player labels its buttons with them).

## What it is not

A `.folio` is a picture of the design, not the design: there are no layers to inspect and nothing
to edit. To change the prototype, change the design file and bake it again.

The full format, field by field, is in the
[specification](https://github.com/baxyz/folioscope/blob/main/docs/spec/folio.md).
