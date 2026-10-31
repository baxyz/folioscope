// SPDX-License-Identifier: AGPL-3.0-or-later
// The bake worker: fetches a design file, bakes it with the WebAssembly bake, and caches the result
// by the file's hash (Cache API), so a second visit skips the bake.
import type { BakeMessage, BakeRequest, FontDeclaration } from "./index.ts";

interface Glue {
  default(init: { module_or_path: URL }): Promise<unknown>;
  bake(
    input: Uint8Array,
    avif: boolean,
    fonts: Map<string, Uint8Array>,
    densities?: Uint8Array,
    hints?: string,
  ): { files: Map<string, Uint8Array>; missingFonts: string[] };
}

// TODO(helpers4): worker requests and a Cache API file store (docs/helpers4-requests.md § 7).
/** Bumped when the bake's output changes, so stale entries are not reused. */
const CACHE_VERSION = "2";
const CACHE = "folioscope";

const post = (message: BakeMessage, transfer: Transferable[] = []) =>
  (globalThis as unknown as Worker).postMessage(message, transfer);

const hex = (digest: ArrayBuffer, bytes = 16) =>
  Array.from(new Uint8Array(digest).slice(0, bytes), (b) => b.toString(16).padStart(2, "0")).join(
    "",
  );
const sha256 = (bytes: Uint8Array<ArrayBuffer>) => crypto.subtle.digest("SHA-256", bytes);

globalThis.addEventListener("message", (event) => {
  bake((event as MessageEvent<BakeRequest>).data).catch((error: unknown) =>
    post({ type: "error", message: error instanceof Error ? error.message : String(error) }),
  );
});

/** The fonts the page declared, fetched; those that could not be are reported, not fatal. */
async function loadFonts(declared: FontDeclaration[]) {
  const loaded = await Promise.all(
    declared.map(async (font, index) => {
      const name = font.url ?? `font-${index}`;
      let bytes = font.data;
      if (!bytes && font.url) {
        const response = await fetch(font.url).catch(() => undefined);
        if (!response?.ok) {
          return {
            failed: `${font.url} (${response ? `HTTP ${response.status}` : "not reachable"})`,
          };
        }
        bytes = new Uint8Array(await response.arrayBuffer());
      }
      if (!bytes) return {};
      const hint = {
        name,
        family: font.family,
        match: font.match,
        weight: font.weight,
        italic: font.italic,
      };
      return { name, bytes, hint };
    }),
  );
  // In the order they were declared: the first of two equal fonts wins, whatever came first.
  const fonts = new Map<string, Uint8Array>();
  const hints: { name: string }[] = [];
  const failed: string[] = [];
  for (const font of loaded) {
    if (font.failed) failed.push(font.failed);
    if (font.name && font.bytes) fonts.set(font.name, font.bytes);
    if (font.hint) hints.push(font.hint);
  }
  return { fonts, hints, failed };
}

async function bake({ url, density, fonts: declared }: BakeRequest): Promise<void> {
  post({ type: "progress", stage: "download" });
  const [response, loaded] = await Promise.all([fetch(url), loadFonts(declared)]);
  if (declared.length) post({ type: "progress", stage: "fonts" });
  if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
  const source = new Uint8Array(await response.arrayBuffer());
  // What decides the bake: the file, and the fonts with what is said about them.
  const hash = hex(await sha256(source));
  const hintsJson = JSON.stringify(loaded.hints);
  let fontKey = "";
  for (const [name, bytes] of loaded.fonts) {
    fontKey += `${name}:${hex(await sha256(bytes as Uint8Array<ArrayBuffer>), 8)},`;
  }
  const fontHash = hex(await sha256(new TextEncoder().encode(fontKey + hintsJson)), 8);
  // Keys are made-up URLs: the Cache API wants URLs, these are never fetched.
  const key = (path: string) =>
    `https://folioscope.invalid/${CACHE_VERSION}/${hash}/${density}/${fontHash}/${encodeURIComponent(path)}`;

  const cache = await globalThis.caches?.open(CACHE).catch(() => undefined);
  const listed = await cache?.match(key("files.json"));
  if (cache && listed) {
    post({ type: "progress", stage: "cache" });
    const { paths, missingFonts } = (await listed.json()) as {
      paths: string[];
      missingFonts: string[];
    };
    const files: [string, Uint8Array][] = [];
    for (const path of paths) {
      const hit = await cache.match(key(path));
      if (!hit) return bakeAndStore();
      files.push([path, new Uint8Array(await hit.arrayBuffer())]);
    }
    return done(files, missingFonts);
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
    const result = glue.bake(source, false, loaded.fonts, Uint8Array.of(density), hintsJson);
    const files = [...result.files];
    const missingFonts = [...loaded.failed, ...result.missingFonts];
    if (cache) {
      await Promise.all(
        files.map(([path, bytes]) =>
          cache.put(key(path), new Response(bytes as Uint8Array<ArrayBuffer>)),
        ),
      );
      await cache.put(
        key("files.json"),
        Response.json({ paths: files.map(([path]) => path), missingFonts }),
      );
    }
    return done(files, missingFonts);
  }
}

function done(files: [string, Uint8Array][], missingFonts: string[]): void {
  post(
    { type: "done", files, missingFonts },
    files.map(([, bytes]) => bytes.buffer),
  );
}
