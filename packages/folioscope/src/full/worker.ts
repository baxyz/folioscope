// SPDX-License-Identifier: AGPL-3.0-or-later
// The bake worker: fetches a design file, bakes it with the WebAssembly bake, and caches the result
// by the file's hash (Cache API), so a second visit skips the bake.
import { createFileStore, sha256Hex } from "helpers4/browser";
import { fetchOrThrow } from "helpers4/fetch";
import { fontFaces, needed } from "./fonts.ts";
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
  families(input: Uint8Array): string[];
}

/** Bumped when the bake's output changes, so stale entries are not reused. */
const CACHE_VERSION = "5";
const store = createFileStore("folioscope");
/** Where the cache keeps the fonts the bake lacked, with the files. */
const MISSING_FONTS = "missing-fonts.json";

const post = (message: BakeMessage, transfer: Transferable[] = []) =>
  (globalThis as unknown as Worker).postMessage(message, transfer);

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
  post({ type: "progress", value: "download" });
  const source = new Uint8Array(await (await fetchOrThrow(url)).arrayBuffer());
  // What decides the bake: the file, and the fonts as the page declares them (by URL), so a
  // second visit reads the cache without loading the WebAssembly.
  const declarations = await Promise.all(
    declared.map(async (font) => ({
      ...font,
      data: font.data && (await sha256Hex(font.data as Uint8Array<ArrayBuffer>, 16)),
    })),
  );
  const key = [
    await sha256Hex(source, 32),
    density,
    await sha256Hex(new TextEncoder().encode(JSON.stringify(declarations)), 16),
  ].join("/");

  const stored = await store.get(key, CACHE_VERSION);
  if (stored) {
    post({ type: "progress", value: "cache" });
    const { [MISSING_FONTS]: missing, ...layers } = stored;
    const files: [string, Uint8Array][] = [];
    for (const [path, blob] of Object.entries(layers)) {
      files.push([path, new Uint8Array(await blob.arrayBuffer())]);
    }
    return done(files, missing ? (JSON.parse(await missing.text()) as string[]) : []);
  }

  const glue = (await import(
    new URL("../../wasm/folioscope_wasm.js", import.meta.url).href
  )) as Glue;
  await glue.default({
    module_or_path: new URL("../../wasm/folioscope_wasm_bg.wasm", import.meta.url),
  });
  // Only the fonts the design's text uses are fetched.
  const used = glue.families(source);
  const expanded = await expand(declared);
  const loaded = await loadFonts(expanded.fonts.filter((font) => needed(font, used)));
  if (declared.length) post({ type: "progress", value: "fonts" });
  post({ type: "progress", value: "bake" });
  const result = glue.bake(
    source,
    false,
    loaded.fonts,
    Uint8Array.of(density),
    JSON.stringify(loaded.hints),
  );
  const files = [...result.files];
  const missingFonts = [...expanded.failed, ...loaded.failed, ...result.missingFonts];
  await store.put(
    key,
    {
      ...Object.fromEntries(files.map(([path, bytes]) => [path, bytes as Uint8Array<ArrayBuffer>])),
      [MISSING_FONTS]: JSON.stringify(missingFonts),
    },
    CACHE_VERSION,
  );
  return done(files, missingFonts);
}

/** One face of a `folioscope-fonts.json` manifest, as `folioscope fonts` writes it. */
interface ManifestFace {
  family: string;
  weight: [number, number];
  italic: boolean;
  source: string;
}

/**
 * The declared fonts with their `base` folders and `css` stylesheets read: one declaration per
 * face, with its family, so it is fetched only when used. Folders and sheets that cannot be read
 * are reported.
 */
async function expand(declared: FontDeclaration[]) {
  const fonts: FontDeclaration[] = [];
  const failed: string[] = [];
  for (const font of declared) {
    if (font.base) {
      const manifest = new URL("folioscope-fonts.json", font.base);
      const faces = await fetchOrThrow(manifest)
        .then((r) => r.json() as Promise<ManifestFace[]>)
        .catch(() => undefined);
      if (!faces) {
        failed.push(`${manifest.href} (not reachable)`);
        continue;
      }
      for (const face of faces) {
        fonts.push({
          url: new URL(face.source, manifest).href,
          family: face.family,
          match: font.match,
          weight: face.weight[0] === face.weight[1] ? face.weight[0] : undefined,
          italic: face.italic,
        });
      }
    } else if (font.css) {
      const sheet = await fetchOrThrow(font.css)
        .then((r) => r.text())
        .catch(() => undefined);
      if (sheet === undefined) {
        failed.push(`${font.css} (not reachable)`);
        continue;
      }
      fonts.push(...fontFaces(sheet, font.css));
    } else {
      fonts.push(font);
    }
  }
  return { fonts, failed };
}

function done(files: [string, Uint8Array][], missingFonts: string[]): void {
  post(
    { type: "result", value: { files, missingFonts } },
    files.map(([, bytes]) => bytes.buffer),
  );
}
