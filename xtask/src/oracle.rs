//! `cargo xtask oracle`: build and inspect the libvisio test oracle.
//!
//! The oracle is the set of six libvisio CLI tools built by
//! `tools/libvisio-oracle/build.sh` (ADR 0010). It is dev-only and
//! optional: a missing build never fails the gate, and tests that need
//! it are `#[ignore]`d and gated on `LARRYGRAM_REQUIRE_ORACLE=1`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::util::{self, Result};

/// Default build directory: the external drive, shared by every
/// worktree (AGENTS.md, "Disk usage"). CI overrides it.
pub const DEFAULT_BUILD_DIR: &str = "/Volumes/Dara/larrygram/build/libvisio-oracle";

/// The six libvisio CLI tools the oracle provides. Keep in sync with
/// `BINARIES` in `tools/libvisio-oracle/build.sh`.
pub const BINARIES: &[&str] = &[
    "vsd2raw",
    "vss2raw",
    "vsd2xhtml",
    "vss2xhtml",
    "vsd2text",
    "vss2text",
];

/// Lines of failing stdout kept in a `build.sh` error message; a
/// failing build prints pages of make output.
const ERROR_TAIL_LINES: usize = 40;

/// Usage text for `cargo xtask oracle`.
const USAGE: &str =
    "usage: cargo xtask oracle [status|build|clean] [--build-dir <path>] [--jobs <n>]

  status              report whether the oracle is built (the default)
  build               clone libvisio and build its CLI tools (ADR 0010)
  clean               remove <build-dir>/bin and <build-dir>/src
  --build-dir <path>  override the build directory (default:
                      LARRYGRAM_LIBVISIO_BUILD_DIR or /Volumes/Dara/larrygram/build/libvisio-oracle)
  --jobs <n>          parallel jobs for make (build only)";

/// Outcome of `build.sh status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OracleStatus {
    /// Every tool exists under `<build-dir>/bin/`.
    Built,
    /// At least one tool is missing, or the build dir cannot be read.
    NotBuilt,
}

impl OracleStatus {
    /// `built` or `not built`, exactly as `build.sh status` prints it.
    pub fn as_str(self) -> &'static str {
        match self {
            OracleStatus::Built => "built",
            OracleStatus::NotBuilt => "not built",
        }
    }
}

/// `cargo xtask oracle` entry point: parses args, runs the command and
/// prints the summary lines. Never fails on `status`.
pub fn run(args: &[String]) -> Result<()> {
    let opts = parse(args)?;
    match opts.cmd.as_str() {
        "help" => {
            println!("{USAGE}");
            Ok(())
        }
        "status" => {
            let (status, path) = match &opts.dir {
                Some(dir) => (status_in(dir), dir.clone()),
                None => status(),
            };
            println!("oracle: {}", status.as_str());
            println!("oracle: build dir: {}", path.display());
            match oracle_dir() {
                Some(dir) => println!(
                    "oracle: oracle dir: {} ({}/{} tools)",
                    dir.display(),
                    count_tools(&dir),
                    BINARIES.len()
                ),
                None => println!(
                    "oracle: oracle dir: not set (LARRYGRAM_LIBVISIO_ORACLE; oracle tests skip)"
                ),
            }
            Ok(())
        }
        "build" => {
            let jobs = opts.jobs.as_deref();
            let out = match &opts.dir {
                Some(dir) => build_in(dir, jobs),
                None if jobs.is_none() => build(),
                None => build_in(&build_dir(), jobs),
            };
            emit(&out?);
            Ok(())
        }
        "clean" => {
            let out = match &opts.dir {
                Some(dir) => clean_in(dir),
                None => clean(),
            };
            emit(&out?);
            Ok(())
        }
        other => Err(format!("oracle: unknown command `{other}`\n\n{USAGE}")),
    }
}

/// Outcome of `build.sh status`: whether the tools are installed, and
/// the build dir that was checked, so the caller can echo both. Never
/// fails: when the script cannot run on this host, the tools are
/// checked directly (see [`status_in`]).
pub fn status() -> (OracleStatus, PathBuf) {
    let dir = build_dir();
    let status = status_in(&dir);
    (status, dir)
}

/// Runs `build.sh build` for the default (or
/// `LARRYGRAM_LIBVISIO_BUILD_DIR`) build dir and returns its stdout.
/// The error carries the failing command's output.
pub fn build() -> Result<String> {
    build_in(&build_dir(), None)
}

/// Runs `build.sh clean` for the default (or
/// `LARRYGRAM_LIBVISIO_BUILD_DIR`) build dir and returns its stdout.
pub fn clean() -> Result<String> {
    clean_in(&build_dir())
}

/// The build directory: `LARRYGRAM_LIBVISIO_BUILD_DIR`, else the
/// default on the external drive.
pub fn build_dir() -> PathBuf {
    build_dir_from(std::env::var_os("LARRYGRAM_LIBVISIO_BUILD_DIR"))
}

/// The tools directory tests use: `LARRYGRAM_LIBVISIO_ORACLE`, but only
/// when it names an existing directory. This is the same helper as in
/// `crates/vsd-binary/tests/common/mod.rs` (ADR 0010: no wrapper
/// crate; the helper is copied per consumer).
pub fn oracle_dir() -> Option<PathBuf> {
    oracle_dir_from(std::env::var_os("LARRYGRAM_LIBVISIO_ORACLE"))
}

/// Runs `build.sh status --build-dir <dir>` and parses its output.
///
/// When the script cannot run (no `/bin/sh`, as on Windows, where the
/// oracle is not built at all), the six tools are checked directly so
/// `status` always has an answer.
pub fn status_in(dir: &Path) -> OracleStatus {
    match run_script(dir, &["status"]) {
        Ok(out) => {
            if out.trim() == "built" {
                OracleStatus::Built
            } else {
                OracleStatus::NotBuilt
            }
        }
        Err(_) => status_local(dir),
    }
}

/// Runs `build.sh build [--jobs <n>] --build-dir <dir>`.
pub fn build_in(dir: &Path, jobs: Option<&str>) -> Result<String> {
    let mut args: Vec<&str> = vec!["build"];
    if let Some(jobs) = jobs {
        args.push("--jobs");
        args.push(jobs);
    }
    run_script(dir, &args)
}

/// Runs `build.sh clean --build-dir <dir>`.
pub fn clean_in(dir: &Path) -> Result<String> {
    run_script(dir, &["clean"])
}

/// How many of the six tools exist directly under `dir` (the directory
/// `LARRYGRAM_LIBVISIO_ORACLE` points at, usually `<build-dir>/bin`).
pub fn count_tools(dir: &Path) -> usize {
    BINARIES
        .iter()
        .filter(|tool| dir.join(tool).is_file())
        .count()
}

/// [`build_dir`] with an explicit value for the environment variable,
/// so tests can pass an override without touching process-global state.
fn build_dir_from(var: Option<OsString>) -> PathBuf {
    match var {
        Some(v) if !v.is_empty() => PathBuf::from(v),
        _ => PathBuf::from(DEFAULT_BUILD_DIR),
    }
}

/// [`oracle_dir`] with an explicit value for the environment variable.
fn oracle_dir_from(var: Option<OsString>) -> Option<PathBuf> {
    let dir = PathBuf::from(var?);
    dir.is_dir().then_some(dir)
}

/// The same check as `build.sh status`, for hosts without a POSIX
/// shell: every tool must exist under `<dir>/bin/`.
fn status_local(dir: &Path) -> OracleStatus {
    for tool in BINARIES {
        if !dir.join("bin").join(tool).is_file() {
            return OracleStatus::NotBuilt;
        }
    }
    OracleStatus::Built
}

/// Runs `tools/libvisio-oracle/build.sh <args...> --build-dir <dir>`
/// with `/bin/sh` (so the exec bit is not required) and returns its
/// stdout. The error carries the exit status and the tail of the
/// output.
fn run_script(dir: &Path, args: &[&str]) -> Result<String> {
    let script = util::root()
        .join("tools")
        .join("libvisio-oracle")
        .join("build.sh");
    let shown = format!(
        "sh {} {} --build-dir {}",
        script.display(),
        args.join(" "),
        dir.display()
    );
    let out = Command::new("/bin/sh")
        .arg(&script)
        .args(args)
        .arg("--build-dir")
        .arg(dir)
        .output()
        .map_err(|e| format!("failed to start {shown}: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "{shown} exited with {}:\n{}{}",
            out.status,
            tail(&String::from_utf8_lossy(&out.stdout), ERROR_TAIL_LINES),
            String::from_utf8_lossy(&out.stderr),
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The last `n` lines of `s`, with a marker when anything was cut.
fn tail(s: &str, n: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    if lines.len() <= n {
        return s.to_owned();
    }
    let skipped = lines.len() - n;
    let kept: Vec<&str> = lines.iter().skip(skipped).copied().collect();
    format!("... ({skipped} earlier lines omitted)\n{}", kept.join("\n"))
}

/// Prints `build.sh`'s stdout, guaranteeing a trailing newline.
fn emit(out: &str) {
    if out.is_empty() {
        return;
    }
    print!("{out}");
    if !out.ends_with('\n') {
        println!();
    }
}

/// Parsed `cargo xtask oracle` arguments.
struct Options {
    /// `status`, `build`, `clean` or `help`.
    cmd: String,
    /// `--build-dir` override, when given.
    dir: Option<PathBuf>,
    /// `--jobs` override, when given (build only).
    jobs: Option<String>,
}

/// Validates `cargo xtask oracle` arguments without running anything
/// (C3: the dispatcher validates arguments before running).
pub fn parse_args(args: &[String]) -> Result<()> {
    parse(args).map(|_| ())
}

/// Parses `cargo xtask oracle` arguments. `status` is the default
/// command, mirroring `build.sh`.
fn parse(args: &[String]) -> Result<Options> {
    let mut opts = Options {
        cmd: "status".to_owned(),
        dir: None,
        jobs: None,
    };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "status" | "build" | "clean" => opts.cmd = arg.clone(),
            "-h" | "--help" => opts.cmd = "help".to_owned(),
            "--build-dir" => {
                let value = it.next().ok_or("--build-dir needs a path")?;
                opts.dir = Some(PathBuf::from(value));
            }
            "--jobs" => {
                let value = it.next().ok_or("--jobs needs a number")?;
                opts.jobs = Some(value.clone());
            }
            other => {
                let (flag, value) = other.split_once('=').unwrap_or((other, ""));
                match flag {
                    "--build-dir" => {
                        if value.is_empty() {
                            return Err("--build-dir needs a path".into());
                        }
                        opts.dir = Some(PathBuf::from(value));
                    }
                    "--jobs" => {
                        if value.is_empty() {
                            return Err("--jobs needs a number".into());
                        }
                        opts.jobs = Some(value.to_owned());
                    }
                    _ => return Err(format!("oracle: unknown argument `{other}`")),
                }
            }
        }
    }
    if let Some(jobs) = &opts.jobs {
        check_jobs(jobs)?;
        if opts.cmd != "build" {
            return Err(format!(
                "--jobs applies to `build` only, not `{}`",
                opts.cmd
            ));
        }
    }
    Ok(opts)
}

/// Validates a `--jobs` value: a positive integer, as `make -j` wants.
fn check_jobs(jobs: &str) -> Result<()> {
    match jobs.parse::<u32>() {
        Ok(n) if n > 0 => Ok(()),
        _ => Err(format!("--jobs needs a positive integer, got `{jobs}`")),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::util::test_support::TempDir;

    /// Creates the six files `build.sh status` looks for.
    fn fake_build(dir: &Path) {
        let bin = dir.join("bin");
        fs::create_dir_all(&bin).unwrap();
        for tool in BINARIES {
            fs::write(bin.join(tool), b"fake tool").unwrap();
        }
    }

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn build_dir_env_override() {
        assert_eq!(build_dir_from(None), PathBuf::from(DEFAULT_BUILD_DIR));
        assert_eq!(
            build_dir_from(Some(OsString::new())),
            PathBuf::from(DEFAULT_BUILD_DIR)
        );
        assert_eq!(
            build_dir_from(Some(OsString::from("/tmp/x"))),
            PathBuf::from("/tmp/x")
        );
    }

    #[test]
    fn build_dir_reads_the_env() {
        let env = std::env::var_os("LARRYGRAM_LIBVISIO_BUILD_DIR");
        let dir = build_dir();
        assert_eq!(dir, build_dir_from(env.clone()));
        match env {
            Some(v) if !v.is_empty() => assert_eq!(dir, PathBuf::from(v)),
            _ => assert_eq!(dir, PathBuf::from(DEFAULT_BUILD_DIR)),
        }
    }

    #[test]
    fn oracle_dir_from_needs_an_existing_directory() {
        assert_eq!(oracle_dir_from(None), None);
        assert_eq!(oracle_dir_from(Some(OsString::new())), None);
        assert_eq!(
            oracle_dir_from(Some(OsString::from("/nonexistent-larrygram-oracle-dir"))),
            None
        );
        let tmp = TempDir::new("oracle-dir");
        assert_eq!(
            oracle_dir_from(Some(tmp.path().as_os_str().to_owned())),
            Some(tmp.path().to_owned())
        );
    }

    #[test]
    fn oracle_dir_is_unset_or_points_at_a_directory() {
        if let Some(dir) = oracle_dir() {
            assert!(dir.is_dir());
        }
    }

    #[test]
    fn status_local_checks_every_tool() {
        let tmp = TempDir::new("status-local");
        assert_eq!(status_local(tmp.path()), OracleStatus::NotBuilt);
        fake_build(tmp.path());
        let first = BINARIES.first().unwrap();
        fs::remove_file(tmp.path().join("bin").join(first)).unwrap();
        assert_eq!(status_local(tmp.path()), OracleStatus::NotBuilt);
        fs::write(tmp.path().join("bin").join(first), b"back").unwrap();
        assert_eq!(status_local(tmp.path()), OracleStatus::Built);
    }

    #[test]
    fn status_in_runs_the_script_with_the_build_dir_override() {
        let tmp = TempDir::new("status-in");
        assert_eq!(status_in(tmp.path()), OracleStatus::NotBuilt);
        fake_build(tmp.path());
        assert_eq!(status_in(tmp.path()), OracleStatus::Built);
    }

    /// The script's own `LARRYGRAM_LIBVISIO_BUILD_DIR` override: tests
    /// pass the variable to the child process, never to this one.
    #[cfg(unix)]
    #[test]
    fn the_script_honours_the_build_dir_env_var() {
        let tmp = TempDir::new("env-override");
        let out = Command::new("/bin/sh")
            .arg(
                util::root()
                    .join("tools")
                    .join("libvisio-oracle")
                    .join("build.sh"),
            )
            .arg("status")
            .env("LARRYGRAM_LIBVISIO_BUILD_DIR", tmp.path())
            .output()
            .unwrap();
        assert!(out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "not built");
        fake_build(tmp.path());
        let out = Command::new("/bin/sh")
            .arg(
                util::root()
                    .join("tools")
                    .join("libvisio-oracle")
                    .join("build.sh"),
            )
            .arg("status")
            .env("LARRYGRAM_LIBVISIO_BUILD_DIR", tmp.path())
            .output()
            .unwrap();
        assert!(out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "built");
    }

    #[test]
    fn status_in_treats_an_unreadable_build_dir_as_not_built() {
        let tmp = TempDir::new("status-file");
        let file = tmp.path().join("not-a-dir");
        fs::write(&file, b"x").unwrap();
        assert_eq!(status_in(&file), OracleStatus::NotBuilt);
    }

    #[test]
    fn status_reports_the_resolved_path() {
        let (status, path) = status();
        assert_eq!(path, build_dir());
        assert_eq!(status, status_in(&path));
    }

    #[test]
    fn count_tools_counts_present_files_only() {
        let tmp = TempDir::new("count");
        assert_eq!(count_tools(tmp.path()), 0);
        fs::write(tmp.path().join("vsd2raw"), b"x").unwrap();
        fs::write(tmp.path().join("vss2raw"), b"x").unwrap();
        assert_eq!(count_tools(tmp.path()), 2);
    }

    #[test]
    fn tail_keeps_short_output_and_trims_long_output() {
        assert_eq!(tail("a\nb\n", 40), "a\nb\n");
        let long: String = (0..10).map(|i| format!("line{i}\n")).collect();
        assert_eq!(
            tail(&long, 3),
            "... (7 earlier lines omitted)\nline7\nline8\nline9"
        );
    }

    #[test]
    fn emit_adds_a_trailing_newline() {
        emit("one\n");
        emit("two");
        emit("");
    }

    #[test]
    fn parses_commands_and_options() {
        let opts = parse(&[]).unwrap();
        assert_eq!(opts.cmd, "status");
        assert_eq!(opts.dir, None);
        assert_eq!(opts.jobs, None);

        let opts = parse(&args(&["build", "--jobs", "2", "--build-dir", "/x"])).unwrap();
        assert_eq!(opts.cmd, "build");
        assert_eq!(opts.dir, Some(PathBuf::from("/x")));
        assert_eq!(opts.jobs.as_deref(), Some("2"));

        let opts = parse(&args(&["clean", "--build-dir=/y"])).unwrap();
        assert_eq!(opts.cmd, "clean");
        assert_eq!(opts.dir, Some(PathBuf::from("/y")));

        let opts = parse(&args(&["build", "--jobs=3"])).unwrap();
        assert_eq!(opts.jobs.as_deref(), Some("3"));

        assert_eq!(parse(&args(&["--help"])).unwrap().cmd, "help");
        assert_eq!(parse(&args(&["-h"])).unwrap().cmd, "help");
    }

    #[test]
    fn rejects_bad_arguments() {
        assert!(parse(&args(&["--build-dir"])).is_err());
        assert!(parse(&args(&["--build-dir="])).is_err());
        assert!(parse(&args(&["--jobs"])).is_err());
        assert!(parse(&args(&["--jobs="])).is_err());
        assert!(parse(&args(&["--what"])).is_err());
        // --jobs only applies to build.
        assert!(parse(&args(&["--jobs", "2"])).is_err());
        assert!(parse(&args(&["clean", "--jobs=2"])).is_err());
        assert!(parse(&args(&["build", "--jobs", "0"])).is_err());
        assert!(parse(&args(&["build", "--jobs", "x"])).is_err());
    }

    #[test]
    fn runs_status_through_the_dispatcher() {
        let tmp = TempDir::new("run-status");
        let path = tmp.path().to_str().unwrap();
        run(&args(&["status", "--build-dir", path])).unwrap();
        fake_build(tmp.path());
        run(&args(&["status", "--build-dir", path])).unwrap();
        run(&args(&["--help"])).unwrap();
        assert!(run(&args(&["status", "--bogus"])).is_err());
        assert!(run(&args(&["build", "--jobs", "0"])).is_err());
    }

    /// Against the real (shared) build dir: read-only either way.
    #[test]
    fn runs_status_for_the_default_build_dir() {
        run(&args(&["status"])).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn build_in_is_a_no_op_when_already_built() {
        let tmp = TempDir::new("build-in");
        fake_build(tmp.path());
        let out = build_in(tmp.path(), None).unwrap();
        assert!(out.contains("already built"), "{out}");
        let out = build_in(tmp.path(), Some("2")).unwrap();
        assert!(out.contains("already built"), "{out}");
    }

    #[cfg(unix)]
    #[test]
    fn clean_in_removes_bin_and_src() {
        let tmp = TempDir::new("clean-in");
        fake_build(tmp.path());
        fs::create_dir_all(tmp.path().join("src")).unwrap();
        let out = clean_in(tmp.path()).unwrap();
        assert!(out.contains("removed"), "{out}");
        assert!(!tmp.path().join("bin").exists());
        assert!(!tmp.path().join("src").exists());
    }

    /// The harness targets macOS and Linux; without a POSIX shell the
    /// script cannot run, and the error must say so (ADR 0010: the
    /// oracle is not built on Windows).
    #[cfg(windows)]
    #[test]
    fn build_and_clean_fail_without_a_posix_shell() {
        let tmp = TempDir::new("no-shell");
        assert!(build_in(tmp.path(), None).is_err());
        assert!(clean_in(tmp.path()).is_err());
    }
}
