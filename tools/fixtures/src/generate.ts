// SPDX-License-Identifier: AGPL-3.0-or-later
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { FigBuilder } from "./fig.ts";

const FIXTURES = join(dirname(fileURLToPath(import.meta.url)), "../../../fixtures");

const PHONE = { width: 375, height: 812 };
const BLUE = "#2563EB";
const GREY = "#E5E7EB";

/** Two screens, a button to go forward, a button to go back. One flow. */
function navigation(): FigBuilder {
  const fig = new FigBuilder();
  const page = fig.page("Prototype");

  const home = fig.frame(page, "Home", { x: 0, y: 0, ...PHONE });
  const details = fig.frame(page, "Details", { x: 475, y: 0, ...PHONE }, "#F9FAFB");
  fig.flowStart(home, "Main flow", "Open the details screen and come back");

  fig.rect(home, "Header", { x: 0, y: 0, width: 375, height: 96 }, GREY);
  const open = fig.rect(home, "Open details", { x: 88, y: 640, width: 200, height: 48 }, BLUE, 24);
  fig.on(open, "ON_CLICK", {
    kind: "navigate",
    to: details,
    transition: { type: "DISSOLVE", durationMs: 300, easing: "OUT_CUBIC" },
  });

  const back = fig.rect(details, "Back", { x: 16, y: 56, width: 80, height: 32 }, GREY, 8);
  fig.on(back, "ON_CLICK", { kind: "back" });

  return fig;
}

/** A menu opened as a centered overlay over a dimmed backdrop, closed by a button or outside. */
function overlay(): FigBuilder {
  const fig = new FigBuilder();
  const page = fig.page("Prototype");

  const home = fig.frame(page, "Home", { x: 0, y: 0, ...PHONE });
  const menu = fig.frame(page, "Menu", { x: 475, y: 0, width: 280, height: 320 }, "#FFFFFF");
  fig.flowStart(home, "Overlay flow");
  fig.overlay(menu, { position: "CENTER", closeOnClickOutside: true, backdrop: "#00000066" });

  const open = fig.rect(home, "Open menu", { x: 88, y: 640, width: 200, height: 48 }, BLUE, 24);
  fig.on(open, "ON_CLICK", {
    kind: "overlay",
    to: menu,
    transition: { type: "MOVE_FROM_BOTTOM", durationMs: 250, easing: "OUT_CUBIC" },
  });
  for (let i = 0; i < 3; i++) {
    fig.rect(menu, `Item ${i + 1}`, { x: 24, y: 24 + i * 64, width: 232, height: 48 }, GREY, 8);
  }
  const close = fig.rect(menu, "Close", { x: 24, y: 248, width: 232, height: 48 }, BLUE, 24);
  fig.on(close, "ON_CLICK", { kind: "close" });

  return fig;
}

/** A vertically scrolling feed with a fixed header and tab bar, and a "back to top" link. */
function scroll(): FigBuilder {
  const fig = new FigBuilder();
  const page = fig.page("Prototype");

  const feed = fig.frame(page, "Feed", { x: 0, y: 0, ...PHONE }, "#F9FAFB");
  fig.flowStart(feed, "Scroll flow");
  fig.scrollable(feed);

  const cards = Array.from({ length: 10 }, (_, i) =>
    fig.rect(
      feed,
      `Card ${i + 1}`,
      { x: 16, y: 112 + i * 216, width: 343, height: 200 },
      i % 2 ? GREY : "#DBEAFE",
      16,
    ),
  );
  const top = fig.rect(
    feed,
    "Back to top",
    { x: 88, y: 112 + 10 * 216, width: 200, height: 48 },
    BLUE,
    24,
  );
  fig.on(top, "ON_CLICK", {
    kind: "scrollTo",
    to: cards[0]!,
    transition: { type: "SCROLL_ANIMATE", durationMs: 400, easing: "INOUT_CUBIC" },
  });
  fig.fixed(fig.rect(feed, "Header", { x: 0, y: 0, width: 375, height: 96 }, "#FFFFFF"));
  fig.fixed(fig.rect(feed, "Tab bar", { x: 0, y: 728, width: 375, height: 84 }, "#FFFFFF"));

  return fig;
}

/** A splash screen that moves on by itself, then a button with hover and press feedback. */
function triggers(): FigBuilder {
  const fig = new FigBuilder();
  const page = fig.page("Prototype");

  const splash = fig.frame(page, "Splash", { x: 0, y: 0, ...PHONE }, BLUE);
  const home = fig.frame(page, "Home", { x: 475, y: 0, ...PHONE });
  const hovered = fig.frame(page, "Home, button hovered", { x: 950, y: 0, ...PHONE });
  const pressed = fig.frame(page, "Home, button pressed", { x: 1425, y: 0, ...PHONE });
  fig.flowStart(splash, "Triggers flow");

  fig.on(
    splash,
    "AFTER_TIMEOUT",
    { kind: "navigate", to: home, transition: { type: "DISSOLVE", durationMs: 500 } },
    1500,
  );
  const button = { x: 88, y: 640, width: 200, height: 48 };
  const plain = fig.rect(home, "Button", button, BLUE, 24);
  fig.on(plain, "ON_HOVER", { kind: "navigate", to: hovered });
  fig.on(plain, "ON_PRESS", { kind: "navigate", to: pressed });
  // While hovering, the hovered screen is shown: pressing must be wired on its button too.
  const hoveredButton = fig.rect(hovered, "Button", button, "#1D4ED8", 24);
  fig.on(hoveredButton, "ON_PRESS", { kind: "navigate", to: pressed });
  fig.rect(pressed, "Button", button, "#1E3A8A", 24);

  return fig;
}

/** Two pages; the first has two flows of two screens each, the second one flow. */
function flowsAndPages(): FigBuilder {
  const fig = new FigBuilder();

  const onboarding = fig.page("Onboarding");
  for (const [i, name] of ["Sign up", "Log in"].entries()) {
    const y = i * 912;
    const first = fig.frame(onboarding, `${name} 1`, { x: 0, y, ...PHONE });
    const second = fig.frame(onboarding, `${name} 2`, { x: 475, y, ...PHONE }, "#F9FAFB");
    fig.flowStart(first, name);
    const next = fig.rect(first, "Next", { x: 88, y: 640, width: 200, height: 48 }, BLUE, 24);
    fig.on(next, "ON_CLICK", {
      kind: "navigate",
      to: second,
      transition: { type: "PUSH_FROM_RIGHT", durationMs: 300, easing: "INOUT_CUBIC" },
    });
    const back = fig.rect(second, "Back", { x: 16, y: 56, width: 80, height: 32 }, GREY, 8);
    fig.on(back, "ON_CLICK", { kind: "back" });
  }

  const app = fig.page("App");
  const dashboard = fig.frame(app, "Dashboard", { x: 0, y: 0, ...PHONE });
  const settings = fig.frame(app, "Settings", { x: 475, y: 0, ...PHONE }, "#F9FAFB");
  fig.flowStart(dashboard, "App");
  const gear = fig.rect(
    dashboard,
    "Settings button",
    { x: 311, y: 56, width: 48, height: 48 },
    GREY,
    24,
  );
  fig.on(gear, "ON_CLICK", {
    kind: "navigate",
    to: settings,
    transition: { type: "SLIDE_FROM_RIGHT", durationMs: 300, easing: "OUT_CUBIC" },
  });
  const done = fig.rect(settings, "Done", { x: 16, y: 56, width: 80, height: 32 }, GREY, 8);
  fig.on(done, "ON_CLICK", { kind: "back" });

  return fig;
}

const fixtures: Record<string, () => FigBuilder> = {
  "fig/navigation.fig": navigation,
  "fig/overlay.fig": overlay,
  "fig/scroll.fig": scroll,
  "fig/triggers.fig": triggers,
  "fig/flows-and-pages.fig": flowsAndPages,
};

for (const [path, build] of Object.entries(fixtures)) {
  const out = join(FIXTURES, path);
  mkdirSync(dirname(out), { recursive: true });
  writeFileSync(out, build().toBytes());
  console.log(`wrote fixtures/${path}`);
}
