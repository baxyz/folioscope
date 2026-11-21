// SPDX-License-Identifier: AGPL-3.0-or-later
// Bakes the demo fixtures into public/folio with the folioscope CLI. Run after changing a fixture
// or the bake: CI fails when the committed demos are out of date.
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, rmSync } from "node:fs";
import { fileURLToPath } from "node:url";

const repo = fileURLToPath(new URL("../../../", import.meta.url));
const demos = fileURLToPath(new URL("../public/folio/", import.meta.url));
const cli = process.env.FOLIOSCOPE_CLI ?? `${repo}target/debug/folioscope`;

rmSync(demos, { recursive: true, force: true });
for (const name of [
  "navigation",
  "flows-and-pages",
  "triggers",
  "overlay",
  "scroll",
  "components",
  "smart-animate",
  "sketch/navigation.sketch",
  "penpot/navigation.penpot",
  "xd/navigation.xd",
]) {
  // `sketch/navigation.sketch` bakes as `sketch-navigation.folio`; bare names are `.fig` files.
  const [file, folio] = name.includes("/")
    ? [name, name.replace("/", "-").replace(/\.\w+$/, "")]
    : [`figma/${name}.fig`, name];
  execFileSync(cli, ["bake", `${repo}fixtures/${file}`, `${demos}${folio}.folio`], {
    stdio: "inherit",
  });
}

// The comparison page's material, from the smart animate fixture: the raw file, for the full
// player, and every screen as a PNG at 2x, as a designer would export them.
// The recording next to them (`record-comparison.ts`) is kept.
const comparison = fileURLToPath(new URL("../public/comparison/", import.meta.url));
mkdirSync(comparison, { recursive: true });
copyFileSync(`${repo}fixtures/figma/smart-animate.fig`, `${comparison}smart-animate.fig`);
for (const [node, name] of [
  ["1:1", "home"],
  ["1:2", "details"],
]) {
  execFileSync(cli, [
    "render",
    `${repo}fixtures/figma/smart-animate.fig`,
    node!,
    `${comparison}smart-animate-${name}.png`,
    "2",
  ]);
  // `render` writes the SVG it rasterized next to the PNG: not part of the export.
  rmSync(`${comparison}smart-animate-${name}.png.svg`, { force: true });
  rmSync(`${comparison}smart-animate-${name}.svg`, { force: true });
}
