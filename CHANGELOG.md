# Changelog

All notable changes to this project are documented in this file.

## [Unreleased]

**The first stable release.** From 1.0, Folioscope follows [semantic versioning](https://semver.org)
for what people build on: the `<folioscope-player>` element (its attributes, methods and events),
the `.folio` format, the options of `folioscope/vite` and the `folioscope` command line. The Rust
library behind the command line (`folioscope::…`) is still settling: it may change in a minor
release.

Since 0.1.0, the four formats draw gradients, shadows, blurs, blend modes and masks; prototypes
play key and drag triggers, components nested in each other's states, frames that scroll inside a
screen with sticky layers, and smart animate between states and overlays; fonts can come from a
folder, a stylesheet or a pattern, fetched only when used; the command line reads
`folioscope.config.json`. The site has a playground, a gallery of demos and an API reference.

**Security.** A design file is no longer trusted with the page that plays it, the site that
serves it or the machine that bakes it. Before this version, a crafted file could:

- run a script in the page playing it, through an "open link" written `javascript:…` (every
  format with links, baked or played raw);
- put a `<script>` into a baked SVG layer, which ran when that `.svg` was opened on its own
  (`.penpot` and `.xd`), or make the bake read an image file of the machine;
- crash `bake` and `inspect` with a shape or a symbol that contains itself (`.penpot`, `.sketch`).

If you baked files you did not make with 0.1.0 or earlier, bake them again. Links now open web,
e-mail and phone addresses only; a layer is served as SVG only when it holds shapes alone. See
`SECURITY.md`.

The npm package, the JSR package and the crate now carry the text of the licence, and the npm
package the licences of the crates and packages it bundles (`THIRD-PARTY-LICENSES.md`).

## [0.1.0] - 2026-11-20

`.xd` prototypes play: artboards, links, transitions, auto-animate, overlays, scrolling. Flat
screens are served as SVG when that is lighter than their images: sharp at any zoom, one file for
every density (`--no-svg`, `svg: false`). The states baked per interactive component are capped
at 16 by default, with a warning past it (`--max-states`, `maxStates`). The full player takes
fonts for the text a file stores no glyphs for.

### Features

- **bake**: Play .xd prototypes: artboards, links, transitions, auto-animate, text
- **bake**: Scroll .xd artboards taller than their viewport
- **bake**: Play .xd overlays: opened at their position, closed by previous artboard
- **bake**: Draw .xd linear and radial gradients, on shapes and as an artboard's fill
- **bake**: Serve flat layers as SVG and cap the states per component
- **cli**: Add --no-svg and --max-states, warn about cut components
- **cli**: Say more about a font than its file does: --family, --match, --weight, --italic
- **vite**: Add the svg and maxStates options, warn about cut components
- **vite**: Give the fonts option a family, match rules, a weight and a style
- **wasm**: Take svg and max_states, return the cut components
- **player**: Give the full player fonts: folioscope-font, shared sets, registerFont
- **player**: Load a layer served as SVG from its single file
- **player**: Close a toggled overlay when it is already open (Penpot's toggle-overlay)
- **site**: Add a comparison page: the CO₂ prototype in Figma's embed and in Folioscope, side by side
- **site**: Add a compatibility page

### Bug fixes

- **bake**: Leave out frames too large to be an image and pages with no flow
- **bake**: Draw .fig shared fill styles, text runs, boolean icons and instance visibility
- **bake**: Find a Penpot text's font when its family is written with CSS quotes
- **bake**: Never serve an SVG layer heavier than its lightest image
- **player**: Show the controls when a framework sets them before inserting the element

## [0.0.7] - 2026-10-30

### Bug fixes

- **build**: Keep the element's registration when player.ts holds it

## Earlier versions

0.0.2 to 0.0.6 and 0.0.8 (October 2026) set up the packages and their publishing to npm, JSR and
crates.io, with nothing to note for the people using them.

[Unreleased]: https://github.com/baxyz/folioscope/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/baxyz/folioscope/compare/v0.0.8...v0.1.0
[0.0.7]: https://github.com/baxyz/folioscope/compare/v0.0.6...v0.0.7
