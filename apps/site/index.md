---
layout: home
hero:
  name: Folioscope
  text: Flip through your prototypes.
  tagline: Not a viewer for design files, a player for their prototypes. Visitors click through the screens as the designer linked them, from a .fig, .sketch, .xd or .penpot file, on your own site. No account, no licence, no third-party service.
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
    details: Clicks, hovers, overlays, scrolling, flows and transitions play as the designer set them, with keyboard support and reduced motion respected.
  - title: Yours
    details: Everything stays on your site, in plain files that outlive the tools that made them. Made for portfolios and case studies, to preserve your work.
---

<div class="demo">
  <folioscope-player src="/folio/navigation.folio" poster="/folio/navigation.folio/poster.webp" controls></folioscope-player>
</div>

## Why “Folioscope”?

_Folioscope_ is the French word for a flip book: a stack of still pictures that come to life when you thumb through them. That is what this player does with your design files: it takes static screens from a `.fig`, `.sketch`, `.xd` or `.penpot` file and flips through them following the prototype links you designed, so the mockup moves again. And yes, the “folio” is a nod to _portfolio_, where it all started.

::: warning Early days
Folioscope is under development: `.fig` prototypes bake and play today, with click, timed, hover and press links, overlays, scrolling and interactive components; `.sketch` prototypes too. `.xd` and `.penpot` follow. See the [roadmap](https://github.com/baxyz/folioscope/blob/main/PROJECT.md).
:::
