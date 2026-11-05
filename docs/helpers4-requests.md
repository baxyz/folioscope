# helpers4: what is left

Folioscope uses helpers4 for everything generic (npm `helpers4` 3.1.11, crate 0.0.11). Only what
is still pending is listed here.

## Waiting on helpers4

- **WOFF2 in `font`** (Rust): `read_font_at`, `font_count` and `to_sfnt` read collections and
  WOFF; only WOFF2 is missing (no Brotli, helpers4/rust#36). The bake opens WOFF2 with `wuff` and
  hands the rest to helpers4.
- **`/regex/` font rules** (Rust `string::parse_pattern`): same recognition as the TS
  `parsePatternOrRegExp`, but it does not validate the regex source (helpers4/rust#35). The bake
  will validate it when the rule lands.

## Not adopted yet

- **Rust `color`** (`Rgba::from_unit`, `to_css`): `to_css` writes `rgb(r g b)` on purpose, which
  changes the SVG text and so the hash of every baked layer. Adopt it with the next change that
  rebakes the demos anyway; no `to_css_legacy()` needed.
- **TS** `animation`, `color`, `fetch`, `string` (`matchGlob`, `parsePatternOrRegExp`) and the rest of
  `browser`: no need yet.
