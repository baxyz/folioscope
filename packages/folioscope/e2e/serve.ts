// SPDX-License-Identifier: AGPL-3.0-or-later
// Test helpers: bake fixtures with the CLI, and serve them with the built player.
import { mediaTypeOfPath } from "helpers4/binary";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join, normalize } from "node:path";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";

const repo = fileURLToPath(new URL("../../../", import.meta.url));

/**
 * Bakes fixtures into a new folder: `navigation` is `fixtures/figma/navigation.fig`, baked as
 * `navigation.folio`; `sketch/navigation.sketch` is that file, baked as `sketch-navigation.folio`.
 */
export function bakeFixtures(names: string[]): string {
  const site = mkdtempSync(join(tmpdir(), "folioscope-e2e-"));
  const cli = process.env.FOLIOSCOPE_CLI ?? join(repo, "target/debug/folioscope");
  for (const name of names) {
    const [file, folio] = name.includes("/")
      ? [name, name.replace("/", "-").replace(/\.\w+$/, "")]
      : [`figma/${name}.fig`, name];
    execFileSync(cli, ["bake", join(repo, "fixtures", file), join(site, `${folio}.folio`)], {
      stdio: "inherit",
    });
  }
  return site;
}

/**
 * Serves `page(query)` at `/`, the built player at `/player.mjs` (and its full player under
 * `/full/` and `/wasm/`), and the files of `site`.
 * Text is gzipped, as any host would.
 */
export async function serve(
  site: string,
  page: (query: URLSearchParams) => string,
): Promise<{ origin: string; close(): void }> {
  const server = createServer((req, res) => {
    const url = new URL(req.url!, "http://x");
    const path = normalize(decodeURIComponent(url.pathname));
    let body: Buffer;
    try {
      body =
        path === "/"
          ? Buffer.from(page(url.searchParams))
          : path === "/player.mjs"
            ? readFileSync(join(repo, "packages/folioscope/dist/index.mjs"))
            : // The chunks the player shares with the full player and the plugin.
              path.startsWith("/lib-")
              ? readFileSync(join(repo, "packages/folioscope/dist", path))
              : // The full player's module and worker load from next to the player, the
                // WebAssembly bake from the package's wasm/.
                path.startsWith("/full/")
                ? readFileSync(join(repo, "packages/folioscope/dist", path))
                : path.startsWith("/wasm/")
                  ? readFileSync(join(repo, "packages/folioscope", path))
                  : readFileSync(join(site, path));
    } catch {
      res.writeHead(404).end();
      return;
    }
    const type = mediaTypeOfPath(path === "/" ? "index.html" : path) ?? "application/octet-stream";
    const gzip = !type.startsWith("image/") && /gzip/.test(req.headers["accept-encoding"] ?? "");
    res.writeHead(200, { "content-type": type, ...(gzip && { "content-encoding": "gzip" }) });
    res.end(gzip ? gzipSync(body) : body);
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const { port } = server.address() as { port: number };
  return { origin: `http://127.0.0.1:${port}`, close: () => server.close() };
}
