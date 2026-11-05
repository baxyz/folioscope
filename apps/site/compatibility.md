---
title: Compatibility
description: What Folioscope plays today for each design format.
---

# Compatibility

What the player does with each format today. The four formats do not all describe the same
prototypes: Sketch only has click links, for instance, so some rows say _not in the format_ rather
than _not yet_.

- **Plays**: baked and played, checked by a test.
- **Partly**: works, with a limit listed below the table.
- **Not yet**: the format has it, Folioscope does not play it yet.
- **Not in the format**: the design tool has no such feature.

## Prototype

| Feature                  | Figma `.fig` | Sketch `.sketch`  | Penpot `.penpot`  | Adobe XD `.xd` |
| ------------------------ | ------------ | ----------------- | ----------------- | -------------- |
| Click or tap links       | Plays        | Plays             | Plays             | Plays          |
| Hover and press          | Plays        | Not in the format | Plays             | Not yet        |
| After a delay            | Plays        | Not in the format | Plays             | Plays          |
| Back, previous screen    | Plays        | Plays             | Plays             | Plays          |
| Overlays                 | Plays        | Not in the format | Plays             | Plays          |
| Scrolling inside screens | Partly       | Not yet           | Not yet           | Not yet        |
| Slide, push, dissolve    | Plays        | Slides only       | Plays             | Plays          |
| Smart animate            | Partly       | Not in the format | Not in the format | Plays          |
| Interactive components   | Partly       | Not in the format | Not yet           | Not yet        |

## Drawing

| Feature                      | Figma `.fig`          | Sketch `.sketch` | Penpot `.penpot` | Adobe XD `.xd`   |
| ---------------------------- | --------------------- | ---------------- | ---------------- | ---------------- |
| Shapes, colour fills, images | Plays                 | Plays            | Plays            | Plays            |
| Gradients                    | Partly                | Plays            | Plays            | Plays            |
| Drop shadows                 | Plays                 | Plays            | Plays            | Plays            |
| Masks and clip paths         | Plays                 | Plays            | Plays            | Plays            |
| Text                         | Plays, no font needed | Needs your fonts | Needs your fonts | Needs your fonts |

A `.fig` file stores the outline of every glyph, so its text is drawn without any font. For the
other formats, give the bake the fonts the design uses (`--fonts`, or the `fonts` option of the
Vite plugin).

## Limits

- **Figma**: only linear gradients are drawn (the others use their first colour), and blend modes are not. A nested frame that scrolls, and sticky elements, are not played yet. Smart animate
  pairs the direct children of a screen, not nested layers. A component variant other than the one
  in the file is drawn at its own size, without its overrides' layout.
- **Sketch**: only the first shadow, and a text takes the style of its first run. Blurs, blend
  modes, inner shadows and masks by alpha are not drawn.
- **Penpot**: only the v3 format (a ZIP of JSON). Blurs, inner shadows and `svg-raw` shapes are not
  drawn; strokes are always centred.
- **XD**: the format is not documented by Adobe and no longer changes. An artboard taller than its
  screen scrolls; elements fixed in place while it scrolls are not read yet. Background blurs
  (frosted glass), text underline and strikethrough are not drawn; image fills other than _cover_
  are not either, and neither are component states. Overlays have no backdrop and do not close on
  a click outside, as in XD's player as far as we know.

This page is written by hand from the project's [roadmap](https://github.com/baxyz/folioscope/blob/main/PROJECT.md).
It will be generated from the tests once each importer reports what it supports.
