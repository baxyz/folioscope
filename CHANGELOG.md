# Changelog

All notable changes to this project are documented in this file.

## [Unreleased]

## [0.1.0] - 2026-11-20

Flat screens are now served as SVG when that is lighter than their images: sharp at any zoom,
one file for every density (ADR 0006; `--no-svg`, `svg: false`). The states baked per interactive
component are capped at 16 by default, with a warning past it (ADR 0007; `--max-states`,
`maxStates`). Penpot texts whose font family is quoted now find their font.

### Features

- **player**: ✨ give the full player fonts: folioscope-font, shared sets, registerFont
- **bake**: ✨ play .xd prototypes: artboards, links, transitions, auto-animate, text
- **cli**: ✨ say more about a font than its file does: --family, --match, --weight, --italic
- **vite**: ✨ give the fonts option a family, match rules, a weight and a style
- **bake**: ✨ scroll .xd artboards taller than their viewport
- **player**: ✨ close a toggled overlay when it is already open (Penpot's toggle-overlay)
- **bake**: ✨ play .xd overlays: opened at their position, closed by previous artboard
- **bake**: ✨ draw .xd linear and radial gradients, on shapes and as an artboard's fill
- **site**: ✨ put the early-days notice on top and draw a flip book beside the name
- **site**: ✨ add a comparison page: the CO₂ prototype in Figma's embed and in Folioscope, side by side
- **bake**: ✨ serve flat layers as SVG and cap the states per component
- **cli**: ✨ add --no-svg and --max-states, warn about cut components
- **wasm**: ✨ take svg and max_states, return the cut components
- **vite**: ✨ add the svg and maxStates options, warn about cut components
- **player**: ✨ load a layer served as SVG from its single file

### Bug fixes

- **test**: 🐛 build the tall artboard test only with the bake feature
- **bake**: 🐛 leave out frames too large to be an image and pages with no flow
- **player**: 🐛 show the controls when a framework sets them before inserting the element
- **bake**: 🐛 draw .fig shared fill styles, text runs, boolean icons and instance visibility
- **site**: 🐛 stack the flip book's pages by z-index and close its loop with a fade
- **bake**: 🐛 find a Penpot text's font when its family is written with CSS quotes
- **bake**: 🐛 never serve an SVG layer heavier than its lightest image

### Documentation

- **docs**: 📝 say why read_font is not adopted yet
- **docs**: 📝 document the .xd graphics, the demo and what is left
- **docs**: 📝 keep only what is left to do in the helpers4 notes
- **docs**: 📝 note the font hints of the CLI and the Vite plugin
- **site**: 📝 add a compatibility page and say .xd plays on the home page
- **docs**: 📝 say .xd artboards scroll, in the format notes, the roadmap and the site
- **docs**: 📝 note what the new real .xd files taught: overlays, gradients, blurs, states
- **site**: 📝 describe the svg and maxStates options of folioscope/vite
- **docs**: 📝 settle D6 and D7 in ADRs 0006 and 0007, measured on real files
- **docs**: 📝 drop settled decisions, done items and helpers4 requests

## [0.0.8] - 2026-10-31

### Documentation

- **ai**: 📝 describe the release of one crate, not three

## [0.0.7] - 2026-10-30

### Bug fixes

- **build**: 🐛 keep the element's registration when player.ts holds it

## [0.0.6] - 2026-10-30

_No notable change._

## [0.0.5] - 2026-10-30

### Documentation

- **docs**: 📝 ask helpers4 for every generic helper Folioscope writes
