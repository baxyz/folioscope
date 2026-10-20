// SPDX-License-Identifier: AGPL-3.0-or-later

//! Phase 0 pre-render spike, by hand: bakes `fixtures/fig/triggers.fig` into layers and a minimal
//! `index.json` for `spikes/prerender/index.html`. Node ids are hard-coded on purpose; the real
//! bake will find the layers itself.
//!
//! `cargo run -p folioscope-bake --example prerender_spike -- fixtures/fig/triggers.fig spikes/prerender/out`

use std::collections::hash_map::DefaultHasher;
use std::fmt::Write as _;
use std::hash::{Hash, Hasher};
use std::path::Path;

use folioscope_bake::{rasterize, svg};
use folioscope_core::fig::{decode, tree::NodeId, tree::Tree};

const DENSITIES: [u8; 2] = [1, 2];

fn id(s: &str) -> NodeId {
    s.parse().unwrap()
}

/// Writes one layer at every density, named by content hash; returns its base name.
fn layer(out: &Path, svg: &svg::Svg, total: &mut usize) -> String {
    let mut hasher = DefaultHasher::new();
    svg.markup.hash(&mut hasher);
    let name = format!("{:016x}", hasher.finish());
    for density in DENSITIES {
        let png = rasterize(&svg.markup, f32::from(density)).unwrap();
        *total += png.len();
        std::fs::write(out.join(format!("layers/{name}@{density}x.png")), png).unwrap();
    }
    name
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [input, out] = args.as_slice() else {
        eprintln!("usage: prerender_spike <triggers.fig> <out dir>");
        std::process::exit(2);
    };
    let out = Path::new(out);
    std::fs::create_dir_all(out.join("layers")).unwrap();
    let file = decode(&std::fs::read(input).unwrap()).unwrap();
    let tree = Tree::new(&file);
    let mut bytes = 0;

    // Splash: a single flat layer, leaving after a delay.
    let splash = layer(
        out,
        &svg::render_node(&tree, id("1:1")).unwrap(),
        &mut bytes,
    );
    // Home: the background without its button, then the button in its three states, each taken
    // from the frame Figma navigates to while hovering or pressing.
    let home = layer(
        out,
        &svg::render_node_without(&tree, id("1:2"), &[id("1:6")]).unwrap(),
        &mut bytes,
    );
    let states = [("default", "1:6"), ("hover", "1:9"), ("press", "1:11")].map(|(state, node)| {
        (
            state,
            layer(out, &svg::render_node(&tree, id(node)).unwrap(), &mut bytes),
        )
    });

    let mut json = String::new();
    let _ = write!(
        json,
        r#"{{
  "irVersion": {ir},
  "densities": [1, 2],
  "start": "splash",
  "frames": {{
    "splash": {{
      "name": "Splash", "width": 375, "height": 812,
      "layers": [{{ "src": "{splash}", "x": 0, "y": 0, "width": 375, "height": 812 }}],
      "after": {{ "delayMs": 1500, "to": "home", "transition": {{ "type": "dissolve", "durationMs": 500 }} }}
    }},
    "home": {{
      "name": "Home", "width": 375, "height": 812,
      "layers": [{{ "src": "{home}", "x": 0, "y": 0, "width": 375, "height": 812 }}],
      "zones": [{{
        "name": "Button", "x": 88, "y": 640, "width": 200, "height": 48,
        "states": {{ {states} }}
      }}]
    }}
  }}
}}
"#,
        ir = folioscope_core::IR_VERSION,
        states = states
            .iter()
            .map(|(state, src)| format!(r#""{state}": "{src}""#))
            .collect::<Vec<_>>()
            .join(", "),
    );
    std::fs::write(out.join("index.json"), &json).unwrap();
    println!(
        "{}: {} layers, {} bytes of PNG at 1x+2x, {} bytes of index.json",
        out.display(),
        2 + states.len(),
        bytes,
        json.len()
    );
}
