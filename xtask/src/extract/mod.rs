//! Phase 1 extractors.
//!
//! Each submodule reads one source under `reference/` and writes one or
//! more files under the workspace (`ui-spec/`, `docs/`,
//! `docs/parity/`). The top-level [`run`] dispatcher is invoked by
//! `cargo xtask extract`. See ADR 0009.
//!
//! Phase 1 task ownership (each module is a stub in P1-02 and is filled
//! in by the named task):
//!
//! | Module          | Implemented in | Output                                    |
//! |-----------------|----------------|-------------------------------------------|
//! | [`fluent`]      | P1-03          | `ui-spec/visio-2016.yaml`, `visio-365.yaml`, `docs/parity/controls.md` |
//! | [`visuicmds`]   | P1-04          | `ui-spec/commands.yaml` (VisUICmds half)  |
//! | [`shapesheet`]  | P1-04          | `ui-spec/shapesheet.toml`                 |
//! | [`shortcuts`]   | P1-04          | `ui-spec/shortcuts.yaml`                  |
//! | [`vsdx_schema`] | P1-05          | `docs/vsdx-inventory.md`                  |
//! | [`diff`]        | P1-03          | `ui-spec/diff-2016-vs-365.md`             |
//! | [`parity`]      | P1-03          | `docs/parity/controls.md`                 |
//! | [`schema`]      | P1-03          | schema validator shared by all extractors |
//!
//! In P1-02 every module returns [`ExtractError::Unimplemented`]; the
//! dispatcher prints a `SKIPPED (P1-NN)` line for each and exits 0 when
//! `--allow-unimplemented` is passed. After P1-03..P1-05 merge the flag
//! is dropped from CI.

pub mod diff;
pub mod fluent;
pub mod parity;
pub mod schema;
pub mod shapesheet;
pub mod shortcuts;
pub mod util;
pub mod visuicmds;
pub mod vsdx_schema;

use std::path::{Path, PathBuf};

use crate::util::Result;

/// Error type shared by every extractor. Each variant names the source
/// file and a human-readable message; no panic, no truncation.
#[allow(dead_code)] // Missing/Parse/Io/Schema are constructed by P1-03..P1-05.
#[derive(Debug)]
pub enum ExtractError {
    /// A required input path is missing. The dispatcher reports this
    /// and stops before invoking any extractor.
    Missing {
        /// The path that was not found.
        path: PathBuf,
    },
    /// The extractor for this output is not implemented yet. The
    /// dispatcher treats this as a soft warning under
    /// `--allow-unimplemented`.
    Unimplemented {
        /// The owning task id (e.g. `P1-03`).
        task: &'static str,
    },
    /// An input file could not be parsed. Includes the file path and a
    /// message; never includes the file content.
    Parse {
        /// The file we tried to parse.
        path: PathBuf,
        /// A short reason; may include a row number or column.
        message: String,
    },
    /// A write to the workspace failed. Includes the destination path.
    Io {
        /// Where we tried to write.
        path: PathBuf,
        /// The underlying `std::io::Error` message.
        source: String,
    },
    /// A generated file failed validation against `ui-spec/schema.json`.
    Schema {
        /// The file that failed.
        path: PathBuf,
        /// The validator's error message.
        message: String,
    },
}

impl std::fmt::Display for ExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing { path } => write!(f, "missing input: {}", path.display()),
            Self::Unimplemented { task } => write!(f, "not implemented ({task})"),
            Self::Parse { path, message } => {
                write!(f, "parse error in {}: {message}", path.display())
            }
            Self::Io { path, source } => {
                write!(f, "io error writing {}: {source}", path.display())
            }
            Self::Schema { path, message } => {
                write!(
                    f,
                    "schema validation failed for {}: {message}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ExtractError {}

/// Convenience alias used across extractors.
pub type ExtractResult<T> = std::result::Result<T, ExtractError>;

/// Arguments for [`run`]. Defaults are the workspace root and
/// `reference/`.
#[derive(Debug, Clone)]
pub struct ExtractArgs {
    /// Directory containing the raw downloads (P1-01's `reference/`
    /// symlink target).
    pub reference: PathBuf,
    /// Directory that owns the output files. Most extractors use the
    /// workspace root and write to `ui-spec/`, `docs/`,
    /// `docs/parity/` inside it.
    pub out: PathBuf,
    /// When true, [`ExtractError::Unimplemented`] is treated as a soft
    /// warning. CI in P1-02 uses this; later phases drop the flag.
    pub allow_unimplemented: bool,
}

impl ExtractArgs {
    /// Defaults: `<workspace>/reference`, `<workspace>`, `false`.
    pub fn default_paths() -> Self {
        let root = crate::util::root();
        Self {
            reference: root.join("reference"),
            out: root,
            allow_unimplemented: false,
        }
    }
}

/// Parses `cargo xtask extract` arguments.
pub fn parse_args(args: &[String]) -> Result<ExtractArgs> {
    let mut out = ExtractArgs::default_paths();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--reference" => {
                let p = it.next().ok_or("--reference needs a path argument")?;
                out.reference = PathBuf::from(p);
            }
            "--out" => {
                let p = it.next().ok_or("--out needs a path argument")?;
                out.out = PathBuf::from(p);
            }
            "--allow-unimplemented" => out.allow_unimplemented = true,
            other => return Err(format!("unknown extract argument `{other}`")),
        }
    }
    Ok(out)
}

/// Outcome of one extractor in a run.
#[derive(Debug)]
pub enum Outcome {
    /// The extractor produced its output and the file passes schema
    /// validation.
    Ok {
        /// Files written by the extractor (sorted, absolute paths).
        outputs: Vec<PathBuf>,
    },
    /// The extractor is a stub; the task id is the owner.
    Skipped {
        /// Owning task id (e.g. `P1-03`).
        task: &'static str,
    },
    /// The extractor ran and failed.
    Failed(ExtractError),
}

/// Runs every extractor in order and returns a per-module outcome.
/// Never panics. A missing `reference/` directory is reported as
/// `Err(ExtractError::Missing { .. })` and no extractor is invoked.
pub fn run(args: &ExtractArgs) -> Result<Vec<(&'static str, Outcome)>> {
    if !args.reference.is_dir() {
        return Err(format!(
            "reference/ not found: {}. Run `cargo xtask corpus` first, or pass --reference <path>.",
            args.reference.display()
        ));
    }
    let in_dir = &args.reference;
    let out_dir = &args.out;

    let modules: &[Module] = &[
        Module {
            name: "fluent",
            run: fluent::run,
        },
        Module {
            name: "visuicmds",
            run: visuicmds::run,
        },
        Module {
            name: "shapesheet",
            run: shapesheet::run,
        },
        Module {
            name: "shortcuts",
            run: shortcuts::run,
        },
        Module {
            name: "vsdx_schema",
            run: vsdx_schema::run,
        },
        Module {
            name: "diff",
            run: diff::run,
        },
        Module {
            name: "parity",
            run: parity::run,
        },
    ];

    let mut results = Vec::new();
    for m in modules {
        let outcome = match (m.run)(in_dir, out_dir) {
            Ok(outputs) => Outcome::Ok { outputs },
            Err(ExtractError::Unimplemented { task }) if args.allow_unimplemented => {
                Outcome::Skipped { task }
            }
            Err(e) => Outcome::Failed(e),
        };
        results.push((m.name, outcome));
    }
    Ok(results)
}

/// One row in the dispatcher's module table. Used to keep the
/// `modules` array readable and to avoid the `clippy::type_complexity`
/// lint.
struct Module {
    /// Stable name printed in the dispatcher's output.
    name: &'static str,
    /// The per-extractor entry point.
    run: fn(&Path, &Path) -> ExtractResult<Vec<PathBuf>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parse_args_defaults() {
        let a = parse_args(&[]).unwrap();
        assert!(a.reference.ends_with("reference"));
        assert!(a.out.ends_with("ferrogram") || a.out.to_string_lossy().contains("ferrogram"));
        assert!(!a.allow_unimplemented);
    }

    #[test]
    fn parse_args_overrides() {
        let a = parse_args(&[
            "--reference".into(),
            "/tmp/ref".into(),
            "--out".into(),
            "/tmp/out".into(),
            "--allow-unimplemented".into(),
        ])
        .unwrap();
        assert_eq!(a.reference, PathBuf::from("/tmp/ref"));
        assert_eq!(a.out, PathBuf::from("/tmp/out"));
        assert!(a.allow_unimplemented);
    }

    #[test]
    fn parse_args_rejects_unknown() {
        assert!(parse_args(&["--bogus".into()]).is_err());
        assert!(parse_args(&["--reference".into()]).is_err());
        assert!(parse_args(&["--out".into()]).is_err());
    }

    #[test]
    fn missing_reference_is_a_clean_error() {
        let args = ExtractArgs {
            reference: PathBuf::from("/nonexistent/never/there"),
            out: crate::util::root(),
            allow_unimplemented: true,
        };
        let err = run(&args).unwrap_err();
        assert!(err.contains("reference/ not found"), "{err}");
    }

    #[test]
    fn error_display_distinct() {
        let e = ExtractError::Unimplemented { task: "P1-03" };
        assert_eq!(e.to_string(), "not implemented (P1-03)");
        let e = ExtractError::Parse {
            path: PathBuf::from("/x/y"),
            message: "bad row 3".into(),
        };
        assert!(e.to_string().contains("bad row 3"));
    }
}
