// SPDX-License-Identifier: AGPL-3.0-or-later
// The comparison page, on the built site (`vitepress build` first), in Chromium and Firefox: Play
// shows the baked prototype and counts what it fetched, and plays again. Figma's side is not
// loaded: it needs Figma, so its requests are refused.
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

for (const engine of [chromium, firefox]) {
  const name = engine.name();
  const browser = await engine.launch();
  const page = await browser.newPage();
  await page.route(/figma\.com/, (route) => route.abort());
  try {
    await page.goto(`${origin}/comparison`);
    for (const round of [1, 2]) {
      await page.locator(".compare .play").click();
      await page.waitForFunction(
        () => document.querySelector<HTMLElement>('.side[data-side="folioscope"]')?.dataset.bytes,
        undefined,
        { timeout: 60_000 },
      );
      const bytes = Number(
        await page.locator('.side[data-side="folioscope"]').getAttribute("data-bytes"),
      );
      if (round === 1 && !(bytes > 10_000)) fail(`the .folio side counted ${bytes} bytes`);
      if (round === 2 && !(await page.locator(".compare .again").isVisible())) {
        fail("playing again does not say the screens come from the cache");
      }
    }
    console.log(
      `✓ comparison (${name}): Play shows the baked prototype and counts its data, again and again`,
    );
  } finally {
    await browser.close();
  }
}
server.close();
