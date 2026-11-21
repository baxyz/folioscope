// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use folioscope::bake::fonts::{FontHints, FontLibrary};

use folioscope::figma::{self, tree::Tree};

const USAGE: &str = "usage:
  folioscope inspect <file.fig|.sketch|.xd|.penpot>
  folioscope render <file.fig> <node-id> <out.png> [scale] [--fonts <file or folder>]...   (also writes <out>.svg)
  folioscope bake <file.fig|.sketch|.xd|.penpot> <out.folio> [--config <file>] [--no-avif] [--no-svg] [--max-states <n>]
      [--all-pages] [--fonts <file or folder>
      [--family <name>] [--match <name or pattern>]... [--weight <100-900>] [--italic]]...
  folioscope fonts <folder>   (writes <folder>/folioscope-fonts.json)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["inspect", path] => inspect(path),
        ["render", path, node, out, rest @ ..] => {
            render_args(rest).and_then(|(scale, fonts)| render(path, node, out, scale, &fonts))
        }
        ["bake", rest @ ..] => match bake_args(rest) {
            Ok((path, out, options)) => bake(path, out, &options),
            Err(message) if message == USAGE => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
            Err(message) => Err(message),
        },
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

/// `[scale] [--fonts <path>]...`: the scale (1 by default) and the fonts for text the file
/// stores no glyphs for.
fn render_args<'a>(args: &[&'a str]) -> Result<(&'a str, Vec<&'a str>), String> {
    let (scale, mut rest) = match args {
        [scale, rest @ ..] if !scale.starts_with("--") => (*scale, rest),
        rest => ("1", rest),
    };
    let mut fonts = Vec::new();
    while let ["--fonts", font, more @ ..] = rest {
        fonts.push(*font);
        rest = more;
    }
    if rest.is_empty() {
        Ok((scale, fonts))
    } else {
        Err(USAGE.to_owned())
    }
}

fn render(path: &str, node: &str, out: &str, scale: &str, fonts: &[&str]) -> Result<(), String> {
    let file = load(path)?;
    let tree = Tree::new(&file);
    let id = node.parse()?;
    let scale: f32 = scale.parse().map_err(|e| format!("scale {scale:?}: {e}"))?;
    let mut library = FontLibrary::default();
    for font in fonts {
        add_fonts(
            &mut library,
            Path::new(font),
            Path::new(font),
            &FontHints::default(),
        )?;
    }
    let mut typesetter = folioscope::bake::text::Typesetter::new(library);
    let svg = folioscope::bake::svg::render_node(&tree, id, Some(&mut typesetter))
        .ok_or(format!("no node {node}"))?;
    for missing in &svg.missing_fonts {
        eprintln!("warning: no font for {missing}");
    }
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

/// `<file> [-o] <out> [--config <file>] [--no-avif] [--no-svg] [--max-states <n>] [--fonts <path>
/// [--family <name>] [--match <pattern>]... [--weight <n>] [--italic]]...`, in any order after the
/// file. What follows a `--fonts` says something about that font (or every font of that folder)
/// instead of what the file says. A configuration file gives defaults the flags override.
fn bake_args<'a>(args: &[&'a str]) -> Result<(&'a str, &'a str, BakeArgs), String> {
    let usage = || USAGE.to_owned();
    // The configuration file first: `--config <file>`, else `folioscope.config.json` here.
    let mut config_path = None;
    let mut scan = args.iter();
    while let Some(&arg) = scan.next() {
        if arg == "--config" {
            config_path = Some(Path::new(*scan.next().ok_or_else(usage)?).to_path_buf());
        }
    }
    let config = Config::load(config_path.as_deref())?;
    let mut options = BakeArgs {
        avif: config.avif.unwrap_or(true),
        svg: config.svg.unwrap_or(true),
        max_states: config
            .max_states
            .unwrap_or(folioscope::bake::folio::DEFAULT_MAX_STATES),
        all_pages: config.all_pages.unwrap_or(false),
        fonts: config.fonts,
    };
    let mut positional = Vec::new();
    let mut args = args.iter();
    let next = |args: &mut std::slice::Iter<'_, &'a str>| args.next().copied().ok_or_else(usage);
    while let Some(&arg) = args.next() {
        match arg {
            "--config" => {
                next(&mut args)?;
            }
            "--no-avif" => options.avif = false,
            "--no-svg" => options.svg = false,
            "--max-states" => {
                options.max_states = next(&mut args)?
                    .parse()
                    .ok()
                    .filter(|&n| n > 0)
                    .ok_or_else(usage)?;
            }
            "--all-pages" => options.all_pages = true,
            "--fonts" => options
                .fonts
                .push((next(&mut args)?.to_owned(), FontHints::default())),
            "--family" | "--match" | "--weight" | "--italic" => {
                let (_, hints) = options.fonts.last_mut().ok_or_else(usage)?;
                match arg {
                    "--family" => hints.family = Some(next(&mut args)?.to_owned()),
                    "--match" => hints.matches.push(next(&mut args)?.to_owned()),
                    "--weight" => {
                        hints.weight = Some(next(&mut args)?.parse().map_err(|_| usage())?);
                    }
                    _ => hints.italic = Some(true),
                }
            }
            "-o" => {}
            _ if arg.starts_with('-') => return Err(usage()),
            _ => positional.push(arg),
        }
    }
    match positional.as_slice() {
        [path, out] => Ok((path, out, options)),
        _ => Err(usage()),
    }
}

struct BakeArgs {
    avif: bool,
    svg: bool,
    max_states: usize,
    all_pages: bool,
    /// Each font path with what was said about it: the configuration's, then the command line's.
    fonts: Vec<(String, FontHints)>,
}

/// `folioscope.config.json`: the options of `folioscope bake`, as `folioscope/vite` takes them.
/// Paths are relative to the file.
#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ConfigFile {
    avif: Option<bool>,
    svg: Option<bool>,
    max_states: Option<usize>,
    all_pages: Option<bool>,
    #[serde(default)]
    fonts: Vec<ConfigFont>,
}

/// A font source: a path, or paths with what to say about them.
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum ConfigFont {
    Path(String),
    Hinted {
        src: OneOrMany,
        family: Option<String>,
        #[serde(rename = "match")]
        matches: Option<OneOrMany>,
        weight: Option<f32>,
        italic: Option<bool>,
    },
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

impl OneOrMany {
    fn into_vec(self) -> Vec<String> {
        match self {
            OneOrMany::One(one) => vec![one],
            OneOrMany::Many(many) => many,
        }
    }
}

/// The configuration, read: options and fonts with their paths resolved.
struct Config {
    avif: Option<bool>,
    svg: Option<bool>,
    max_states: Option<usize>,
    all_pages: Option<bool>,
    fonts: Vec<(String, FontHints)>,
}

impl Config {
    /// Reads `path`, or `folioscope.config.json` in the current folder when it exists.
    fn load(path: Option<&Path>) -> Result<Self, String> {
        let default = Path::new(CONFIG_FILE);
        let path = match path {
            Some(path) => path,
            None if default.is_file() => default,
            None => return Ok(Self::from(ConfigFile::default(), Path::new("."))),
        };
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let file: ConfigFile =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Self::from(file, path.parent().unwrap_or(Path::new("."))))
    }

    fn from(file: ConfigFile, base: &Path) -> Self {
        let resolve = |p: &str| base.join(p).to_string_lossy().into_owned();
        let mut fonts = Vec::new();
        for font in file.fonts {
            match font {
                ConfigFont::Path(path) => fonts.push((resolve(&path), FontHints::default())),
                ConfigFont::Hinted {
                    src,
                    family,
                    matches,
                    weight,
                    italic,
                } => {
                    let hints = FontHints {
                        family,
                        matches: matches.map(OneOrMany::into_vec).unwrap_or_default(),
                        weight,
                        italic,
                    };
                    for path in src.into_vec() {
                        fonts.push((resolve(&path), hints.clone()));
                    }
                }
            }
        }
        Self {
            avif: file.avif,
            svg: file.svg,
            max_states: file.max_states,
            all_pages: file.all_pages,
            fonts,
        }
    }
}

/// The configuration file `folioscope bake` reads in the current folder.
const CONFIG_FILE: &str = "folioscope.config.json";

fn bake(path: &str, out: &str, args: &BakeArgs) -> Result<(), String> {
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
            svg: args.svg,
            max_states: args.max_states,
            all_pages: args.all_pages,
            fonts,
            ..Default::default()
        },
    )
    .map_err(|e| e.to_string())?;
    eprintln!(
        "{out}: {} frames, {} layer files ({} SVG), {} KB, in {} ms",
        report.frames,
        report.layers,
        report.svg_layers,
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
    for c in &report.capped_components {
        eprintln!(
            "warning: component {} in frame {} has {} states: only the first {} are baked (--max-states)",
            c.name, c.frame, c.states, c.kept
        );
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
    fn a_configuration_file_gives_defaults_the_flags_override() {
        let dir = std::env::temp_dir().join(format!("folioscope-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("folioscope.config.json");
        std::fs::write(
            &path,
            r#"{ "svg": false, "maxStates": 4, "fonts": ["fonts/", { "src": ["inter.ttf"], "match": "/^SF Pro/i", "weight": 500 }] }"#,
        )
        .unwrap();
        let config = path.to_string_lossy().into_owned();
        let args = [
            "app.fig",
            "--config",
            &config,
            "--max-states",
            "8",
            "out.folio",
        ];
        let (_, _, options) = bake_args(&args).unwrap();
        assert!(!options.svg && options.avif);
        assert_eq!(options.max_states, 8, "the flag wins");
        let [(folder, _), (inter, hints)] = options.fonts.as_slice() else {
            panic!("two fonts expected");
        };
        assert_eq!(Path::new(folder), dir.join("fonts/"));
        assert_eq!(Path::new(inter), dir.join("inter.ttf"));
        assert_eq!(
            (hints.matches.as_slice(), hints.weight),
            (&["/^SF Pro/i".to_owned()][..], Some(500.0))
        );
        // An unknown option is a mistake, not ignored.
        std::fs::write(&path, r#"{ "colour": true }"#).unwrap();
        assert!(bake_args(&args).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

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
        assert_eq!((first.as_str(), second.as_str()), ("a.ttf", "b/"));
        assert_eq!(hints.family.as_deref(), Some("Brand"));
        assert_eq!(hints.matches, ["SF Pro*", "Helvetica"]);
        assert_eq!((hints.weight, hints.italic), (Some(500.0), Some(true)));
        assert_eq!(*other, FontHints::default());
    }

    #[test]
    fn a_hint_needs_a_font_and_a_value() {
        assert!(bake_args(&["app.fig", "out", "--family", "Brand"]).is_err());
        assert!(bake_args(&["app.fig", "out", "--fonts", "a.ttf", "--weight", "bold"]).is_err());
        assert!(bake_args(&["app.fig", "out", "--fonts", "a.ttf", "--match"]).is_err());
    }
}
