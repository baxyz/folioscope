---
title: Compatibility
description: What Folioscope plays today for each design format.
---

# Compatibility

What the player does with each format today. The four formats do not all describe the same
prototypes: Sketch only has click links, for instance, so some rows say _not in the format_ rather
than _not yet_.

- **Plays**: baked and played, checked by a test and in the design tool or on a real file.
- **Plays, unverified**: checked by a test on files we made, not yet in the design tool.
- **Not yet**: the format has it, Folioscope does not play it yet; the [Roadmap](/roadmap) says
  what it needs.
- **Not in the format**: the design tool has no such feature.

A `.fig` file stores the outline of every glyph, so its text is drawn without any font. For the
other formats, give the bake the fonts the design uses (`--fonts`, or the `fonts` option of the
Vite plugin).

<FeatureMatrix />
