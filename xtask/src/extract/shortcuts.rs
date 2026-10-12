//! Keyboard shortcuts extractor (P1-04).
//!
//! Reads `reference/support/shortcuts/keyboard-shortcuts-visio.html`
//! (a Wayback Machine snapshot of the Microsoft Support page) and
//! produces `ui-spec/shortcuts.yaml`.
//!
//! See ADR 0007 for the schema, ADR 0009 for the extractor contract,
//! and `docs/sources.md` (source 5) for the snapshot URL recorded in
//! the generated file headers.
//!
//! Status: stub. Returns [`ExtractError::Unimplemented`] with task id
//! `P1-04`.

use std::path::{Path, PathBuf};

use super::{ExtractError, ExtractResult};

/// Per-extractor entry point.
pub fn run(in_dir: &Path, out_dir: &Path) -> ExtractResult<Vec<PathBuf>> {
    let _ = (in_dir, out_dir);
    Err(ExtractError::Unimplemented { task: "P1-04" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_stub() {
        let err = run(Path::new("."), Path::new(".")).unwrap_err();
        assert!(matches!(err, ExtractError::Unimplemented { task: "P1-04" }));
    }
}
