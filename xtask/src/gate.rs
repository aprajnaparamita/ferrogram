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
    "codegen", "names", "parity", "fmt", "clippy", "test", "doc", "coverage", "deny", "layers",
    "wasm", "fuzz",
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
    /// The old-name check (task P7-00): no committed file carries the
    /// pre-rename product name or the old layout term.
    Names,
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
    run_to(args, &mut std::io::stdout())
}

/// [`run`] with the summary written to `out` (the tests capture it:
/// `std::io::stdout()` bypasses the test harness's output capture).
fn run_to(args: &[String], out: &mut dyn std::io::Write) -> Result<()> {
    let skip = parse_args(args)?;
    let coverage_runs = !skip.iter().any(|s| s == "coverage");
    drive(&skip, &mut |step| run_step(step, coverage_runs), out)
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

/// The `test` step when the `coverage` step also runs. `cargo llvm-cov`
/// runs every unit and integration test and fails on a failing test, so
/// running them again here only doubles the gate's longest step. Doc
/// tests are the one kind `llvm-cov` leaves out, so they run here.
const DOC_TESTS_ONLY: CargoStep = CargoStep {
    args: &["test", "--workspace", "--all-features", "--doc"],
    env: None,
    needs: None,
};

/// Maps one step name to the work it runs. Unknown names are an
/// error; every name in [`STEPS`] has a plan. `coverage_runs` says
/// whether the `coverage` step runs in this gate: then `test` runs the
/// doc tests only ([`DOC_TESTS_ONLY`]).
fn plan(step: &str, coverage_runs: bool) -> Result<Plan> {
    match step {
        "test" if coverage_runs => Ok(Plan::Cargo(DOC_TESTS_ONLY)),
        // ADR 0015: `--quick` does not skip this step (it is fast).
        "codegen" => Ok(Plan::Codegen),
        "names" => Ok(Plan::Names),
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
                "larrygram-web",
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
        Plan::Names => names(),
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

// --- the names step (task P7-00) ------------------------------------------

/// The pre-rename product name and the old layout term, split in two
/// so this source file carries neither word: the step scans every
/// committed file, its own source included.
const OLD_PRODUCT: &str = concat!("ferro", "gram");
const OLD_LAYOUT_TERM: &str = concat!("rib", "bon");

/// Files that name one of the two words in order to forbid or rename
/// it: the rename's own task file and the Phase 7 docs that quote the
/// terms (task P7-00's explicit path list).
const NAMES_ALLOWED_PATHS: &[&str] = &[
    "docs/tasks/P7-00.md",
    "docs/tasks/P7-01.md",
    "docs/plans/phase-7.md",
    "docs/phase-7/README.md",
    "KICKOFF.md",
];

/// Verbatim third-party identifiers that carry the old layout term
/// (task P7-00): the two VisUICmds constants whose derived command ids
/// once carried it, one upstream worksheet cell value and one upstream
/// asset file name. A word occurrence inside one of these is not a hit.
const NAMES_ALLOWED_IDENTIFIERS: &[&str] = &[
    "visCmdCloseInkToolsRibbonTab",
    "visCmdMinimizeRibbonToggle",
    "Not in the Ribbon",
    "layouts/ribbon.yaml",
];

/// One finding of [`names_in`]: the committed file, the line number
/// (0 for a file-name hit) and the offending text.
#[derive(Debug, Clone, PartialEq, Eq)]
struct NameHit {
    /// The committed path, as `git ls-files` prints it.
    file: String,
    /// The 1-based line number, or 0 when the hit is the file name.
    line: usize,
    /// The offending line (the path for a file-name hit).
    text: String,
}

impl std::fmt::Display for NameHit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.file, self.line, self.text)
    }
}

/// True when `text` carries one of the two words outside the allowed
/// identifiers. The words match case-insensitively, so every spelling
/// of the old names counts (task P7-00).
fn names_hit(text: &str) -> bool {
    let mut scrubbed = text.to_owned();
    for id in NAMES_ALLOWED_IDENTIFIERS {
        scrubbed = scrubbed.replace(*id, "");
    }
    let lowered = scrubbed.to_ascii_lowercase();
    lowered.contains(OLD_PRODUCT) || lowered.contains(OLD_LAYOUT_TERM)
}

/// Scans the listed committed files under `root` and returns every hit
/// in file, then line, order. `listed` holds paths relative to `root`,
/// as `git ls-files` prints them. A file that cannot be read is an
/// error, never a panic.
fn names_in(root: &Path, listed: &[String]) -> Result<Vec<NameHit>> {
    let mut hits: Vec<NameHit> = Vec::new();
    for rel in listed {
        if NAMES_ALLOWED_PATHS.contains(&rel.as_str()) {
            continue;
        }
        if names_hit(rel) {
            hits.push(NameHit {
                file: rel.clone(),
                line: 0,
                text: rel.clone(),
            });
        }
        let bytes = std::fs::read(root.join(rel)).map_err(|e| format!("cannot read {rel}: {e}"))?;
        let text = String::from_utf8_lossy(&bytes);
        for (index, line) in text.lines().enumerate() {
            if names_hit(line) {
                hits.push(NameHit {
                    file: rel.clone(),
                    line: index + 1,
                    text: line.to_owned(),
                });
            }
        }
    }
    Ok(hits)
}

/// The names step over the workspace: `git ls-files` lists the
/// committed files (untracked files are never read), and every hit
/// fails the step with its file and line.
fn names() -> Result<()> {
    names_run(&util::root())
}

/// [`names`] over an explicit root (the tests use a scratch tree).
fn names_run(root: &Path) -> Result<()> {
    let listed = git_ls_files(root)?;
    let hits = names_in(root, &listed)?;
    if hits.is_empty() {
        return Ok(());
    }
    let mut message = format!("{} old-name hit(s) in committed files:", hits.len());
    for hit in &hits {
        message.push('\n');
        message.push_str(&hit.to_string());
    }
    Err(message)
}

/// The committed file list of the repository at `root` (`git ls-files`).
fn git_ls_files(root: &Path) -> Result<Vec<String>> {
    let mut cmd = Command::new("git");
    cmd.args(["ls-files", "-z"]).current_dir(root);
    let output = cmd
        .output()
        .map_err(|e| format!("cannot run git ls-files: {e}"))?;
    if !output.status.success() {
        return Err(format!("git ls-files exited with {}", output.status));
    }
    Ok(output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .map(|entry| String::from_utf8_lossy(entry).into_owned())
        .collect())
}

/// Plans and executes one step.
fn run_step(step: &str, coverage_runs: bool) -> Outcome {
    if step == "test" && coverage_runs {
        println!("gate: test runs the doc tests only; the coverage step runs every other test");
    }
    match plan(step, coverage_runs) {
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
                "codegen", "names", "parity", "clippy", "test", "doc", "coverage", "deny",
                "layers", "wasm"
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
        assert_summary_line(summary[3], "FAIL", "fmt");
        assert!(summary[3].ends_with("  fmt is broken"), "{:?}", summary[3]);
        assert_summary_line(summary[5], "FAIL", "test");
        // Only the first line of a multi-line reason is shown.
        assert!(summary[5].ends_with("  tests failed"), "{:?}", summary[5]);
        assert!(!output.contains("second line"), "{output}");
    }

    /// A single failure is RED (1 step).
    #[test]
    fn one_failure_is_red() {
        let mut fake = Fake::failing("fuzz", Outcome::Fail("no fuzz".into()));
        let (result, output) = drive_fake(&[], &mut fake);
        assert_eq!(result.unwrap_err(), "gate: RED (1 step(s) failed)");
        let summary = summary(&output);
        assert_summary_line(summary[11], "FAIL", "fuzz");
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
        assert_summary_line(summary[3], "FAIL", "fmt");
        assert_summary_line(summary[6], "SKIP", "doc");
        assert!(summary[6].ends_with("  --skip/--quick"), "{:?}", summary[6]);
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
            run_step("bogus", false),
            Outcome::Fail("unknown step `bogus`".to_owned())
        );
    }

    /// The two fast `cargo xtask` steps are wired to their runners
    /// (check mode: read-only).
    #[test]
    fn the_fast_xtask_steps_pass() {
        assert_eq!(run_step("codegen", false), Outcome::Pass);
        assert_eq!(run_step("parity", false), Outcome::Pass);
    }

    /// The gate entry point with every step skipped runs no step and
    /// is GREEN (nothing but the summary is printed). The summary goes
    /// to a buffer: written to the real stdout, it would leak a fake
    /// `gate: GREEN` into the real gate's test output.
    #[test]
    fn the_real_entry_point_with_all_steps_skipped_is_green() {
        let args = s(&["--skip", &STEPS.join(",")]);
        let mut out = Vec::new();
        assert_eq!(run_to(&args, &mut out), Ok(()));
        let text = String::from_utf8(out).expect("utf-8 summary");
        assert!(text.ends_with("gate: GREEN\n"), "{text}");
    }

    // ---- step plans (P4-17: "step selection") ---------------------

    #[test]
    fn every_step_has_a_pinned_plan() {
        let expected: &[(&str, Plan)] = &[
            ("codegen", Plan::Codegen),
            ("names", Plan::Names),
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
                        "larrygram-web",
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
            assert_eq!(&plan(name, false).unwrap(), want, "plan of {name}");
        }
    }

    /// When coverage runs, `test` runs the doc tests only (coverage runs
    /// the rest); every other step keeps its plan.
    #[test]
    fn the_test_step_runs_doc_tests_only_when_coverage_runs() {
        assert_eq!(
            plan("test", true).unwrap(),
            Plan::Cargo(CargoStep {
                args: &["test", "--workspace", "--all-features", "--doc"],
                env: None,
                needs: None,
            })
        );
        for step in STEPS.iter().filter(|step| **step != "test") {
            assert_eq!(plan(step, true), plan(step, false), "plan of {step}");
        }
    }

    #[test]
    fn planning_an_unknown_step_is_an_error() {
        assert_eq!(plan("bogus", false).unwrap_err(), "unknown step `bogus`");
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
    // --- the names step (task P7-00) --------------------------------

    /// Writes `files` into a scratch tree and returns it.
    fn names_tree(files: &[(&str, &str)]) -> crate::util::test_support::TempDir {
        let tmp = crate::util::test_support::TempDir::new("names");
        for (rel, text) in files {
            let path = tmp.path().join(rel);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(path, text).unwrap();
        }
        tmp
    }

    /// The file list shape of `git ls-files`.
    fn listed(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    /// A hit is reported with its file, line and text; the words match
    /// case-insensitively (an upper-case variable spelling counts).
    #[test]
    fn names_reports_file_line_and_text() {
        let old_product = OLD_PRODUCT.to_ascii_uppercase();
        let text = format!("first line\nset {old_product}_HOME=1\nlast line\n");
        let tree = names_tree(&[("docs/a.md", &text)]);
        let hits = names_in(tree.path(), &listed(&["docs/a.md"])).unwrap();
        assert_eq!(
            hits,
            vec![NameHit {
                file: "docs/a.md".to_owned(),
                line: 2,
                text: format!("set {old_product}_HOME=1"),
            }]
        );
        assert_eq!(
            hits[0].to_string(),
            format!("docs/a.md:2: set {old_product}_HOME=1")
        );
    }

    /// The old layout term counts too, in prose and in identifiers.
    #[test]
    fn names_finds_the_old_layout_term_too() {
        let text = format!(
            "a {} layout\na {}-ui crate\n",
            OLD_LAYOUT_TERM, OLD_LAYOUT_TERM
        );
        let tree = names_tree(&[("x.rs", &text)]);
        let hits = names_in(tree.path(), &listed(&["x.rs"])).unwrap();
        assert_eq!(hits.len(), 2, "{hits:?}");
        assert_eq!(hits[0].line, 1);
        assert_eq!(hits[1].line, 2);
    }

    /// The verbatim third-party identifiers are not hits.
    #[test]
    fn allowed_identifiers_are_not_hits() {
        let mut text = String::new();
        for id in NAMES_ALLOWED_IDENTIFIERS {
            text.push_str(id);
            text.push('\n');
        }
        let tree = names_tree(&[("x.rs", &text)]);
        let hits = names_in(tree.path(), &listed(&["x.rs"])).unwrap();
        assert!(hits.is_empty(), "{hits:?}");
    }

    /// An allowed identifier covers only its own text.
    #[test]
    fn an_allowed_identifier_does_not_cover_the_rest_of_the_line() {
        let text = format!(
            "{} and a {} walk in",
            NAMES_ALLOWED_IDENTIFIERS[0], OLD_LAYOUT_TERM
        );
        let tree = names_tree(&[("x.rs", &text)]);
        let hits = names_in(tree.path(), &listed(&["x.rs"])).unwrap();
        assert_eq!(hits.len(), 1, "{hits:?}");
    }

    /// The rename's own docs are skipped by path, and a file name is a
    /// hit reported with line 0.
    #[test]
    fn allowed_paths_are_skipped_and_file_names_count() {
        let bad = format!("notes-{}.md", OLD_LAYOUT_TERM);
        let tree = names_tree(&[
            ("docs/tasks/P7-00.md", OLD_PRODUCT),
            (bad.as_str(), "clean"),
        ]);
        let hits = names_in(tree.path(), &listed(&["docs/tasks/P7-00.md", bad.as_str()])).unwrap();
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0].line, 0, "a file-name hit has line 0");
        assert_eq!(hits[0].file, bad);
    }

    /// A file that cannot be read is an error, never a panic.
    #[test]
    fn a_missing_file_is_an_error() {
        let tree = names_tree(&[]);
        let error = names_in(tree.path(), &listed(&["gone.md"])).unwrap_err();
        assert!(error.contains("cannot read gone.md"), "{error}");
    }

    /// The real committed tree passes, this source file included.
    #[test]
    fn the_committed_tree_is_clean() {
        names_run(&util::root()).expect("no committed file carries an old name");
    }
}
