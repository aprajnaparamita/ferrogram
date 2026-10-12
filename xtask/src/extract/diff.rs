//! Structural diff between `visio-2016.yaml` and `visio-365.yaml` (P1-03).
//!
//! Reads the two layout files written by [`super::fluent`] and
//! produces `ui-spec/diff-2016-vs-365.md`. The diff is a markdown
//! table of control ids in only one, only the other, or both, with
//! control type and ribbon path.
//!
//! See ADR 0009 for the extractor contract.
//!
//! Status: stub. Returns [`ExtractError::Unimplemented`] with task id
//! `P1-03`.

use std::path::{Path, PathBuf};

use super::{ExtractError, ExtractResult};

/// Per-extractor entry point.
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
