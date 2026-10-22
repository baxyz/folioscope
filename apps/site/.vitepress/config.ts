// SPDX-License-Identifier: AGPL-3.0-or-later
import { defineConfig } from "vitepress";

export default defineConfig({
  lang: "en",
  title: "Folioscope",
  description:
    "Flip through your prototypes: a web component that plays interactive .fig, .sketch, .xd and .penpot prototypes.",
  cleanUrls: true,
  lastUpdated: false,
  // The build output is not a source: watching it only spends file watchers.
  vite: { server: { watch: { ignored: ["**/.vitepress/dist/**"] } } },
  vue: {
    template: { compilerOptions: { isCustomElement: (tag) => tag.startsWith("folioscope-") } },
  },
  themeConfig: {
    nav: [
      { text: "Getting started", link: "/guide/getting-started" },
      { text: "Demos", link: "/demos" },
      { text: "About", link: "/about" },
    ],
    socialLinks: [{ icon: "github", link: "https://github.com/baxyz/folioscope" }],
    footer: {
      message:
        "AGPL-3.0-or-later. Not affiliated with Figma, Sketch, Adobe or Penpot; their names only describe file compatibility.",
    },
  },
});
