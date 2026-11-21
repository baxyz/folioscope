// SPDX-License-Identifier: AGPL-3.0-or-later
import { describe, expect, test } from "vite-plus/test";
import { fontFaces, needed } from "../src/full/fonts.ts";

describe("fonts in the browser", () => {
  test("only the fonts the design uses are fetched", () => {
    const used = ["Inter", "Brand-Bold", "Brand"];
    expect(needed({ url: "a.ttf" }, used)).toBe(true); // its file says its family
    expect(needed({ url: "a.ttf", family: "inter" }, used)).toBe(true);
    expect(needed({ url: "a.ttf", family: "Roboto" }, used)).toBe(false);
    expect(needed({ url: "a.ttf", family: "Roboto", match: ["Br?nd*"] }, used)).toBe(true);
    expect(needed({ url: "a.ttf", family: "Roboto", match: ["/^SF Pro/i"] }, used)).toBe(false);
    expect(needed({ url: "a.ttf", family: "Roboto", match: ["/^brand$/i"] }, used)).toBe(true);
    expect(needed({ data: new Uint8Array(1), family: "Roboto" }, used)).toBe(true);
  });

  test("reads @font-face rules, WOFF2 first", () => {
    const sheet = `
      @font-face { font-family: "Brand"; src: local("Brand"), url(brand.ttf) format("truetype"), url('brand.woff2') format("woff2"); font-weight: 700; font-style: italic; }
      @font-face { font-family: Plain; src: url(/fonts/plain.otf); }
      @font-face { font-family: "No source"; }`;
    expect(fontFaces(sheet, "https://example.com/css/fonts.css")).toEqual([
      { url: "https://example.com/css/brand.woff2", family: "Brand", weight: 700, italic: true },
      {
        url: "https://example.com/fonts/plain.otf",
        family: "Plain",
        weight: undefined,
        italic: undefined,
      },
    ]);
  });
});
