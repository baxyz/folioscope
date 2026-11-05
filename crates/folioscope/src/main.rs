// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use folioscope::bake::fonts::{FontHints, FontLibrary};

use folioscope::figma::{self, tree::Tree};

const USAGE: &str = "usage:
  folioscope inspect <file.fig|.sketch|.xd|.penpot>
  folioscope render <file.fig> <node-id> <out.png> [scale]   (also writes <out>.svg)
  folioscope bake <file.fig|.sketch|.xd|.penpot> <out.folio> [--no-avif] [--all-pages] [--fonts <file or folder>
      [--family <name>] [--match <name or pattern>]... [--weight <100-900>] [--italic]]...
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
                folioscope::IR_VERSION
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
    let summary = folioscope::summarize(&bytes).map_err(|e| format!("{path}: {e}"))?;
    // A closed pipe (`| head`) is not an error worth reporting.
    let _ = write!(std::io::stdout(), "{summary}");
    Ok(())
}

fn render(path: &str, node: &str, out: &str, scale: &str) -> Result<(), String> {
    let file = load(path)?;
    let tree = Tree::new(&file);
    let id = node.parse()?;
    let scale: f32 = scale.parse().map_err(|e| format!("scale {scale:?}: {e}"))?;
    let svg = folioscope::bake::svg::render_node(&tree, id).ok_or(format!("no node {node}"))?;
    let png = folioscope::bake::rasterize(&svg.markup, scale).map_err(|e| e.to_string())?;
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

/// `<file> [-o] <out> [--no-avif] [--fonts <path> [--family <name>] [--match <pattern>]...
/// [--weight <n>] [--italic]]...`, in any order after the file. What follows a `--fonts` says
/// something about that font (or every font of that folder) instead of what the file says.
fn bake_args<'a>(args: &[&'a str]) -> Option<(&'a str, &'a str, BakeArgs<'a>)> {
    let mut positional = Vec::new();
    let mut options = BakeArgs {
        avif: true,
        all_pages: false,
        fonts: Vec::new(),
    };
    let mut args = args.iter();
    while let Some(&arg) = args.next() {
        match arg {
            "--no-avif" => options.avif = false,
            "--all-pages" => options.all_pages = true,
            "--fonts" => options.fonts.push((*args.next()?, FontHints::default())),
            "--family" => options.fonts.last_mut()?.1.family = Some((*args.next()?).to_owned()),
            "--match" => options
                .fonts
                .last_mut()?
                .1
                .matches
                .push((*args.next()?).to_owned()),
            "--weight" => options.fonts.last_mut()?.1.weight = Some(args.next()?.parse().ok()?),
            "--italic" => options.fonts.last_mut()?.1.italic = Some(true),
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
    all_pages: bool,
    /// Each font path with what was said about it.
    fonts: Vec<(&'a str, FontHints)>,
}

fn bake(path: &str, out: &str, args: &BakeArgs<'_>) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let mut fonts = FontLibrary::default();
    for (source, hints) in &args.fonts {
        add_fonts(&mut fonts, Path::new(source), Path::new(source), hints)?;
    }
    let started = std::time::Instant::now();
    let report = folioscope::bake::folio::bake_file(
        &bytes,
        Path::new(out),
        &folioscope::bake::folio::BakeOptions {
            avif: args.avif,
            all_pages: args.all_pages,
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
    if !report.skipped_pages.is_empty() {
        eprintln!(
            "note: {} pages have no flow and were left out (--all-pages keeps them): {}",
            report.skipped_pages.len(),
            report.skipped_pages.join(", ")
        );
    }
    for frame in &report.skipped_frames {
        eprintln!("warning: frame {frame} is too large to be an image: left out");
    }
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
    add_fonts(&mut library, root, root, &FontHints::default())?;
    let manifest = root.join("folioscope-fonts.json");
    let json = serde_json::to_string_pretty(library.faces()).map_err(|e| e.to_string())?;
    std::fs::write(&manifest, json + "\n").map_err(|e| format!("{}: {e}", manifest.display()))?;
    eprintln!("{}: {} faces", manifest.display(), library.faces().len());
    Ok(())
}

const FONT_EXTENSIONS: [&str; 5] = ["ttf", "otf", "ttc", "woff", "woff2"];

/// Adds the font at `path`, or every font under it, named by their path from `root`.
fn add_fonts(
    library: &mut FontLibrary,
    root: &Path,
    path: &Path,
    hints: &FontHints,
) -> Result<(), String> {
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
                add_fonts(library, root, &entry, hints)?;
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
    if let Err(e) = library.add_with(&name, &bytes, hints) {
        eprintln!("warning: {e}, skipped");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_follows_a_font_says_something_about_it() {
        let args = [
            "app.fig",
            "--fonts",
            "a.ttf",
            "--family",
            "Brand",
            "--match",
            "SF Pro*",
            "--match",
            "Helvetica",
            "--weight",
            "500",
            "--italic",
            "--fonts",
            "b/",
            "out.folio",
        ];
        let (path, out, options) = bake_args(&args).unwrap();
        assert_eq!((path, out), ("app.fig", "out.folio"));
        let [(first, hints), (second, other)] = options.fonts.as_slice() else {
            panic!("two fonts expected");
        };
        assert_eq!((*first, *second), ("a.ttf", "b/"));
        assert_eq!(hints.family.as_deref(), Some("Brand"));
        assert_eq!(hints.matches, ["SF Pro*", "Helvetica"]);
        assert_eq!((hints.weight, hints.italic), (Some(500.0), Some(true)));
        assert_eq!(*other, FontHints::default());
    }

    #[test]
    fn a_hint_needs_a_font_and_a_value() {
        assert!(bake_args(&["app.fig", "out", "--family", "Brand"]).is_none());
        assert!(bake_args(&["app.fig", "out", "--fonts", "a.ttf", "--weight", "bold"]).is_none());
        assert!(bake_args(&["app.fig", "out", "--fonts", "a.ttf", "--match"]).is_none());
    }
}
