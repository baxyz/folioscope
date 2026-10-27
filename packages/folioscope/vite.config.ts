// SPDX-License-Identifier: AGPL-3.0-or-later
import { defineConfig } from "vite-plus";

export default defineConfig({
  pack: {
    // full/ is loaded on demand by the player, its worker by full/: entries of their own, so their
    // relative URLs (./worker.mjs, ../../wasm/) hold in dist.
    entry: ["src/index.ts", "src/vite/index.ts", "src/full/index.ts", "src/full/worker.ts"],
    // The player is loaded straight from a CDN as one module: bundle its dependencies.
    deps: { alwaysBundle: ["thumbhash", "helpers4", /^@helpers4\//] },
    dts: {
      generator: "tsgo",
    },
    exports: true,
    // Published minified: the CDN serves dist/index.mjs as is, and the 15 KB budget counts it.
    minify: true,
  },
});
