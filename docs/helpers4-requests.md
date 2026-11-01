# helpers4: what is left

Folioscope uses helpers4 for everything generic (npm `helpers4` 3.1.10, crate 0.0.10). Only what
is still pending is listed here.

## Waiting on helpers4

- **Bundle size**: the lite player is 11.8 KB gzipped (budget 15 KB) because helpers4 ships one
  module per sub-path. Fixed in helpers4/typescript#210, in the release after 3.1.10: measure again
  then.
- **`font::read_font`** (Rust): no WOFF2, only the first face of a `.ttc`, no decoded data
  (helpers4/rust#36). The bake keeps reading fonts with `skrifa` and `wuff`.
- **`/regex/` font rules** (Rust `string::parse_pattern`): same recognition as the TS
  `parsePatternOrRegExp`, but it does not validate the regex source (helpers4/rust#35). The bake
  will validate it when the rule lands.

## Not adopted yet

- **Rust `color`** (`Rgba::from_unit`, `to_css`): `to_css` writes `rgb(r g b)` on purpose, which
  changes the SVG text and so the hash of every baked layer. Adopt it with the next change that
  rebakes the demos anyway; no `to_css_legacy()` needed.
- **TS** `animation`, `color`, `string` (`matchGlob`, `parsePatternOrRegExp`) and the rest of
  `browser`: no need yet.
