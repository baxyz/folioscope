# folioscope

Reads the interactive prototype stored in a `.fig`, `.sketch`, `.xd` or `.penpot` file, with no
design tool, and bakes it into a **`.folio` folder** that the
[Folioscope](https://folioscope.baxyz.dev) player plays in the browser. It is the command-line
tool, and the library behind it.

```sh
cargo install folioscope
```

## The command-line tool

```sh
folioscope inspect app.fig                  # pages, frames, flows and links of any format
folioscope bake app.fig app.folio           # the images and description the player plays
folioscope bake app.sketch app.folio --fonts ./fonts
folioscope bake app.sketch app.folio --fonts Inter.ttf --match 'SF Pro*' --match Helvetica
folioscope fonts ./fonts                    # writes ./fonts/folioscope-fonts.json
folioscope render app.fig 1:23 out.png 2    # one node of a .fig, as PNG (and SVG)
```

`bake` writes a `.folio` folder: put it next to your page and play it with
`<folioscope-player src="/app.folio">` (npm package `folioscope`, also on JSR as
`@baxyz/folioscope`).

- `.fig` files store the outline of every glyph: no font is needed. `.sketch`, `.xd` and `.penpot`
  text is laid out with the fonts you give with `--fonts` (a file or a folder, repeatable); the
  bake warns about each missing one. Options after a `--fonts` say more about it than the file does:
  `--family <name>` (another name it answers to), `--match <pattern>` (names it stands in for,
  with `*` and `?`, repeatable), `--weight <100-900>` and `--italic`.
- `--no-avif` keeps every layer in lossless WebP; by default, photo-heavy screens become AVIF.
- If your site builds with Vite, its plugin (`folioscope/vite`) bakes for you.

## The library

- **Importers**: `figma`, `sketch`, `penpot` and `xd` decode their archives. `figma` reads Kiwi, the
  binary format of `.fig` files (`kiwi`), and resolves instances with their overrides.
- **`ir`**: the intermediate model, version 1: pages, flows, frames, hotspots with frame-relative
  bounds, triggers, actions and transitions, as `serde` JSON. Every format converts to it
  (`figma::ir::to_ir`, `sketch::ir::to_ir`, `xd::ir::to_ir`, `penpot::ir::to_ir`).
- **`summarize`**: a readable summary of any supported file, as `folioscope inspect` prints it.

```rust,no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use folioscope::{figma, summarize};

let bytes = std::fs::read("app.fig")?;
print!("{}", summarize(&bytes)?); // pages, frames, flows and links
let document = figma::ir::to_ir(&figma::decode(&bytes)?);
println!("{}", serde_json::to_string_pretty(&document)?);
# Ok(())
# }
```

### The `bake` feature

On by default: rendering and the `.folio` writer (`folioscope::bake`), with the command-line tool.
Frames become SVG, drawn straight from the file, then rasterized with
[resvg](https://github.com/linebender/resvg); `.sketch`, `.xd` and `.penpot` text is laid out with
the fonts you give, through parley and skrifa. Nothing reads the system's fonts: a bake gives the same
result everywhere. A `.folio` holds `index.json`, layers named by content hash at 1x and 2x
(lossless WebP, or AVIF for photos), a poster and a
[ThumbHash](https://evanw.github.io/thumbhash/) placeholder per frame.

```rust,no_run
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use std::path::Path;

use folioscope::bake::fonts::FontLibrary;
use folioscope::bake::folio::{bake_file, BakeOptions};

let mut fonts = FontLibrary::default();
fonts.add("Inter.ttf", &std::fs::read("Inter.ttf")?)?;

let bytes = std::fs::read("app.fig")?; // or .sketch, .xd, .penpot
let options = BakeOptions { fonts, ..BakeOptions::default() };
let report = bake_file(&bytes, Path::new("app.folio"), &options)?;
println!("{} frames, {} layers", report.frames, report.layers);
# Ok(())
# }
```

To only read files, turn the feature off: `folioscope = { version = "…", default-features = false }`
leaves about 30 dependencies instead of about 200, and no command-line tool.

### Files you did not make

A design file is read as data: nothing of it is run, no path of your machine is read for it, and
what the bake writes carries no script of its own (the
[bake's specification](https://github.com/baxyz/folioscope/blob/main/docs/spec/bake.md#untrusted-files)
says how). The work is not bounded, though: an archive can unzip to gigabytes, a drawing can be
slow. Give a bake of files from strangers limits on memory and time, as for any converter.

### Versions

The command line and the `.folio` folders it writes follow semantic versioning from 1.0. The
library does not yet: its modules are public so that the WebAssembly bindings and the tests reach
them, and `folioscope::bake` above all still moves with the formats. A minor release may change
it; pin the version (`folioscope = "=1.0.0"`) if you build on it, and read the
[changelog](https://github.com/baxyz/folioscope/blob/main/CHANGELOG.md) before you move.

## License

AGPL-3.0-or-later. The design files you read or bake remain yours: the license covers the code, not
your mockups. Folioscope is not affiliated with Figma, Sketch, Adobe or Penpot; their names only
describe file compatibility.
