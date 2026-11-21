---
title: Playground
description: Drop a .fig, .sketch, .xd or .penpot file and play its prototype, in your browser.
---

# Playground

Drop a design file to play its prototype. **Nothing is uploaded**: the file stays in your
browser, which bakes it (WebAssembly, in a worker) and keeps the result in its cache for your next
visit. A `.fig` file plays as it is; `.sketch`, `.xd` and `.penpot` files need the fonts their text
uses: give them below the file.

<Playground />

For your own site, bake the file at build time instead: visitors then download a few images and
under 15 KB of JavaScript, see [Getting started](/guide/getting-started).
