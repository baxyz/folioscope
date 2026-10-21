// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io::Write;
use std::process::ExitCode;

use folioscope_core::figma::{self, tree::Tree};

const USAGE: &str = "usage:
  folioscope inspect <file.fig|.sketch|.xd|.penpot>
  folioscope render <file.fig> <node-id> <out.png> [scale]   (also writes <out>.svg)
  folioscope bake <file.fig> <out.folio>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["inspect", path] => inspect(path),
        ["render", path, node, out] => render(path, node, out, "1"),
        ["render", path, node, out, scale] => render(path, node, out, scale),
        ["bake", path, out] | ["bake", path, "-o", out] => bake(path, out),
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
    let svg_path = std::path::Path::new(out).with_extension("svg");
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

fn bake(path: &str, out: &str) -> Result<(), String> {
    let file = load(path)?;
    let started = std::time::Instant::now();
    let report = folioscope_bake::folio::bake_fig(
        &file,
        std::path::Path::new(out),
        &folioscope_bake::folio::BakeOptions::default(),
    )
    .map_err(|e| e.to_string())?;
    eprintln!(
        "{out}: {} frames, {} layer files, {} KB, in {} ms",
        report.frames,
        report.layers,
        report.bytes / 1024,
        started.elapsed().as_millis()
    );
    Ok(())
}
