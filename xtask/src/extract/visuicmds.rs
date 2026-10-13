//! VisUICmds extractor (P1-04).
//!
//! Reads `reference/learn/visuicmds/visuicmds.md` and returns the
//! parsed `VisUICmds` enumeration (one [`VisCmd`] per `visCmd*`
//! constant). Since P1-04 this module is a **parser**, not a
//! dispatcher module: the final `ui-spec/commands.yaml` is written by
//! [`super::commands`], which merges these constants with the Fluent
//! half from the P1-03 layouts and calls [`run`] to get the constants.
//!
//! Input: `reference/learn/visuicmds/visuicmds.md`, a markdown page
//! with one `| Constant | Value |` table (679 rows). 664 of them
//! carry `visCmd*` constants that match the `commands.yaml` schema
//! pattern `^visCmd[A-Za-z0-9]+$`; the other 15 rows are the
//! `visCmdSetDynConnLineJumpStyle\_2pt` family whose names carry
//! escaped underscores and are outside that pattern (they are
//! skipped; see the task's Result section). See
//! ADR 0007 (`commands.yaml`) and ADR 0009 (extractor contract).
//!
//! Clean-room note (AGENTS.md section 4): only the constant names and
//! their integer values are kept. No Microsoft prose is copied.

use std::path::{Path, PathBuf};

use super::markdown::{cell_text, tables};
use super::{ExtractError, ExtractResult};

/// One row of the `VisUICmds` enumeration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisCmd {
    /// The constant name, e.g. `visCmdFileSave`.
    pub constant: String,
    /// The integer value passed to `Application.DoCmd`.
    pub value: i64,
}

/// Parses the VisUICmds enumeration from `in_dir` (the `reference/`
/// root). Returns the constants in source order (alphabetical, as
/// they appear in the table).
///
/// This entry point writes nothing; [`super::commands::run`] consumes
/// the result when it builds `ui-spec/commands.yaml`.
pub fn run(in_dir: &Path) -> ExtractResult<Vec<VisCmd>> {
    let path = input_path(in_dir);
    // R2: only a missing file is `Missing`; every other read error
    // keeps the io error text.
    let text = super::util::read_source(&path)?;
    parse_visuicmds_md(&text, &path)
}

/// Parses the `| Constant | Value |` table of a VisUICmds page.
///
/// Rows whose constant is empty are skipped (spacers). A row with a
/// constant that does not start with `visCmd`, or with a value that is
/// not an integer, is a hard [`ExtractError::Parse`]: the enumeration
/// is a stable format and a deviation means the source changed under
/// us.
fn parse_visuicmds_md(text: &str, path: &Path) -> ExtractResult<Vec<VisCmd>> {
    let mut out: Vec<VisCmd> = Vec::new();
    let mut saw_table = false;
    for table in tables(text) {
        // The VisUICmds page has exactly one two-column table
        // (`Constant`, `Value`); tolerate a Description column if a
        // future page adds one by only reading the first two cells.
        let header: Vec<String> = table.header.iter().map(|c| cell_text(c)).collect();
        let looks_right = header
            .first()
            .map(|h| h.eq_ignore_ascii_case("constant"))
            .unwrap_or(false)
            && header
                .get(1)
                .map(|h| h.eq_ignore_ascii_case("value"))
                .unwrap_or(false);
        if !looks_right {
            continue;
        }
        saw_table = true;
        for row in &table.rows {
            let constant = row.first().map(|c| cell_text(c)).unwrap_or_default();
            if constant.is_empty() {
                continue;
            }
            let value_raw = row.get(1).map(|c| cell_text(c)).unwrap_or_default();
            if !constant.starts_with("visCmd") {
                return Err(ExtractError::Parse {
                    path: path.to_path_buf(),
                    message: format!("row with non-visCmd constant {constant:?}"),
                });
            }
            // The `commands.yaml` schema fixes the `visuicmd` pattern
            // as `^visCmd[A-Za-z0-9]+$`. The 15 rows whose constants
            // carry escaped underscores
            // (`visCmdSetDynConnLineJumpStyle\_2pt`) fall outside it
            // and are skipped rather than written.
            if !constant.chars().all(|c| c.is_ascii_alphanumeric()) {
                continue;
            }
            let value: i64 = value_raw.parse().map_err(|_| ExtractError::Parse {
                path: path.to_path_buf(),
                message: format!("row {constant}: value {value_raw:?} is not an integer"),
            })?;
            out.push(VisCmd { constant, value });
        }
    }
    if !saw_table {
        return Err(ExtractError::Parse {
            path: path.to_path_buf(),
            message: "no `| Constant | Value |` table found".into(),
        });
    }
    if out.is_empty() {
        return Err(ExtractError::Parse {
            path: path.to_path_buf(),
            message: "VisUICmds table has no data rows".into(),
        });
    }
    Ok(out)
}

/// Resolves the `reference/learn/visuicmds/visuicmds.md` path so the
/// tests can assert the documented input location and error messages
/// can name the real source file.
pub fn input_path(in_dir: &Path) -> PathBuf {
    in_dir.join("learn").join("visuicmds").join("visuicmds.md")
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "\
# VisUICmds enumeration (Visio)

Some intro prose.

| Constant | Value |
| --- | --- |
| **visCmdABarAutoHeight** | 1684 |
| **visCmdFileSave** | 1004 |
| **visCmdZoomIn** | 1216 |
";

    #[test]
    fn parses_constant_value_rows() {
        let rows = parse_visuicmds_md(FIXTURE, Path::new("fixture.md")).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].constant, "visCmdABarAutoHeight");
        assert_eq!(rows[0].value, 1684);
        assert_eq!(rows[1].constant, "visCmdFileSave");
        assert_eq!(rows[1].value, 1004);
    }

    #[test]
    fn tolerance_for_bold_markers_and_spacing() {
        // The real page writes `| **visCmdX** | n |`; the enums write
        // `| **visX**|n|Description|`. Both must parse.
        let text = "| Constant | Value |\n|:-----|:-----|\n| **visCmdEditUndo**|1017|desc|\n";
        let rows = parse_visuicmds_md(text, Path::new("f.md")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].constant, "visCmdEditUndo");
        assert_eq!(rows[0].value, 1017);
    }

    #[test]
    fn missing_table_is_an_error() {
        let err = parse_visuicmds_md("# nothing here\n", Path::new("f.md")).unwrap_err();
        assert!(err.to_string().contains("no `| Constant | Value |`"));
    }

    #[test]
    fn empty_table_is_an_error() {
        let text = "| Constant | Value |\n|---|---|\n| | |\n";
        let err = parse_visuicmds_md(text, Path::new("f.md")).unwrap_err();
        assert!(err.to_string().contains("no data rows"), "{err}");
    }

    #[test]
    fn non_integer_value_is_a_parse_error() {
        let text = "| Constant | Value |\n|---|---|\n| visCmdX | oops |\n";
        let err = parse_visuicmds_md(text, Path::new("f.md")).unwrap_err();
        assert!(err.to_string().contains("not an integer"), "{err}");
    }

    #[test]
    fn non_viscmd_constant_is_a_parse_error() {
        let text = "| Constant | Value |\n|---|---|\n| visNotACmd | 1 |\n";
        let err = parse_visuicmds_md(text, Path::new("f.md")).unwrap_err();
        assert!(err.to_string().contains("non-visCmd"), "{err}");
    }

    #[test]
    fn empty_input_is_total() {
        assert!(parse_visuicmds_md("", Path::new("f.md")).is_err());
    }

    #[test]
    fn run_reads_the_documented_path() {
        // With a temp reference root that lacks the file: a clean
        // Missing error naming the documented path.
        let tmp = crate::util::test_support::TempDir::new("visuicmds");
        let err = run(tmp.path()).unwrap_err();
        assert!(matches!(err, ExtractError::Missing { .. }), "{err}");
        assert!(err.to_string().contains("visuicmds.md"), "{err}");
    }

    /// R2: a directory in the way is an Io error naming the file.
    #[test]
    fn run_with_a_directory_in_the_way_is_an_io_error() {
        let tmp = crate::util::test_support::TempDir::new("visuicmds-dir");
        std::fs::create_dir_all(tmp.join("learn/visuicmds/visuicmds.md")).unwrap();
        let err = run(tmp.path()).unwrap_err();
        assert!(matches!(err, ExtractError::Io { .. }), "{err}");
        assert!(err.to_string().contains("visuicmds.md"), "{err}");
    }

    /// R2: a non-UTF-8 source is an Io error keeping the io error text.
    #[test]
    fn run_with_non_utf8_input_is_an_io_error() {
        let tmp = crate::util::test_support::TempDir::new("visuicmds-bin");
        let dir = tmp.join("learn/visuicmds");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("visuicmds.md"), [0xff, 0xfe, 0xfd]).unwrap();
        let err = run(tmp.path()).unwrap_err();
        assert!(matches!(err, ExtractError::Io { .. }), "{err}");
        assert!(
            err.to_string()
                .contains("stream did not contain valid UTF-8"),
            "{err}"
        );
    }

    #[test]
    fn run_parses_a_staged_file() {
        let tmp = crate::util::test_support::TempDir::new("visuicmds2");
        let dir = tmp.join("learn").join("visuicmds");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("visuicmds.md"), FIXTURE).unwrap();
        let rows = run(tmp.path()).unwrap();
        assert_eq!(rows.len(), 3);
    }

    /// Real-data smoke test: the committed reference yields the 664
    /// `^visCmd[A-Za-z0-9]+$` constants (P1-01's count); the table's
    /// 15 `\_`-escaped underscore rows are skipped. Skipped when
    /// `reference/` is not present (CI without P1-01).
    #[test]
    fn real_reference_has_664_constants() {
        let root = crate::util::root();
        let path = root.join("reference").join("learn").join("visuicmds");
        if !path.is_dir() {
            eprintln!("reference/ not present; skipping");
            return;
        }
        let rows = run(&root.join("reference")).expect("reference VisUICmds parse");
        assert_eq!(rows.len(), 664, "VisUICmds row count");
        assert!(rows.iter().all(|r| r.constant.starts_with("visCmd")));
        assert!(
            rows.iter()
                .all(|r| r.constant.chars().all(|c| c.is_ascii_alphanumeric()))
        );
        // Values are unique per constant name; names are unique.
        let mut names: Vec<&str> = rows.iter().map(|r| r.constant.as_str()).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len(), "duplicate constant names");
    }
}
