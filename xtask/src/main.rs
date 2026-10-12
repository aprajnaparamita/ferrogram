//! Ferrogram developer tasks.
//!
//! Run with `cargo xtask <task>`:
//!
//! - `gate [--quick] [--skip <step>[,<step>...]]`: the full quality gate (see AGENTS.md).
//! - `coverage [--update-ratchet]`: coverage only; optionally record the ratchet.
//! - `layers`: check crate layering.

#![forbid(unsafe_code)]

mod coverage;
mod gate;
mod layers;
mod util;

use std::process::ExitCode;

const USAGE: &str = "usage: cargo xtask <task>

tasks:
  gate [--quick] [--skip <step>,...]   run the quality gate
                                       steps: fmt clippy test doc coverage deny layers wasm fuzz
                                       --quick skips coverage, wasm and fuzz
  coverage [--update-ratchet]          run coverage and check thresholds
  layers                               check crate layering rules";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("gate") => gate::run(args.get(1..).unwrap_or_default()),
        Some("coverage") => {
            let update = args.iter().any(|a| a == "--update-ratchet");
            coverage::run(update)
        }
        Some("layers") => layers::run(),
        Some("-h" | "--help" | "help") | None => {
            println!("{USAGE}");
            Ok(())
        }
        Some(other) => Err(format!("unknown task `{other}`\n\n{USAGE}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("xtask: {msg}");
            ExitCode::FAILURE
        }
    }
}
