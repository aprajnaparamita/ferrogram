//! `cargo xtask gate`: the single quality-gate command (AGENTS.md, "Quality gate").
//!
//! The gate is a thin driver over [`STEPS`]: [`plan`] maps every step
//! name to the work it runs, [`execute`] runs one plan, and [`drive`]
//! walks the steps, collects outcomes and prints the summary. The
//! tests drive the gate with a fake step runner and a buffer (and the
//! fuzz step with a fake build), so no cargo runs under `cargo test`
//! and the summary format stays pinned.

use std::io::Write;
use std::path::{Path, PathBuf};
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
#[derive(Debug, Clone, PartialEq, Eq)]
enum Outcome {
    Pass,
    Skipped(String),
    Fail(String),
}

/// What one step runs. The plan is pure data: [`plan`] maps every
/// step name to its plan, so the tests pin the mapping without
/// running anything.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Plan {
    /// `cargo xtask codegen --check` (ADR 0015).
    Codegen,
    /// `cargo xtask parity --check` (ADR 0037).
    Parity,
    /// `cargo llvm-cov` against `coverage.toml`.
    Coverage,
    /// The layering check (ADR 0002).
    Layers,
    /// `cargo +nightly fuzz build` in every `crates/*/fuzz`.
    Fuzz,
    /// A cargo command in the workspace root.
    Cargo(CargoStep),
}

/// One `cargo <args>` step.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CargoStep {
    /// The cargo arguments (the first is the subcommand).
    args: &'static [&'static str],
    /// One environment variable the step sets (`doc`'s `RUSTDOCFLAGS`).
    env: Option<(&'static str, &'static str)>,
    /// A cargo subcommand the step needs installed (`deny`).
    needs: Option<&'static str>,
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
    drive(&skip, &mut run_step, &mut std::io::stdout())
}

/// Drives the gate over [`STEPS`]: runs every step that is not
/// skipped through `runner`, then prints the summary to `out`. The
/// tests replace `runner` (so no cargo runs) and capture `out` (so
/// the summary format is pinned); [`run`] injects [`run_step`] and
/// stdout. Every step runs even after a failure: the summary lists
/// all of them at once.
fn drive(
    skip: &[String],
    runner: &mut dyn FnMut(&str) -> Outcome,
    out: &mut dyn Write,
) -> Result<()> {
    let mut results = Vec::new();
    for step in STEPS {
        let started = Instant::now();
        let outcome = if skip.iter().any(|s| s == step) {
            Outcome::Skipped("--skip/--quick".into())
        } else {
            writeln!(out, "\n==> gate: {step}").map_err(write_err)?;
            runner(step)
        };
        results.push((*step, outcome, started.elapsed()));
    }

    writeln!(out, "\n==> gate summary").map_err(write_err)?;
    let mut failed = 0;
    for (step, outcome, took) in &results {
        let secs = took.as_secs_f32();
        match outcome {
            Outcome::Pass => {
                writeln!(out, "  PASS  {step:<9} {secs:>7.1}s").map_err(write_err)?;
            }
            Outcome::Skipped(why) => {
                writeln!(out, "  SKIP  {step:<9} {secs:>7.1}s  {why}").map_err(write_err)?;
            }
            Outcome::Fail(why) => {
                failed += 1;
                writeln!(
                    out,
                    "  FAIL  {step:<9} {secs:>7.1}s  {}",
                    why.lines().next().unwrap_or("")
                )
                .map_err(write_err)?;
            }
        }
    }
    if failed == 0 {
        writeln!(out, "gate: GREEN").map_err(write_err)?;
        Ok(())
    } else {
        Err(format!("gate: RED ({failed} step(s) failed)"))
    }
}

/// The one error mapping for [`drive`]'s writes.
fn write_err(e: std::io::Error) -> String {
    format!("cannot write gate output: {e}")
}

fn cargo() -> Command {
    Command::new(util::cargo())
}

/// Maps one step name to the work it runs. Unknown names are an
/// error; every name in [`STEPS`] has a plan.
fn plan(step: &str) -> Result<Plan> {
    match step {
        // ADR 0015: `--quick` does not skip this step (it is fast).
        "codegen" => Ok(Plan::Codegen),
        "parity" => Ok(Plan::Parity),
        "fmt" => Ok(Plan::Cargo(CargoStep {
            args: &["fmt", "--all", "--", "--check"],
            env: None,
            needs: None,
        })),
        "clippy" => Ok(Plan::Cargo(CargoStep {
            args: &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ],
            env: None,
            needs: None,
        })),
        "test" => Ok(Plan::Cargo(CargoStep {
            args: &["test", "--workspace", "--all-features"],
            env: None,
            needs: None,
        })),
        "doc" => Ok(Plan::Cargo(CargoStep {
            args: &["doc", "--workspace", "--no-deps"],
            env: Some(("RUSTDOCFLAGS", "-D warnings")),
            needs: None,
        })),
        "coverage" => Ok(Plan::Coverage),
        "deny" => Ok(Plan::Cargo(CargoStep {
            args: &["deny", "check"],
            env: None,
            needs: Some("deny"),
        })),
        "layers" => Ok(Plan::Layers),
        "wasm" => Ok(Plan::Cargo(CargoStep {
            args: &[
                "build",
                "--package",
                "ferrogram-web",
                "--target",
                "wasm32-unknown-unknown",
            ],
            env: None,
            needs: None,
        })),
        "fuzz" => Ok(Plan::Fuzz),
        other => Err(format!("unknown step `{other}`")),
    }
}

/// Runs one plan. Everything that spawns a process happens here; the
/// tests replace the step runner instead of executing plans. The
/// `fuzz` plan is [`fuzz`]'s own [`Outcome`] (it can skip itself);
/// every other plan becomes `Pass` or `Fail`.
fn execute(plan: Plan) -> Outcome {
    let result: Result<()> = match plan {
        Plan::Codegen => crate::codegen::run(&crate::codegen::Options { check: true }),
        Plan::Parity => crate::parity::run(&crate::parity::Options { check: true }),
        Plan::Coverage => coverage::run(false),
        Plan::Layers => layers::run(),
        Plan::Fuzz => return fuzz(),
        Plan::Cargo(step) => {
            if let Some(needs) = step.needs
                && !util::has_cargo_subcommand(needs)
            {
                return fail(format!(
                    "cargo-{needs} is not installed: `cargo install --locked cargo-{needs}`"
                ));
            }
            let mut cmd = cargo();
            cmd.args(step.args);
            if let Some((key, value)) = step.env {
                cmd.env(key, value);
            }
            util::run(&mut cmd)
        }
    };
    match result {
        Ok(()) => Outcome::Pass,
        Err(e) => fail(e),
    }
}

/// The one error mapping: reports the error on stderr and turns it
/// into a [`Outcome::Fail`] summary row.
fn fail(e: String) -> Outcome {
    eprintln!("{e}");
    Outcome::Fail(e)
}

/// Plans and executes one step.
fn run_step(step: &str) -> Outcome {
    match plan(step) {
        Ok(plan) => execute(plan),
        Err(e) => fail(e),
    }
}

/// The `fuzz` step: `cargo +nightly fuzz build` in every `crates/*/fuzz`.
fn fuzz() -> Outcome {
    fuzz_with(
        &util::root(),
        &mut || util::has_cargo_subcommand("fuzz"),
        &mut fuzz_build,
    )
}

/// [`fuzz`] over an explicit root, a cargo-fuzz probe and one build
/// function (the tests use a scratch tree and fakes, so no build
/// runs). A build failure stops the step; the order is sorted.
fn fuzz_with(
    root: &Path,
    has_fuzz: &mut dyn FnMut() -> bool,
    build: &mut dyn FnMut(&Path) -> Result<()>,
) -> Outcome {
    let dirs = match fuzz_dirs_in(root) {
        Ok(d) => d,
        Err(e) => return Outcome::Fail(e),
    };
    if dirs.is_empty() {
        return Outcome::Skipped("no fuzz targets yet".into());
    }
    if !has_fuzz() {
        return Outcome::Fail(
            "cargo-fuzz is not installed: `cargo install --locked cargo-fuzz`".into(),
        );
    }
    for crate_dir in dirs {
        if let Err(e) = build(&crate_dir) {
            return Outcome::Fail(e);
        }
    }
    Outcome::Pass
}

/// One crate directory's `cargo +nightly fuzz build` (the real build
/// behind [`fuzz_with`]).
fn fuzz_build(crate_dir: &Path) -> Result<()> {
    let shown = crate_dir.display().to_string();
    // `cargo +nightly` must go through rustup's proxy, not $CARGO.
    let mut cmd = Command::new("cargo");
    cmd.args(["+nightly", "fuzz", "build"])
        .env_remove("RUSTUP_TOOLCHAIN");
    let status = cmd
        .current_dir(crate_dir)
        .status()
        .map_err(|e| format!("cannot run fuzz build in {shown}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("fuzz build in {shown} exited with {status}"))
    }
}

/// Crate directories that contain a `fuzz/Cargo.toml`, under `root`.
fn fuzz_dirs_in(root: &Path) -> Result<Vec<PathBuf>> {
    let crates = root.join("crates");
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

    // ---- the driver (P4-17, P4-12 major 5) ------------------------

    /// A fake step runner: records what it is asked to run and
    /// answers from a fixed table (default `Pass`).
    #[derive(Default)]
    struct Fake {
        asked: Vec<String>,
        outcomes: std::collections::BTreeMap<String, Outcome>,
    }

    impl Fake {
        /// A fake where `step` answers `outcome`.
        fn failing(step: &str, outcome: Outcome) -> Self {
            let mut fake = Self::default();
            fake.outcomes.insert(step.to_owned(), outcome);
            fake
        }

        fn run(&mut self, step: &str) -> Outcome {
            self.asked.push(step.to_owned());
            self.outcomes.remove(step).unwrap_or(Outcome::Pass)
        }
    }

    /// Drives the gate with `fake` and returns `(result, output)`.
    fn drive_fake(skip: &[String], fake: &mut Fake) -> (Result<()>, String) {
        let mut out: Vec<u8> = Vec::new();
        let result = drive(skip, &mut |step| fake.run(step), &mut out);
        (result, String::from_utf8(out).unwrap())
    }

    /// The summary step lines (everything between `==> gate summary`
    /// and the final `gate:` verdict).
    fn summary(output: &str) -> Vec<&str> {
        let mut lines: Vec<&str> = Vec::new();
        let mut in_summary = false;
        for line in output.lines() {
            if line == "==> gate summary" {
                in_summary = true;
                continue;
            }
            if in_summary {
                if line.starts_with("gate: ") {
                    break;
                }
                lines.push(line);
            }
        }
        lines
    }

    /// Asserts one summary line has the pinned shape:
    /// `  PASS  codegen     0.0s` (status, a `{:<9}` step name, a
    /// `{:>7.1}` seconds field ending in `s`).
    fn assert_summary_line(line: &str, status: &str, step: &str) {
        assert!(
            line.starts_with(&format!("  {status}  {step:<9} ")),
            "unexpected summary line: {line:?}"
        );
        let rest = &line[2 + status.len() + 2 + 9..];
        let secs = rest.split_whitespace().next().unwrap_or("");
        assert!(secs.ends_with('s'), "no seconds field in: {line:?}");
        assert!(
            secs[..secs.len() - 1].parse::<f32>().is_ok(),
            "seconds field is not a float in: {line:?}"
        );
    }

    /// Step selection: every step runs, in order, and the run line is
    /// printed before each one.
    #[test]
    fn every_step_runs_in_order() {
        let mut fake = Fake::default();
        let (result, output) = drive_fake(&[], &mut fake);
        assert_eq!(result, Ok(()));
        assert_eq!(fake.asked, s(STEPS));
        for step in STEPS {
            assert!(output.contains(&format!("\n==> gate: {step}\n")), "{step}");
        }
        assert!(output.ends_with("gate: GREEN\n"), "{output}");
    }

    /// The `--quick` skip rules: the slow steps never reach the
    /// runner and are reported as skipped, the rest run.
    #[test]
    fn quick_skips_slow_steps_at_run_time() {
        let skip = parse_args(&s(&["--quick"])).unwrap();
        let mut fake = Fake::default();
        let (result, output) = drive_fake(&skip, &mut fake);
        assert_eq!(result, Ok(()));
        assert_eq!(
            fake.asked,
            s(&STEPS
                .iter()
                .copied()
                .filter(|step| !SLOW_STEPS.contains(step))
                .collect::<Vec<_>>())
        );
        let summary = summary(&output);
        assert_eq!(summary.len(), STEPS.len());
        for (index, step) in STEPS.iter().enumerate() {
            if SLOW_STEPS.contains(step) {
                assert_summary_line(summary[index], "SKIP", step);
                assert!(
                    summary[index].ends_with("  --skip/--quick"),
                    "{:?}",
                    summary[index]
                );
            } else {
                assert_summary_line(summary[index], "PASS", step);
            }
        }
    }

    /// A `--skip` list skips exactly the named steps.
    #[test]
    fn skip_list_skips_only_the_named_steps() {
        let skip = parse_args(&s(&["--skip", "fmt,fuzz"])).unwrap();
        let mut fake = Fake::default();
        let (result, _output) = drive_fake(&skip, &mut fake);
        assert_eq!(result, Ok(()));
        assert_eq!(
            fake.asked,
            s(&[
                "codegen", "parity", "clippy", "test", "doc", "coverage", "deny", "layers", "wasm"
            ])
        );
    }

    /// Failure aggregation: every step runs even after failures, the
    /// summary reports each failure once, and the gate is RED with
    /// the count.
    #[test]
    fn failures_are_aggregated_and_do_not_stop_the_run() {
        let mut fake = Fake::default();
        fake.outcomes
            .insert("fmt".to_owned(), Outcome::Fail("fmt is broken".into()));
        fake.outcomes.insert(
            "test".to_owned(),
            Outcome::Fail("tests failed\nsecond line".into()),
        );
        let (result, output) = drive_fake(&[], &mut fake);
        assert_eq!(result.unwrap_err(), "gate: RED (2 step(s) failed)");
        assert_eq!(fake.asked, s(STEPS));
        let summary = summary(&output);
        assert_eq!(summary.len(), STEPS.len());
        assert_summary_line(summary[2], "FAIL", "fmt");
        assert!(summary[2].ends_with("  fmt is broken"), "{:?}", summary[2]);
        assert_summary_line(summary[4], "FAIL", "test");
        // Only the first line of a multi-line reason is shown.
        assert!(summary[4].ends_with("  tests failed"), "{:?}", summary[4]);
        assert!(!output.contains("second line"), "{output}");
    }

    /// A single failure is RED (1 step).
    #[test]
    fn one_failure_is_red() {
        let mut fake = Fake::failing("fuzz", Outcome::Fail("no fuzz".into()));
        let (result, output) = drive_fake(&[], &mut fake);
        assert_eq!(result.unwrap_err(), "gate: RED (1 step(s) failed)");
        let summary = summary(&output);
        assert_summary_line(summary[10], "FAIL", "fuzz");
    }

    /// The full summary line format: two spaces, a four-letter
    /// status, two spaces, a `{:<9}` step name and a `{:>7.1}s`
    /// duration.
    #[test]
    fn the_summary_line_format_is_pinned() {
        let skip = parse_args(&s(&["--skip", "doc"])).unwrap();
        let mut fake = Fake::failing("fmt", Outcome::Fail("bad".into()));
        let (_result, output) = drive_fake(&skip, &mut fake);
        let summary = summary(&output);
        assert_eq!(summary.len(), STEPS.len());
        assert_summary_line(summary[0], "PASS", "codegen");
        assert_summary_line(summary[2], "FAIL", "fmt");
        assert_summary_line(summary[5], "SKIP", "doc");
        assert!(summary[5].ends_with("  --skip/--quick"), "{:?}", summary[5]);
        // The `{:<9}` field is padded, not truncated.
        assert!(
            summary[0].contains(&format!("{:<9}", "codegen")),
            "{:?}",
            summary[0]
        );
    }

    /// An unknown step name fails only its own step (the error text
    /// is unchanged).
    #[test]
    fn an_unknown_step_name_fails_the_step() {
        assert_eq!(
            run_step("bogus"),
            Outcome::Fail("unknown step `bogus`".to_owned())
        );
    }

    /// The two fast `cargo xtask` steps are wired to their runners
    /// (check mode: read-only).
    #[test]
    fn the_fast_xtask_steps_pass() {
        assert_eq!(run_step("codegen"), Outcome::Pass);
        assert_eq!(run_step("parity"), Outcome::Pass);
    }

    /// The gate entry point with every step skipped runs no step and
    /// is GREEN (nothing but the summary is printed).
    #[test]
    fn the_real_entry_point_with_all_steps_skipped_is_green() {
        let args = s(&["--skip", &STEPS.join(",")]);
        assert_eq!(run(&args), Ok(()));
    }

    // ---- step plans (P4-17: "step selection") ---------------------

    #[test]
    fn every_step_has_a_pinned_plan() {
        let expected: &[(&str, Plan)] = &[
            ("codegen", Plan::Codegen),
            ("parity", Plan::Parity),
            (
                "fmt",
                Plan::Cargo(CargoStep {
                    args: &["fmt", "--all", "--", "--check"],
                    env: None,
                    needs: None,
                }),
            ),
            (
                "clippy",
                Plan::Cargo(CargoStep {
                    args: &[
                        "clippy",
                        "--workspace",
                        "--all-targets",
                        "--all-features",
                        "--",
                        "-D",
                        "warnings",
                    ],
                    env: None,
                    needs: None,
                }),
            ),
            (
                "test",
                Plan::Cargo(CargoStep {
                    args: &["test", "--workspace", "--all-features"],
                    env: None,
                    needs: None,
                }),
            ),
            (
                "doc",
                Plan::Cargo(CargoStep {
                    args: &["doc", "--workspace", "--no-deps"],
                    env: Some(("RUSTDOCFLAGS", "-D warnings")),
                    needs: None,
                }),
            ),
            ("coverage", Plan::Coverage),
            (
                "deny",
                Plan::Cargo(CargoStep {
                    args: &["deny", "check"],
                    env: None,
                    needs: Some("deny"),
                }),
            ),
            ("layers", Plan::Layers),
            (
                "wasm",
                Plan::Cargo(CargoStep {
                    args: &[
                        "build",
                        "--package",
                        "ferrogram-web",
                        "--target",
                        "wasm32-unknown-unknown",
                    ],
                    env: None,
                    needs: None,
                }),
            ),
            ("fuzz", Plan::Fuzz),
        ];
        assert_eq!(expected.len(), STEPS.len());
        for (name, want) in expected {
            assert!(STEPS.contains(name), "{name} is not a gate step");
            assert_eq!(&plan(name).unwrap(), want, "plan of {name}");
        }
    }

    #[test]
    fn planning_an_unknown_step_is_an_error() {
        assert_eq!(plan("bogus").unwrap_err(), "unknown step `bogus`");
    }

    // ---- the fuzz step (P4-17) -----------------------------------

    /// A scratch root with one `crates/<name>/fuzz/Cargo.toml` per name.
    fn fuzz_root(label: &str, names: &[&str]) -> crate::util::test_support::TempDir {
        let tmp = crate::util::test_support::TempDir::new(label);
        std::fs::create_dir_all(tmp.join("crates")).unwrap();
        for name in names {
            let fuzz = tmp.join("crates").join(name).join("fuzz");
            std::fs::create_dir_all(&fuzz).unwrap();
            std::fs::write(fuzz.join("Cargo.toml"), "[package]\n").unwrap();
        }
        tmp
    }

    /// A recording build closure: fails where `fail` names the crate.
    fn fake_build<'a>(
        built: &'a mut Vec<String>,
        fail: Option<&'a str>,
    ) -> impl FnMut(&Path) -> Result<()> + 'a {
        move |dir| {
            let name = dir.file_name().unwrap().to_string_lossy().into_owned();
            built.push(name.clone());
            if Some(name.as_str()) == fail {
                Err(format!("fuzz build in {dir:?} exploded"))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn the_fuzz_step_is_skipped_without_targets() {
        let tmp = fuzz_root("fuzz-empty", &[]);
        let mut probed = 0;
        let mut built = Vec::new();
        let outcome = fuzz_with(
            tmp.path(),
            &mut || {
                probed += 1;
                true
            },
            &mut fake_build(&mut built, None),
        );
        assert_eq!(outcome, Outcome::Skipped("no fuzz targets yet".to_owned()));
        assert_eq!(probed, 0, "the probe must not run without targets");
        assert!(built.is_empty());
    }

    #[test]
    fn the_fuzz_step_needs_cargo_fuzz() {
        let tmp = fuzz_root("fuzz-nocargo", &["a"]);
        let mut built = Vec::new();
        let outcome = fuzz_with(tmp.path(), &mut || false, &mut fake_build(&mut built, None));
        assert_eq!(
            outcome,
            Outcome::Fail(
                "cargo-fuzz is not installed: `cargo install --locked cargo-fuzz`".into()
            )
        );
        assert!(built.is_empty());
    }

    #[test]
    fn the_fuzz_step_builds_every_crate_in_sorted_order() {
        let tmp = fuzz_root("fuzz-all", &["b", "a"]);
        let mut built = Vec::new();
        let outcome = fuzz_with(tmp.path(), &mut || true, &mut fake_build(&mut built, None));
        assert_eq!(outcome, Outcome::Pass);
        assert_eq!(built, vec!["a".to_owned(), "b".to_owned()]);
    }

    #[test]
    fn a_failed_fuzz_build_fails_the_step() {
        let tmp = fuzz_root("fuzz-fail", &["a", "b"]);
        let mut built = Vec::new();
        let outcome = fuzz_with(
            tmp.path(),
            &mut || true,
            &mut fake_build(&mut built, Some("a")),
        );
        match outcome {
            Outcome::Fail(message) => assert!(message.contains("exploded"), "{message}"),
            other => panic!("expected a failure, got {other:?}"),
        }
        assert_eq!(built, vec!["a".to_owned()], "the failure stops the step");
    }

    #[test]
    fn a_missing_crates_directory_is_an_error() {
        let tmp = crate::util::test_support::TempDir::new("fuzz-nocrates");
        let error = fuzz_dirs_in(tmp.path()).unwrap_err();
        assert!(error.contains("cannot list"), "{error}");
    }

    /// The real workspace has fuzz crates (render, ui, ui-spec, vsdx)
    /// and they are found sorted.
    #[test]
    fn the_workspace_fuzz_crates_are_listed() {
        let dirs = fuzz_dirs_in(&util::root()).unwrap();
        assert!(!dirs.is_empty());
        let mut sorted = dirs.clone();
        sorted.sort();
        assert_eq!(dirs, sorted);
        assert!(
            dirs.iter()
                .any(|d| d.ends_with(Path::new("crates").join("ui")))
        );
    }
}
