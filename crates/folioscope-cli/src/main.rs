// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use folioscope_bake::fonts::FontLibrary;

use folioscope_core::figma::{self, tree::Tree};

const USAGE: &str = "usage:
  folioscope inspect <file.fig|.sketch|.xd|.penpot>
  folioscope render <file.fig> <node-id> <out.png> [scale]   (also writes <out>.svg)
  folioscope bake <file.fig|.sketch|.penpot> <out.folio> [--no-avif] [--fonts <file or folder>]...
  folioscope fonts <folder>   (writes <folder>/folioscope-fonts.json)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["inspect", path] => inspect(path),
        ["render", path, node, out] => render(path, node, out, "1"),
        ["render", path, node, out, scale] => render(path, node, out, scale),
        ["bake", rest @ ..] => {
            let Some((path, out, options)) = bake_args(rest) else {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            };
            bake(path, out, &options)
        }
        ["fonts", folder] => fonts(folder),
        ["--help" | "-h"] => {
            println!("{USAGE}");
            Ok(())
        }
        ["--version" | "-V"] => {
            println!(
                "folioscope {} (IR v{})",
                env!("CARGO_PKG_VERSION"),
                folioscope_core::IR_VERSION
            );
            Ok(())
        }
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("folioscope: {e}");
            ExitCode::FAILURE
        }
    }
}

fn load(path: &str) -> Result<figma::FigFile, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    figma::decode(&bytes).map_err(|e| format!("{path}: {e}"))
}

fn inspect(path: &str) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let summary = folioscope_core::summarize(&bytes).map_err(|e| format!("{path}: {e}"))?;
    // A closed pipe (`| head`) is not an error worth reporting.
    let _ = write!(std::io::stdout(), "{summary}");
    Ok(())
}

fn render(path: &str, node: &str, out: &str, scale: &str) -> Result<(), String> {
    let file = load(path)?;
    let tree = Tree::new(&file);
    let id = node.parse()?;
    let scale: f32 = scale.parse().map_err(|e| format!("scale {scale:?}: {e}"))?;
    let svg = folioscope_bake::svg::render_node(&tree, id).ok_or(format!("no node {node}"))?;
    let png = folioscope_bake::rasterize(&svg.markup, scale).map_err(|e| e.to_string())?;
    let svg_path = Path::new(out).with_extension("svg");
    std::fs::write(&svg_path, &svg.markup).map_err(|e| format!("{}: {e}", svg_path.display()))?;
    std::fs::write(out, png).map_err(|e| format!("{out}: {e}"))?;
    eprintln!(
        "{out}: {}x{} at {scale}x, {} bytes of SVG",
        svg.width,
        svg.height,
        svg.markup.len()
    );
    Ok(())
}

/// `<file> [-o] <out> [--no-avif] [--fonts <path>]...`, in any order after the file.
fn bake_args<'a>(args: &[&'a str]) -> Option<(&'a str, &'a str, BakeArgs<'a>)> {
    let mut positional = Vec::new();
    let mut options = BakeArgs {
        avif: true,
        fonts: Vec::new(),
    };
    let mut args = args.iter();
    while let Some(&arg) = args.next() {
        match arg {
            "--no-avif" => options.avif = false,
            "--fonts" => options.fonts.push(*args.next()?),
            "-o" => {}
            _ if arg.starts_with('-') => return None,
            _ => positional.push(arg),
        }
    }
    match positional.as_slice() {
        [path, out] => Some((path, out, options)),
        _ => None,
    }
}

struct BakeArgs<'a> {
    avif: bool,
    fonts: Vec<&'a str>,
}

fn bake(path: &str, out: &str, args: &BakeArgs<'_>) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let mut fonts = FontLibrary::default();
    for source in &args.fonts {
        add_fonts(&mut fonts, Path::new(source), Path::new(source))?;
    }
    let started = std::time::Instant::now();
    let report = folioscope_bake::folio::bake_file(
        &bytes,
        Path::new(out),
        &folioscope_bake::folio::BakeOptions {
            avif: args.avif,
            fonts,
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;
    eprintln!(
        "{out}: {} frames, {} layer files, {} KB, in {} ms",
        report.frames,
        report.layers,
        report.bytes / 1024,
        started.elapsed().as_millis()
    );
    for font in &report.missing_fonts {
        eprintln!(
            "warning: no font for {font}: its text is left out or keeps another style (--fonts)"
        );
    }
    Ok(())
}

/// Writes `folioscope-fonts.json` in `folder`: each font's family, weights and style, by path
/// relative to the folder, for the player's `base` mode.
fn fonts(folder: &str) -> Result<(), String> {
    let root = Path::new(folder);
    let mut library = FontLibrary::default();
    add_fonts(&mut library, root, root)?;
    let manifest = root.join("folioscope-fonts.json");
    let json = serde_json::to_string_pretty(library.faces()).map_err(|e| e.to_string())?;
    std::fs::write(&manifest, json + "\n").map_err(|e| format!("{}: {e}", manifest.display()))?;
    eprintln!("{}: {} faces", manifest.display(), library.faces().len());
    Ok(())
}

const FONT_EXTENSIONS: [&str; 5] = ["ttf", "otf", "ttc", "woff", "woff2"];

/// Adds the font at `path`, or every font under it, named by their path from `root`.
fn add_fonts(library: &mut FontLibrary, root: &Path, path: &Path) -> Result<(), String> {
    let error = |e: std::io::Error| format!("{}: {e}", path.display());
    if path.is_dir() {
        let mut entries: Vec<_> = std::fs::read_dir(path)
            .map_err(error)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .collect();
        // Sorted, so the manifest and font precedence do not depend on the file system.
        entries.sort();
        for entry in entries {
            let font = entry
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| FONT_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()));
            if entry.is_dir() || font {
                add_fonts(library, root, &entry)?;
            }
        }
        return Ok(());
    }
    let bytes = std::fs::read(path).map_err(error)?;
    let name = path
        .strip_prefix(root)
        .ok()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    if let Err(e) = library.add(&name, &bytes) {
        eprintln!("warning: {e}, skipped");
    }
    Ok(())
}
