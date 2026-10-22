// SPDX-License-Identifier: AGPL-3.0-or-later
// Bakes the demo fixtures into public/folio with the folioscope CLI. Run after changing a fixture
// or the bake: CI fails when the committed demos are out of date.
import { execFileSync } from "node:child_process";
import { rmSync } from "node:fs";
import { fileURLToPath } from "node:url";

const repo = fileURLToPath(new URL("../../../", import.meta.url));
const demos = fileURLToPath(new URL("../public/folio/", import.meta.url));
const cli = process.env.FOLIOSCOPE_CLI ?? `${repo}target/debug/folioscope`;

rmSync(demos, { recursive: true, force: true });
for (const name of ["navigation", "flows-and-pages", "triggers", "overlay"]) {
  execFileSync(cli, ["bake", `${repo}fixtures/figma/${name}.fig`, `${demos}${name}.folio`], {
    stdio: "inherit",
  });
}
