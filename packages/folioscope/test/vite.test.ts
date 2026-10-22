// SPDX-License-Identifier: AGPL-3.0-or-later
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, test } from "vite-plus/test";
// oxlint-disable-next-line vite-plus/prefer-vite-plus-imports
import { build, createServer } from "vite";
import { folioscope } from "../src/vite/index.ts";

const fixture = fileURLToPath(new URL("../../../fixtures/figma/navigation.fig", import.meta.url));
const wasmBuilt = existsSync(
  fileURLToPath(new URL("../wasm/folioscope_wasm_bg.wasm", import.meta.url)),
);

/** A site with one design file; the plugin bakes it into /folio/app.folio/. */
function site(): string {
  const root = mkdtempSync(join(tmpdir(), "folioscope-vite-"));
  mkdirSync(join(root, "designs"));
  cpSync(fixture, join(root, "designs/app.fig"));
  writeFileSync(
    join(root, "index.html"),
    '<folioscope-player src="/folio/app.folio"></folioscope-player>',
  );
  return root;
}

// Needs the WebAssembly bake: `pnpm --filter folioscope build:wasm` (CI builds it in the e2e job).
describe.skipIf(!wasmBuilt)("folioscope/vite", () => {
  test("bakes design files into the build output, then reuses the cache", async () => {
    const root = site();
    const config = {
      root,
      logLevel: "silent" as const,
      plugins: [folioscope({ include: "designs/*.fig" })],
    };
    await build(config);

    const indexPath = join(root, "dist/folio/app.folio/index.json");
    const raw = readFileSync(indexPath, "utf8");
    const index = JSON.parse(raw);
    expect(index.pages[0].frames.map((f: { name: string }) => f.name)).toEqual(["Home", "Details"]);
    const layer = index.pages[0].frames[0].layers[0].src;
    expect(existsSync(join(root, `dist/folio/app.folio/layers/${layer}@2x.webp`))).toBe(true);

    // The second build reads the cache instead of baking again.
    const started = performance.now();
    await build(config);
    expect(performance.now() - started).toBeLessThan(2000);
    expect(readFileSync(indexPath, "utf8")).toBe(raw);
  });

  test("serves baked files in dev", async () => {
    const root = site();
    const server = await createServer({
      root,
      logLevel: "silent",
      server: { port: 0, host: "127.0.0.1" },
      plugins: [folioscope({ include: "designs/*.fig" })],
    });
    await server.listen();
    try {
      const url = server.resolvedUrls!.local[0]!;
      const response = await fetch(`${url}folio/app.folio/index.json`);
      expect(response.headers.get("content-type")).toBe("application/json");
      expect((await response.json()).folio).toBe(1);
      expect((await fetch(`${url}folio/missing.folio/index.json`)).status).toBe(404);
    } finally {
      await server.close();
    }
  });

  test("refuses two designs served under the same name", async () => {
    const root = site();
    mkdirSync(join(root, "other"));
    cpSync(fixture, join(root, "other/app.fig"));
    await expect(build({ root, logLevel: "silent", plugins: [folioscope()] })).rejects.toThrow(
      /both be served as app\.folio/,
    );
  });
});
