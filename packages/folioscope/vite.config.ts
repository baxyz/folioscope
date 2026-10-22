// SPDX-License-Identifier: AGPL-3.0-or-later
import { defineConfig } from "vite-plus";

export default defineConfig({
  pack: {
    entry: ["src/index.ts", "src/vite/index.ts"],
    // The player is loaded straight from a CDN as one module: bundle its only dependency.
    deps: { alwaysBundle: ["thumbhash"] },
    dts: {
      generator: "tsgo",
    },
    exports: true,
  },
});
