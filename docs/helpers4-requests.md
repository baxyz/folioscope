# helpers4 requests from Folioscope

Generic helpers Folioscope needs from helpers4 (npm `helpers4`, sub-path imports; crate
`helpers4`, one feature per module). Each one is generic; none is tied to design files.

## Delivered and used (helpers4 3.1.9 on npm, 0.0.9 crate)

- Rust `bytes::ByteReader`: the Kiwi reader (`.fig`) reads bytes, LEB128 and zigzag varints and
  strings with it. Kiwi's 64-bit varints and its floats have encodings of their own and stay in
  Folioscope.
- Rust `geometry::Matrix`: node transforms and bounding boxes (`figma::ir`), and the inverted
  gradient transform of `.fig` paints.
- TS `helpers4/browser` `prefersReducedMotion`, `helpers4/geometry` `contains` and `fitContain`, in
  the player (bundled into it).
- Also delivered, not used yet: TS `binary`, `animation` (`cubicBezier`, `parseEasing`,
  `spring`…), `structure` (`createBackStack`), the rest of `browser` (`reflectAttribute`,
  `observeResize`, `decodeImage`, `preloadImages`…).

## 1. `structure.createBackStack`: `replace` — TS

The player's history (`runtime.ts`) is a back stack of prototype states (a screen and the
overlays over it), but two moves change the current entry without adding one:

- closing an overlay whose opening is not the previous entry: the current entry becomes "the same
  screen, one overlay less", and `back` must not reopen it;
- "while hovering" and "while pressing": a frame shows for as long as the pointer stays, then the
  previous one comes back; neither goes into the history.

```ts
stack.replace(value): void   // sets the current entry, keeping the entries underneath
stack.previous: T | undefined // the entry `back` would return to, to compare before popping
```

With both, `runtime.ts` can drop its own history array.

## 2. `color` extensions (to check against 3.1.9) — P2, TS and Rust

- blend modes on premultiplied RGBA: `multiply`, `screen`, `overlay`, `darken`, `lighten`,
  `colorDodge`, `colorBurn`, `hardLight`, `softLight`, `difference`, `exclusion`, `hue`,
  `saturation`, `color`, `luminosity`;
- `premultiply` / `unpremultiply`;
- `rgbToHsb` / `hsbToRgb` (design tools show HSB);
- `toCss(rgba)` → `rgb(r g b / a)`; parsing of 8-digit hex with alpha.

## 3. `font` (to check against 3.1.9) — P3, TS

Loading fonts with `FontFace`: register from URL or `ArrayBuffer`, `document.fonts` integration,
closest weight/style match (`pickClosest(available, { weight, style })`, CSS matching algorithm),
and detection of a missing family.

## 4. `string` glob additions (to check against 3.1.9) — P3, TS and Rust

`globToRegExp` exists in TS. Missing: `matchGlob(pattern, text, { caseInsensitive })` and
`parsePatternOrRegExp("/…/flags" | glob)`; the Rust `string` feature has neither.
