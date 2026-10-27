// SPDX-License-Identifier: AGPL-3.0-or-later
/**
 * The full player's part: bakes a raw design file in the browser, in a Web Worker, with the same
 * WebAssembly bake as `folioscope/vite`. `<folioscope-player>` loads it only when `src` is a raw
 * file with no `.folio` next to it.
 */

export type Stage = "download" | "cache" | "bake";

export interface BakeInBrowserOptions {
  /** The pixel density to bake, 1 or 2: one is enough for the screen at hand. */
  density: number;
  /** Fonts for text the file stores no glyphs for, by name. */
  fonts?: Map<string, Uint8Array>;
  onProgress?: (stage: Stage) => void;
}

export interface BakedInBrowser {
  /** The `.folio` folder's files by path: `index.json`, `layers/…`. */
  files: Map<string, Uint8Array>;
  /** Fonts some text needed and was not given. */
  missingFonts: string[];
}

/** Requests from the page to the worker. */
export interface BakeRequest {
  url: string;
  density: number;
  fonts: [string, Uint8Array][];
}

/** Messages from the worker to the page. */
export type BakeMessage =
  | { type: "progress"; stage: Stage }
  | { type: "done"; files: [string, Uint8Array][]; missingFonts: string[] }
  | { type: "error"; message: string };

/** Bakes the design file at `url` (absolute) into the files of a `.folio` folder. */
export function bakeInBrowser(url: string, options: BakeInBrowserOptions): Promise<BakedInBrowser> {
  const worker = new Worker(new URL("./worker.mjs", import.meta.url), { type: "module" });
  return new Promise<BakedInBrowser>((resolve, reject) => {
    worker.addEventListener("message", (event: MessageEvent<BakeMessage>) => {
      const message = event.data;
      if (message.type === "progress") return options.onProgress?.(message.stage);
      worker.terminate();
      if (message.type === "error") reject(new Error(message.message));
      else resolve({ files: new Map(message.files), missingFonts: message.missingFonts });
    });
    worker.addEventListener("error", (event) => {
      worker.terminate();
      reject(new Error(event.message || "folioscope: the bake worker failed"));
    });
    const request: BakeRequest = {
      url,
      density: options.density,
      fonts: [...(options.fonts ?? [])],
    };
    worker.postMessage(request);
  });
}
