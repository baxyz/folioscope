// SPDX-License-Identifier: AGPL-3.0-or-later
// End-to-end: bakes fixtures/fig/navigation.fig, plays it in Firefox through the built player.
// Run with `pnpm --filter folioscope test:e2e` after `vp pack` and `cargo build -p folioscope-cli`.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";
import { firefox } from "playwright";

const repo = fileURLToPath(new URL("../../../", import.meta.url));
const site = mkdtempSync(join(tmpdir(), "folioscope-e2e-"));
const cli = process.env.FOLIOSCOPE_CLI ?? join(repo, "target/debug/folioscope");
execFileSync(
  cli,
  ["bake", join(repo, "fixtures/fig/navigation.fig"), join(site, "navigation.folio")],
  { stdio: "inherit" },
);

const page = `<!doctype html><meta charset="utf-8">
<script type="module" src="/player.mjs"></script>
<folioscope-player src="/navigation.folio" style="width: 375px"></folioscope-player>`;
const types: Record<string, string> = {
  ".png": "image/png",
  ".json": "application/json",
  ".mjs": "text/javascript",
};
const server = createServer((req, res) => {
  const path = normalize(decodeURIComponent(new URL(req.url!, "http://x").pathname));
  try {
    const body =
      path === "/"
        ? page
        : path === "/player.mjs"
          ? readFileSync(join(repo, "packages/folioscope/dist/index.mjs"))
          : readFileSync(join(site, path));
    res.writeHead(200, {
      "content-type":
        path === "/" ? "text/html" : (types[extname(path)] ?? "application/octet-stream"),
    });
    res.end(body);
  } catch {
    res.writeHead(404).end();
  }
});
await new Promise<void>((resolve) => server.listen(0, resolve));
const port = (server.address() as { port: number }).port;

const browser = await firefox.launch();
try {
  const tab = await browser.newPage({ viewport: { width: 375, height: 812 } });
  const errors: string[] = [];
  tab.on("pageerror", (e) => errors.push(e.message));
  await tab.goto(`http://localhost:${port}/`);

  // Settled on `name`: the only frame left once a transition is over.
  const settledOn = (name: string) =>
    tab.waitForFunction((name) => {
      const frames = document
        .querySelector("folioscope-player")
        ?.shadowRoot?.querySelectorAll(".frame");
      return frames?.length === 1 && frames[0]!.getAttribute("aria-label") === name;
    }, name);
  await settledOn("Home");
  console.log("✓ starts on the flow's first frame (Home)");

  await tab.locator('folioscope-player button[aria-label="Open details"]').click();
  await settledOn("Details");
  console.log("✓ the hotspot navigates to Details, with its transition");

  await tab.locator('folioscope-player button[aria-label="Back"]').click();
  await settledOn("Home");
  console.log("✓ the back hotspot returns to Home");

  await tab.locator('folioscope-player button[aria-label="Open details"]').focus();
  await tab.keyboard.press("Enter");
  await settledOn("Details");
  await tab.keyboard.press("Escape");
  await settledOn("Home");
  console.log("✓ keyboard: Enter follows a hotspot, Escape goes back");

  assert.deepEqual(errors, []);
} finally {
  await browser.close();
  server.close();
}
