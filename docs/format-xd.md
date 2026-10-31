# The `.xd` format

Working notes on the parts of the Adobe XD format Folioscope reads. Adobe never documented it and
XD has been frozen since 2023, so these notes come from decoding five public files found on
GitHub (manifest format 6, interactions versions 0.22 to 0.24, 211 interactions), kept locally in
the git-ignored `fixtures/external/xd/` (their licences are unknown, so they are never committed).
The committed fixtures (`fixtures/xd/`) are written by `tools/fixtures` in the same shape, with
only what the importer reads; the real files check the importer on the full range (every
interaction of each file lands on a hotspot, and the baked home artboards match XD's own
`preview.png` to within a few percent, text aside).

## Container

A ZIP archive (an Adobe "UCF" package) with:

| Entry                                               | Contents                                            |
| --------------------------------------------------- | --------------------------------------------------- |
| `mimetype`                                          | `application/vnd.adobe.sparkler.project+dcxucf`     |
| `manifest`                                          | JSON tree of the package: artboards, names, bounds  |
| `artwork/artboard-<id>/graphics/graphicContent.agc` | JSON graphics tree of one artboard                  |
| `artwork/pasteboard/graphics/graphicContent.agc`    | Layers lying outside any artboard                   |
| `resources/graphics/graphicContent.agc`             | Shared resources: components, gradients, clip paths |
| `interactions/interactions.json`                    | The prototype                                       |
| `thumbnail.png`, `preview.png`, `renditions/`       | Previews                                            |

The extension is `.agc` in every file seen. The manifest's root has `children`; the one with
`path: "artwork"` lists the artboards as `{ path: "artboard-<id>", name, "uxdesign#bounds":
{ x, y, width, height }, "uxdesign#viewport": { height } }`, bounds in canvas coordinates. XD has a
single canvas: no pages. `uxdesign#specName` holds the document's name.

## Graphics trees

Each `graphicContent.agc` is `{ version, children, resources, artboards }`. Its root child is the
artboard itself (`type: "artboard"`, same id as the folder name), whose `artboard.children` are the
layers: `type` `text`, `shape`, `group`, `syncRef` (component instance)…, each with `id`, `name`,
`transform` and `style`.

## Prototype: `interactions/interactions.json`

```json
{
  "version": "0.22",
  "homeArtboard": "<artboard id>",
  "interactions": {
    "<source node id>": [
      {
        "id": "…",
        "triggerEvent": "tap",
        "action": "artboard-transition",
        "properties": {
          "destination": "<artboard id>",
          "transition": "dissolve",
          "duration": 0.3,
          "easing": "ease-out",
          "preserveScrollPosition": false
        }
      }
    ]
  }
}
```

- **Sources** are node ids from the graphics trees, or an artboard id (for timed transitions).
- **`homeArtboard`** is the start point; one per file.
- **Triggers** seen: `tap`, `time` (with `properties.delay` in seconds), `drag`, `keys-gamepad`
  (with `keysGamepad…` properties), and `voice` (`voiceLocale`).
- **Actions** seen: `artboard-transition` (navigate, with `transition`: `dissolve`, `slide-left`,
  `slide-right`…) and `auto-animate` (XD's smart animate). XD also has overlays, "previous
  artboard", scroll-to and audio actions, not seen in our sample files yet.
- **Easings**: `ease-in`, `ease-out`, `ease-in-out`, `linear`, and XD's `wind-up` (overshoot).

## Graphics: what the bake draws

- **Coordinates**: the children of an artboard are in the canvas' coordinates (the artboard's
  `uxdesign#bounds` corner is the frame's origin); a group's children are in the group's. A
  `transform` is `{ a, b, c, d, tx, ty }`, as in CSS.
- **Shapes** (`type: "shape"`, `shape.type`): `rect` (`x`, `y`, `width`, `height`, `r` as four corner
  radii), `circle` (`cx`, `cy`, `r`), `ellipse`, `line`, `polygon` (`points`), `path` and
  `compound` (a boolean result: its `path` is already computed; `winding: "evenodd"` is a fill
  rule).
- **Style**: `fill` (`solid` with `color: { mode, value: { r, g, b }, alpha }`, `pattern` for images,
  `none`), `stroke` (`width`, `align`: `inside`, `outside` or centred, `cap`, `join`, `miterLimit`),
  `opacity`, `filters` (`dropShadow` with `params.dropShadows[{ dx, dy, r, color }]`, and
  `uxdesign#blur`, a background blur), `clipPath: { ref }`.
- **Images**: a `pattern` fill names its image by `pattern.meta.ux.uid`, the entry under
  `resources/` (a hash of the content); `scaleBehavior` is `fill` or `cover`. The `href` is a path
  on the author's machine, no use here.
- **Clip paths** live in the shared resources (`resources.clipPaths[ref].children`), as shapes in the
  group's coordinates.
- **Symbols** (components): an instance is a group with `meta.ux.symbolId` and all its children
  written out, overrides applied: it needs no special handling to be drawn.
- **Text**: XD stores the layout. `text.rawText` is the string; `text.paragraphs[].lines[][]` are
  segments `{ x, y, from, to }`, `y` being the baseline (the node's origin is the baseline of a
  point text's first line) and `from`/`to` positions in `rawText`. The style of each run is in
  `meta.ux.rangedStyles` (`length`, `fontFamily`, `fontStyle`, `fontSize`, `charSpacing` in
  thousandths of an em, `fill` as an `0xAARRGGBB` integer); an area text (`frame.type: "area"`)
  has a `width` and `height`. Glyphs are not stored: the fonts are given to the bake, as for
  `.sketch` and `.penpot`.
- **Artboards taller than their viewport**: `uxdesign#viewport.height` is the visible height of a
  scrolling artboard. Not read yet: such an artboard is baked at its full height.
- **Start point**: `homeArtboard`; without one (some files), XD starts on the top-left artboard.
- **Gradients**: `resources.gradients` was empty in every file we have, so they are not drawn yet.
