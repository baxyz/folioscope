// SPDX-License-Identifier: AGPL-3.0-or-later
// Performance budget: the first frame of a baked prototype shows in under a second on an average
// mobile on 4G. Chromium, throttled through the DevTools protocol: WebPageTest's "4G" profile
// (170 ms of latency, 9 Mb/s down) and a CPU four times slower than the runner's, as Lighthouse
// does for mobile. Lighthouse's own "slow 4G" adds 560 ms to every request, which no page made of
// four chained requests can meet; connection setup is not counted, the server is local.
// Run with `pnpm --filter folioscope test:budget` after `vp pack` and `cargo build -p folioscope`.
import assert from "node:assert/strict";
import { chromium } from "playwright";
import { bakeFixtures, serve } from "./serve.ts";

const BUDGET_MS = 1000;
const RUNS = 5;

const site = bakeFixtures(["navigation"]);
// Records, relative to the navigation start, when the poster and then the first frame are painted.
const probe = `<script>
  const marks = (window.marks = {});
  const root = () => document.querySelector("folioscope-player")?.shadowRoot;
  const poll = () => {
    const poster = root()?.querySelector(".poster");
    if (poster?.complete && poster.naturalWidth > 0) marks.poster ??= performance.now();
    if (root()?.querySelector(".frame")) marks.frame = performance.now();
    else requestAnimationFrame(poll);
  };
  requestAnimationFrame(poll);
</script>`;
const server = await serve(site, (query) =>
  query.has("hinted")
    ? // The documented fast setup: a poster, and index.json fetched along with the player.
      `<!doctype html><meta charset="utf-8">${probe}
<link rel="preload" href="/navigation.folio/index.json" as="fetch" crossorigin>
<script type="module" src="/player.mjs"></script>
<folioscope-player src="/navigation.folio" poster="/navigation.folio/poster.webp" style="width: 100%"></folioscope-player>`
    : `<!doctype html><meta charset="utf-8">${probe}
<script type="module" src="/player.mjs"></script>
<folioscope-player src="/navigation.folio" style="width: 100%"></folioscope-player>`,
);

const browser = await chromium.launch();
try {
  const measure = async (query: string): Promise<{ poster?: number; frame: number }> => {
    const context = await browser.newContext({
      viewport: { width: 390, height: 844 },
      deviceScaleFactor: 2,
      isMobile: true,
    });
    const tab = await context.newPage();
    const cdp = await context.newCDPSession(tab);
    await cdp.send("Network.enable");
    await cdp.send("Network.emulateNetworkConditions", {
      offline: false,
      latency: 170,
      downloadThroughput: (9_000 * 1000) / 8,
      uploadThroughput: (9_000 * 1000) / 8,
    });
    await cdp.send("Emulation.setCPUThrottlingRate", { rate: 4 });
    await tab.goto(`${server.origin}/?${query}`);
    await tab.waitForFunction(() => (window as { marks?: { frame?: number } }).marks?.frame);
    const marks = await tab.evaluate(() => (window as { marks?: object }).marks);
    await context.close();
    return marks as { poster?: number; frame: number };
  };
  const median = (values: number[]) => values.toSorted((a, b) => a - b)[values.length >> 1]!;

  for (const [label, query] of [
    ["bare player", ""],
    ["poster and preload", "hinted"],
  ] as const) {
    const runs = [];
    for (let i = 0; i < RUNS; i++) runs.push(await measure(query));
    const frame = Math.round(median(runs.map((r) => r.frame)));
    const posters = runs.flatMap((r) => (r.poster === undefined ? [] : [r.poster]));
    const poster = posters.length ? `, poster at ${Math.round(median(posters))} ms` : "";
    console.log(`${label}: first frame at ${frame} ms${poster} (median of ${RUNS})`);
    if (query === "hinted") assert.ok(frame < BUDGET_MS, `${frame} ms is over ${BUDGET_MS} ms`);
  }
  console.log(`✓ first frame under ${BUDGET_MS} ms on 4G with the documented setup`);
} finally {
  await browser.close();
  server.close();
}
