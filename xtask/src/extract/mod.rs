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
//! | [`fluent`]      | P1-03          | `ui-spec/panel-2016.yaml`, `panel-365.yaml`, `docs/parity/controls.md` |
//! | [`commands`]    | P1-04          | `ui-spec/commands.yaml` (+ fills layout `command` fields) |
//! | [`visuicmds`]   | P1-04          | VisUICmds parser used by [`commands`]     |
//! | [`shapesheet`]  | P1-04          | `ui-spec/shapesheet.toml`                 |
//! | [`shortcuts`]   | P1-04          | `ui-spec/shortcuts.yaml`                  |
//! | [`vsdx_schema`] | P1-05          | `docs/vsdx-inventory.md`                  |
//! | [`markdown`]    | P1-05          | markdown table + fence parser (shared)    |
//! | [`diff`]        | P1-03          | `ui-spec/diff-2016-vs-365.md`             |
//! | [`parity`]      | P1-03          | `docs/parity/controls.md`                 |
//! | [`schema`]      | P1-03          | schema validator shared by all extractors |
//!
//! The dispatcher logs a `commands` row (not `visuicmds`): the merge
//! lives in [`commands`], which calls [`visuicmds::run`] for the
//! enumeration half.

pub mod commands;
pub mod diff;
pub mod fluent;
pub mod markdown;
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
            "reference/ not found: {}. See docs/sources.md and task P1-01 for setup, or pass --reference <path>.",
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
        // P1-04: the `commands` merge takes the old `visuicmds` slot
        // (it calls `visuicmds::run` for the enumeration half). The
        // per-extractor output list logs `commands`, not `visuicmds`.
        Module {
            name: "commands",
            run: commands::run,
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
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    /// Loader check (P1-04, widened by P1-08 goal 2): every committed
    /// `ui-spec/*.yaml` passes `Schema::validate_file` with the xtask
    /// validator.
    #[test]
    fn committed_commands_and_shortcuts_pass_schema_validation() {
        let root = crate::util::root();
        let schema_path = root.join("ui-spec").join("schema.json");
        if !schema_path.is_file() {
            eprintln!("schema.json not present; skipping");
            return;
        }
        let schema = schema::Schema::load(&schema_path).expect("schema loads");
        for name in [
            "panel-2016.yaml",
            "panel-365.yaml",
            "commands.yaml",
            "shortcuts.yaml",
        ] {
            let path = root.join("ui-spec").join(name);
            if !path.is_file() {
                eprintln!("{name} not present; skipping");
                continue;
            }
            schema
                .validate_file(&path)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
        }
    }

    /// P1-08 goal 2: `ui-spec/shapesheet.toml` parses and carries no
    /// duplicate section, row or cell names.
    #[test]
    fn committed_shapesheet_toml_parses_without_duplicate_names() {
        let path = crate::util::root().join("ui-spec").join("shapesheet.toml");
        if !path.is_file() {
            eprintln!("shapesheet.toml not present; skipping");
            return;
        }
        let text = std::fs::read_to_string(&path).expect("shapesheet.toml reads");
        let value: toml::Value = toml::from_str(&text).expect("shapesheet.toml parses as TOML");

        let rows = value
            .get("rows")
            .and_then(toml::Value::as_array)
            .expect("shapesheet.toml has [[rows]]");
        check_unique("row", rows.iter().filter_map(|r| r.get("name")));

        let sections = value
            .get("sections")
            .and_then(toml::Value::as_array)
            .expect("shapesheet.toml has [[sections]]");
        check_unique("section", sections.iter().filter_map(|s| s.get("name")));
        for section in sections {
            let name = section
                .get("name")
                .and_then(toml::Value::as_str)
                .unwrap_or("<unnamed>");
            let empty = Vec::new();
            let cells = section
                .get("cells")
                .and_then(toml::Value::as_array)
                .unwrap_or(&empty);
            check_unique(
                &format!("cell of section {name}"),
                cells.iter().filter_map(|c| c.get("name")),
            );
        }
    }

    /// Asserts that every `name` in `values` is a string and appears at
    /// most once.
    fn check_unique<'a>(what: &str, values: impl Iterator<Item = &'a toml::Value>) {
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for value in values {
            let name = value
                .as_str()
                .unwrap_or_else(|| panic!("{what} name is not a string: {value}"));
            assert!(seen.insert(name), "duplicate {what} name: {name}");
        }
    }

    /// Loader check (P1-04): the committed quartet loads with the
    /// runtime [`ferrogram_ui_spec::Loader`], including the three ADR
    /// 0007 cross-references.
    #[test]
    fn committed_ui_spec_loads_with_the_runtime_loader() {
        use ferrogram_ui_spec::Loader;

        let root = crate::util::root();
        let ui = root.join("ui-spec");
        let files = [
            "panel-2016.yaml",
            "panel-365.yaml",
            "commands.yaml",
            "shortcuts.yaml",
        ];
        if files.iter().any(|f| !ui.join(f).is_file()) {
            eprintln!("committed ui-spec not complete; skipping");
            return;
        }
        let tmp =
            std::env::temp_dir().join(format!("ferrogram-p1-04-loader-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let data = tmp.join("ui-spec");
        std::fs::create_dir_all(&data).unwrap();
        for f in files {
            std::fs::copy(ui.join(f), data.join(f)).unwrap();
        }

        let loader = Loader::new(&tmp).expect("loader compiles");
        let sets = loader.load_all().expect("the committed quartet loads");
        assert!(!sets.visio_365.commands.entries.is_empty());
        assert!(!sets.visio_365.shortcuts.entries.is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }

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
        // m3: the error must point to docs/sources.md / P1-01, not
        // to `cargo xtask corpus` (the corpus fetcher does not
        // populate reference/).
        assert!(err.contains("docs/sources.md"), "{err}");
        assert!(!err.contains("cargo xtask corpus"), "{err}");
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
