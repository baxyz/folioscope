# The player

`packages/folioscope` (npm `folioscope`): the `<folioscope-player>` custom element, its lite
player for `.folio` folders, its full player for raw design files, and the `folioscope/vite`
plugin. Vanilla custom elements in a shadow root: no UI framework.

## Entry points

| Import            | Contents                                                                  | Budget                      |
| ----------------- | ------------------------------------------------------------------------- | --------------------------- |
| `folioscope`      | `<folioscope-player>`, the lite player; loads the full one only if needed | under 15 KB of JS gzipped   |
| `folioscope/full` | the full player, loaded up front                                          | WebAssembly under 2.5 MB gz |
| `folioscope/vite` | the Vite plugin: bakes design files at build time, serves them in dev     | build time only             |

## Element

```html
<folioscope-player src="/app.folio" poster="/app.folio/poster.webp" controls></folioscope-player>
```

| Attribute  | Values                                 | Role                                                                                   |
| ---------- | -------------------------------------- | -------------------------------------------------------------------------------------- |
| `src`      | URL                                    | a `.folio` folder (or its `index.json`), or a `.fig`, `.sketch`, `.xd`, `.penpot` file |
| `poster`   | URL                                    | an image shown before the prototype loads, usually `app.folio/poster.webp`             |
| `flow`     | name or index                          | the starting flow; the first one by default                                            |
| `page`     | name or index                          | the page; the first one by default                                                     |
| `fit`      | `contain` (default), `none`            | scales the prototype to the element's width (and in fullscreen, its height)            |
| `hotspots` | `on-miss` (default), `always`, `never` | shows the clickable areas; `on-miss` flashes them after a click beside them            |
| `preload`  | `neighbors` (default), `all`, `none`   | which frames' layers are fetched ahead                                                 |
| `controls` | boolean                                | the control bar: restart, back, page and flow pickers, fullscreen                      |
| `fonts`    | id                                     | a shared `<folioscope-fonts id="…">` set                                               |

- **Methods**: `restart()`, `back()`, `goTo(frameId)`, `registerFont(…)`.
- **Events** (bubbling, composed): `folioscope:load` (`pages`), `folioscope:navigate` (`frame`,
  `name`), `folioscope:error` (`error`), `folioscope:progress` (`stage`: `download`, `fonts`,
  `cache`, `bake`), `folioscope:font-missing` (`font`).
- **Styling**: CSS parts (`controls`…) and custom properties
  (`--folioscope-controls-background`, `--folioscope-controls-color`).

## Lite player

- **Layers are `<img>`** elements at their bounds, decoded (`img.decode()`) before the frame is
  shown; the frame's ThumbHash placeholder shows meanwhile when there is no poster.
- **Hotspots are transparent, focusable DOM elements** labelled with their node's name: a click,
  Enter or Space follows them; a frame is labelled with its own name.
- **The runtime** (`runtime.ts`, independent of the DOM) holds the current frame, the history (a
  back stack from helpers4), open overlays and component states. Escape goes back.
- **Triggers**: click, hover and press (a frame while they last), mouse enter and leave,
  mouse down and up, after delay. **Actions**: navigate, change to, scroll to, open, swap and
  toggle overlays, back, close, open a URL.
- **Scrolling**: a scrolling frame's content sits in a native scroller between its background and
  its fixed layers.
- **Overlays** are positioned over the screen, on their backdrop, and close on a click outside when
  the file says so.
- **Preloading**: on load, the start frame, then the frames one interaction away (`neighbors`), or
  every frame of the page (`all`).

## Transitions

Baked images animated with the Web Animations API, no rendering in the browser:

- **Whole screens**: dissolve, slide, push, move in and out. Easing is CSS: `cubic-bezier()`, or
  `linear()` sampled from a spring (helpers4 `animation`), in which case the spring sets the
  duration.
- **Smart animate**: the parts of the two frames ([format](folio.md#layers)) are paired by their
  path (`Card/Title`, three levels deep). Each pair goes from its first place to its second, its
  matrix interpolated (so it turns too) and its image scaled, while the first image fades into the
  second (colour, opacity and contents change too); a plain rounded rectangle is one box whose
  size, corner radius and colour are animated, so its corners keep their shape. A part on one side
  only fades out or in; the backgrounds cross-fade. Without parts, smart animate plays as a
  dissolve.
- **Smart animate between a component's states** and **when swapping overlays** pairs their
  parts the same way.
- **Component states** otherwise change with a dissolve.
- **`prefers-reduced-motion`** turns every transition into a cut.

## Full player

`src` pointing at a raw design file:

1. **A `.folio` first**: for `app.fig`, the player looks for `app.folio/index.json` next to it and
   plays it when it is there, with no WebAssembly.
2. Otherwise it imports `folioscope/full`, whose **worker** downloads the file and the declared
   fonts, and **runs the bake** (the same WebAssembly as the Vite plugin) at the screen's density,
   in lossless WebP or SVG, into a `.folio` in memory played through `blob:` URLs.
3. **Cache**: the files go into the Cache API, keyed by the SHA-256 of the design file, the density
   and the fonts; a second visit reads them back without baking.
4. Fonts the bake lacked become `folioscope:font-missing` events.

## Fonts in the browser

Fonts matter only to the full player (a `.folio` holds no font). They are declared on the page:

```html
<folioscope-player src="/mockups/app.sketch">
  <folioscope-font src="/fonts/Inter.var.woff2"></folioscope-font>
  <folioscope-font
    family="Roboto"
    weight="700"
    style="italic"
    src="/fonts/Roboto-BoldItalic.woff2"
  ></folioscope-font>
  <folioscope-font match="SF Pro*" src="/fonts/Inter.var.woff2"></folioscope-font>
  <folioscope-font match="/^Helvetica( Neue)?$/i" src="/fonts/Arimo.var.woff2"></folioscope-font>
  <folioscope-font base="/fonts/"></folioscope-font>
  <folioscope-font css="/fonts/fonts.css"></folioscope-font>
</folioscope-player>
```

| Mode         | Attributes                                    | Loading                            | Use                                                        |
| ------------ | --------------------------------------------- | ---------------------------------- | ---------------------------------------------------------- |
| Auto         | `src`                                         | always: the file says its family   | a few fonts                                                |
| Family       | `family`, `src`, optionally `weight`, `style` | only if the design uses the family | the family is known                                        |
| Substitution | `match`, `src`                                | only if a family matches           | replace a font you may not serve with a free one           |
| Folder       | `base`                                        | only the families the design uses  | a folder with `folioscope-fonts.json` (`folioscope fonts`) |
| CSS          | `css`                                         | only the families the design uses  | reuse a site's `@font-face` rules                          |

- `match` is a glob (`*`, `?`, case-insensitive), or a regex between slashes (`/…/flags`).
- `<folioscope-font>` must be closed explicitly: `/>` does not close an element in HTML.
- **Shared set**: `<folioscope-fonts id="portfolio">` holds fonts several players use, by
  `fonts="portfolio"`; a player's own `<folioscope-font>` children come first.
- **JavaScript**: `player.registerFont({ src | family | match | base | css | data, … })`,
  `registerFont("Roboto", "/fonts/Roboto.woff2")`, and `new FolioscopeFontSet()` assigned to
  `player.fonts`.
- **Resolution order** for each family: a font embedded in the design file; an exact `family`; the
  first matching `match` rule, in document order; auto fonts; the `base` manifests, then the `css`
  stylesheets; otherwise the text is left out and `folioscope:font-missing` fires.

## Accessibility

Hotspots and controls are reachable and usable with the keyboard; Escape goes back; frames and
hotspots carry labels from their names; the control bar is a labelled toolbar;
`prefers-reduced-motion` removes the animations.
