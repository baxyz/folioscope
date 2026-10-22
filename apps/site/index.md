---
layout: home
hero:
  name: Folioscope
  text: Flip through your prototypes.
  tagline: A web component that plays the interactive prototype in a .fig, .sketch, .xd or .penpot file. No account, no licence, no third-party service.
  actions:
    - theme: brand
      text: Get started
      link: /guide/getting-started
    - theme: alt
      text: Demos
      link: /demos
features:
  - title: Instant
    details: Prototypes are baked ahead of time into layered images. Visitors get a few kilobytes of JavaScript, no WebAssembly and no fonts.
  - title: Interactive
    details: Clicks, back, flows and timed transitions play as the designer set them, with keyboard support and reduced motion respected.
  - title: Yours
    details: Everything stays on your site. Made for portfolios and case studies, and for design files whose tools you no longer have.
---

<div class="demo">
  <folioscope-player src="/folio/navigation.folio" poster="/folio/navigation.folio/poster.webp" controls></folioscope-player>
</div>

## Why “Folioscope”?

_Folioscope_ is the French word for a flip book: a stack of still pictures that come to life when you thumb through them. That is what this player does with your design files: it takes static screens from a `.fig`, `.sketch`, `.xd` or `.penpot` file and flips through them following the prototype links you designed, so the mockup moves again. And yes, the “folio” is a nod to _portfolio_, where it all started.

::: warning Early days
Folioscope is under development: `.fig` prototypes bake and play today, with click, timed, hover and press links, overlays and scrolling. The other formats follow. See the [roadmap](https://github.com/baxyz/folioscope/blob/main/PROJECT.md).
:::
