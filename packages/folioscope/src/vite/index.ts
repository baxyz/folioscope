// SPDX-License-Identifier: AGPL-3.0-or-later
// The plugin runs in the user's Vite, not in Vite+: its types come from "vite".
// oxlint-disable-next-line vite-plus/prefer-vite-plus-imports
import type { Plugin, ResolvedConfig } from "vite";
import { mediaTypeOfPath } from "helpers4/binary";
import { createHash } from "node:crypto";
import {
  existsSync,
  globSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, extname, join, relative, resolve } from "node:path";

export interface FolioscopeOptions {
  /**
   * Design files to bake, as globs relative to the Vite root. Each one is served as
   * `/<base>/<file name>.folio/`. Defaults to `**\/*.{fig,sketch,xd,penpot}` outside `node_modules`.
   */
  include?: string | string[];
  /** URL folder the baked prototypes are served from. Defaults to `folio`. */
  base?: string;
  /** Where baked files are cached, by hash of their source. Defaults to Vite's cache folder. */
  cacheDir?: string;
  /**
   * Encode large, photo-like screens as AVIF when it beats lossless WebP: much lighter, slower
   * to bake, lossy. Defaults to `true`.
   */
  avif?: boolean;
  /**
   * Fonts for text the design file stores no glyphs for: font files, folders of them, or globs,
   * relative to the Vite root (TTF, OTF, TTC, WOFF, WOFF2). A `.fig` file rarely needs any.
   */
  fonts?: string | string[];
}

/** Files of one `.folio` folder, by path inside it (`index.json`, `layers/…`). */
type Folio = Map<string, Uint8Array>;

interface Wasm {
  bake(
    input: Uint8Array,
    avif: boolean,
    fonts: Map<string, Uint8Array>,
    densities?: Uint8Array,
  ): { files: Map<string, Uint8Array>; missingFonts: string[] };
}

/** Bumped when the bake's output changes, so stale cache entries are not reused. */
const CACHE_VERSION = "3";

const FONT_FILE = /\.(ttf|otf|ttc|woff2?)$/i;

/**
 * Bakes design files into `.folio` folders while the site builds, and serves them in dev
 * (ADR 0009: the bake runs as WebAssembly, so no Rust is needed).
 *
 * ```ts
 * import { folioscope } from "folioscope/vite";
 * export default { plugins: [folioscope({ include: "designs/*.fig" })] };
 * ```
 * then `<folioscope-player src="/folio/app.folio">` for `designs/app.fig`.
 */
export function folioscope(options: FolioscopeOptions = {}): Plugin {
  const base = (options.base ?? "folio").replace(/^\/+|\/+$/g, "");
  const include = [options.include ?? "**/*.{fig,sketch,xd,penpot}"].flat();
  let config: ResolvedConfig;
  let cacheDir: string;
  const baked = new Map<string, Promise<Folio>>();

  /** Design files by the name they are served under (`app` for `designs/app.fig`). */
  const designs = (): Map<string, string> => {
    const found = new Map<string, string>();
    for (const file of globSync(include, {
      cwd: config.root,
      exclude: (f) => f.includes("node_modules"),
    })) {
      const path = resolve(config.root, file);
      const name = basename(file, extname(file));
      const other = found.get(name);
      if (other && other !== path) {
        throw new Error(
          `folioscope: ${relative(config.root, other)} and ${file} would both be served as ${name}.folio`,
        );
      }
      found.set(name, path);
    }
    return found;
  };

  let fonts: Map<string, Uint8Array> | undefined;
  /** The `fonts` option's files, by path from the root, read once. */
  const loadFonts = (): Map<string, Uint8Array> => {
    if (fonts) return fonts;
    fonts = new Map();
    for (const pattern of [options.fonts ?? []].flat()) {
      const matches = globSync(pattern, { cwd: config.root });
      for (const match of matches.toSorted()) {
        const path = resolve(config.root, match);
        const files = statSync(path).isDirectory()
          ? readdirSync(path, { recursive: true, encoding: "utf8" })
              .toSorted()
              .map((f) => join(path, f))
          : [path];
        for (const file of files) {
          if (FONT_FILE.test(file)) fonts.set(relative(config.root, file), readFileSync(file));
        }
      }
    }
    return fonts;
  };

  const bake = (path: string): Promise<Folio> => {
    let folio = baked.get(path);
    if (!folio) {
      folio = bakeCached(path, cacheDir, options.avif ?? true, loadFonts(), (font) =>
        config.logger.warn(
          `folioscope: ${relative(config.root, path)}: no font for ${font}; its text is left out or keeps another style (fonts option)`,
        ),
      );
      baked.set(path, folio);
    }
    return folio;
  };

  return {
    name: "folioscope",

    configResolved(resolved) {
      config = resolved;
      cacheDir = options.cacheDir
        ? resolve(config.root, options.cacheDir)
        : join(config.cacheDir, "folioscope");
    },

    async buildStart() {
      if (config.command !== "build") return;
      for (const [name, path] of designs()) {
        this.addWatchFile(path);
        for (const [file, source] of await bake(path)) {
          this.emitFile({ type: "asset", fileName: `${base}/${name}.folio/${file}`, source });
        }
      }
    },

    configureServer(server) {
      const watched = designs();
      server.watcher.add([...watched.values()]);
      server.watcher.on("change", (path) => {
        if (baked.delete(resolve(path))) server.ws.send({ type: "full-reload" });
      });
      server.middlewares.use((req, res, next) => {
        const match = /^\/([^?#]+?)\.folio\/([^?#]+)/.exec(decodeURIComponent(req.url ?? ""));
        const prefix = `${base}/`;
        if (!match || !match[1]!.startsWith(prefix)) return next();
        // Under the prototypes' folder, a miss is a 404, not Vite's index.html fallback.
        const notFound = () => {
          res.statusCode = 404;
          res.end();
        };
        const path = designs().get(match[1]!.slice(prefix.length));
        if (!path) return notFound();
        bake(path).then(
          (folio) => {
            const body = folio.get(match[2]!);
            if (!body) return notFound();
            res.setHeader("content-type", mediaTypeOfPath(match[2]!) ?? "application/octet-stream");
            res.end(body);
          },
          (error: unknown) => next(error),
        );
      });
    },
  };
}

/** Bakes `path`, or reads the result of an earlier bake of the same bytes from `cacheDir`. */
async function bakeCached(
  path: string,
  cacheDir: string,
  avif: boolean,
  fonts: Map<string, Uint8Array>,
  warn: (font: string) => void,
): Promise<Folio> {
  const source = readFileSync(path);
  const hash = createHash("sha256").update(`${CACHE_VERSION}:${avif}`).update(source);
  for (const [name, bytes] of fonts) hash.update(name).update(bytes);
  const key = hash.digest("hex").slice(0, 32);
  const dir = join(cacheDir, key);
  if (existsSync(join(dir, "index.json"))) return readFolder(dir);

  const { files: folio, missingFonts } = (await loadWasm()).bake(source, avif, fonts);
  missingFonts.forEach(warn);
  for (const [file, bytes] of folio) {
    mkdirSync(dirname(join(dir, file)), { recursive: true });
    writeFileSync(join(dir, file), bytes);
  }
  return folio;
}

function readFolder(dir: string): Folio {
  const folio: Folio = new Map();
  for (const file of readdirSync(dir, { recursive: true, withFileTypes: true })) {
    if (file.isFile()) {
      const path = join(file.parentPath, file.name);
      folio.set(relative(dir, path).split("\\").join("/"), readFileSync(path));
    }
  }
  return folio;
}

let wasm: Promise<Wasm> | undefined;

/** Loads the WebAssembly bake once, on the first design that is not cached. */
function loadWasm(): Promise<Wasm> {
  wasm ??= (async () => {
    // Same relative path from src/vite/ and from dist/vite/: the package's wasm/ folder.
    const glue = new URL("../../wasm/folioscope_wasm.js", import.meta.url);
    const binary = new URL("../../wasm/folioscope_wasm_bg.wasm", import.meta.url);
    if (!existsSync(binary)) {
      throw new Error(
        "folioscope/vite: the WebAssembly bake is missing; in the folioscope repository, run `pnpm --filter folioscope build:wasm`",
      );
    }
    const module = (await import(glue.href)) as Wasm & {
      initSync(init: { module: Uint8Array }): unknown;
    };
    module.initSync({ module: readFileSync(binary) });
    return module;
  })();
  return wasm;
}
