// SPDX-License-Identifier: AGPL-3.0-or-later
// Builds the WebAssembly bake (crates/folioscope-wasm) into wasm/, for folioscope/vite (ADR 0009).
// Needs Rust with the wasm32-unknown-unknown target and wasm-bindgen-cli at the version pinned in
// crates/folioscope-wasm/Cargo.toml. Run before publishing; the lite player does not need it.
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const repo = fileURLToPath(new URL("../../../", import.meta.url));
const out = fileURLToPath(new URL("../wasm/", import.meta.url));
const target = `${repo}target/wasm-simd`;

execFileSync(
  "cargo",
  [
    "build",
    "--release",
    "-p",
    "folioscope-wasm",
    "--target",
    "wasm32-unknown-unknown",
    "--target-dir",
    target,
  ],
  { stdio: "inherit", env: { ...process.env, RUSTFLAGS: "-C target-feature=+simd128" } },
);
execFileSync(
  "wasm-bindgen",
  [
    "--target",
    "web",
    "--out-dir",
    out,
    `${target}/wasm32-unknown-unknown/release/folioscope_wasm.wasm`,
  ],
  { stdio: "inherit" },
);
