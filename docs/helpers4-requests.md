# helpers4 requests from Folioscope

Generic helpers Folioscope needs from helpers4 (npm `helpers4`, sub-path imports; crate
`helpers4`, one feature per module). Each one is generic; none is tied to design files.

## Delivered and used (helpers4 3.1.10 on npm, 0.0.10 crate)

- Rust `kiwi`: the `.fig` files' schema-driven Kiwi decoder (`Schema::parse`, `Schema::decode`,
  `Value`), re-exported as `folioscope::kiwi`. It replaced Folioscope's own, which was built on
  `bytes::ByteReader`.
- Rust `font`: `pick_closest` (CSS font matching), `weight_of_style`, `is_italic` and
  `split_postscript`, in the bake's font library.
- Rust `string::match_glob_ignore_case`: the family patterns of `<folioscope-font match="…">`.
- Rust `bytes::media_type_of`: the image type of a `data:` URI in the SVG the bake renders.
- Rust `geometry`: `rounded_rect_path` and `ellipse_path` (Penpot shapes), `transform_path` (moving
  Sketch's boolean group paths), and `Matrix` (node transforms, the inverted gradient transform of
  `.fig` paints).
- Rust `bytes::ByteReader`: no longer used by Folioscope since the Kiwi decoder moved to helpers4.
- TS `structure.createBackStack` (`replace`, `previous`): the player's history (`runtime.ts`).
- TS `geometry`: `rectToRect` and `matrixToCss` (smart animate), `contains` and `fitContain`.
- TS `binary.mediaTypeOfPath`: blob types in the player, the dev server of the Vite plugin and the
  test server.
- TS `browser`: `requestWorker`, `createFileStore` and `sha256Hex` (the full player's worker and its
  cache), `prefersReducedMotion`.

## Delivered, not used yet

- TS `animation` (`cubicBezier`, `parseEasing`…), `color` (`rgbToUnit`, `unitToRgb`), `string`
  (`matchGlob`, `parsePatternOrRegExp`), the rest of `browser` (`reflectAttribute`, `observeResize`,
  `decodeImage`, `preloadImages`…).
- Rust `color` (`Rgba::from_unit`, blend modes, `premultiply`): the three renderers still write
  `rgb(r,g,b)` themselves. `to_css` writes `rgb(r g b)`, which changes the SVG text and so the hash
  of every baked layer, for no change in pixels: adopt it together with the next change that
  rebakes the demos anyway.
- Rust `geometry::parse_path`, `format_path`: no need yet.

## Still open

- Rust `font::read_font` is delivered but not adopted: it has no WOFF2 (no Brotli decoder), reads
  only the first font of a `.ttc` collection and does not hand back the decoded font. The bake
  needs all three (`<folioscope-font>` takes WOFF2, and `skrifa` shapes the text from the decoded
  file), so it keeps reading faces with `skrifa` and `wuff`.
- A `/regex/` pattern for font substitution rules (`match="/^SF Pro/i"`) in the Rust bake: the TS
  `parsePatternOrRegExp` exists, the Rust `string::parse_pattern` has to be checked against it
  when the rule lands (PROJECT.md, fonts).
