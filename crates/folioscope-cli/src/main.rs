// SPDX-License-Identifier: AGPL-3.0-or-later

use std::process::ExitCode;

const USAGE: &str = "usage: folioscope inspect <file.fig>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["inspect", path] => inspect(path),
        ["--version" | "-V"] => {
            println!(
                "folioscope {} (IR v{})",
                env!("CARGO_PKG_VERSION"),
                folioscope_core::IR_VERSION
            );
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn inspect(path: &str) -> ExitCode {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("folioscope: cannot read {path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    match folioscope_core::fig::decode(&bytes) {
        Ok(file) => {
            print!("{}", folioscope_core::fig::prototype::summarize(&file));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("folioscope: {path}: {e}");
            ExitCode::FAILURE
        }
    }
}
