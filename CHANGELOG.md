# Changelog

All notable changes to this project are documented in this file.

## [Unreleased]

## [1.0.1] - 2026-11-23

_No notable change._

## [1.0.0] - 2026-11-23

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

### Features

- **site**: Generate the roadmap and compatibility pages from a feature list
- **bake**: Draw .fig radial, angular and diamond gradients, shadows, blurs, blend modes
- **bake**: Lay out text whose runs have their own style, underline and strikethrough
- **bake**: Draw .sketch shadows, blurs, blend modes, aligned borders, alpha masks, rich text
- **bake**: Draw .penpot shadows, blur, blend, aligned strokes, svg-raw shapes, decorations
- **bake**: Draw .xd shadows, object and background blurs, blend, image fit, decorations
- **player**: Slide the screen underneath a little on a slide, not on a move
- **core**: Read the keys of .fig key triggers into the IR
- **player**: Play key triggers, key combinations included, and drags
- **core**: Scroll .sketch artboards taller than their prototype viewport
- **core**: Read .xd hover triggers
- **core**: Nest interactive components in each other's states, three levels deep
- **player**: Play components nested in a state, and after delay inside a state
- **bake**: Pair smart animate parts by path, with matrices and plain boxes
- **player**: Smart animate turns, keeps round corners, and plays between states and overlays
- **bake**: Substitute fonts by regular expression (/^SF Pro/i), refuse invalid ones
- **cli**: Read folioscope.config.json, the options of the Vite plugin, flags winning
- **bake**: Find Penpot's default font as Source Sans Pro or Source Sans 3
- **core**: List the font families a design file's text uses
- **wasm**: Export families, for the player to fetch only the fonts used
- **player**: Fonts by folder manifest, stylesheet, regex; only those used; FolioscopeFontSet
- **bake**: Draw .fig masks, tiled and cropped images, and .sketch tiled images
- **core**: Read .fig frames that scroll inside a screen, and sticky children
- **bake**: Draw scrollers' content and sticky children as layers of their own
- **player**: Scroll frames inside a screen, and hold sticky layers at the top
- **site**: Add a playground: drop a design file and play it, nothing uploaded
- **site**: Compare against the raw file, a screen recording and PNGs, on a chosen network
- **bake**: Lay out another variant again for the instance's overrides
- **bake**: Lay out .fig text without stored glyphs on screens too, and in `render`
- **site**: Turn the demos into a gallery: pick one, play it, see what to try
- **site**: Remove the early-days notice from the home page
- **site**: Rework the home page: a prototype in the hero, how it works in three steps
- **site**: Compact the compatibility table, give the roadmap a summary and cards
- **site**: Simplify the comparison: Figma against Folioscope, and the other ways in a table

### Bug fixes

- **core**: Read a Penpot slide without offset effect as a move
- **player**: Keep working when a framework sets attributes as properties; play blob: files
- **build**: Build the WebAssembly bake before the site, on Workers Builds too
- **site**: Keep the comparison usable: Firefox's worker, errors, another choice
- **bake**: Draw a state's smart animate parts through the instance's overrides
- **player**: Hide the state left behind during a smart animate between states
- **player**: Bake raw files with the player loaded from a CDN
- **bake**: Draw the component an instance was swapped for, with its look
- **player**: Stack smart animate parts as the screen they go to stacks them
- **bake**: Draw smart animate parts with room for what overflows their box
- **player**: Open only web, e-mail and phone links from a design file
- **core**: Keep only web, e-mail and phone links of a design file
- **bake**: Keep a design file's strings out of the markup and of SVG layers
- **core**: End on a file that loops, read an archive that lists its folders
- **player**: Bake again what an earlier version cached

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

[Unreleased]: https://github.com/baxyz/folioscope/compare/v1.0.1...HEAD
[1.0.1]: https://github.com/baxyz/folioscope/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/baxyz/folioscope/compare/v0.1.0...v1.0.0
[0.1.0]: https://github.com/baxyz/folioscope/compare/v0.0.8...v0.1.0
[0.0.7]: https://github.com/baxyz/folioscope/compare/v0.0.6...v0.0.7
