---
title: Fonts
description: "When a design file needs fonts, and how to give them to the bake, the Vite plugin and the player."
---

# Fonts

## Does your file need any?

| Format    | Fonts needed?                                                                                                                            |
| --------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `.fig`    | No: Figma stores the outline of every letter it laid out. Only a text that changes style between variants of a component needs its font. |
| `.sketch` | Yes, unless the document embeds them (Sketch can)                                                                                        |
| `.penpot` | Yes (custom fonts uploaded to Penpot are not read from the file yet: give them too)                                                      |
| `.xd`     | Yes                                                                                                                                      |

A `.folio` folder holds no font: the bake draws the text into the screens. So fonts matter where
the bake runs: on your machine (CLI), in your build (Vite), or in the visitor's browser (raw files).

Without the font, the text is left out and the bake says which family and style it lacked.

## How a font is chosen

For each family a text asks for (`Inter`, `SF Pro Text`, …), in order:

1. a font the design file embeds (a `.sketch` can);
2. a font you gave whose family has that name (case does not matter), with the closest weight and
   the same style, as browsers do;
3. a font you gave that stands in for it: its `family` or `match` names it;
4. otherwise, the text is left out and reported.

The system's fonts are never used, so the bake gives the same result on every machine.

`match` takes globs (`SF Pro*`, `Helvetica?Neue`; `*` and `?`, case-insensitive), several
separated by commas, or one regular expression between slashes (`/^Helvetica( Neue)?$/i`).

## At bake time

::: code-group

```sh [CLI]
folioscope bake app.sketch public/app.folio \
  --fonts fonts/ \
  --fonts fonts/Inter.ttf --match "SF Pro*"
```

```ts [Vite]
folioscope({
  include: "designs/*.sketch",
  fonts: ["fonts", { src: "fonts/Inter.ttf", match: ["SF Pro*"] }],
});
```

:::

Font files: TrueType, OpenType and collections (`.ttf`, `.otf`, `.ttc`), WOFF and WOFF2.

## In the browser

Only for raw design files: the player bakes them, so it needs the fonts. They are declared inside
the player:

```html
<folioscope-player src="/designs/app.sketch">
  <!-- The file says its family (Inter): used for "Inter" -->
  <folioscope-font src="/fonts/Inter.var.woff2"></folioscope-font>

  <!-- A family name, a weight and a style the file does not say -->
  <folioscope-font
    src="/fonts/Brand-BoldItalic.woff2"
    family="Brand"
    weight="700"
    style="italic"
  ></folioscope-font>

  <!-- A free font standing in for one you may not serve -->
  <folioscope-font src="/fonts/Inter.var.woff2" match="SF Pro*"></folioscope-font>
  <folioscope-font src="/fonts/Arimo.var.woff2" match="/^Helvetica( Neue)?$/i"></folioscope-font>
</folioscope-player>
```

Close each `<folioscope-font>` with `</folioscope-font>`: `/>` does not close an element in HTML.

### A folder of fonts

With many fonts, describe a folder once and let the player fetch only what the file uses:

```sh
folioscope fonts public/fonts   # writes public/fonts/folioscope-fonts.json
```

```html
<folioscope-player src="/designs/app.penpot">
  <folioscope-font base="/fonts/"></folioscope-font>
</folioscope-player>
```

### Your site's stylesheet

Reuse the `@font-face` rules your site already has: the player reads them and fetches the faces
of the families the file uses.

```html
<folioscope-player src="/designs/app.xd">
  <folioscope-font css="/styles/fonts.css"></folioscope-font>
</folioscope-player>
```

### Shared between players

```html
<folioscope-fonts id="brand">
  <folioscope-font base="/fonts/"></folioscope-font>
</folioscope-fonts>

<folioscope-player src="/designs/home.sketch" fonts="brand"></folioscope-player>
<folioscope-player src="/designs/checkout.sketch" fonts="brand"></folioscope-player>
```

A player's own `<folioscope-font>` children come before the shared set.

### From JavaScript

```js
import { FolioscopeFontSet } from "folioscope";

player.registerFont({ src: "/fonts/Inter.woff2", match: "SF Pro*" });
player.registerFont("Brand", "/fonts/Brand.woff2");
player.registerFont({ data: bytes }); // a font already in memory, from a file input

const brand = new FolioscopeFontSet().add({ base: "/fonts/" });
for (const p of document.querySelectorAll("folioscope-player")) p.fonts = brand;
```

A file already baked is baked again with the new fonts. A font the file needs and was not given
fires `folioscope:font-missing` (`event.detail.font`, such as `"SF Pro Text Semibold"`).

## Licences

Serving a font to visitors is distributing it: check its licence. Fonts under the SIL Open Font
License (Inter, Roboto, Source Sans and most Google Fonts) may be served. For a font you may not
serve, give a free one that stands in for it with `match`; the shapes differ, the layout stays
close.
