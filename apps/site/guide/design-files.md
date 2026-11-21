---
title: Your design file
description: "How to save a design file from Figma, Sketch, Penpot and Adobe XD, and what it needs for its prototype to play."
---

# Your design file

Folioscope reads the file a design tool saves. It plays the file's **prototype**: the screens a
flow starts from, and the links between them. Screens with no flow are left out (the CLI's
`--all-pages` keeps their pages).

| Tool     | File      | How to get it                                                        | What makes a flow                                   |
| -------- | --------- | -------------------------------------------------------------------- | --------------------------------------------------- |
| Figma    | `.fig`    | **File → Save local copy…**                                          | a flow starting point on a frame (Prototype panel)  |
| Sketch   | `.sketch` | the document you save; from a Workspace, download it                 | **Prototyping → Use as Start Point** on an artboard |
| Penpot   | `.penpot` | from the dashboard, a file's menu: export it as a `.penpot` file     | a flow starting board (Prototype tab)               |
| Adobe XD | `.xd`     | the document you save; a cloud document: save it as a local document | the home artboard (Prototype mode)                  |

What each format plays, feature by feature, is on the [Compatibility](/compatibility) page; what
is left to do, on the [Roadmap](/roadmap).

## Before you bake

- **Name your layers.** Smart animate pairs layers by name, and the player labels its buttons
  with them for screen readers.
- **Fonts.** A `.fig` stores the outline of every letter: no font needed. `.sketch`, `.xd` and
  `.penpot` files only name their fonts: give the bake the font files (see [Fonts](./fonts)).
- **Images** are kept as they are in the file: a 4000-pixel photo in a phone screen is baked at
  the screen's size, at 1x and 2x.
- **Interactive components** with many variants: each state is drawn once per instance; past 16
  states per component, the bake leaves the rest out and says so (`maxStates`).

## Your files stay yours

Baking happens on your machine or in your build. Nothing is uploaded: the Playground and the
in-browser bake also run entirely in the visitor's browser.
