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

## 2. `kiwi` — Rust, P2

A decoder for [Kiwi](https://github.com/evanw/kiwi), Evan Wallace's schema-based binary format
(used by `.fig` files, but generic): read a binary schema, then decode a message against it into a
dynamic value (objects, arrays, enums by name, the usual scalars), with precise errors on truncated
or invalid data. Folioscope's is in `crates/folioscope/src/kiwi/` (about 400 lines, on
`bytes::ByteReader`), with Kiwi's own 64-bit varints and floats. The `kiwi-schema` crate exists;
check whether it decodes dynamically before taking ours.

```rust
let schema = kiwi::Schema::decode(bytes)?;           // definitions: enums, structs, messages
let value = schema.decode(message_bytes, "Message")?; // kiwi::Value, with get(name), as_str()…
```

## 3. `font` — Rust, P2 (TS delivered: `pickClosest`, `browser.loadFont`)

The Rust counterpart of `helpers4/font`, for the bake (Folioscope's
`crates/folioscope/src/bake/fonts.rs`):

- `pick_closest(faces, request)`: CSS font matching, the style first, then the closest weight, a
  variable face covering a weight range;
- `weight_of_style("Semi Bold Italic") -> 600.0` and `is_italic(style)`: the style names design
  tools write;
- `split_postscript("NeutrifPro-SemiBold") -> ("NeutrifPro", "SemiBold")`, and family names
  compared without spaces or case;
- optional, behind a feature (it needs a font parser): read a font file's family (typographic
  first), PostScript name, weight or `wght` range and style, WOFF and WOFF2 decoded.

## 4. Paths and shapes — Rust and TS, P2 (`geometry`)

What the Sketch and Penpot renderers write by hand (`sketch.rs`, `penpot.rs`):

- SVG path data: parse into commands, and transform absolute path data by a `Matrix` (moving it is
  the common case);
- a rounded rectangle's path with one radius per corner, clamped to half the size, and an
  ellipse's path in a box;
- `rectToRect(from, to): Matrix`: the translation and scale that put one rectangle on another
  (smart animate in the player, `player/lite.ts`).

## 5. Media types — Rust and TS, P2

Folioscope keeps four tables of extensions and a magic-byte sniffer:

- `media_type_of(bytes)` / `mediaTypeOf(bytes)`: PNG, JPEG, GIF, WebP, AVIF, SVG from their first
  bytes (`bake/src/svg/paint.rs`);
- `mediaTypeOfPath("layers/a@2x.webp")`: by extension, for dev servers and `blob:` URLs
  (`vite/index.ts`, `element.ts`, `e2e/serve.ts`).

## 6. `color` extensions (to check against 3.1.9) — P2, TS and Rust

- blend modes on premultiplied RGBA: `multiply`, `screen`, `overlay`, `darken`, `lighten`,
  `colorDodge`, `colorBurn`, `hardLight`, `softLight`, `difference`, `exclusion`, `hue`,
  `saturation`, `color`, `luminosity`;
- `premultiply` / `unpremultiply`;
- `rgbToHsb` / `hsbToRgb` (design tools show HSB);
- `toCss(rgba)` → `rgb(r g b / a)`; parsing of 8-digit hex with alpha; colours with 0–1 float
  channels, as Figma, Sketch and Penpot store them, to CSS and to `#rrggbbaa` (Rust: the three
  renderers each convert them).

## 7. Browser plumbing — TS, P3 (`browser`)

What the full player writes around its worker (`player/full/`):

- a typed request to a module worker with progress messages and transferable results, ending the
  worker on completion or error;
- a file store on the Cache API: put and read back a set of files under a key (with a version),
  keys being made-up URLs;
- `toHex(bytes)` (and a truncated SHA-256 hex digest), for content keys.

## 8. `string` glob additions (to check against 3.1.9) — P3, TS and Rust

`globToRegExp` exists in TS. Missing: `matchGlob(pattern, text, { caseInsensitive })` and
`parsePatternOrRegExp("/…/flags" | glob)` (for font substitution rules, `match="SF Pro*"`); the
Rust `string` feature has neither.
