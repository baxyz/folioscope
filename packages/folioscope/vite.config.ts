// SPDX-License-Identifier: AGPL-3.0-or-later
import { defineConfig } from "vite-plus";

export default defineConfig({
  pack: {
    entry: ["src/index.ts", "src/vite/index.ts"],
    dts: {
      generator: "tsgo",
    },
    exports: true,
  },
});
