// SPDX-License-Identifier: AGPL-3.0-or-later
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { effect, FigmaBuilder, gradient } from "./figma.ts";
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

  // A frame whose children move on their own, and an arrow that turns a quarter.
  const panel = figma.frame(home, "Panel", { x: 24, y: 300, width: 200, height: 120 }, GREY);
  figma.rect(panel, "Label", { x: 16, y: 16, width: 120, height: 24 }, "#FFFFFF", 4);
  figma.ellipse(panel, "Dot", { x: 150, y: 60, width: 24, height: 24 }, "#16A34A");
  figma.rect(home, "Arrow", { x: 280, y: 140, width: 48, height: 8 }, "#111827", 4);
  const panel2 = figma.frame(details, "Panel", { x: 24, y: 620, width: 327, height: 80 }, GREY);
  figma.rect(panel2, "Label", { x: 180, y: 28, width: 120, height: 24 }, "#FFFFFF", 4);
  figma.ellipse(panel2, "Dot", { x: 16, y: 28, width: 24, height: 24 }, "#16A34A");
  const arrow = figma.rect(
    details,
    "Arrow",
    { x: 340, y: 360, width: 48, height: 8 },
    "#111827",
    4,
  );
  figma.set(arrow, { transform: { m00: 0, m01: -1, m02: 340, m10: 1, m11: 0, m12: 360 } });

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

  // "More" swaps the menu for a taller one, with smart animate: the shared items move.
  const more = figma.frame(
    page,
    "Menu, more",
    { x: 800, y: 0, width: 280, height: 448 },
    "#FFFFFF",
  );
  figma.overlay(more, { position: "CENTER", closeOnClickOutside: true, backdrop: "#00000066" });
  for (let i = 0; i < 5; i++) {
    figma.rect(more, `Item ${i + 1}`, { x: 24, y: 24 + i * 64, width: 232, height: 48 }, GREY, 8);
  }
  const less = figma.rect(more, "Close", { x: 24, y: 376, width: 232, height: 48 }, BLUE, 24);
  figma.on(less, "ON_CLICK", { kind: "close" });
  const expand = figma.rect(menu, "More", { x: 200, y: 200, width: 56, height: 32 }, "#DBEAFE", 8);
  figma.on(expand, "ON_CLICK", {
    kind: "swap",
    to: more,
    transition: { type: "SMART_ANIMATE", durationMs: 300 },
  });

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

  // Shift + right arrow, or a swipe on the card, opens the details; Escape-free back.
  const details = figma.frame(page, "Details", { x: 1900, y: 0, ...PHONE }, "#F9FAFB");
  figma.on(
    home,
    "ON_KEY_DOWN",
    { kind: "navigate", to: details, transition: { type: "SLIDE_FROM_RIGHT", durationMs: 300 } },
    0,
    [16, 39],
  );
  const card = figma.rect(home, "Swipe card", { x: 24, y: 160, width: 327, height: 200 }, GREY, 16);
  figma.on(card, "DRAG", {
    kind: "navigate",
    to: details,
    transition: { type: "SLIDE_FROM_RIGHT", durationMs: 300 },
  });
  const back = figma.rect(details, "Back", { x: 16, y: 56, width: 80, height: 32 }, GREY, 8);
  figma.on(back, "ON_CLICK", { kind: "back" });

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
  // The switch's knob slides from one state to the other: smart animate between states.
  const slide = { type: "SMART_ANIMATE", durationMs: 200 } as const;
  figma.on(off, "ON_CLICK", { kind: "changeTo", to: on, transition: slide });
  figma.on(on, "ON_CLICK", { kind: "changeTo", to: off, transition: slide });

  // A card that expands on click, with a switch inside its expanded state: an interactive
  // component nested in another's state.
  const card = { width: 327, height: 120 };
  const cards = figma.componentSet(
    library,
    "Card",
    { x: 0, y: 240, width: 375, height: 312 },
    "Open",
    ["false", "true"],
  );
  const closed = figma.variant(cards, "Open=false", { x: 24, y: 24, ...card }, GREY, 16);
  const opened = figma.variant(cards, "Open=true", { x: 24, y: 168, ...card }, "#DBEAFE", 16);
  figma.on(closed, "ON_CLICK", { kind: "changeTo", to: opened, transition: toggle });
  const fold = figma.rect(opened, "Fold", { x: 16, y: 16, width: 120, height: 32 }, BLUE, 8);
  figma.on(fold, "ON_CLICK", { kind: "changeTo", to: closed, transition: toggle });
  figma.instance(opened, off, "Inner switch", { x: 247, y: 68, ...track });

  // A toast that hides itself after a delay: "after delay" inside a state.
  const toasts = figma.componentSet(
    library,
    "Toast",
    { x: 400, y: 240, width: 280, height: 168 },
    "Shown",
    ["true", "false"],
  );
  const shown = figma.variant(
    toasts,
    "Shown=true",
    { x: 24, y: 24, width: 232, height: 48 },
    "#111827",
    12,
  );
  const hidden = figma.variant(
    toasts,
    "Shown=false",
    { x: 24, y: 96, width: 232, height: 48 },
    "#FFFFFF",
    12,
  );
  figma.on(shown, "AFTER_TIMEOUT", { kind: "changeTo", to: hidden, transition: toggle }, 1000);

  const home = figma.frame(page, "Home", { x: 0, y: 0, ...PHONE });
  figma.flowStart(home, "Components flow");
  figma.instance(home, idle, "Button", { x: 88, y: 640, ...size });
  figma.instance(home, off, "Switch", { x: 156, y: 400, ...track });
  figma.instance(home, closed, "Card", { x: 24, y: 120, ...card });
  figma.instance(home, shown, "Toast", { x: 72, y: 720, width: 232, height: 48 });

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

/**
 * The `.fig` drawing features, and an "open link" action: linear, radial, angular and diamond
 * gradients, drop and inner shadows, a layer blur, a background blur, a blend mode.
 */
function figmaFeatures(): FigmaBuilder {
  const figma = new FigmaBuilder();
  const page = figma.page("Prototype");
  const home = figma.frame(page, "Home", { x: 0, y: 0, ...PHONE });
  figma.flowStart(home, "Features");

  const kinds = ["GRADIENT_LINEAR", "GRADIENT_RADIAL", "GRADIENT_ANGULAR", "GRADIENT_DIAMOND"];
  kinds.forEach((kind, i) => {
    const node = figma.rect(home, kind, { x: 16 + i * 88, y: 24, width: 80, height: 80 }, BLUE, 8);
    figma.set(node, { fillPaints: [gradient(kind, "#2563EB", "#F59E0B")] });
  });
  const drop = figma.rect(
    home,
    "Drop shadow",
    { x: 16, y: 136, width: 160, height: 96 },
    "#FFFFFF",
    12,
  );
  figma.set(drop, {
    effects: [effect("DROP_SHADOW", 12, 4), effect("DROP_SHADOW", 2, 1, "#0000001A")],
  });
  const inner = figma.rect(
    home,
    "Inner shadow",
    { x: 199, y: 136, width: 160, height: 96 },
    GREY,
    12,
  );
  figma.set(inner, { effects: [effect("INNER_SHADOW", 8, 4, "#00000066")] });
  const blurred = figma.ellipse(home, "Layer blur", { x: 16, y: 264, width: 96, height: 96 }, BLUE);
  figma.set(blurred, { effects: [effect("FOREGROUND_BLUR", 8)] });
  for (let i = 0; i < 9; i++) {
    const color = i % 2 ? "#F59E0B" : "#1E3A8A";
    figma.rect(home, `Stripe ${i + 1}`, { x: 136 + i * 25, y: 264, width: 25, height: 96 }, color);
  }
  const glass = figma.rect(
    home,
    "Frosted glass",
    { x: 160, y: 288, width: 176, height: 48 },
    "#FFFFFF66",
    12,
  );
  figma.set(glass, { effects: [effect("BACKGROUND_BLUR", 16)] });
  figma.ellipse(home, "Under", { x: 100, y: 392, width: 120, height: 120 }, "#3B82F6");
  const multiply = figma.ellipse(
    home,
    "Multiply",
    { x: 40, y: 392, width: 120, height: 120 },
    "#F59E0B",
  );
  figma.set(multiply, { blendMode: "MULTIPLY" });

  const site = figma.rect(home, "Website", { x: 88, y: 640, width: 200, height: 48 }, BLUE, 24);
  figma.on(site, "ON_CLICK", { kind: "url", url: "https://folioscope.baxyz.dev" });
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
 * Every Sketch scenario in one file: two pages, a start point on each, links with slides and back;
 * and the drawing features: linear and radial gradients, a drop shadow, a bitmap, a mask.
 */
function sketchFeatures(): SketchBuilder {
  const sketch = new SketchBuilder();
  const page = sketch.page("Prototype");
  const home = sketch.artboard(page, "Home", { x: 0, y: 0, ...PHONE });
  const details = sketch.artboard(page, "Details", { x: 475, y: 0, ...PHONE }, "#F9FAFB");
  sketch.flowHome(home);

  const banner = sketch.rect(home, "Banner", { x: 0, y: 0, width: 375, height: 160 }, BLUE);
  sketch.gradient(banner, 0, "#2563EB", "#7C3AED");
  const glow = sketch.oval(home, "Glow", { x: 24, y: 184, width: 96, height: 96 }, "#F59E0B");
  sketch.gradient(glow, 1, "#FDE68A", "#F59E0B");
  const card = sketch.rect(home, "Card", { x: 144, y: 184, width: 207, height: 96 }, "#FFFFFF", 12);
  sketch.shadow(card, 4, 12, "#00000040");
  sketch.bitmap(
    home,
    "Photo",
    { x: 24, y: 304, width: 120, height: 80 },
    { r: 0.9, g: 0.3, b: 0.3 },
  );
  const avatar = sketch.group(home, "Avatar", { x: 168, y: 304, width: 64, height: 64 });
  sketch.mask(sketch.oval(avatar, "Circle mask", { x: 0, y: 0, width: 64, height: 64 }, "#000000"));
  sketch.rect(avatar, "Square", { x: 0, y: 0, width: 64, height: 64 }, "#EC4899");
  const spin = sketch.oval(home, "Angular", { x: 256, y: 304, width: 64, height: 64 }, BLUE);
  sketch.gradient(spin, 2, "#2563EB", "#F59E0B");
  const inner = sketch.rect(
    home,
    "Inner shadow",
    { x: 24, y: 400, width: 96, height: 64 },
    GREY,
    12,
  );
  sketch.innerShadow(inner, 4, 8, "#00000066");
  const blurred = sketch.oval(home, "Layer blur", { x: 136, y: 400, width: 64, height: 64 }, BLUE);
  sketch.blur(blurred, 0, 4);
  for (let i = 0; i < 6; i++) {
    const stripe = { x: 216 + i * 22, y: 400, width: 22, height: 64 };
    sketch.rect(home, `Stripe ${i + 1}`, stripe, i % 2 ? "#F59E0B" : "#1E3A8A");
  }
  const glass = sketch.rect(
    home,
    "Frosted glass",
    { x: 228, y: 412, width: 108, height: 40 },
    "#FFFFFF66",
    8,
  );
  sketch.blur(glass, 3, 8);
  sketch.rect(home, "Under", { x: 24, y: 488, width: 64, height: 64 }, "#3B82F6");
  const multiply = sketch.oval(
    home,
    "Multiply",
    { x: 56, y: 488, width: 64, height: 64 },
    "#F59E0B",
  );
  sketch.blend(multiply, 2);
  const inside = sketch.rect(
    home,
    "Inside border",
    { x: 144, y: 488, width: 64, height: 64 },
    "#FFFFFF",
    8,
  );
  sketch.border(inside, "#111827", 4, 1);
  const outside = sketch.rect(
    home,
    "Outside border",
    { x: 232, y: 488, width: 64, height: 64 },
    "#FFFFFF",
    8,
  );
  sketch.border(outside, "#111827", 4, 2);
  const faded = sketch.group(home, "Faded", { x: 312, y: 488, width: 48, height: 64 });
  const fade = sketch.rect(faded, "Fade mask", { x: 0, y: 0, width: 48, height: 64 }, "#000000");
  sketch.gradient(fade, 0, "#000000FF", "#00000000");
  sketch.alphaMask(fade);
  sketch.rect(faded, "Faded square", { x: 0, y: 0, width: 48, height: 64 }, "#EC4899");
  sketch.richText(home, "Rich text", { x: 24, y: 576, width: 320, height: 48 }, [
    { text: "Plain, ", font: "Inter-Regular", size: 18, fill: "#111827" },
    { text: "bold blue", font: "Inter-Bold", size: 18, fill: "#2563EB", underline: true },
    { text: ", struck", font: "Inter-Regular", size: 18, fill: "#6B7280", strike: true },
  ]);

  const open = sketch.rect(
    home,
    "Open details",
    { x: 88, y: 640, width: 200, height: 48 },
    BLUE,
    24,
  );
  sketch.link(open, details, "slideFromRight");
  const back = sketch.rect(details, "Back", { x: 16, y: 56, width: 80, height: 32 }, GREY, 8);
  sketch.link(back, "back", "slideFromLeft");

  const second = sketch.page("Second page");
  const about = sketch.artboard(second, "About", { x: 0, y: 0, ...PHONE });
  sketch.flowHome(about);
  sketch.rect(about, "Logo", { x: 138, y: 300, width: 100, height: 100 }, BLUE, 50);
  return sketch;
}

/** An artboard taller than its viewport: it scrolls under a fixed header; a link at the bottom. */
function sketchScroll(): SketchBuilder {
  const sketch = new SketchBuilder();
  const page = sketch.page("Prototype");
  const feed = sketch.artboard(page, "Feed", { x: 0, y: 0, width: 375, height: 1600 }, "#F9FAFB");
  const details = sketch.artboard(page, "Details", { x: 475, y: 0, ...PHONE });
  sketch.viewport(feed, "iPhone", 375, 812);
  sketch.flowHome(feed);
  for (let i = 0; i < 6; i++) {
    const card = { x: 16, y: 112 + i * 240, width: 343, height: 216 };
    sketch.rect(feed, `Card ${i + 1}`, card, i % 2 ? GREY : "#DBEAFE", 16);
  }
  sketch.fixed(sketch.rect(feed, "Header", { x: 0, y: 0, width: 375, height: 96 }, BLUE));
  const open = sketch.rect(
    feed,
    "Open details",
    { x: 88, y: 1520, width: 200, height: 48 },
    BLUE,
    24,
  );
  sketch.link(open, details, "slideFromRight");
  const back = sketch.rect(details, "Back", { x: 16, y: 56, width: 80, height: 32 }, GREY, 8);
  sketch.link(back, "back", "slideFromLeft");
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

/**
 * Every Penpot scenario in one file: two pages and three flows; a splash that leaves after a
 * delay; click, press and hover (mouse enter and leave); overlays centred, manual and toggled,
 * closed from inside; a link; and the drawing features: linear and radial gradients, a drop
 * shadow, a path, an image, a masked group, dashed strokes, and text in two weights.
 */
function penpotFeatures(): PenpotBuilder {
  const penpot = new PenpotBuilder("Features");
  const page = penpot.page("Prototype");
  const splash = penpot.board(page, "Splash", { x: 0, y: 0, ...PHONE }, "#1E3A8A");
  const home = penpot.board(page, "Home", { x: 475, y: 0, ...PHONE });
  const details = penpot.board(page, "Details", { x: 950, y: 0, ...PHONE }, "#F9FAFB");
  const menu = penpot.board(page, "Menu", { x: 1425, y: 0, width: 280, height: 320 });
  const tip = penpot.board(page, "Tip", { x: 1425, y: 400, width: 200, height: 80 }, "#111827");
  penpot.flow(page, splash, "Main flow");
  penpot.flow(page, home, "Skip the splash");

  penpot.interact(splash, {
    eventType: "after-delay",
    delay: 1500,
    actionType: "navigate",
    destination: home.id,
    animation: { animationType: "dissolve", duration: 300, easing: "ease-in-out" },
  });

  // Drawing.
  penpot.shape(
    page,
    home,
    "Banner",
    "rect",
    { x: 475, y: 0, width: 375, height: 160 },
    {
      fills: [
        {
          fillOpacity: 1,
          fillColorGradient: {
            type: "linear",
            startX: 0,
            startY: 0.5,
            endX: 1,
            endY: 0.5,
            width: 1,
            stops: [
              { color: "#2563EB", opacity: 1, offset: 0 },
              { color: "#7C3AED", opacity: 1, offset: 1 },
            ],
          },
        },
      ],
    },
  );
  penpot.shape(
    page,
    home,
    "Glow",
    "circle",
    { x: 499, y: 184, width: 96, height: 96 },
    {
      fills: [
        {
          fillOpacity: 1,
          fillColorGradient: {
            type: "radial",
            startX: 0.5,
            startY: 0.5,
            endX: 1,
            endY: 0.5,
            width: 1,
            stops: [
              { color: "#FDE68A", opacity: 1, offset: 0 },
              { color: "#F59E0B", opacity: 1, offset: 1 },
            ],
          },
        },
      ],
    },
  );
  penpot.shape(
    page,
    home,
    "Card",
    "rect",
    { x: 619, y: 184, width: 207, height: 96 },
    {
      r1: 12,
      r2: 12,
      r3: 12,
      r4: 12,
      shadow: [
        {
          id: "s",
          style: "drop-shadow",
          offsetX: 0,
          offsetY: 4,
          blur: 12,
          spread: 0,
          hidden: false,
          color: { color: "#000000", opacity: 0.25 },
        },
      ],
      strokes: [
        {
          strokeColor: "#9CA3AF",
          strokeOpacity: 1,
          strokeWidth: 2,
          strokeStyle: "dashed",
          strokeAlignment: "center",
        },
      ],
    },
  );
  penpot.shape(
    page,
    home,
    "Arrow",
    "path",
    { x: 499, y: 304, width: 48, height: 48 },
    {
      content: [
        { command: "move-to", params: { x: 499, y: 304 } },
        { command: "line-to", params: { x: 547, y: 328 } },
        { command: "line-to", params: { x: 499, y: 352 } },
        { command: "close-path", params: {} },
      ],
      fills: [{ fillColor: "#10B981", fillOpacity: 1 }],
    },
  );
  penpot.image(
    page,
    home,
    "Photo",
    { x: 571, y: 304, width: 120, height: 80 },
    { r: 0.9, g: 0.3, b: 0.3 },
  );
  const masked = penpot.shape(
    page,
    home,
    "Avatar",
    "group",
    { x: 715, y: 304, width: 64, height: 64 },
    {
      maskedGroup: true,
      fills: [],
    },
  );
  // The mask is the group's first shape, at the bottom of the list: added last.
  penpot.shape(
    page,
    masked,
    "Square",
    "rect",
    { x: 715, y: 304, width: 64, height: 64 },
    {
      fills: [{ fillColor: "#EC4899", fillOpacity: 1 }],
    },
  );
  penpot.shape(page, masked, "Circle mask", "circle", { x: 715, y: 304, width: 64, height: 64 });
  // Effects: inner shadow, layer blur, blend mode, inside and outside strokes, a path stored as
  // SVG path data, and an imported SVG element (svg-raw).
  penpot.shape(
    page,
    home,
    "Inner",
    "rect",
    { x: 499, y: 480, width: 64, height: 48 },
    {
      fills: [{ fillColor: "#E5E7EB", fillOpacity: 1 }],
      shadow: [
        {
          id: "i",
          style: "inner-shadow",
          offsetX: 0,
          offsetY: 4,
          blur: 8,
          spread: 0,
          hidden: false,
          color: { color: "#000000", opacity: 0.4 },
        },
      ],
    },
  );
  penpot.shape(
    page,
    home,
    "Blurred",
    "circle",
    { x: 579, y: 480, width: 48, height: 48 },
    {
      fills: [{ fillColor: "#2563EB", fillOpacity: 1 }],
      blur: { id: "b", type: "layer-blur", value: 4, hidden: false },
    },
  );
  penpot.shape(
    page,
    home,
    "Screen",
    "circle",
    { x: 611, y: 480, width: 48, height: 48 },
    {
      fills: [{ fillColor: "#F59E0B", fillOpacity: 1 }],
      blendMode: "screen",
    },
  );
  penpot.shape(
    page,
    home,
    "Inner stroke",
    "rect",
    { x: 675, y: 480, width: 48, height: 48 },
    {
      strokes: [
        { strokeColor: "#111827", strokeOpacity: 1, strokeWidth: 4, strokeAlignment: "inner" },
      ],
    },
  );
  penpot.shape(
    page,
    home,
    "Outer stroke",
    "rect",
    { x: 739, y: 480, width: 48, height: 48 },
    {
      strokes: [
        {
          strokeColor: "#111827",
          strokeOpacity: 1,
          strokeWidth: 4,
          strokeAlignment: "outer",
          strokeCapStart: "round",
          strokeCapEnd: "round",
        },
      ],
    },
  );
  penpot.shape(
    page,
    home,
    "Check",
    "path",
    { x: 803, y: 480, width: 32, height: 24 },
    {
      content: "M803.0,492.0L813.0,502.0L835.0,480.0",
      fills: [],
      strokes: [
        {
          strokeColor: "#10B981",
          strokeOpacity: 1,
          strokeWidth: 3,
          strokeCapStart: "round",
          strokeCapEnd: "round",
        },
      ],
    },
  );
  const svgText = penpot.shape(
    page,
    home,
    "svg-text",
    "svg-raw",
    { x: 475, y: 0, width: 375, height: 812 },
    {
      svgViewbox: { x: 0, y: 0, width: 375, height: 812, x1: 0, y1: 0, x2: 375, y2: 812 },
      content: {
        tag: "text",
        attrs: {
          x: "24",
          y: "548",
          fontFamily: "Inter, sans-serif",
          fontSize: "14",
          fill: "#6B7280",
        },
        content: ["Imported SVG text"],
      },
      fills: [{ fillColor: "#6B7280" }],
    },
  );
  penpot.shape(
    page,
    svgText,
    "Imported SVG text",
    "svg-raw",
    { x: 475, y: 0, width: 375, height: 812 },
    {
      content: "Imported SVG text",
    },
  );
  penpot.shape(
    page,
    home,
    "svg-path",
    "svg-raw",
    { x: 475, y: 0, width: 375, height: 812 },
    {
      svgViewbox: { x: 0, y: 0, width: 375, height: 812, x1: 0, y1: 0, x2: 375, y2: 812 },
      content: {
        tag: "rect",
        attrs: { x: "300", y: "536", width: "40", height: "16", rx: "4", fill: "#EC4899" },
      },
    },
  );
  penpot.text(page, home, "Title", { x: 499, y: 400, width: 327, height: 80 }, [
    { text: "Hello Inter", family: "Inter", size: 24, color: "#111827" },
    {
      text: "Bold and blue",
      family: '"Inter"',
      weight: 700,
      size: 18,
      color: "#2563EB",
      decoration: "underline",
    },
  ]);

  // Prototype.
  const open = penpot.rect(
    page,
    home,
    "Open details",
    { x: 563, y: 560, width: 200, height: 48 },
    BLUE,
    24,
  );
  penpot.navigate(open, details, {
    animationType: "slide",
    duration: 300,
    easing: "ease-out",
    way: "in",
    direction: "left",
  });
  const hold = penpot.rect(
    page,
    home,
    "Hold me",
    { x: 563, y: 624, width: 200, height: 48 },
    GREY,
    24,
  );
  penpot.interact(hold, {
    eventType: "mouse-press",
    actionType: "navigate",
    destination: details.id,
  });
  const hover = penpot.rect(
    page,
    home,
    "Hover me",
    { x: 563, y: 688, width: 200, height: 48 },
    GREY,
    24,
  );
  penpot.interact(hover, {
    eventType: "mouse-enter",
    actionType: "open-overlay",
    destination: tip.id,
    overlayPosType: "manual",
    overlayPosition: { x: 0, y: -96 },
    closeClickOutside: false,
    positionRelativeTo: hover.id,
  });
  penpot.interact(hover, {
    eventType: "mouse-leave",
    actionType: "close-overlay",
    destination: tip.id,
  });
  const more = penpot.rect(
    page,
    home,
    "Menu button",
    { x: 795, y: 40, width: 40, height: 40 },
    GREY,
    20,
  );
  penpot.interact(more, {
    eventType: "click",
    actionType: "toggle-overlay",
    destination: menu.id,
    overlayPosType: "center",
    closeClickOutside: true,
    backgroundOverlay: true,
  });
  const site = penpot.rect(
    page,
    home,
    "Website",
    { x: 563, y: 752, width: 200, height: 32 },
    GREY,
    8,
  );
  penpot.interact(site, {
    eventType: "click",
    actionType: "open-url",
    url: "https://folioscope.baxyz.dev",
  });
  const close = penpot.rect(
    page,
    menu,
    "Close",
    { x: 1449, y: 248, width: 232, height: 48 },
    BLUE,
    24,
  );
  penpot.interact(close, { eventType: "click", actionType: "close-overlay", destination: menu.id });
  const back = penpot.rect(
    page,
    details,
    "Back",
    { x: 966, y: 56, width: 80, height: 32 },
    GREY,
    8,
  );
  penpot.back(back);

  const second = penpot.page("Second page");
  const about = penpot.board(second, "About", { x: 0, y: 0, ...PHONE });
  penpot.flow(second, about, "About flow");
  penpot.rect(second, about, "Logo", { x: 138, y: 300, width: 100, height: 100 }, BLUE, 50);
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

/** An artboard taller than the screen: it scrolls, and its last link shows only at the bottom. */
function xdScroll(): XdBuilder {
  const xd = new XdBuilder("Scroll");
  const feed = xd.artboard("Feed", { x: 0, y: 0, width: 375, height: 1600 }, "#F9FAFB", 812);
  const details = xd.artboard("Details", { x: 475, y: 0, ...PHONE }, "#FFFFFF");
  xd.start(feed);
  for (let i = 0; i < 6; i++) {
    xd.rect(
      feed,
      `Card ${i + 1}`,
      { x: 16, y: 16 + i * 240, width: 343, height: 216 },
      i % 2 ? GREY : "#DBEAFE",
      16,
    );
  }
  const open = xd.rect(feed, "Open details", { x: 88, y: 1500, width: 200, height: 48 }, BLUE, 24);
  xd.link(open, details, { transition: "slide-left", easing: "ease-out" });
  const back = xd.rect(details, "Back", { x: 16, y: 56, width: 80, height: 32 }, GREY, 8);
  xd.link(back, feed, { transition: "dissolve" });
  return xd;
}

/** A bottom sheet opened over the screen at a position, and closed with "previous artboard". */
function xdOverlay(): XdBuilder {
  const xd = new XdBuilder("Overlay");
  const home = xd.artboard("Home", { x: 0, y: 0, ...PHONE });
  const sheet = xd.artboard("Sheet", { x: 475, y: 0, width: 375, height: 363 }, "#F9FAFB");
  xd.start(home);
  const open = xd.rect(home, "Open sheet", { x: 88, y: 640, width: 200, height: 48 }, BLUE, 24);
  xd.link(open, sheet, {
    action: "overlay-transition",
    transition: "slide-up",
    position: { x: 0, y: 449 },
  });
  const close = xd.rect(sheet, "Close", { x: 88, y: 24, width: 200, height: 48 }, GREY, 24);
  xd.link(close, home, { action: "previous-artboard" });
  return xd;
}

/**
 * Every XD scenario in one file: a splash that leaves after a delay, taps with slides and
 * auto-animate, an overlay; and the drawing features: linear and radial gradients, a drop
 * shadow, an image fill, a clip path.
 */
function xdFeatures(): XdBuilder {
  const xd = new XdBuilder("Features");
  const splash = xd.artboard("Splash", { x: 0, y: 0, ...PHONE }, "#1E3A8A");
  const home = xd.artboard("Home", { x: 475, y: 0, ...PHONE });
  const details = xd.artboard("Details", { x: 950, y: 0, ...PHONE }, "#F9FAFB");
  xd.start(splash);
  xd.link(splash, home, { trigger: "time", delay: 1.5, transition: "dissolve" });

  const banner = xd.rect(home, "Banner", { x: 0, y: 0, width: 375, height: 160 }, BLUE);
  xd.style(banner, { fill: xd.gradientFill("linear", "#2563EB", "#7C3AED") });
  const glow = xd.circle(home, "Glow", { x: 24, y: 184, width: 96, height: 96 }, "#F59E0B");
  xd.style(glow, { fill: xd.gradientFill("radial", "#FDE68A", "#F59E0B") });
  const card = xd.rect(home, "Card", { x: 144, y: 184, width: 207, height: 96 }, "#FFFFFF", 12);
  xd.style(card, xd.dropShadow(4, 12, "#000000", 0.25));
  xd.image(home, "Photo", { x: 24, y: 304, width: 120, height: 80 }, { r: 0.9, g: 0.3, b: 0.3 });
  xd.clippedCircle(home, "Avatar", { x: 168, y: 304, width: 64, height: 64 }, "#EC4899");
  // Effects: an object blur, a background blur over stripes, a blend mode, an image fit whole,
  // and underlined text.
  const blurred = xd.circle(home, "Object blur", { x: 256, y: 304, width: 64, height: 64 }, BLUE);
  xd.style(blurred, {
    filters: [
      { type: "uxdesign#blur", visible: true, params: { blurAmount: 4, backgroundEffect: false } },
    ],
  });
  for (let i = 0; i < 6; i++) {
    const stripe = { x: 24 + i * 22, y: 400, width: 22, height: 64 };
    xd.rect(home, `Stripe ${i + 1}`, stripe, i % 2 ? "#F59E0B" : "#1E3A8A");
  }
  const glass = xd.rect(
    home,
    "Frosted glass",
    { x: 36, y: 412, width: 108, height: 40 },
    "#FFFFFF",
    8,
  );
  xd.style(glass, {
    opacity: 0.6,
    filters: [
      {
        type: "uxdesign#blur",
        visible: true,
        params: { blurAmount: 8, brightnessAmount: 0, fillOpacity: 0.4, backgroundEffect: true },
      },
    ],
  });
  xd.rect(home, "Under", { x: 176, y: 400, width: 64, height: 64 }, "#3B82F6");
  const multiply = xd.circle(
    home,
    "Multiply",
    { x: 208, y: 400, width: 64, height: 64 },
    "#F59E0B",
  );
  xd.style(multiply, { blendMode: "multiply" });
  const fit = xd.image(
    home,
    "Fit",
    { x: 288, y: 400, width: 64, height: 32 },
    { r: 0.2, g: 0.6, b: 0.3 },
  );
  const fill = (fit.style as { fill: { pattern: { meta: { ux: Record<string, unknown> } } } }).fill;
  fill.pattern.meta.ux.scaleBehavior = "fit";
  const label = xd.text(home, "Underlined", { x: 24, y: 520 }, "Underlined Inter", {
    family: "Inter",
    style: "Regular",
    size: 18,
  });
  const ranged = (label.meta as { ux: { rangedStyles: Record<string, unknown>[] } }).ux
    .rangedStyles;
  ranged[0]!.underline = true;

  const open = xd.rect(home, "Open details", { x: 88, y: 640, width: 200, height: 48 }, BLUE, 24);
  xd.link(open, details, { action: "auto-animate", easing: "ease-in-out", duration: 0.4 });
  const peek = xd.rect(home, "Peek", { x: 88, y: 720, width: 200, height: 32 }, GREY, 16);
  xd.link(peek, details, { trigger: "hover", transition: "dissolve" });
  const back = xd.rect(details, "Back", { x: 16, y: 56, width: 80, height: 32 }, GREY, 8);
  xd.link(back, home, { action: "previous-artboard" });
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
  "figma/features.fig": figmaFeatures,
  "sketch/navigation.sketch": sketchNavigation,
  "sketch/text.sketch": sketchText,
  "sketch/features.sketch": sketchFeatures,
  "sketch/scroll.sketch": sketchScroll,
  "penpot/navigation.penpot": penpotNavigation,
  "penpot/features.penpot": penpotFeatures,
  "xd/navigation.xd": xdNavigation,
  "xd/overlay.xd": xdOverlay,
  "xd/scroll.xd": xdScroll,
  "xd/text.xd": xdText,
  "xd/features.xd": xdFeatures,
};

for (const [path, build] of Object.entries(fixtures)) {
  const out = join(FIXTURES, path);
  mkdirSync(dirname(out), { recursive: true });
  writeFileSync(out, build().toBytes());
  console.log(`wrote fixtures/${path}`);
}
