// SPDX-License-Identifier: AGPL-3.0-or-later
// What the registries publish: a registry only takes files from the folder of the manifest, so
// the licence is copied there, and the npm package lists the licences of what it carries.
import { readFileSync } from "node:fs";
import { describe, expect, test } from "vite-plus/test";

const read = (path: string) => readFileSync(new URL(path, import.meta.url), "utf8");

describe("published packages", () => {
  test("the npm package and the crate carry the licence of the repository", () => {
    const license = read("../../../LICENSE");
    expect(read("../LICENSE")).toBe(license);
    expect(read("../../../crates/folioscope/LICENSE")).toBe(license);
  });

  test("the npm package ships the licences of what it bundles", () => {
    const manifest = JSON.parse(read("../package.json")) as { files: string[] };
    expect(manifest.files).toContain("THIRD-PARTY-LICENSES.md");
    const jsr = JSON.parse(read("../jsr.json")) as { publish: { include: string[] } };
    expect(jsr.publish.include).toContain("LICENSE");
  });
});
