# ADR 0011 — Fonts and text layout for the bake

- **Status:** Accepted
- **Date:** 2026-10-22

## Context

A `.fig` file stores the outline of every laid-out glyph, so the bake has drawn text without a
single font so far (ADR 0001). That stops where Figma stored nothing:

- a variant other than the one an instance shows in the file: its text is overridden (the
  "Lunch" of a menu pill), but Figma derived glyphs only for the variant on screen;
- `.sketch`, `.xd` and `.penpot` files, which store the text and its style, not its glyphs (a
  `.sketch` file may embed its fonts, a `.penpot` file its custom ones).

There, the bake must lay text out itself: pick the font, shape the characters, break the lines in
the box, align them, then draw the glyphs. It must stay deterministic (the same bake on every
machine, and in WebAssembly for `folioscope/vite`), so it never reads the system's fonts: only
fonts embedded in the file and fonts the user gives it.

## Options for layout

| Option                                    | For                                                                                     | Against                                                  |
| ----------------------------------------- | --------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| A. SVG `<text>`, rendered by resvg (usvg) | Nothing to write                                                                        | No line breaking in the box; usvg's text pulls fontdb in |
| B. cosmic-text                            | Complete: shaping, line breaking, bidi                                                  | Built around its own font system and swash rendering     |
| C. **parley** (linebender, like resvg)    | Rich text layout: styled runs, line breaking, alignment, variable fonts; no system font | Young API (0.12)                                         |

## Decision

- **Layout: parley.** Shaping through HarfRust, line breaking and alignment in the box, font
  fallback limited to the fonts registered. Pure Rust, it builds for WebAssembly unchanged.
- **Glyphs: skrifa**, the outline reader parley uses: each glyph becomes an SVG path, as with the
  outlines a `.fig` stores, so text goes through the same rasterization (resvg) as everything
  else, with no font in the `.folio`.
- **Font files: TTF, OTF and collections, and WOFF or WOFF2**, decoded first with `wuff` (pure
  Rust). Each face is read for its family (typographic family first), weight, style and, for a
  variable font, its weight range.
- **Where fonts come from**: the design file's embedded fonts first, then the user's, given as
  files or folders (`folioscope bake --fonts <path>`, the `fonts` option of `folioscope/vite`).
  `folioscope fonts <folder>` writes `folioscope-fonts.json`, the manifest the player's `base`
  mode reads (§ Fonts of the project plan).
- **Matching**, for each family a text asks for: the same family, case-insensitive, then the
  closest weight and the same style (CSS font matching); otherwise the user's substitution rules;
  otherwise the text is left out and the bake reports the missing family and style, once per
  file, with where it is used.

## Consequences

- `.fig` files keep baking with no font: fonts only fill in what Figma did not store.
- The WebAssembly module grows by parley, HarfRust and skrifa (measured when first used).
- Test fonts live in `fixtures/fonts/`: Inter, cut down to Latin (SIL OFL 1.1).
