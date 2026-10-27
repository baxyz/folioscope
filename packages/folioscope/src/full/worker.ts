// SPDX-License-Identifier: AGPL-3.0-or-later
// The bake worker: fetches a design file, bakes it with the WebAssembly bake, and caches the result
// by the file's hash (Cache API), so a second visit skips the bake.
import type { BakeMessage, BakeRequest } from "./index.ts";

interface Glue {
  default(init: { module_or_path: URL }): Promise<unknown>;
  bake(
    input: Uint8Array,
    avif: boolean,
    fonts: Map<string, Uint8Array>,
    densities?: Uint8Array,
  ): { files: Map<string, Uint8Array>; missingFonts: string[] };
}

// TODO(helpers4): worker requests and a Cache API file store (docs/helpers4-requests.md § 7).
/** Bumped when the bake's output changes, so stale entries are not reused. */
const CACHE_VERSION = "1";
const CACHE = "folioscope";

const post = (message: BakeMessage, transfer: Transferable[] = []) =>
  (globalThis as unknown as Worker).postMessage(message, transfer);

globalThis.addEventListener("message", (event) => {
  bake((event as MessageEvent<BakeRequest>).data).catch((error: unknown) =>
    post({ type: "error", message: error instanceof Error ? error.message : String(error) }),
  );
});

async function bake({ url, density, fonts }: BakeRequest): Promise<void> {
  post({ type: "progress", stage: "download" });
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
  const source = new Uint8Array(await response.arrayBuffer());
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", source));
  const hash = Array.from(digest.slice(0, 16), (b) => b.toString(16).padStart(2, "0")).join("");
  const fontKey = fonts.map(([name, bytes]) => `${name}:${bytes.length}`).join(",");
  // Keys are made-up URLs: the Cache API wants URLs, these are never fetched.
  const key = (path: string) =>
    `https://folioscope.invalid/${CACHE_VERSION}/${hash}/${density}/${encodeURIComponent(fontKey)}/${path}`;

  const cache = await globalThis.caches?.open(CACHE).catch(() => undefined);
  const listed = await cache?.match(key("files.json"));
  if (cache && listed) {
    post({ type: "progress", stage: "cache" });
    const paths = (await listed.json()) as string[];
    const files: [string, Uint8Array][] = [];
    for (const path of paths) {
      const hit = await cache.match(key(path));
      if (!hit) return bakeAndStore();
      files.push([path, new Uint8Array(await hit.arrayBuffer())]);
    }
    return done(files, []);
  }
  return bakeAndStore();

  async function bakeAndStore(): Promise<void> {
    post({ type: "progress", stage: "bake" });
    const glue = (await import(
      new URL("../../wasm/folioscope_wasm.js", import.meta.url).href
    )) as Glue;
    await glue.default({
      module_or_path: new URL("../../wasm/folioscope_wasm_bg.wasm", import.meta.url),
    });
    const result = glue.bake(source, false, new Map(fonts), Uint8Array.of(density));
    const files = [...result.files];
    if (cache) {
      await Promise.all(
        files.map(([path, bytes]) =>
          cache.put(key(path), new Response(bytes as Uint8Array<ArrayBuffer>)),
        ),
      );
      await cache.put(key("files.json"), Response.json(files.map(([path]) => path)));
    }
    return done(files, result.missingFonts);
  }
}

function done(files: [string, Uint8Array][], missingFonts: string[]): void {
  post(
    { type: "done", files, missingFonts },
    files.map(([, bytes]) => bytes.buffer),
  );
}
