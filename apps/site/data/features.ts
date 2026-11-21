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
/** What would validate a feature drawn from our own files only, per format. */
const FIG_CHECK = "a .fig file using it, to compare with Figma's own rendering";
const SKETCH_CHECK = "a .sketch file using it, to compare with Sketch's or Lunacy's rendering";
const PENPOT_CHECK = "a Penpot file using it, to compare with Penpot's own rendering";
const XD_CHECK = "an .xd file using it: the fields are read as documented, not seen in a real file";

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
      xd: unverified("rust: xd_triggers", XD_CHECK, "hover"),
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
      sketch: unverified(
        "e2e: sketch: an artboard taller than its prototype viewport scrolls under fixed layers",
        SKETCH_CHECK,
      ),
      penpot: todo(
        "Penpot shows a board at its size and the browser scrolls the page; a player that keeps a device's height would scroll it",
      ),
      xd: works(
        "e2e: xd: an artboard taller than its viewport scrolls",
        "elements fixed while it scrolls are not read: an .xd file with one would show how XD stores them",
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
      penpot: todo(
        "Penpot's prototype has no action that changes a variant in the files we have; a file with one would show how it is stored",
      ),
      xd: todo(
        "an .xd file with component states, to see how XD stores them and their state-transition links",
      ),
    },
  },
  {
    id: "nested-components",
    group: "Prototype",
    name: "Interactive components inside a component's state",
    formats: {
      fig: unverified(
        "e2e: nested components: a component in another's state, and after delay in a state",
        FIG_CHECK,
      ),
      sketch: na,
      penpot: todo("needs interactive components first"),
      xd: todo("needs component states first"),
    },
  },
  {
    id: "state-delay",
    group: "Prototype",
    name: "After delay inside a component's state",
    formats: {
      fig: unverified(
        "e2e: nested components: a component in another's state, and after delay in a state",
        FIG_CHECK,
      ),
      sketch: na,
      penpot: na,
      xd: todo("needs component states first"),
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
      xd: na,
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
      fig: unverified(
        "e2e: keys and drags: a key combination and a swipe follow their links",
        FIG_CHECK,
        "a swipe follows the link; the screen does not follow the finger",
      ),
      sketch: na,
      penpot: na,
      xd: unverified(
        "e2e: keys and drags: a key combination and a swipe follow their links",
        XD_CHECK,
      ),
    },
  },
  {
    id: "keys",
    group: "Prototype",
    name: "Keyboard triggers, keys and combinations",
    formats: {
      fig: unverified(
        "e2e: keys and drags: a key combination and a swipe follow their links",
        FIG_CHECK,
      ),
      sketch: na,
      penpot: na,
      xd: todo("an .xd file with a key trigger, to see where XD keeps which keys"),
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
    name: "Slide in moves the screen underneath a little; move in does not",
    formats: {
      fig: unverified(
        "e2e: slide in: the screen underneath moves a little too",
        "a side-by-side with Figma's own player, to match how far the screen underneath moves",
      ),
      sketch: na,
      penpot: unverified("rust: penpot_slides", PENPOT_CHECK),
      xd: na,
    },
  },
  {
    id: "smart-animate",
    group: "Transitions",
    name: "Smart animate, auto-animate (layers paired by name)",
    formats: {
      fig: works("e2e: smart animate: paired parts move from one frame to the other, both ways"),
      sketch: na,
      penpot: na,
      xd: works("rust: xd_auto_animate", "the artboard's direct children"),
    },
  },
  {
    id: "smart-animate-nested",
    group: "Transitions",
    name: "Smart animate of nested layers, rotation, corner radii that keep their shape",
    formats: {
      fig: unverified(
        "e2e: smart animate: paired parts move from one frame to the other, both ways",
        FIG_CHECK,
        "three levels deep",
      ),
      sketch: na,
      penpot: na,
      xd: todo(
        "an .xd file with groups that auto-animate, to check XD pairs nested layers the same way",
      ),
    },
  },
  {
    id: "smart-animate-states",
    group: "Transitions",
    name: "Smart animate between component states and overlays",
    formats: {
      fig: unverified(
        "e2e: overlays: a swap with smart animate moves the shared layers",
        FIG_CHECK,
      ),
      sketch: na,
      penpot: na,
      xd: na,
    },
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
      fig: unverified("rust: figma_other_gradients", FIG_CHECK),
      sketch: unverified("rust: sketch_borders_and_masks", SKETCH_CHECK, "radial and angular"),
      penpot: works("rust: penpot_gradients", "radial; Penpot has no angular gradient"),
      xd: works("rust: xd_gradients", "radial; XD has no angular gradient"),
    },
  },
  {
    id: "shadows",
    group: "Drawing",
    name: "Drop shadows, several per layer",
    formats: {
      fig: works("rust: figma_shadows"),
      sketch: works("rust: sketch_draws_shapes"),
      penpot: works("rust: penpot_draws_shapes"),
      xd: works("rust: xd_draws_shapes"),
    },
  },
  {
    id: "inner-shadows",
    group: "Drawing",
    name: "Inner shadows",
    formats: {
      fig: unverified("rust: figma_shadows", FIG_CHECK),
      sketch: unverified("rust: sketch_effects", SKETCH_CHECK),
      penpot: unverified("rust: penpot_effects", PENPOT_CHECK),
      xd: na,
    },
  },
  {
    id: "layer-blur",
    group: "Drawing",
    name: "Layer blur",
    formats: {
      fig: unverified("rust: figma_blurs", FIG_CHECK),
      sketch: unverified(
        "rust: sketch_effects",
        SKETCH_CHECK,
        "motion and zoom blurs drawn as Gaussian",
      ),
      penpot: unverified("rust: penpot_effects", PENPOT_CHECK),
      xd: unverified("rust: xd_blurs", XD_CHECK),
    },
  },
  {
    id: "background-blur",
    group: "Drawing",
    name: "Background blur (frosted glass)",
    formats: {
      fig: unverified("rust: figma_blurs", FIG_CHECK),
      sketch: unverified("rust: sketch_effects", SKETCH_CHECK),
      penpot: na,
      xd: unverified("rust: xd_blurs", XD_CHECK),
    },
  },
  {
    id: "blend-modes",
    group: "Drawing",
    name: "Blend modes",
    formats: {
      fig: unverified("rust: figma_blend_modes", FIG_CHECK),
      sketch: unverified("rust: sketch_effects", SKETCH_CHECK),
      penpot: unverified("rust: penpot_effects", PENPOT_CHECK),
      xd: unverified("rust: xd_blend_and_image_fit", XD_CHECK),
    },
  },
  {
    id: "strokes",
    group: "Drawing",
    name: "Inside and outside strokes",
    formats: {
      fig: works("rust: frame_renders_to_svg_and_png", "Figma stores the stroke's outline"),
      sketch: unverified("rust: sketch_borders_and_masks", SKETCH_CHECK),
      penpot: works("rust: penpot_strokes_and_paths"),
      xd: works("rust: xd_draws_shapes"),
    },
  },
  {
    id: "masks",
    group: "Drawing",
    name: "Masks and clip paths",
    formats: {
      fig: unverified("rust: figma_masks_and_images", FIG_CHECK),
      sketch: works("rust: sketch_draws_shapes"),
      penpot: works("rust: penpot_draws_shapes"),
      xd: works("rust: xd_draws_shapes"),
    },
  },
  {
    id: "alpha-masks",
    group: "Drawing",
    name: "Masks by transparency (alpha)",
    formats: {
      fig: unverified(
        "rust: figma_masks_and_images",
        FIG_CHECK,
        "alpha and luminance; an outline mask is drawn by its alpha",
      ),
      sketch: unverified("rust: sketch_borders_and_masks", SKETCH_CHECK),
      penpot: na,
      xd: na,
    },
  },
  {
    id: "image-fills",
    group: "Drawing",
    name: "Image fills: fill, fit, stretch",
    formats: {
      fig: works("rust: frame_renders_to_svg_and_png"),
      sketch: works("rust: sketch_draws_shapes"),
      penpot: works("rust: penpot_draws_shapes", "Penpot only fills"),
      xd: unverified("rust: xd_blend_and_image_fit", XD_CHECK, "fit and stretch"),
    },
  },
  {
    id: "image-tiles",
    group: "Drawing",
    name: "Image fills: tile and crop",
    formats: {
      fig: unverified("rust: figma_masks_and_images", FIG_CHECK),
      sketch: unverified("rust: sketch_tiles", SKETCH_CHECK, "tiles"),
      penpot: na,
      xd: na,
    },
  },
  {
    id: "svg-raw",
    group: "Drawing",
    name: "Shapes imported from an SVG",
    formats: { fig: na, sketch: na, penpot: works("rust: penpot_svg_raw"), xd: na },
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
      sketch: unverified("rust: sketch_rich_text", SKETCH_CHECK),
      penpot: works("rust: penpot_text"),
      xd: works("rust: xd_text_needs_its_fonts"),
    },
  },
  {
    id: "text-decoration",
    group: "Text",
    name: "Underline and strikethrough",
    formats: {
      fig: works("rust: frame_renders_to_svg_and_png", "the file stores the decoration's outline"),
      sketch: unverified("rust: sketch_rich_text", SKETCH_CHECK),
      penpot: unverified("rust: penpot_text_decoration", PENPOT_CHECK),
      xd: unverified("rust: xd_text_decoration", XD_CHECK),
    },
  },
  {
    id: "embedded-fonts",
    group: "Text",
    name: "Fonts embedded in the design file",
    formats: {
      fig: na,
      sketch: works("rust: sketch_text_needs_its_fonts"),
      penpot: todo(
        "a .penpot file with a custom font, to see how Penpot exports it: none of the public files has one",
      ),
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
    formats: { all: works("rust: a_font_answers_to_regular_expressions") },
  },
  {
    id: "font-config",
    group: "Text",
    name: "A folioscope.config.json the CLI reads",
    formats: { all: works("rust: a_configuration_file_gives_defaults_the_flags_override") },
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
    name: "Fonts in the browser: a folder's manifest, a stylesheet, regular expressions, only the fonts used, FolioscopeFontSet",
    formats: {
      all: works(
        "e2e: fonts: a folder's manifest, a stylesheet, a regular expression, only what is used",
      ),
    },
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
