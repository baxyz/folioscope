// SPDX-License-Identifier: AGPL-3.0-or-later
// The comparison page, on the built site (`vitepress build` first), in Chromium: each scenario
// made of our own material loads both sides, counts their data through the page's service
// worker, and a network profile slows them down. Figma's embed is left out: it needs Figma.
import { readFileSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium, firefox } from "playwright";

const dist = fileURLToPath(new URL("../.vitepress/dist/", import.meta.url));
const TYPES: Record<string, string> = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".mjs": "text/javascript",
  ".css": "text/css",
  ".json": "application/json",
  ".wasm": "application/wasm",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".webp": "image/webp",
  ".avif": "image/avif",
  ".webm": "video/webm",
  ".woff2": "font/woff2",
};

/** The built site as a host serves it: clean URLs, a 404 page. */
const server = createServer((req, res) => {
  const path = normalize(decodeURIComponent(new URL(req.url!, "http://x").pathname));
  for (const file of [path, `${path}.html`, join(path, "index.html")]) {
    try {
      const body = readFileSync(join(dist, file));
      res.writeHead(200, { "content-type": TYPES[extname(file)] ?? "application/octet-stream" });
      res.end(body);
      return;
    } catch {
      // The next candidate.
    }
  }
  res.writeHead(404, { "content-type": "text/html" }).end(readFileSync(join(dist, "404.html")));
});
await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
const { port } = server.address() as { port: number };
const origin = `http://127.0.0.1:${port}`;

const fail = (message: string): never => {
  throw new Error(message);
};

// Both engines: Firefox does not always hand a page to an active worker (the page asks it to
// claim it), and queues the worker's messages until asked.
for (const engine of [chromium, firefox]) {
  const name = engine.name();
  const browser = await engine.launch();
  const page = await browser.newPage();
  try {
    await page.goto(`${origin}/comparison`);
    const play = page.locator(".compare .play");
    await play.waitFor();
    // Enabled once the service worker controls the page.
    await page.waitForFunction(() => {
      const button = document.querySelector<HTMLButtonElement>(".compare .play");
      return button && !button.disabled && navigator.serviceWorker.controller;
    });
    console.log(`✓ comparison (${name}): the page's service worker starts`);

    /**
     * Plays a scenario; the time to first screen and the bytes of each side. The first run is
     * started by hand; the others from the page's URL, as "Play again" does, on a fresh page.
     */
    let first = true;
    async function measure(against: string, network = "yours") {
      if (first) {
        first = false;
        await page.selectOption('select[name="scenario"]', against);
        await page.selectOption('select[name="network"]', network);
        await play.click();
      } else {
        await page.goto(`${origin}/comparison?against=${against}&network=${network}&play`);
      }
      await page.waitForFunction(
        () =>
          [...document.querySelectorAll<HTMLElement>(".compare .side")].every(
            (side) => side.dataset.ms !== undefined,
          ),
        undefined,
        { timeout: 120_000 },
      );
      // The data counts arrive just after the files.
      await page.waitForTimeout(500);
      const read = (side: string) =>
        page.locator(`.side[data-side="${side}"]`).evaluate((el: HTMLElement) => ({
          ms: Number(el.dataset.ms),
          bytes: Number(el.dataset.bytes),
        }));
      return { left: await read("left"), right: await read("right") };
    }

    const raw = await measure("raw");
    if (raw.left.bytes < 1_000_000)
      fail(`the raw side counted ${raw.left.bytes} bytes, no WebAssembly`);
    if (!(raw.right.bytes > 0 && raw.right.bytes < raw.left.bytes / 10)) {
      fail(`the .folio side counted ${raw.right.bytes} bytes against ${raw.left.bytes}`);
    }
    console.log(
      `✓ comparison (${name}): the raw file is baked in the browser, its WebAssembly counted (${raw.left.bytes} against ${raw.right.bytes} bytes)`,
    );

    // Another choice after a run stops it: the pickers stay usable, the stages wait for Play.
    await page.selectOption('select[name="scenario"]', "images");
    await page.locator(".compare .idle").first().waitFor();
    await page.selectOption('select[name="network"]', "fast-4g");
    console.log(
      `✓ comparison (${name}): another choice after a run stops it, the pickers stay usable`,
    );

    const video = await measure("video");
    if (!(video.left.bytes > 50_000 && video.right.bytes > 0)) {
      fail(`the recording counted ${video.left.bytes} bytes, the .folio ${video.right.bytes}`);
    }
    console.log(`✓ comparison (${name}): a screen recording plays beside the .folio`);

    const images = await measure("images");
    if (!(images.left.bytes > 20_000 && images.right.bytes > 0)) {
      fail(`the PNGs counted ${images.left.bytes} bytes, the .folio ${images.right.bytes}`);
    }
    console.log(`✓ comparison (${name}): one PNG per screen loads beside the .folio`);

    const slow = await measure("images", "slow-4g");
    // At least a round trip for index.json, then one for a layer.
    if (slow.right.ms < 2 * 560 || slow.left.ms < 560) {
      fail(`Slow 4G took ${slow.left.ms} ms and ${slow.right.ms} ms`);
    }
    console.log(
      `✓ comparison (${name}): a network profile slows both sides (${slow.left.ms} and ${slow.right.ms} ms)`,
    );

    // Leaving by a link: the site's own navigation, without a reload.
    await page.locator('.VPNavBar a[href="/roadmap"]').first().click();
    await page.waitForURL(/\/roadmap/);
    await page.waitForFunction(
      async () => (await navigator.serviceWorker.getRegistrations()).length === 0,
    );
    console.log(`✓ comparison (${name}): leaving the page removes its service worker`);
  } finally {
    await browser.close();
  }
}
server.close();
