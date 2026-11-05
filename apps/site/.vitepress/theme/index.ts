// SPDX-License-Identifier: AGPL-3.0-or-later
import DefaultTheme from "vitepress/theme";
import type { Theme } from "vitepress";
import { h } from "vue";
import Comparison from "./Comparison.vue";
import EarlyDays from "./EarlyDays.vue";
import FlipBook from "./FlipBook.vue";
import "./style.css";

export default {
  extends: DefaultTheme,
  // The early-days notice sits on top of the home page, above the hero.
  Layout: () => h(DefaultTheme.Layout, null, { "home-hero-before": () => h(EarlyDays) }),
  enhanceApp({ app }) {
    app.component("Comparison", Comparison);
    app.component("FlipBook", FlipBook);
    // The player defines <folioscope-player> in the browser only.
    if (!import.meta.env.SSR) void import("folioscope");
  },
} satisfies Theme;
