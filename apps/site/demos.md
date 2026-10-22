# Demos

Each demo is one of the test files in the repository, baked with `folioscope bake` and played by the player as published.

## Navigation

Two screens: the blue button opens the details with a dissolve, the grey button comes back.

<div class="demo">
  <folioscope-player src="/folio/navigation.folio" poster="/folio/navigation.folio/poster.webp" controls></folioscope-player>
</div>

```html
<folioscope-player src="/folio/navigation.folio" controls></folioscope-player>
```

## Flows and pages

Two pages, three flows: pick them in the control bar. Push and slide transitions.

<div class="demo">
  <folioscope-player src="/folio/flows-and-pages.folio" poster="/folio/flows-and-pages.folio/poster.webp" controls></folioscope-player>
</div>

```html
<folioscope-player src="/folio/flows-and-pages.folio" flow="Log in" controls></folioscope-player>
```

## After a delay, while hovering, while pressing

A splash screen that moves on by itself after 1.5 seconds. Then hover the button, and press it:
each state is a frame of its own, shown while the pointer stays over the button or the button is
held. On a touch screen, there is no hover: a tap shows the pressed state.

<div class="demo">
  <folioscope-player src="/folio/triggers.folio" poster="/folio/triggers.folio/poster.webp" controls></folioscope-player>
</div>

## Overlay

A menu opens over the screen, moving in from the bottom, centered on a dimmed backdrop. Close it
with its button, or with a click outside.

<div class="demo">
  <folioscope-player src="/folio/overlay.folio" poster="/folio/overlay.folio/poster.webp" controls></folioscope-player>
</div>
