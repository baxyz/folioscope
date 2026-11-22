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
      {
        text: "Guide",
        link: "/guide/getting-started",
        activeMatch: "^/guide/(getting-started|folio|design-files)",
      },
      { text: "Player", link: "/guide/player", activeMatch: "^/guide/(player|raw-files|fonts)" },
      { text: "Bake", link: "/guide/vite", activeMatch: "^/guide/(vite|cli)" },
      { text: "API", link: "/guide/api" },
      { text: "Demos", link: "/demos" },
      { text: "Playground", link: "/playground" },
      {
        text: "Project",
        items: [
          { text: "Compatibility", link: "/compatibility" },
          { text: "Roadmap", link: "/roadmap" },
          { text: "Comparison", link: "/comparison" },
          { text: "About", link: "/about" },
        ],
      },
    ],
    // One sidebar for the docs, in four parts: the guides, using the player, baking, the reference.
    sidebar: {
      "/guide/": [
        {
          text: "Guide",
          items: [
            { text: "Getting started", link: "/guide/getting-started" },
            { text: "The .folio format", link: "/guide/folio" },
            { text: "Your design file", link: "/guide/design-files" },
          ],
        },
        {
          text: "Player",
          items: [
            { text: "Embed the player", link: "/guide/player" },
            { text: "Play a design file", link: "/guide/raw-files" },
            { text: "Fonts", link: "/guide/fonts" },
          ],
        },
        {
          text: "Bake",
          items: [
            { text: "With Vite", link: "/guide/vite" },
            { text: "With the CLI", link: "/guide/cli" },
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
