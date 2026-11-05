---
title: Comparison
description: The same prototype through Figma's embed and through Folioscope, side by side.
---

# Comparison

Showing a prototype on a site today usually means an iframe pointing at the design tool. Here is
the same prototype both ways, loading at the same time, in your browser.

<Comparison />

## Why it is lighter

Folioscope does the heavy work once, when you build your site: it renders each screen to an image
and keeps the links as plain areas on top. A visitor downloads those images and a few kilobytes of
JavaScript; nothing is drawn by a design tool's engine in their browser, and no third-party server is
contacted. The price is that a baked prototype is a picture of the design, not the design: no
layers to inspect, no editing. That is the point.
