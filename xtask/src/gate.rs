//! `cargo xtask gate`: the single quality-gate command (AGENTS.md, "Quality gate").

use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use crate::util::{self, Result};
use crate::{coverage, layers};

/// Gate steps in execution order.
pub const STEPS: &[&str] = &[
    "codegen", "parity", "fmt", "clippy", "test", "doc", "coverage", "deny", "layers", "wasm",
    "fuzz",
];

/// Steps skipped by `--quick`.
const SLOW_STEPS: &[&str] = &["coverage", "wasm", "fuzz"];

/// Outcome of one step.
enum Outcome {
    Pass,
    Skipped(String),
    Fail(String),
}

/// Parses gate arguments and returns the steps to skip.
pub fn parse_args(args: &[String]) -> Result<Vec<String>> {
    let mut skip = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--quick" => skip.extend(SLOW_STEPS.iter().map(|s| (*s).to_owned())),
            "--skip" => {
                let list = it
                    .next()
                    .ok_or("--skip needs a comma-separated list of steps")?;
                for s in list.split(',').filter(|s| !s.is_empty()) {
                    if !STEPS.contains(&s) {
                        return Err(format!("unknown step `{s}`; steps: {}", STEPS.join(" ")));
                    }
                    skip.push(s.to_owned());
                }
            }
            other => return Err(format!("unknown gate argument `{other}`")),
        }
    }
    Ok(skip)
}

/// Runs the gate. Fails if any step fails; skipped steps are reported.
pub fn run(args: &[String]) -> Result<()> {
    let skip = parse_args(args)?;
    let mut results = Vec::new();
    for step in STEPS {
        let started = Instant::now();
        let outcome = if skip.iter().any(|s| s == step) {
            Outcome::Skipped("--skip/--quick".into())
        } else {
            println!("\n==> gate: {step}");
            run_step(step)
        };
        results.push((*step, outcome, started.elapsed()));
    }

    println!("\n==> gate summary");
    let mut failed = 0;
    for (step, outcome, took) in &results {
        let secs = took.as_secs_f32();
        match outcome {
            Outcome::Pass => println!("  PASS  {step:<9} {secs:>7.1}s"),
            Outcome::Skipped(why) => println!("  SKIP  {step:<9} {secs:>7.1}s  {why}"),
            Outcome::Fail(why) => {
                failed += 1;
                println!(
                    "  FAIL  {step:<9} {secs:>7.1}s  {}",
                    why.lines().next().unwrap_or("")
                );
            }
        }
    }
    if failed == 0 {
        println!("gate: GREEN");
        Ok(())
    } else {
        Err(format!("gate: RED ({failed} step(s) failed)"))
    }
}

fn cargo() -> Command {
    Command::new(util::cargo())
}

fn run_step(step: &str) -> Outcome {
    let result = match step {
        // ADR 0015: `--quick` does not skip this step (it is fast).
        "codegen" => crate::codegen::run(&crate::codegen::Options { check: true }),
        "parity" => crate::parity::run(&crate::parity::Options { check: true }),
        "fmt" => util::run(cargo().args(["fmt", "--all", "--", "--check"])),
        "clippy" => util::run(cargo().args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ])),
        "test" => util::run(cargo().args(["test", "--workspace", "--all-features"])),
        "doc" => util::run(
            cargo()
                .args(["doc", "--workspace", "--no-deps"])
                .env("RUSTDOCFLAGS", "-D warnings"),
        ),
        "coverage" => coverage::run(false),
        "deny" => {
            if !util::has_cargo_subcommand("deny") {
                Err("cargo-deny is not installed: `cargo install --locked cargo-deny`".into())
            } else {
                util::run(cargo().args(["deny", "check"]))
            }
        }
        "layers" => layers::run(),
        "wasm" => util::run(cargo().args([
            "build",
            "--package",
            "ferrogram-web",
            "--target",
            "wasm32-unknown-unknown",
        ])),
        "fuzz" => return fuzz(),
        other => Err(format!("unknown step `{other}`")),
    };
    match result {
        Ok(()) => Outcome::Pass,
        Err(e) => {
            eprintln!("{e}");
            Outcome::Fail(e)
        }
    }
}

/// Builds every `crates/*/fuzz` project with `cargo +nightly fuzz build`.
fn fuzz() -> Outcome {
    let dirs = match fuzz_dirs() {
        Ok(d) => d,
        Err(e) => return Outcome::Fail(e),
    };
    if dirs.is_empty() {
        return Outcome::Skipped("no fuzz targets yet".into());
    }
    if !util::has_cargo_subcommand("fuzz") {
        return Outcome::Fail(
            "cargo-fuzz is not installed: `cargo install --locked cargo-fuzz`".into(),
        );
    }
    for crate_dir in dirs {
        // `cargo +nightly` must go through rustup's proxy, not $CARGO.
        let mut cmd = Command::new("cargo");
        cmd.args(["+nightly", "fuzz", "build"])
            .env_remove("RUSTUP_TOOLCHAIN");
        let shown = crate_dir.display().to_string();
        let status = cmd.current_dir(&crate_dir).status();
        match status {
            Ok(s) if s.success() => {}
            Ok(s) => return Outcome::Fail(format!("fuzz build in {shown} exited with {s}")),
            Err(e) => return Outcome::Fail(format!("cannot run fuzz build in {shown}: {e}")),
        }
    }
    Outcome::Pass
}

/// Crate directories that contain a `fuzz/Cargo.toml`.
fn fuzz_dirs() -> Result<Vec<PathBuf>> {
    let crates = util::root().join("crates");
    let entries =
        std::fs::read_dir(&crates).map_err(|e| format!("cannot list {}: {e}", crates.display()))?;
    let mut out: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join("fuzz").join("Cargo.toml").is_file())
        .collect();
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| (*x).to_owned()).collect()
    }

    #[test]
    fn quick_skips_slow_steps() {
        assert_eq!(parse_args(&s(&["--quick"])).unwrap(), s(SLOW_STEPS));
    }

    /// ADR 0015: the `codegen` step is fast and never skipped by `--quick`.
    #[test]
    fn quick_does_not_skip_codegen() {
        assert!(STEPS.contains(&"codegen"));
        assert!(!SLOW_STEPS.contains(&"codegen"));
        assert!(
            !parse_args(&s(&["--quick"]))
                .unwrap()
                .contains(&"codegen".to_owned())
        );
    }

    #[test]
    fn skip_list_is_validated() {
        assert_eq!(
            parse_args(&s(&["--skip", "doc,deny"])).unwrap(),
            s(&["doc", "deny"])
        );
        assert!(parse_args(&s(&["--skip", "bogus"])).is_err());
        assert!(parse_args(&s(&["--skip"])).is_err());
        assert!(parse_args(&s(&["--what"])).is_err());
    }

    #[test]
    fn no_args_skips_nothing() {
        assert!(parse_args(&[]).unwrap().is_empty());
    }
}
