//! Ferrogram developer tasks.
//!
//! Run with `cargo xtask <task>`:
//!
//! - `gate [--quick] [--skip <step>[,<step>...]]`: the full quality gate (see AGENTS.md).
//! - `coverage [--update-ratchet]`: coverage only; optionally record the ratchet.
//! - `layers`: check crate layering rules.
//! - `corpus [--verify-only] [--offline <dir>]`: fetch and verify the test corpus.
//! - `extract [--reference <path>] [--out <path>] [--allow-unimplemented] [--check]`:
//!   regenerate every Phase 1 output from `reference/`, or verify the
//!   committed files with `--check`. See ADR 0009.
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
  extract [--reference <path>] [--out <path>] [--allow-unimplemented] [--check]
                                       regenerate Phase 1 outputs from reference/
                                       --check regenerates into a scratch dir and
                                       compares byte-for-byte with the committed files
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
/// `--allow-unimplemented`). With `--check` it regenerates into a
/// scratch directory and compares byte-for-byte with the committed
/// files instead (P1-08 goal 3).
fn extract_dispatch(args: &[String]) -> Result<(), String> {
    let parsed = extract::parse_args(args)?;
    if parsed.check {
        let report = extract::check(&parsed)?;
        return if report.is_clean() {
            println!("extract --check: OK (every output matches the committed files)");
            Ok(())
        } else {
            let mut lines: Vec<String> = Vec::new();
            if !report.differing.is_empty() {
                lines.push(format!(
                    "{} output file(s) differ from the committed files \
                     (re-run `cargo xtask extract` and commit):",
                    report.differing.len()
                ));
                lines.extend(report.differing.iter().map(|p| format!("  {}", p.display())));
            }
            if !report.stale.is_empty() {
                lines.push(format!(
                    "{} committed generated file(s) were produced by no extractor \
                     (remove them, or list them in GENERATED_OUTPUTS):",
                    report.stale.len()
                ));
                lines.extend(report.stale.iter().map(|p| format!("  {}", p.display())));
            }
            Err(format!("extract --check failed:\n{}", lines.join("\n")))
        };
    }
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
