---
title: Comparison
description: The same prototype through Folioscope and through the other ways to show one on a site, side by side.
---

# Comparison

Showing a prototype on a site today usually means an iframe pointing at the design tool, a screen
recording, or a picture of each screen. Here is the same prototype each way next to Folioscope,
loading at the same time, in your browser, on the network you pick.

<Comparison />

## Why it is lighter

Folioscope does the heavy work once, when you build your site: it renders each screen to an image
and keeps the links as plain areas on top. A visitor downloads those images and a few kilobytes of
JavaScript; nothing is drawn by a design tool's engine in their browser, and no third-party server is
contacted. The price is that a baked prototype is a picture of the design, not the design: no
layers to inspect, no editing. That is the point.

The full player can take the design file itself and bake it in the browser, for a file you cannot
bake ahead (a visitor's own, in the [Playground](/playground)): it costs the WebAssembly bake, once
per visitor.
