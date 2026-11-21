// SPDX-License-Identifier: AGPL-3.0-or-later
// The player loaded from another origin, as from a CDN: it plays a raw design file of the page's
// origin, its bake worker started through a module of the page's own (a worker must come from
// the page's origin), its WebAssembly fetched from the CDN.
import { readFileSync } from "node:fs";
import { createServer, type RequestListener } from "node:http";
import { extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium, firefox } from "playwright";

const pkg = fileURLToPath(new URL("../", import.meta.url));
const repo = fileURLToPath(new URL("../../../", import.meta.url));
const TYPES: Record<string, string> = {
  ".mjs": "text/javascript",
  ".js": "text/javascript",
  ".wasm": "application/wasm",
};

const servers: { close(): void }[] = [];
async function listen(handler: RequestListener): Promise<string> {
  const server = createServer(handler);
  servers.push(server);
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  return `http://127.0.0.1:${(server.address() as { port: number }).port}`;
}

/** The package as a CDN serves it: its files, readable from any origin. */
const cdn = await listen((req, res) => {
  const path = normalize(decodeURIComponent(new URL(req.url!, "http://x").pathname));
  let body: Buffer;
  try {
    body = readFileSync(join(pkg, path));
  } catch {
    res.writeHead(404).end();
    return;
  }
  res.writeHead(200, {
    "content-type": TYPES[extname(path)] ?? "application/octet-stream",
    "access-control-allow-origin": "*",
  });
  res.end(body);
});

/** The site: a page, the design file and its font. */
const files: Record<string, string> = {
  "/navigation.fig": "fixtures/figma/navigation.fig",
  "/Inter.woff": "fixtures/fonts/Inter-Latin.woff",
};
const site = await listen((req, res) => {
  const path = new URL(req.url!, "http://x").pathname;
  if (path === "/") {
    res.writeHead(200, { "content-type": "text/html" });
    res.end(`<!doctype html><meta charset="utf-8">
<script type="module" src="${cdn}/dist/index.mjs"></script>
<folioscope-player src="/navigation.fig" style="width: 375px">
  <folioscope-font src="/Inter.woff"></folioscope-font>
</folioscope-player>`);
    return;
  }
  const file = files[path];
  if (!file) {
    res.writeHead(404).end();
    return;
  }
  res.writeHead(200);
  res.end(readFileSync(join(repo, file)));
});

try {
  for (const engine of [chromium, firefox]) {
    const browser = await engine.launch();
    const page = await browser.newPage();
    await page.goto(`${site}/`);
    await page.waitForFunction(
      () => (document.querySelector("folioscope-player") as { frame?: unknown } | null)?.frame,
      undefined,
      { timeout: 60_000 },
    );
    await browser.close();
    console.log(`✓ cdn (${engine.name()}): a player from another origin bakes a raw file`);
  }
} finally {
  for (const server of servers) server.close();
}
