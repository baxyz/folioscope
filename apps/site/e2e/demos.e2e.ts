// SPDX-License-Identifier: AGPL-3.0-or-later
// The demos page, on the built site (`vitepress build` first), in Chromium: every demo of the
// gallery loads its prototype, the address keeps the one picked, and the clickable areas can be
// shown.
import { readFileSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const dist = fileURLToPath(new URL("../.vitepress/dist/", import.meta.url));
const TYPES: Record<string, string> = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".css": "text/css",
  ".json": "application/json",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".webp": "image/webp",
  ".avif": "image/avif",
  ".woff2": "font/woff2",
};
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
  res.writeHead(404).end();
});
await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
const origin = `http://127.0.0.1:${(server.address() as { port: number }).port}`;

const browser = await chromium.launch();
const page = await browser.newPage();
const fail = (message: string): never => {
  throw new Error(message);
};
/** Waits for the gallery's player to show a frame. */
const played = () =>
  page.waitForFunction(
    () =>
      (document.querySelector(".gallery folioscope-player") as { frame?: unknown } | null)?.frame,
    undefined,
    { timeout: 30_000 },
  );

try {
  await page.goto(`${origin}/demos`);
  await played();
  const items = page.locator(".gallery .list .item");
  const count = await items.count();
  for (let i = 0; i < count; i++) {
    const item = items.nth(i);
    const name = (await item.locator(".name").textContent())!;
    await item.click();
    await played().catch(() => fail(`the demo “${name}” did not play`));
  }
  console.log(`✓ demos: each of the ${count} demos plays`);

  await page.goto(`${origin}/demos#smart-animate`);
  await played();
  const title = await page.locator(".gallery .head h2").textContent();
  if (title !== "Smart animate") fail(`#smart-animate opened “${title}”`);
  await page.locator(".gallery .item", { hasText: "Overlays" }).click();
  if (!page.url().endsWith("#overlay")) fail(`picking a demo left the address at ${page.url()}`);
  console.log("✓ demos: the address opens a demo and keeps the one picked");

  await page.locator(".gallery .toggle input").check();
  await played();
  const hotspots = await page.locator(".gallery folioscope-player").getAttribute("hotspots");
  if (hotspots !== "always") fail(`the toggle set hotspots to ${hotspots}`);
  console.log("✓ demos: the clickable areas can be shown");
} finally {
  await browser.close();
  server.close();
}
