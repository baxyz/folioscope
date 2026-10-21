// SPDX-License-Identifier: AGPL-3.0-or-later
import DefaultTheme from "vitepress/theme";
import type { Theme } from "vitepress";
import "./style.css";

export default {
  extends: DefaultTheme,
  enhanceApp() {
    // The player defines <folioscope-player> in the browser only.
    if (!import.meta.env.SSR) void import("folioscope");
  },
} satisfies Theme;
