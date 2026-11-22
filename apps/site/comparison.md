---
title: Comparison
description: "The same prototype through Figma's embed and through Folioscope, side by side, and the other ways to show a prototype on a site."
---

# Comparison

## Figma's embed and Folioscope

The same Figma prototype, loaded at the same time in your browser: on the left, Figma's own
embed; on the right, the same file baked by Folioscope.

<Comparison />

Folioscope does the heavy work once, when the site is built: it turns each screen into images and
keeps the links as plain areas on top. Your visitors download those images and a few kilobytes of
JavaScript; no design tool runs in their browser, and no third-party server is contacted.

## Other ways to show a prototype

What each design tool offers to show a prototype on your own page, and the usual stand-ins. Only
Figma's embed can be loaded live here: the others need the file hosted on the tool's own service,
under an account.

| Way                                                                                               | Clickable | Where it runs                 | What it needs                                                      |
| ------------------------------------------------------------------------------------------------- | --------- | ----------------------------- | ------------------------------------------------------------------ |
| **Figma** embed ([docs](https://developers.figma.com/docs/embeds/embed-figma-prototype))          | yes       | Figma's servers, in an iframe | the file on Figma, shared publicly                                 |
| **Sketch** prototype link ([docs](https://www.sketch.com/docs/prototyping/sharing-prototypes/))   | yes       | Sketch's web app              | a Sketch workspace; embedding through oEmbed                       |
| **Penpot** view mode link ([docs](https://help.penpot.app/user-guide/view-mode/))                 | yes       | the Penpot instance           | the file on Penpot; no official embed                              |
| **Adobe XD** shared link ([docs](https://helpx.adobe.com/xd/help/share-prototypes.html))          | yes       | Adobe's servers, in an iframe | Creative Cloud; XD is in maintenance mode                          |
| Open-source `.sketch` viewer ([sketch-web-viewer](https://github.com/AnimaApp/sketch-web-viewer)) | no        | your page                     | a still view of old Sketch files; unmaintained since 2019          |
| Screen recording                                                                                  | no        | your page                     | one path through the prototype, as a video                         |
| One picture per screen                                                                            | no        | your page                     | an export of each screen                                           |
| **Folioscope**, design file in the page                                                           | yes       | your page                     | the design file and its fonts published; 2 MB of WebAssembly, once |
| **Folioscope**, baked `.folio`                                                                    | yes       | your page                     | a bake at build time                                               |

The design tools' links run their own player: the prototype plays exactly as in the tool, and
stays on their service. Folioscope plays a picture of each screen on your page, with no account
and nothing to keep paying for; what it plays of each format is on the
[Compatibility](/compatibility) page.
