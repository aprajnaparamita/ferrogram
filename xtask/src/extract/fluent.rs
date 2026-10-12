//! Fluent UI ribbon extractor (P1-03).
//!
//! Reads `reference/fluent-ui-ids/Office 2016/visiocontrols.xlsx` and
//! `reference/fluent-ui-ids/Microsoft 365/Current Channel/visiocontrols.xlsx`
//! and produces:
//!
//! - `ui-spec/visio-2016.yaml`
//! - `ui-spec/visio-365.yaml`
//!
//! See ADR 0007 for the layout file format and ADR 0009 for the
//! extractor contract.
//!
//! Status: stub. Returns [`ExtractError::Unimplemented`] with task id
//! `P1-03`. P1-03 implements the body.

use std::path::{Path, PathBuf};

use super::{ExtractError, ExtractResult};

/// Per-extractor entry point. Reads from `in_dir` (the
/// `reference/` root), writes to `out_dir` (the workspace root), and
/// returns the list of files written.
pub fn run(in_dir: &Path, out_dir: &Path) -> ExtractResult<Vec<PathBuf>> {
    let _ = (in_dir, out_dir);
    Err(ExtractError::Unimplemented { task: "P1-03" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_stub() {
        let err = run(Path::new("."), Path::new(".")).unwrap_err();
        assert!(matches!(err, ExtractError::Unimplemented { task: "P1-03" }));
    }
}
