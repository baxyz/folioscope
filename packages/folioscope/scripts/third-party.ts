// SPDX-License-Identifier: AGPL-3.0-or-later
// Writes THIRD-PARTY-LICENSES.md: the licences of what the npm package carries that is not ours.
// The WebAssembly bake (wasm/) holds the Rust crates it is built from, and the player (dist/) the
// packages listed in `inlinedDependencies`; their licences ask for their notices to travel with
// them. Run by `build:wasm`, from the crates Cargo resolved for that build.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repo = fileURLToPath(new URL("../../../", import.meta.url));
const pkg = fileURLToPath(new URL("../", import.meta.url));

interface Component {
  name: string;
  version: string;
  license: string;
  authors: string[];
  repository: string;
  /** The folder its licence files are in. */
  dir: string;
}

interface Metadata {
  packages: {
    id: string;
    name: string;
    version: string;
    license: string | null;
    authors: string[];
    repository: string | null;
    manifest_path: string;
    source: string | null;
  }[];
  resolve: {
    nodes: { id: string; deps: { pkg: string; dep_kinds: { kind: string | null }[] }[] }[];
  };
}

/** The crates built into the WebAssembly bake: what `folioscope-wasm` depends on, ours aside. */
function crates(): Component[] {
  const metadata = JSON.parse(
    execFileSync(
      "cargo",
      [
        "metadata",
        "--format-version",
        "1",
        "--locked",
        "--filter-platform",
        "wasm32-unknown-unknown",
      ],
      { cwd: repo, encoding: "utf8", maxBuffer: 256 * 1024 * 1024 },
    ),
  ) as Metadata;
  const packages = new Map(metadata.packages.map((p) => [p.id, p]));
  const nodes = new Map(metadata.resolve.nodes.map((n) => [n.id, n]));
  const root = metadata.packages.find((p) => p.name === "folioscope-wasm")!.id;
  const seen = new Set<string>();
  const stack = [root];
  while (stack.length > 0) {
    const id = stack.pop()!;
    if (seen.has(id)) continue;
    seen.add(id);
    for (const dep of nodes.get(id)?.deps ?? []) {
      // Not the crates only tests use.
      if (dep.dep_kinds.some((k) => k.kind !== "dev")) stack.push(dep.pkg);
    }
  }
  return [...seen]
    .map((id) => packages.get(id)!)
    .filter((p) => p.source !== null)
    .map((p) => ({
      name: p.name,
      version: p.version,
      license: p.license ?? "see its repository",
      authors: p.authors,
      repository: p.repository ?? `https://crates.io/crates/${p.name}`,
      dir: dirname(p.manifest_path),
    }));
}

/** The npm packages bundled into the player: `inlinedDependencies`, one entry per package. */
function bundled(): Component[] {
  const manifest = JSON.parse(readFileSync(join(pkg, "package.json"), "utf8")) as {
    inlinedDependencies: Record<string, string>;
  };
  const require = createRequire(join(pkg, "package.json"));
  // The @helpers4/* packages are the parts of `helpers4`, which carries their licence.
  const names = new Set(
    Object.keys(manifest.inlinedDependencies).map((n) =>
      n.startsWith("@helpers4/") ? "helpers4" : n,
    ),
  );
  return [...names].map((name) => {
    const dir = dirname(require.resolve(`${name}/package.json`));
    const meta = JSON.parse(readFileSync(join(dir, "package.json"), "utf8")) as {
      version: string;
      license?: string;
      author?: string | { name: string };
      repository?: string | { url: string };
      homepage?: string;
    };
    const author = typeof meta.author === "string" ? meta.author : meta.author?.name;
    const repository = typeof meta.repository === "string" ? meta.repository : meta.repository?.url;
    return {
      name,
      version: meta.version,
      license: meta.license ?? "see its repository",
      authors: author ? [author] : [],
      repository:
        repository?.replace(/^git\+/, "").replace(/\.git$/, "") ??
        meta.homepage ??
        `https://www.npmjs.com/package/${name}`,
      dir,
    };
  });
}

const LICENSE_FILE = /^(licen[cs]e|copying|notice|unlicense)/i;

/** The licence files a component ships, by name. */
function texts(component: Component): [string, string][] {
  if (!existsSync(component.dir)) return [];
  return readdirSync(component.dir, { withFileTypes: true })
    .filter((f) => f.isFile() && LICENSE_FILE.test(f.name))
    .map((f): [string, string] => [
      f.name,
      readFileSync(join(component.dir, f.name), "utf8").replaceAll("\r\n", "\n").trim(),
    ])
    .filter(([, text]) => text.length > 0)
    .toSorted(([a], [b]) => a.localeCompare(b));
}

const by = (a: Component, b: Component) =>
  a.name.localeCompare(b.name) || a.version.localeCompare(b.version);

function table(components: Component[]): string {
  const rows = components.toSorted(by).map((c) => {
    const authors = c.authors
      .map((author) => author.replaceAll(/\s*<[^>]*>/g, "").trim())
      .join(", ")
      .replaceAll("|", "\\|");
    return `| [${c.name}](${c.repository}) | ${c.version} | ${c.license} | ${authors} |`;
  });
  return ["| Name | Version | Licence | Authors |", "| --- | --- | --- | --- |", ...rows].join(
    "\n",
  );
}

const rust = crates();
const npm = bundled();

// The same text is written once, with everything that ships it.
const shared = new Map<string, { text: string; users: Set<string> }>();
for (const component of [...npm, ...rust].toSorted(by)) {
  for (const [, text] of texts(component)) {
    const key = createHash("sha256").update(text.replaceAll(/\s+/g, " ")).digest("hex");
    const entry = shared.get(key) ?? { text, users: new Set<string>() };
    entry.users.add(`${component.name} ${component.version}`);
    shared.set(key, entry);
  }
}

const fence = (text: string) => {
  const ticks = "`".repeat(Math.max(3, ...[...text.matchAll(/`+/g)].map((m) => m[0].length + 1)));
  return `${ticks}text\n${text}\n${ticks}`;
};

const out = `# Third-party licences

Folioscope is licensed under the GNU Affero General Public License, version 3 or later (see
\`LICENSE\`). This package also carries the work of others, under their own licences:

- \`dist/\` (the player) bundles the npm packages of the first table;
- \`wasm/folioscope_wasm_bg.wasm\` (the bake, for \`folioscope/vite\` and for design files played
  without a build) is compiled from the Rust crates of the second.

Where a crate offers several licences, Folioscope uses it under one that the AGPL allows. The
licence texts those components ship follow the tables.

## npm packages bundled in the player

${table(npm)}

## Rust crates compiled into the WebAssembly bake

${table(rust)}

## Licence texts

${[...shared.values()]
  .map(({ text, users }) => `### ${[...users].join(", ")}\n\n${fence(text)}`)
  .join("\n\n")}
`;

writeFileSync(join(pkg, "THIRD-PARTY-LICENSES.md"), out);
console.log(
  `third-party: ${npm.length} npm packages, ${rust.length} crates, ${shared.size} licence texts`,
);
