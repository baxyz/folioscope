# helpers4: what is left

Folioscope uses helpers4 for everything generic (npm `helpers4` 3.1.12, crate 0.0.11). No request
to helpers4 is pending: what it lacks is handled in Folioscope, and listed here.

## Handled in Folioscope

- **WOFF2** (Rust `font`): `read_font_at`, `font_count` and `to_sfnt` read collections and WOFF,
  not WOFF2 (no Brotli decoder). The bake opens WOFF2 with `wuff` and hands the rest to helpers4.
- **`/regex/` font rules** (Rust `string::parse_pattern`): same recognition as the TS
  `parsePatternOrRegExp`, but it does not validate the regex source. The bake will validate it
  when the rule lands.
- **The prototype's state machine** stays in `runtime.ts`, on helpers4's `createBackStack`: there
  is no `fsm` module, and none is asked for. DOM needs are covered by `helpers4/browser`.

## Not adopted yet

- **TS** `animation` (beyond `spring` and `toCssLinear`), `color`, `string` (`matchGlob`,
  `parsePatternOrRegExp`) and the rest of `browser`: no need yet.
