---
title: Bake with the CLI
description: "Install the folioscope command, bake design files, inspect them, and run it in CI."
---

# Bake with the CLI

The `folioscope` command bakes design files into `.folio` folders, for any site and any build.

## Install

It is a Rust program, on [crates.io](https://crates.io/crates/folioscope). With
[Rust](https://rustup.rs):

```sh
cargo install folioscope
folioscope --version
```

Or from the repository: `cargo build --release -p folioscope` gives `target/release/folioscope`.

## Bake

```sh
folioscope bake app.fig public/app.folio
```

```text
public/app.folio: 4 frames, 12 layer files (3 SVG), 640 KB, in 2100 ms
```

| Option                           | What it does                                                              |
| -------------------------------- | ------------------------------------------------------------------------- |
| `--fonts <file or folder>`       | Fonts for text the file stores no glyphs for; repeat it for more          |
| `--family <name>`                | After a `--fonts`: another family name that font answers to               |
| `--match <pattern>`              | After a `--fonts`: names it stands in for (`SF Pro*`, or `/^Helvetica/i`) |
| `--weight <100-900>`, `--italic` | After a `--fonts`: what the file does not say about itself                |
| `--no-avif`                      | Lossless WebP for every screen                                            |
| `--no-svg`                       | An image for every layer, never an SVG                                    |
| `--max-states <n>`               | The most states drawn per interactive component (16)                      |
| `--all-pages`                    | Keep the pages that have no flow                                          |
| `--config <file>`                | Options from a JSON file instead of `folioscope.config.json`              |

```sh
# A .sketch file with its brand font, and a free font standing in for San Francisco
folioscope bake app.sketch public/app.folio \
  --fonts fonts/Brand.otf \
  --fonts fonts/Inter.ttf --match "SF Pro*"
```

### A configuration file

`folioscope.config.json`, in the current folder, takes the options of the
[Vite plugin](./vite), paths relative to it. Flags win over it.

```json
{
  "fonts": ["fonts", { "src": "fonts/Inter.ttf", "match": ["SF Pro*"] }],
  "maxStates": 24,
  "avif": false
}
```

## Look inside a file

```sh
folioscope inspect app.fig
```

```text
page "Mobile" (0:1)
  frame "Home" (1:1) 375x812 at 0,0, starts flow "Onboarding"
    "Next" (1:6) ON_CLICK -> navigate "Details" (1:2) with DISSOLVE 300ms OUT_CUBIC
```

It lists pages, frames, flows and links, for any format: a quick way to see what the player will
play. `folioscope render app.fig 1:1 home.png 2` draws one node as a PNG (and its SVG), and
`folioscope fonts <folder>` writes the manifest the player's [folder mode](./fonts#in-the-browser)
reads.

## In CI

Bake in the job that builds your site, before it copies `public/`:

```yaml
# .github/workflows/site.yml (excerpt)
- uses: dtolnay/rust-toolchain@stable
- run: cargo install folioscope --locked
- run: folioscope bake designs/app.fig public/app.folio --fonts fonts
```

A bake is reproducible: the same file and fonts give the same `.folio`, byte for byte, so you can
also commit the folder and bake only when the design changes.
