// SPDX-License-Identifier: AGPL-3.0-or-later
// Records the comparison page's screen recording: the smart animate demo played in Chromium, as a
// visitor would (open the details, come back), saved as public/comparison/smart-animate.webm.
// Run once after `bake-demos`, with the player built: `node scripts/record-comparison.ts`.
import { chromium } from "playwright";
import { copyFileSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { serve } from "../../../packages/folioscope/e2e/serve.ts";

const out = fileURLToPath(new URL("../public/comparison/smart-animate.webm", import.meta.url));
// The demo as `bake-demos` baked it, with its font.
const site = fileURLToPath(new URL("../public/folio/", import.meta.url));
const server = await serve(
  site,
  () => `<!doctype html><meta charset="utf-8"><style>body{margin:0}</style>
<script type="module" src="/player.mjs"></script>
<folioscope-player src="/smart-animate.folio" style="width: 375px"></folioscope-player>`,
);
const videos = mkdtempSync(join(tmpdir(), "folioscope-video-"));
const browser = await chromium.launch();
const context = await browser.newContext({
  viewport: { width: 375, height: 812 },
  recordVideo: { dir: videos, size: { width: 375, height: 812 } },
});
const tab = await context.newPage();
await tab.goto(server.origin);
await tab.locator("folioscope-player .stage > .frame").waitFor();
await tab.waitForTimeout(1200);
await tab.locator('folioscope-player .frame button[aria-label="Open"]').click();
await tab.waitForTimeout(1500);
await tab.locator('folioscope-player .frame button[aria-label="Back"]').click();
await tab.waitForTimeout(1500);
const video = tab.video()!;
await context.close();
copyFileSync(await video.path(), out);
await browser.close();
server.close();
console.log(`wrote ${out}`);
