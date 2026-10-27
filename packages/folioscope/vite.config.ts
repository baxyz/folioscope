// SPDX-License-Identifier: AGPL-3.0-or-later
import { defineConfig } from "vite-plus";

export default defineConfig({
  pack: {
    entry: ["src/index.ts", "src/vite/index.ts"],
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
