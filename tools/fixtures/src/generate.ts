// SPDX-License-Identifier: AGPL-3.0-or-later
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { FigmaBuilder } from "./figma.ts";
import { PenpotBuilder } from "./penpot.ts";
import { SketchBuilder } from "./sketch.ts";
import { XdBuilder } from "./xd.ts";

const FIXTURES = join(dirname(fileURLToPath(import.meta.url)), "../../../fixtures");

const PHONE = { width: 375, height: 812 };
const BLUE = "#2563EB";
const GREY = "#E5E7EB";

/** Two screens, a button to go forward, a button to go back. One flow. */
function navigation(): FigmaBuilder {
  const figma = new FigmaBuilder();
  const page = figma.page("Prototype");

  const home = figma.frame(page, "Home", { x: 0, y: 0, ...PHONE });
  const details = figma.frame(page, "Details", { x: 475, y: 0, ...PHONE }, "#F9FAFB");
  figma.flowStart(home, "Main flow", "Open the details screen and come back");

  figma.rect(home, "Header", { x: 0, y: 0, width: 375, height: 96 }, GREY);
  const open = figma.rect(
    home,
    "Open details",
    { x: 88, y: 640, width: 200, height: 48 },
    BLUE,
    24,
  );
  figma.on(open, "ON_CLICK", {
    kind: "navigate",
    to: details,
    transition: { type: "DISSOLVE", durationMs: 300, easing: "OUT_CUBIC" },
  });

  const back = figma.rect(details, "Back", { x: 16, y: 56, width: 80, height: 32 }, GREY, 8);
  figma.on(back, "ON_CLICK", { kind: "back" });

  return figma;
}

/**
 * Smart animate: the card of Home grows into the header of Details, the title fades out and a body
 * fades in; the back button returns the same way.
 */
function smartAnimate(): FigmaBuilder {
  const figma = new FigmaBuilder();
  const page = figma.page("Prototype");
  const home = figma.frame(page, "Home", { x: 0, y: 0, ...PHONE });
  const details = figma.frame(page, "Details", { x: 475, y: 0, ...PHONE });
  figma.flowStart(home, "Smart animate flow");

  figma.rect(home, "Title", { x: 24, y: 64, width: 200, height: 32 }, GREY, 8);
  figma.rect(home, "Card", { x: 24, y: 128, width: 160, height: 120 }, BLUE, 16);
  const open = figma.rect(home, "Open", { x: 88, y: 640, width: 200, height: 48 }, "#1E3A8A", 24);
  figma.on(open, "ON_CLICK", {
    kind: "navigate",
    to: details,
    transition: { type: "SMART_ANIMATE", durationMs: 400, easing: "INOUT_CUBIC" },
  });

  figma.rect(details, "Card", { x: 0, y: 0, width: 375, height: 320 }, BLUE, 0);
  figma.rect(details, "Body", { x: 24, y: 352, width: 327, height: 240 }, GREY, 12);
  const back = figma.rect(details, "Back", { x: 16, y: 56, width: 80, height: 32 }, "#FFFFFF", 16);
  figma.on(back, "ON_CLICK", { kind: "back" });

  return figma;
}

/** A menu opened as a centered overlay over a dimmed backdrop, closed by a button or outside. */
function overlay(): FigmaBuilder {
  const figma = new FigmaBuilder();
  const page = figma.page("Prototype");

  const home = figma.frame(page, "Home", { x: 0, y: 0, ...PHONE });
  const menu = figma.frame(page, "Menu", { x: 475, y: 0, width: 280, height: 320 }, "#FFFFFF");
  figma.flowStart(home, "Overlay flow");
  figma.overlay(menu, { position: "CENTER", closeOnClickOutside: true, backdrop: "#00000066" });

  const open = figma.rect(home, "Open menu", { x: 88, y: 640, width: 200, height: 48 }, BLUE, 24);
  figma.on(open, "ON_CLICK", {
    kind: "overlay",
    to: menu,
    transition: { type: "MOVE_FROM_BOTTOM", durationMs: 250, easing: "OUT_CUBIC" },
  });
  for (let i = 0; i < 3; i++) {
    figma.rect(menu, `Item ${i + 1}`, { x: 24, y: 24 + i * 64, width: 232, height: 48 }, GREY, 8);
  }
  const close = figma.rect(menu, "Close", { x: 24, y: 248, width: 232, height: 48 }, BLUE, 24);
  figma.on(close, "ON_CLICK", { kind: "close" });

  return figma;
}

/**
 * A vertically scrolling feed with a fixed header and tab bar, and a "back to top" link. Figma
 * plays it only with a device set on the page and the content grouped in a frame (checked by
 * importing variants).
 */
function scroll(): FigmaBuilder {
  const figma = new FigmaBuilder();
  const page = figma.page("Prototype");
  figma.device(page, "APPLE_IPHONE_11PRO_SPACEGREY", 375, 812);

  const feed = figma.frame(page, "Feed", { x: 0, y: 0, ...PHONE }, "#F9FAFB");
  figma.flowStart(feed, "Scroll flow");
  figma.scrollable(feed);

  // Room below the last item for the fixed tab bar (84) and a margin.
  const height = 10 * 216 + 48 + 16 + 84 + 16;
  const content = figma.frame(feed, "Content", { x: 0, y: 112, width: 375, height }, "#F9FAFB");
  const cards = Array.from({ length: 10 }, (_, i) =>
    figma.rect(
      content,
      `Card ${i + 1}`,
      { x: 16, y: i * 216, width: 343, height: 200 },
      i % 2 ? GREY : "#DBEAFE",
      16,
    ),
  );
  const top = figma.rect(
    content,
    "Back to top",
    { x: 88, y: 10 * 216, width: 200, height: 48 },
    BLUE,
    24,
  );
  figma.on(top, "ON_CLICK", {
    kind: "scrollTo",
    to: cards[0]!,
    transition: { type: "SCROLL_ANIMATE", durationMs: 400, easing: "INOUT_CUBIC" },
  });
  figma.fixed(figma.rect(feed, "Header", { x: 0, y: 0, width: 375, height: 96 }, "#FFFFFF"));
  figma.fixed(figma.rect(feed, "Tab bar", { x: 0, y: 728, width: 375, height: 84 }, "#FFFFFF"));

  return figma;
}

/** A splash screen that moves on by itself, then a button with hover and press feedback. */
function triggers(): FigmaBuilder {
  const figma = new FigmaBuilder();
  const page = figma.page("Prototype");

  const splash = figma.frame(page, "Splash", { x: 0, y: 0, ...PHONE }, BLUE);
  const home = figma.frame(page, "Home", { x: 475, y: 0, ...PHONE });
  const hovered = figma.frame(page, "Home, button hovered", { x: 950, y: 0, ...PHONE });
  const pressed = figma.frame(page, "Home, button pressed", { x: 1425, y: 0, ...PHONE });
  figma.flowStart(splash, "Triggers flow");

  figma.on(
    splash,
    "AFTER_TIMEOUT",
    { kind: "navigate", to: home, transition: { type: "DISSOLVE", durationMs: 500 } },
    1500,
  );
  const button = { x: 88, y: 640, width: 200, height: 48 };
  const plain = figma.rect(home, "Button", button, BLUE, 24);
  figma.on(plain, "ON_HOVER", { kind: "navigate", to: hovered });
  figma.on(plain, "ON_PRESS", { kind: "navigate", to: pressed });
  // While hovering, the hovered screen is shown: pressing must be wired on its button too.
  const hoveredButton = figma.rect(hovered, "Button", button, "#1D4ED8", 24);
  figma.on(hoveredButton, "ON_PRESS", { kind: "navigate", to: pressed });
  figma.rect(pressed, "Button", button, "#1E3A8A", 24);

  return figma;
}

/**
 * Interactive components: a button whose variants follow the pointer (while hovering, while
 * pressing), and a switch toggled by clicks. The components live on a page of their own.
 */
function components(): FigmaBuilder {
  const figma = new FigmaBuilder();
  const page = figma.page("Prototype");
  const library = figma.page("Components");

  const size = { width: 200, height: 48 };
  const buttons = figma.componentSet(
    library,
    "Button",
    { x: 0, y: 0, width: 248, height: 192 },
    "State",
    ["Default", "Hover", "Pressed"],
  );
  const idle = figma.variant(buttons, "State=Default", { x: 24, y: 24, ...size }, BLUE, 24);
  const hover = figma.variant(buttons, "State=Hover", { x: 24, y: 72, ...size }, "#1D4ED8", 24);
  const pressed = figma.variant(
    buttons,
    "State=Pressed",
    { x: 24, y: 120, ...size },
    "#1E3A8A",
    24,
  );
  figma.on(idle, "ON_HOVER", { kind: "changeTo", to: hover });
  figma.on(hover, "ON_PRESS", { kind: "changeTo", to: pressed });

  const track = { width: 64, height: 36 };
  const knob = (x: number) => ({ x, y: 4, width: 28, height: 28 });
  const switches = figma.componentSet(
    library,
    "Switch",
    { x: 300, y: 0, width: 112, height: 132 },
    "On",
    ["false", "true"],
  );
  const off = figma.variant(switches, "On=false", { x: 24, y: 24, ...track }, GREY, 18);
  figma.rect(off, "Knob", knob(4), "#FFFFFF", 14);
  const on = figma.variant(switches, "On=true", { x: 24, y: 72, ...track }, "#16A34A", 18);
  figma.rect(on, "Knob", knob(32), "#FFFFFF", 14);
  const toggle = { type: "DISSOLVE", durationMs: 200 } as const;
  figma.on(off, "ON_CLICK", { kind: "changeTo", to: on, transition: toggle });
  figma.on(on, "ON_CLICK", { kind: "changeTo", to: off, transition: toggle });

  const home = figma.frame(page, "Home", { x: 0, y: 0, ...PHONE });
  figma.flowStart(home, "Components flow");
  figma.instance(home, idle, "Button", { x: 88, y: 640, ...size });
  figma.instance(home, off, "Switch", { x: 156, y: 400, ...track });

  return figma;
}

/** Two pages; the first has two flows of two screens each, the second one flow. */
function flowsAndPages(): FigmaBuilder {
  const figma = new FigmaBuilder();

  const onboarding = figma.page("Onboarding");
  for (const [i, name] of ["Sign up", "Log in"].entries()) {
    const y = i * 912;
    const first = figma.frame(onboarding, `${name} 1`, { x: 0, y, ...PHONE });
    const second = figma.frame(onboarding, `${name} 2`, { x: 475, y, ...PHONE }, "#F9FAFB");
    figma.flowStart(first, name);
    const next = figma.rect(first, "Next", { x: 88, y: 640, width: 200, height: 48 }, BLUE, 24);
    figma.on(next, "ON_CLICK", {
      kind: "navigate",
      to: second,
      transition: { type: "PUSH_FROM_RIGHT", durationMs: 300, easing: "INOUT_CUBIC" },
    });
    const back = figma.rect(second, "Back", { x: 16, y: 56, width: 80, height: 32 }, GREY, 8);
    figma.on(back, "ON_CLICK", { kind: "back" });
  }

  const app = figma.page("App");
  const dashboard = figma.frame(app, "Dashboard", { x: 0, y: 0, ...PHONE });
  const settings = figma.frame(app, "Settings", { x: 475, y: 0, ...PHONE }, "#F9FAFB");
  figma.flowStart(dashboard, "App");
  const gear = figma.rect(
    dashboard,
    "Settings button",
    { x: 311, y: 56, width: 48, height: 48 },
    GREY,
    24,
  );
  figma.on(gear, "ON_CLICK", {
    kind: "navigate",
    to: settings,
    transition: { type: "SLIDE_FROM_RIGHT", durationMs: 300, easing: "OUT_CUBIC" },
  });
  const done = figma.rect(settings, "Done", { x: 16, y: 56, width: 80, height: 32 }, GREY, 8);
  figma.on(done, "ON_CLICK", { kind: "back" });

  return figma;
}

/** The `.sketch` twin of `navigation`, shaped like a converter's output that Lunacy plays. */
function sketchNavigation(): SketchBuilder {
  const sketch = new SketchBuilder();
  const page = sketch.page("Prototype");
  const home = sketch.artboard(page, "Home", { x: 0, y: 0, ...PHONE });
  const details = sketch.artboard(page, "Details", { x: 475, y: 0, ...PHONE }, "#F9FAFB");
  sketch.flowHome(home);

  sketch.rect(home, "Header", { x: 0, y: 0, width: 375, height: 96 }, GREY);
  const open = sketch.rect(
    home,
    "Open details",
    { x: 88, y: 640, width: 200, height: 48 },
    BLUE,
    24,
  );
  sketch.link(open, details, "slideFromRight");

  const back = sketch.rect(details, "Back", { x: 16, y: 56, width: 80, height: 32 }, GREY, 8);
  sketch.link(back, "back");

  return sketch;
}

/**
 * Text in two fonts, neither stored in the file: Inter, and a brand font that only a rule can
 * stand in for. Sketch stores no glyphs, so a bake draws nothing without the fonts.
 */
function sketchText(): SketchBuilder {
  const sketch = new SketchBuilder();
  const page = sketch.page("Prototype");
  const home = sketch.artboard(page, "Home", { x: 0, y: 0, ...PHONE });
  sketch.flowHome(home);
  sketch.text(home, "Title", { x: 24, y: 64, width: 240, height: 40 }, "Hello Inter", {
    name: "Inter-Regular",
    size: 32,
  });
  sketch.text(home, "Tagline", { x: 24, y: 128, width: 240, height: 24 }, "Hello Brand", {
    name: "Brand-Bold",
    size: 20,
  });
  return sketch;
}

/** The `.penpot` twin of `navigation`. */
function penpotNavigation(): PenpotBuilder {
  const penpot = new PenpotBuilder("Navigation");
  const page = penpot.page("Prototype");
  const home = penpot.board(page, "Home", { x: 0, y: 0, ...PHONE });
  const details = penpot.board(page, "Details", { x: 475, y: 0, ...PHONE }, "#F9FAFB");
  penpot.flow(page, home, "Main flow");

  penpot.rect(page, home, "Header", { x: 0, y: 0, width: 375, height: 96 }, GREY);
  const open = penpot.rect(
    page,
    home,
    "Open details",
    { x: 88, y: 640, width: 200, height: 48 },
    BLUE,
    24,
  );
  penpot.navigate(open, details, { animationType: "dissolve", duration: 300, easing: "ease-out" });

  const back = penpot.rect(
    page,
    details,
    "Back",
    { x: 491, y: 56, width: 80, height: 32 },
    GREY,
    8,
  );
  penpot.back(back);

  return penpot;
}

/** The `.xd` twin of `navigation`: a slide, and a timed auto-animate back. */
function xdNavigation(): XdBuilder {
  const xd = new XdBuilder("Navigation");
  const home = xd.artboard("Home", { x: 0, y: 0, ...PHONE });
  const details = xd.artboard("Details", { x: 475, y: 0, ...PHONE }, "#F9FAFB");
  xd.start(home);

  xd.rect(home, "Header", { x: 0, y: 0, width: 375, height: 96 }, GREY);
  const open = xd.rect(home, "Open details", { x: 88, y: 640, width: 200, height: 48 }, BLUE, 24);
  xd.link(open, details, { transition: "slide-left", easing: "ease-out" });

  const back = xd.rect(details, "Back", { x: 16, y: 56, width: 80, height: 32 }, GREY, 8);
  xd.link(back, home, { transition: "dissolve" });
  xd.circle(details, "Dot", { x: 150, y: 300, width: 75, height: 75 }, BLUE);

  return xd;
}

/** Text in Inter, which an `.xd` does not carry: a bake draws nothing without the font. */
function xdText(): XdBuilder {
  const xd = new XdBuilder("Text");
  const home = xd.artboard("Home", { x: 0, y: 0, ...PHONE });
  xd.start(home);
  xd.text(home, "Title", { x: 24, y: 96 }, "Hello Inter", {
    family: "Inter",
    style: "Regular",
    size: 32,
  });
  return xd;
}

const fixtures: Record<string, () => { toBytes(): Uint8Array }> = {
  "figma/navigation.fig": navigation,
  "figma/components.fig": components,
  "figma/overlay.fig": overlay,
  "figma/smart-animate.fig": smartAnimate,
  "figma/scroll.fig": scroll,
  "figma/triggers.fig": triggers,
  "figma/flows-and-pages.fig": flowsAndPages,
  "sketch/navigation.sketch": sketchNavigation,
  "sketch/text.sketch": sketchText,
  "penpot/navigation.penpot": penpotNavigation,
  "xd/navigation.xd": xdNavigation,
  "xd/text.xd": xdText,
};

for (const [path, build] of Object.entries(fixtures)) {
  const out = join(FIXTURES, path);
  mkdirSync(dirname(out), { recursive: true });
  writeFileSync(out, build().toBytes());
  console.log(`wrote fixtures/${path}`);
}
