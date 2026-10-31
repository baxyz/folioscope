// SPDX-License-Identifier: AGPL-3.0-or-later
/**
 * The full player's part: bakes a raw design file in the browser, in a Web Worker, with the same
 * WebAssembly bake as `folioscope/vite`. `<folioscope-player>` loads it only when `src` is a raw
 * file with no `.folio` next to it.
 */
import { requestWorker, type WorkerReply } from "helpers4/browser";

export type Stage = "download" | "fonts" | "cache" | "bake";

/**
 * A font the page gives (`<folioscope-font>`, `registerFont()`): its file, by `url` or `data`, and
 * what to say about it instead of what the file says.
 */
export interface FontDeclaration {
  /** Where to fetch the font file: an absolute URL. */
  url?: string;
  /** The font file, when the page already has its bytes. */
  data?: Uint8Array;
  /** A family name the font also answers to, besides its own. */
  family?: string;
  /** Family names or patterns (`SF Pro*`) it also answers to: it stands in for them. */
  match?: string[];
  /** The weight it is, when the file says otherwise. */
  weight?: number;
  /** Whether it is italic, when the file says otherwise. */
  italic?: boolean;
}

export interface BakeInBrowserOptions {
  /** The pixel density to bake, 1 or 2: one is enough for the screen at hand. */
  density: number;
  /** Fonts for text the file stores no glyphs for. */
  fonts?: FontDeclaration[];
  onProgress?: (stage: Stage) => void;
}

export interface BakedInBrowser {
  /** The `.folio` folder's files by path: `index.json`, `layers/…`. */
  files: Map<string, Uint8Array>;
  /** Fonts some text needed and was not given, and fonts that could not be loaded. */
  missingFonts: string[];
}

/** Requests from the page to the worker. */
export interface BakeRequest {
  url: string;
  density: number;
  fonts: FontDeclaration[];
}

/** What the worker bakes: the files of the `.folio` folder, as pairs for a transfer. */
export interface BakeResult {
  files: [string, Uint8Array][];
  missingFonts: string[];
}

/** Messages from the worker to the page. */
export type BakeMessage = WorkerReply<BakeResult, Stage>;

/** Bakes the design file at `url` (absolute) into the files of a `.folio` folder. */
export async function bakeInBrowser(
  url: string,
  options: BakeInBrowserOptions,
): Promise<BakedInBrowser> {
  const request: BakeRequest = { url, density: options.density, fonts: options.fonts ?? [] };
  const { files, missingFonts } = await requestWorker<BakeRequest, BakeResult, Stage>(
    () => new Worker(new URL("./worker.mjs", import.meta.url), { type: "module" }),
    request,
    { onProgress: options.onProgress },
  );
  return { files: new Map(files), missingFonts };
}
