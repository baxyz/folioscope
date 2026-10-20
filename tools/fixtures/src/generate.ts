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

const fixtures: Record<string, () => FigBuilder> = {
  "fig/navigation.fig": navigation,
};

for (const [path, build] of Object.entries(fixtures)) {
  const out = join(FIXTURES, path);
  mkdirSync(dirname(out), { recursive: true });
  writeFileSync(out, build().toBytes());
  console.log(`wrote fixtures/${path}`);
}
