// SPDX-License-Identifier: AGPL-3.0-or-later
// Every feature of Folioscope and its status, per design format where it depends on one. The
// Roadmap and Compatibility pages are generated from this file; change it in the same pull
// request as the feature. `test` names the test that proves it: an end-to-end scenario
// (`e2e: <its ✓ line>`), a Rust test (`rust: <fn name>`) or a Vitest one (`vitest: <its name>`);
// `packages/folioscope/test/features.test.ts` checks that each one exists.

export type Format = "fig" | "sketch" | "penpot" | "xd";

export const FORMATS: { id: Format; name: string }[] = [
  { id: "fig", name: "Figma .fig" },
  { id: "sketch", name: "Sketch .sketch" },
  { id: "penpot", name: "Penpot .penpot" },
  { id: "xd", name: "Adobe XD .xd" },
];

export type Status =
  /** Tested, and checked in the real design tool or on a real file. */
  | { status: "works"; test: string; note?: string }
  /** Tested on files we generated, not yet checked in the real tool or on a real file. */
  | { status: "unverified"; test: string; missing: string; note?: string }
  /** Not done: `missing` says what it needs (often a file to test it with). */
  | { status: "todo"; missing: string }
  /** The design tool has no such feature. */
  | { status: "na" };

export interface Feature {
  id: string;
  group: string;
  name: string;
  /** Per format; or `all` for what does not depend on the format. */
  formats: Partial<Record<Format, Status>> | { all: Status };
}

const works = (test: string, note?: string): Status => ({ status: "works", test, note });
const unverified = (test: string, missing: string, note?: string): Status => ({
  status: "unverified",
  test,
  missing,
  note,
});
const todo = (missing: string): Status => ({ status: "todo", missing });
const na: Status = { status: "na" };

const NAV = "e2e: the hotspot navigates to Details, with its transition";

export const FEATURES: Feature[] = [
  // Prototype
  {
    id: "links",
    group: "Prototype",
    name: "Click or tap links between screens",
    formats: {
      fig: works(NAV),
      sketch: works("e2e: sketch: a .sketch prototype plays the same way"),
      penpot: works("e2e: penpot: a .penpot prototype plays the same way"),
      xd: works("e2e: xd: an .xd prototype plays the same way"),
    },
  },
  {
    id: "flows",
    group: "Prototype",
    name: "Several flows and pages, with pickers",
    formats: {
      fig: works("e2e: controls: flow and page pickers, restart, back state"),
      sketch: works("rust: sketch_flows_and_pages"),
      penpot: works("rust: penpot_flows_and_pages"),
      xd: todo(
        "XD's named flows are not read, only its home artboard; an .xd file with several flows would show how they are stored",
      ),
    },
  },
  {
    id: "back",
    group: "Prototype",
    name: "Back, previous screen",
    formats: {
      fig: works("e2e: the back hotspot returns to Home"),
      sketch: works("e2e: sketch: a .sketch prototype plays the same way"),
      penpot: works("e2e: penpot: a .penpot prototype plays the same way"),
      xd: works("e2e: xd: an overlay opens at its position and closes with previous artboard"),
    },
  },
  {
    id: "hover-press",
    group: "Prototype",
    name: "Hover and press",
    formats: {
      fig: works("e2e: hover and press: a frame while they last, then back"),
      sketch: na,
      penpot: works("rust: penpot_triggers"),
      xd: todo("XD has hover triggers; none of our files uses one"),
    },
  },
  {
    id: "delay",
    group: "Prototype",
    name: "After a delay",
    formats: {
      fig: works("e2e: hover and press: a frame while they last, then back"),
      sketch: na,
      penpot: works("rust: penpot_triggers"),
      xd: works("rust: xd_triggers"),
    },
  },
  {
    id: "overlays",
    group: "Prototype",
    name: "Overlays: open, close, swap, toggle",
    formats: {
      fig: works("e2e: overlays: centered on a backdrop, closed by a button or a click outside"),
      sketch: na,
      penpot: works("rust: penpot_overlays"),
      xd: works("e2e: xd: an overlay opens at its position and closes with previous artboard"),
    },
  },
  {
    id: "scroll",
    group: "Prototype",
    name: "Screens that scroll, with fixed elements",
    formats: {
      fig: works(
        "e2e: scroll: the content scrolls under fixed layers, scroll to reaches its target",
      ),
      sketch: todo("Sketch's prototypeViewport is not read yet"),
      penpot: todo("scrolling boards are not read yet"),
      xd: works(
        "e2e: xd: an artboard taller than its viewport scrolls",
        "elements fixed while the artboard scrolls are not read yet",
      ),
    },
  },
  {
    id: "nested-scroll",
    group: "Prototype",
    name: "A frame that scrolls inside a screen, sticky elements",
    formats: {
      fig: todo("not played yet"),
      sketch: na,
      penpot: todo("not played yet"),
      xd: todo("not played yet"),
    },
  },
  {
    id: "components",
    group: "Prototype",
    name: "Interactive components (variants changed on click, hover, press)",
    formats: {
      fig: works("e2e: components: variants change while hovering, while pressing and on click"),
      sketch: na,
      penpot: todo("Penpot 2 variants are not read as interactive components yet"),
      xd: todo("XD component states are not read yet"),
    },
  },
  {
    id: "nested-components",
    group: "Prototype",
    name: "Interactive components inside a component's state",
    formats: {
      fig: todo("drawn, not interactive yet"),
      sketch: na,
      penpot: todo("needs variants first"),
      xd: todo("needs states first"),
    },
  },
  {
    id: "state-cap",
    group: "Prototype",
    name: "At most 16 states per component (configurable), with a warning past it",
    formats: { all: works("rust: states_past_the_cap_are_left_out_and_reported") },
  },
  {
    id: "open-url",
    group: "Prototype",
    name: "Open a link",
    formats: {
      fig: works("rust: figma_ir_reads_open_url"),
      sketch: na,
      penpot: works("rust: penpot_triggers"),
      xd: todo("not read yet"),
    },
  },
  {
    id: "variables",
    group: "Prototype",
    name: "Prototype variables, conditions and expressions",
    formats: {
      fig: todo("no .fig file with prototype variables to test with"),
      sketch: na,
      penpot: na,
      xd: na,
    },
  },
  {
    id: "gestures",
    group: "Prototype",
    name: "Drag and swipe gestures",
    formats: {
      fig: todo("drag triggers are read, not played"),
      sketch: na,
      penpot: na,
      xd: todo("drag triggers are read, not played"),
    },
  },
  {
    id: "keys",
    group: "Prototype",
    name: "Keyboard triggers",
    formats: {
      fig: todo("key triggers are read, not played"),
      sketch: na,
      penpot: na,
      xd: todo("key triggers are read, not played"),
    },
  },

  // Transitions
  {
    id: "screen-transitions",
    group: "Transitions",
    name: "Dissolve, slide, push, move in and out",
    formats: {
      fig: works(NAV),
      sketch: works("e2e: sketch: a .sketch prototype plays the same way", "Sketch only slides"),
      penpot: works("e2e: penpot: a .penpot prototype plays the same way"),
      xd: works("e2e: xd: an .xd prototype plays the same way"),
    },
  },
  {
    id: "slide-in",
    group: "Transitions",
    name: "Slide in moves the screen below a little; move in does not",
    formats: {
      fig: todo("both slide the new screen only"),
      sketch: na,
      penpot: todo("both slide the new screen only"),
      xd: na,
    },
  },
  {
    id: "smart-animate",
    group: "Transitions",
    name: "Smart animate, auto-animate (layers paired by name)",
    formats: {
      fig: works(
        "e2e: smart animate: paired parts move from one frame to the other, both ways",
        "direct children only",
      ),
      sketch: na,
      penpot: na,
      xd: works("rust: xd_auto_animate", "direct children only"),
    },
  },
  {
    id: "smart-animate-nested",
    group: "Transitions",
    name: "Smart animate of nested layers, rotation, corner radii",
    formats: {
      fig: todo("direct children only"),
      sketch: na,
      penpot: na,
      xd: todo("direct children only"),
    },
  },
  {
    id: "smart-animate-states",
    group: "Transitions",
    name: "Smart animate between component states and overlays",
    formats: { fig: todo("states and overlays dissolve"), sketch: na, penpot: na, xd: na },
  },
  {
    id: "reduced-motion",
    group: "Transitions",
    name: "prefers-reduced-motion turns transitions into cuts",
    formats: { all: works("vitest: reduced motion") },
  },

  // Drawing
  {
    id: "shapes",
    group: "Drawing",
    name: "Shapes, colour fills, images",
    formats: {
      fig: works("rust: frame_renders_to_svg_and_png"),
      sketch: works("rust: sketch_draws_shapes"),
      penpot: works("rust: penpot_draws_shapes"),
      xd: works("rust: xd_draws_shapes"),
    },
  },
  {
    id: "gradients",
    group: "Drawing",
    name: "Linear gradients",
    formats: {
      fig: works("rust: figma_linear_gradient"),
      sketch: works("rust: sketch_gradients"),
      penpot: works("rust: penpot_gradients"),
      xd: works("rust: xd_gradients"),
    },
  },
  {
    id: "radial-gradients",
    group: "Drawing",
    name: "Radial, angular and diamond gradients",
    formats: {
      fig: todo("drawn with their first colour"),
      sketch: works(
        "rust: sketch_gradients",
        "radial only; Sketch's angular gradient is not drawn",
      ),
      penpot: works("rust: penpot_gradients", "radial; Penpot has no angular gradient"),
      xd: works("rust: xd_gradients", "radial; XD has no angular gradient"),
    },
  },
  {
    id: "shadows",
    group: "Drawing",
    name: "Drop shadows",
    formats: {
      fig: works("rust: frame_renders_to_svg_and_png"),
      sketch: works("rust: sketch_draws_shapes", "the first shadow only"),
      penpot: works("rust: penpot_draws_shapes"),
      xd: works("rust: xd_draws_shapes"),
    },
  },
  {
    id: "inner-shadows",
    group: "Drawing",
    name: "Inner shadows",
    formats: {
      fig: todo("not drawn"),
      sketch: todo("not drawn"),
      penpot: todo("not drawn"),
      xd: na,
    },
  },
  {
    id: "blurs",
    group: "Drawing",
    name: "Layer and background blurs",
    formats: {
      fig: todo("not drawn"),
      sketch: todo("not drawn"),
      penpot: todo("not drawn"),
      xd: todo("not drawn"),
    },
  },
  {
    id: "blend-modes",
    group: "Drawing",
    name: "Blend modes",
    formats: {
      fig: todo("not drawn"),
      sketch: todo("not drawn"),
      penpot: todo("not drawn"),
      xd: todo("not drawn"),
    },
  },
  {
    id: "strokes",
    group: "Drawing",
    name: "Inside and outside strokes",
    formats: {
      fig: works("rust: frame_renders_to_svg_and_png", "Figma stores the stroke's outline"),
      sketch: todo("drawn centred"),
      penpot: todo("drawn centred"),
      xd: works("rust: xd_draws_shapes"),
    },
  },
  {
    id: "masks",
    group: "Drawing",
    name: "Masks and clip paths",
    formats: {
      fig: works("rust: frame_renders_to_svg_and_png"),
      sketch: works("rust: sketch_draws_shapes", "masks by alpha are not drawn"),
      penpot: works("rust: penpot_draws_shapes"),
      xd: works("rust: xd_draws_shapes"),
    },
  },
  {
    id: "image-fills",
    group: "Drawing",
    name: "Image fills: fill, fit, crop, tile",
    formats: {
      fig: works("rust: frame_renders_to_svg_and_png", "fill and fit"),
      sketch: works("rust: sketch_draws_shapes", "fill"),
      penpot: works("rust: penpot_draws_shapes", "fill"),
      xd: works("rust: xd_draws_shapes", "cover only"),
    },
  },
  {
    id: "svg-raw",
    group: "Drawing",
    name: "Raw SVG shapes",
    formats: { fig: na, sketch: na, penpot: todo("svg-raw shapes are not drawn"), xd: na },
  },
  {
    id: "variant-size",
    group: "Drawing",
    name: "A variant other than the one in the file, with its overrides and layout",
    formats: {
      fig: todo("drawn at its own size, without the overrides' layout"),
      sketch: na,
      penpot: na,
      xd: na,
    },
  },

  // Text
  {
    id: "text",
    group: "Text",
    name: "Text",
    formats: {
      fig: works(
        "rust: frame_renders_to_svg_and_png",
        "no font needed: the file stores the glyphs",
      ),
      sketch: works("rust: sketch_text_needs_its_fonts", "needs the fonts it uses"),
      penpot: works("rust: penpot_text", "needs the fonts it uses"),
      xd: works("rust: xd_text_needs_its_fonts", "needs the fonts it uses"),
    },
  },
  {
    id: "text-runs",
    group: "Text",
    name: "Several styles within one text",
    formats: {
      fig: works("rust: frame_renders_to_svg_and_png"),
      sketch: todo("a text takes its first run's style"),
      penpot: works("rust: penpot_text"),
      xd: works("rust: xd_text_needs_its_fonts"),
    },
  },
  {
    id: "text-decoration",
    group: "Text",
    name: "Underline and strikethrough",
    formats: {
      fig: works("rust: frame_renders_to_svg_and_png"),
      sketch: todo("not drawn"),
      penpot: todo("not drawn"),
      xd: todo("not drawn"),
    },
  },
  {
    id: "embedded-fonts",
    group: "Text",
    name: "Fonts embedded in the design file",
    formats: {
      fig: na,
      sketch: works("rust: sketch_text_needs_its_fonts"),
      penpot: todo("custom fonts in objects/ are not read yet"),
      xd: na,
    },
  },
  {
    id: "font-options",
    group: "Text",
    name: "Fonts for the bake: --fonts, family, match patterns, weight, style",
    formats: { all: works("vitest: passes what the fonts option says about its fonts") },
  },
  {
    id: "font-regex",
    group: "Text",
    name: "Font substitution by regular expression (/^SF Pro/i)",
    formats: { all: todo("not read yet") },
  },
  {
    id: "font-config",
    group: "Text",
    name: "A folioscope.config file read by the CLI",
    formats: { all: todo("not read yet") },
  },

  // Player
  {
    id: "lite-player",
    group: "Player",
    name: "Lite player: plays a .folio, under 15 KB of JavaScript, no WebAssembly",
    formats: { all: works("e2e: starts on the flow's first frame (Home)") },
  },
  {
    id: "first-frame",
    group: "Player",
    name: "First frame under 1 s on 4G",
    formats: {
      all: works("e2e: first frame under ${BUDGET_MS} ms on 4G with the documented setup"),
    },
  },
  {
    id: "placeholder",
    group: "Player",
    name: "Poster and blurred placeholder",
    formats: { all: works("e2e: placeholder: a blurred preview until the first frame is decoded") },
  },
  {
    id: "controls",
    group: "Player",
    name: "Controls: restart, back, pages, flows, fullscreen",
    formats: { all: works("e2e: controls: flow and page pickers, restart, back state") },
  },
  {
    id: "hotspot-hints",
    group: "Player",
    name: "Show the clickable areas",
    formats: {
      all: works("e2e: hotspots: a click beside them flashes them, unless hotspots=never"),
    },
  },
  {
    id: "full-player",
    group: "Player",
    name: "Full player: a raw design file baked in the browser, then cached",
    formats: {
      all: works("e2e: full player: a raw .fig plays, baked in the browser, then from the cache"),
    },
  },
  {
    id: "browser-fonts",
    group: "Player",
    name: "Fonts in the browser: <folioscope-font> (src, family, match), shared sets, registerFont()",
    formats: {
      all: works(
        "e2e: fonts: <folioscope-font>, a family or a pattern, a shared set and registerFont()",
      ),
    },
  },
  {
    id: "browser-fonts-modes",
    group: "Player",
    name: "Fonts in the browser: base (manifest) and css modes, regex match, lazy loading, FolioscopeFontSet",
    formats: { all: todo("not done yet") },
  },
  {
    id: "dynamic-layers",
    group: "Player",
    name: "Dynamic layers (variables, conditions) drawn on the fly",
    formats: {
      fig: todo("no .fig file with prototype variables to test with"),
      sketch: na,
      penpot: na,
      xd: na,
    },
  },
  {
    id: "keyboard",
    group: "Accessibility",
    name: "Keyboard: hotspots focusable, Enter follows, Escape goes back",
    formats: { all: works("e2e: keyboard: Enter follows a hotspot, Escape goes back") },
  },
  {
    id: "screen-readers",
    group: "Accessibility",
    name: "Screen readers",
    formats: { all: todo("labels are set; not checked with a screen reader yet") },
  },

  // Bake
  {
    id: "svg-layers",
    group: "Bake",
    name: "Flat layers served as SVG when lighter",
    formats: { all: works("rust: navigation_bakes_to_a_folio_folder") },
  },
  {
    id: "vite",
    group: "Bake",
    name: "Vite plugin: bakes at build time, serves in dev, caches",
    formats: {
      all: works("vitest: bakes design files into the build output, then reuses the cache"),
    },
  },
  {
    id: "compat-from-tests",
    group: "Site",
    name: "This page generated from the tests",
    formats: { all: works("vitest: every feature that works names a test that exists") },
  },
  {
    id: "comparison",
    group: "Site",
    name: "Comparison page: Folioscope against other ways to show a prototype",
    formats: {
      all: works("e2e: comparison", "the CO₂ prototype, Figma's embed against the .folio"),
    },
  },
];
