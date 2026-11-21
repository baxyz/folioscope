// SPDX-License-Identifier: AGPL-3.0-or-later
// Runs before `pnpm run build`: the site ships the full player, which needs the WebAssembly bake
// (packages/folioscope/wasm/, not committed). Where it is missing:
// - on Cloudflare Workers Builds (`WORKERS_CI`), whose image has no Rust, it installs the
//   toolchain of rust-toolchain.toml and the pinned wasm-bindgen, then builds it;
// - elsewhere, it builds it when cargo and wasm-bindgen are there, else warns: the site then
//   builds without it, and its Playground and raw-file demos cannot bake.
import { execFileSync, execSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const repo = fileURLToPath(new URL("../", import.meta.url));
const wasm = join(repo, "packages/folioscope/wasm/folioscope_wasm_bg.wasm");

const has = (command: string) => {
  try {
    execFileSync(command, ["--version"], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
};

const buildWasm = () =>
  execFileSync("node", [join(repo, "packages/folioscope/scripts/build-wasm.ts")], {
    stdio: "inherit",
  });

if (existsSync(wasm)) {
  console.log("ensure-wasm: the WebAssembly bake is built");
} else if (process.env.WORKERS_CI) {
  const cargoBin = join(homedir(), ".cargo/bin");
  process.env.PATH = `${cargoBin}:${process.env.PATH}`;
  if (!has("cargo")) {
    console.log("ensure-wasm: installing Rust");
    // The toolchain itself (version, wasm32 target) comes from rust-toolchain.toml.
    execSync(
      "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none",
      { stdio: "inherit" },
    );
    // No argument: the toolchain rust-toolchain.toml names, with its wasm32 target.
    execFileSync("rustup", ["toolchain", "install"], { stdio: "inherit", cwd: repo });
  }
  if (!has("wasm-bindgen")) {
    const version = /wasm-bindgen = "=([\d.]+)"/.exec(
      readFileSync(join(repo, "crates/folioscope-wasm/Cargo.toml"), "utf8"),
    )![1];
    const name = `wasm-bindgen-${version}-x86_64-unknown-linux-musl`;
    console.log(`ensure-wasm: installing ${name}`);
    const dir = mkdtempSync(join(tmpdir(), "wasm-bindgen-"));
    execSync(
      `curl -sSfL https://github.com/rustwasm/wasm-bindgen/releases/download/${version}/${name}.tar.gz | tar -xz -C ${dir}`,
      { stdio: "inherit" },
    );
    execFileSync("install", ["-m", "755", join(dir, name, "wasm-bindgen"), cargoBin]);
  }
  buildWasm();
} else if (has("cargo") && has("wasm-bindgen")) {
  buildWasm();
} else {
  console.warn(
    "ensure-wasm: no WebAssembly bake, and no cargo or wasm-bindgen to build it: the site's Playground will not bake",
  );
}
