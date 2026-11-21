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
      { text: "Guide", link: "/guide/getting-started", activeMatch: "^/guide/(?!api)" },
      { text: "API", link: "/guide/api" },
      { text: "Demos", link: "/demos" },
      { text: "Playground", link: "/playground" },
      { text: "Compatibility", link: "/compatibility" },
      { text: "Roadmap", link: "/roadmap" },
      { text: "Comparison", link: "/comparison" },
      { text: "About", link: "/about" },
    ],
    sidebar: {
      "/guide/": [
        {
          text: "Guide",
          items: [
            { text: "Getting started", link: "/guide/getting-started" },
            { text: "Your design file", link: "/guide/design-files" },
            { text: "Embed the player", link: "/guide/player" },
            { text: "Bake with Vite", link: "/guide/vite" },
            { text: "Bake with the CLI", link: "/guide/cli" },
            { text: "Raw design files", link: "/guide/raw-files" },
            { text: "Fonts", link: "/guide/fonts" },
          ],
        },
        { text: "Reference", items: [{ text: "API", link: "/guide/api" }] },
      ],
    },
    socialLinks: [{ icon: "github", link: "https://github.com/baxyz/folioscope" }],
    footer: {
      message:
        "AGPL-3.0-or-later. Not affiliated with Figma, Sketch, Adobe or Penpot; their names only describe file compatibility.",
    },
  },
});
