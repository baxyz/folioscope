// SPDX-License-Identifier: AGPL-3.0-or-later
// The site's API reference is written by hand: it must name every attribute the element reads,
// every public method and event, and every export of the package.
import { readFileSync } from "node:fs";
import { describe, expect, test } from "vite-plus/test";

const read = (path: string) => readFileSync(new URL(path, import.meta.url), "utf8");
const page = read("../../../apps/site/guide/api.md");
const element = read("../src/element.ts");
const player = read("../src/player.ts");

const missing = (names: Iterable<string>) => [...new Set(names)].filter((n) => !page.includes(n));

describe("API reference", () => {
  test("names every attribute the player reads", () => {
    const attributes = [
      ...element.matchAll(/this\.getAttribute\("([a-z-]+)"\)/g),
      ...element.matchAll(/^\s+"([a-z-]+)",$/gm),
    ].map((m) => `\`${m[1]}\``);
    expect(missing(attributes)).toEqual([]);
  });

  test("names every public method and property of the element and the font set", () => {
    const members = [...element.matchAll(/^ {2}(?:get |set )?([a-z][A-Za-z]*)\(/gm)]
      .map((m) => m[1]!)
      .filter((name) => !name.endsWith("Callback") && name !== "constructor");
    expect(missing(members.map((m) => `${m}`))).toEqual([]);
  });

  test("names every event", () => {
    const events = [...element.matchAll(/emit\("(folioscope:[a-z-]+)"/g)].map((m) => m[1]!);
    expect(events.length).toBeGreaterThan(3);
    expect(missing(events)).toEqual([]);
  });

  test("names every export", () => {
    const exports = [...player.matchAll(/export \{([^}]+)\}/g)]
      .flatMap((m) => m[1]!.split(","))
      .map((name) => name.replace("type ", "").trim())
      .filter(Boolean);
    expect(missing(exports)).toEqual([]);
  });
});
