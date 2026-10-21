# The `.xd` format

Working notes on the parts of the Adobe XD format Folioscope reads. Adobe never documented it and
XD has been frozen since 2023, so these notes come from decoding five public files found on
GitHub (manifest format 6, interactions versions 0.22 to 0.24, 211 interactions), kept locally in
the git-ignored `fixtures/external/xd/`. We cannot create `.xd` fixtures ourselves: tests rely on
such real files when present.

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
