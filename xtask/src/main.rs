//! Ferrogram developer tasks.
//!
//! Run with `cargo xtask <task>`:
//!
//! - `gate [--quick] [--skip <step>[,<step>...]]`: the full quality gate (see AGENTS.md).
//! - `coverage [--update-ratchet]`: coverage only; optionally record the ratchet.
//! - `layers`: check crate layering rules.
//! - `corpus [--verify-only] [--offline <dir>]`: fetch and verify the test corpus.
//! - `extract [--reference <path>] [--out <path>] [--allow-unimplemented]`:
//!   regenerate every Phase 1 output from `reference/`. See ADR 0009.
//! - `oracle [status|build|clean] [--build-dir <path>] [--jobs <n>]`:
//!   build or inspect the libvisio test oracle. See ADR 0010.

#![forbid(unsafe_code)]

mod corpus;
mod coverage;
mod extract;
mod gate;
mod layers;
mod oracle;
mod util;

use std::process::ExitCode;

const USAGE: &str = "usage: cargo xtask <task>

tasks:
  gate [--quick] [--skip <step>,...]   run the quality gate
                                       steps: fmt clippy test doc coverage deny layers wasm fuzz
                                       --quick skips coverage, wasm and fuzz
  coverage [--update-ratchet]          run coverage and check thresholds
  layers                               check crate layering rules
  corpus [--verify-only] [--offline d] fetch and verify the test corpus (ADR 0005)
                                       --offline copies from a local dir instead of the network
  extract [--reference <path>] [--out <path>] [--allow-unimplemented]
                                       regenerate Phase 1 outputs from reference/
  oracle [status|build|clean] [--build-dir <path>] [--jobs <n>]
                                       build/inspect the libvisio test oracle (ADR 0010)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("gate") => gate::run(args.get(1..).unwrap_or_default()),
        Some("coverage") => {
            let update = args.iter().any(|a| a == "--update-ratchet");
            coverage::run(update)
        }
        Some("layers") => layers::run(),
        Some("corpus") => corpus::run(args.get(1..).unwrap_or_default()),
        Some("extract") => extract_dispatch(args.get(1..).unwrap_or_default()),
        Some("oracle") => oracle::run(args.get(1..).unwrap_or_default()),
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

/// `cargo xtask extract` entry point. Parses args, runs the
/// dispatcher, prints one line per extractor, and returns Ok(()) only
/// when every extractor succeeds (or is `Skipped` under
/// `--allow-unimplemented`).
fn extract_dispatch(args: &[String]) -> Result<(), String> {
    let parsed = extract::parse_args(args)?;
    let results = extract::run(&parsed)?;
    let mut had_failure = false;
    for (name, outcome) in &results {
        match outcome {
            extract::Outcome::Ok { outputs } => {
                println!("extract[{name}]: OK ({} file(s))", outputs.len());
            }
            extract::Outcome::Skipped { task } => {
                println!("extract[{name}]: SKIPPED (not implemented, {task})");
            }
            extract::Outcome::Failed(e) => {
                had_failure = true;
                eprintln!("extract[{name}]: FAIL: {e}");
            }
        }
    }
    if had_failure {
        Err("one or more extractors failed".into())
    } else {
        Ok(())
    }
}
