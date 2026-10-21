# helpers4 requests from Folioscope

Generic helpers Folioscope needs and helpers4 does not have yet (checked against `helpers4`
3.1.4 on npm and the `helpers4` 0.0.7 crate). Each one is generic; none is tied to design files.
Folioscope will depend on the single `helpers4` package (sub-path imports) and on the `helpers4`
crate (one feature per module).

Priorities follow Folioscope's roadmap: **P1** blocks Phase 1 (bake and lite player, next),
**P2** Phase 2–3, **P3** later.

Already available and used, nothing to do: Rust `bytes::read_u32` + `Endian`, `hex::encode`,
`hex::base64_encode`; TS `function` (`debounce`, `throttle`, `once`), `number` (`clamp`, `lerp`),
`string.globToRegExp`, `color` (`hexToRgb`, `rgbToHex`, `rgbToHsl`).

## 1. `binary` — P1, Rust and TS

A cursor over a byte slice for binary formats (Folioscope decodes Kiwi, Figma's format, which is
varint-based). Stopgap today: a private `Reader` in `folioscope-core/src/kiwi.rs`
(`TODO(helpers4)`).

Rust (merge into the `bytes` feature, or a new `binary` feature):

```rust
pub struct ByteReader<'a> { /* data, position */ }

impl<'a> ByteReader<'a> {
    pub fn new(data: &'a [u8]) -> Self;
    pub fn position(&self) -> usize;
    pub fn remaining(&self) -> usize;
    pub fn read_u8(&mut self) -> Result<u8, Eof>;
    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], Eof>;
    pub fn read_u32(&mut self, endian: Endian) -> Result<u32, Eof>;   // also u16, u64, i*, f32, f64
    pub fn read_var_u32(&mut self) -> Result<u32, Eof>;   // LEB128, at most 5 bytes
    pub fn read_var_u64(&mut self) -> Result<u64, Eof>;   // LEB128, at most 9 bytes
    pub fn read_var_i32(&mut self) -> Result<i32, Eof>;   // zigzag over var_u32
    pub fn read_var_i64(&mut self) -> Result<i64, Eof>;   // zigzag over var_u64
    pub fn read_cstr(&mut self) -> Result<&'a str, Utf8OrEof>;   // NUL-terminated UTF-8
}
// And free functions: zigzag_encode_32/64, zigzag_decode_32/64.
```

TS (`helpers4/binary`): the same over `DataView`, plus a `ByteWriter` (growable buffer, same
methods in write form). Folioscope's fixture generator and a possible JS decoder would use both.

Edge cases worth tests: truncated varints, 5th byte overflow, `-1`/`i32::MIN` zigzag, empty
string, invalid UTF-8.

## 2. `geometry` — P1, Rust and TS

2D geometry for hit-testing, layer bounds, scaling a frame into a viewport, and gradients. Stopgap
today: an inline affine inversion in `folioscope-bake/src/svg.rs` (`gradient_line`,
`TODO(helpers4)`).

```ts
type Point = { x: number; y: number };
type Size = { width: number; height: number };
type Rect = { x: number; y: number; width: number; height: number };
// Affine matrix, column-major like CSS/SVG matrix(a b c d e f):
// x' = a·x + c·y + e,  y' = b·x + d·y + f
type Matrix = { a: number; b: number; c: number; d: number; e: number; f: number };

identity(): Matrix
translate(tx, ty) / scale(sx, sy?) / rotate(radians): Matrix
compose(...matrices): Matrix          // left to right: compose(m1, m2) applies m1 then m2
invert(m): Matrix | undefined         // undefined when the determinant is ~0
applyToPoint(m, p): Point
applyToRect(m, r): Rect               // bounding box of the transformed rectangle
rectUnion(rects): Rect | undefined
rectIntersection(a, b): Rect | undefined
intersects(a, b): boolean
containsPoint(r, p): boolean
fitContain(content: Size, box: Size): { scale; x; y }   // letterbox
fitCover(content: Size, box: Size): { scale; x; y }     // crop
fitWidth(content: Size, box: Size): { scale; x; y }
```

Rust: the same as `Point`, `Size`, `Rect`, `Matrix` (`f32` or generic over `f32`/`f64`), with
`Matrix::from_figma(m00, m01, m02, m10, m11, m12)`-style constructors left to callers.

## 3. `fsm` — P1, TS (Rust P3)

A small typed state machine with a history stack: the prototype runtime (current screen,
overlays, back) and many UIs.

```ts
const machine = createMachine<State, Event>({
  initial,
  transition(state, event): State | undefined,   // undefined: event ignored
});
machine.state; machine.send(event); machine.back(); machine.canGoBack;
machine.reset(); machine.subscribe(listener): unsubscribe;
// history: push on transition, pop on back(); options.historyLimit
```

## 4. `dom` — P1, TS only

Custom Element plumbing every vanilla Web Component re-writes (Folioscope ships no framework):

- `reflectAttribute(element, name, { type: "string" | "number" | "boolean" | "enum", values? })`
  or a decorator-free helper that defines a typed property backed by an attribute;
- `observeResize(element, callback, { debounceMs })` → disconnect function;
- `onPointerIntent(element, { hoverStart, hoverEnd, pressStart, pressEnd, tap })` with tap vs.
  drag distinction (movement threshold) and pointer capture;
- `prefersReducedMotion(): boolean` and a subscription to its changes;
- `decodeImage(src): Promise<HTMLImageElement>` (load + `img.decode()`), `preloadImages(urls)`;
- `whenIdle(callback, { timeout })` wrapping `requestIdleCallback` with a fallback.

## 5. `easing` — P2, TS and Rust

Prototype transitions, and any animation:

```ts
cubicBezier(x1, y1, x2, y2): (t: number) => number   // CSS-exact, Newton + bisection
linear, easeIn, easeOut, easeInOut                    // CSS keyword curves
spring({ mass, stiffness, damping, initialVelocity }): { at(t): number; durationMs }
springToCssLinear(spring, samples): string            // CSS linear() for Web Animations
```

Figma, XD and Penpot easings (including overshoot ones such as XD's "wind-up" and Figma's
"gentle spring") will be expressed with these.

## 6. `color` extensions — P2, TS and Rust

- blend modes on premultiplied RGBA: `multiply`, `screen`, `overlay`, `darken`, `lighten`,
  `colorDodge`, `colorBurn`, `hardLight`, `softLight`, `difference`, `exclusion`, `hue`,
  `saturation`, `color`, `luminosity`;
- `premultiply` / `unpremultiply`;
- `rgbToHsb` / `hsbToRgb` (design tools show HSB);
- `toCss(rgba)` → `rgb(r g b / a)`; parsing of 8-digit hex with alpha.

## 7. `font` — P3, TS

Loading fonts with `FontFace`: register from URL or `ArrayBuffer`, `document.fonts` integration,
closest weight/style match (`pickClosest(available, { weight, style })`, CSS matching algorithm),
and detection of a missing family.

## 8. `string` glob additions — P3, TS and Rust

`globToRegExp` exists in TS. Missing: `matchGlob(pattern, text, { caseInsensitive })` and
`parsePatternOrRegExp("/…/flags" | glob)`; the Rust `string` feature has neither.
