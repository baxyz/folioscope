// SPDX-License-Identifier: AGPL-3.0-or-later
// The site's Roadmap and Compatibility pages come from apps/site/data/features.ts: every feature
// said to work must name a test that exists, and every one left to do must say what it needs.
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, test } from "vite-plus/test";
import { FEATURES, type Status } from "../../../apps/site/data/features.ts";

const repo = fileURLToPath(new URL("../../../", import.meta.url));

/** The text of every file under `dir` whose name ends with `ext`, joined. */
function sources(dir: string, ext: string): string {
  return readdirSync(join(repo, dir), { recursive: true, encoding: "utf8" })
    .filter((f) => f.endsWith(ext) && !f.includes("node_modules"))
    .map((f) => readFileSync(join(repo, dir, f), "utf8"))
    .join("\n");
}

const rust = sources("crates", ".rs");
const vitest = sources("packages/folioscope/test", ".ts");
const e2e = sources("packages/folioscope/e2e", ".ts");

/** Whether the test a status names exists. */
function exists(reference: string): boolean {
  const [kind, ...rest] = reference.split(": ");
  const name = rest.join(": ");
  if (kind === "rust") return new RegExp(`\\bfn ${name}\\(`).test(rust);
  if (kind === "vitest") return vitest.includes(`test("${name}"`);
  if (kind === "e2e") return e2e.includes(`✓ ${name}`);
  return false;
}

const statuses: [string, Status][] = FEATURES.flatMap((f) =>
  Object.entries(f.formats).map(([format, s]) => [`${f.id} (${format})`, s] as [string, Status]),
);

describe("features", () => {
  test("every feature that works names a test that exists", () => {
    const missing = statuses
      .filter(([, s]) => s.status === "works" || s.status === "unverified")
      .filter(([, s]) => !exists((s as { test: string }).test))
      .map(([id, s]) => `${id}: ${(s as { test: string }).test}`);
    expect(missing).toEqual([]);
  });

  test("every feature left to do or unverified says what it needs", () => {
    const silent = statuses
      .filter(([, s]) => (s.status === "todo" || s.status === "unverified") && !s.missing.trim())
      .map(([id]) => id);
    expect(silent).toEqual([]);
  });

  test("feature ids are unique", () => {
    const ids = FEATURES.map((f) => f.id);
    expect(new Set(ids).size).toBe(ids.length);
  });
});
