//! Parity matrix extractor (P1-03).
//!
//! Reads the two layout files and `ui-spec/commands.yaml`, and
//! produces `docs/parity/controls.md`: every control id in both
//! layouts, with a status of `implemented`, `stub`, or `out-of-scope`.
//!
//! Initial state (Phase 1) is `stub` for every control. Implementers
//! in later phases update the matrix by adding their crate's
//! `commands.yaml` to the registry; the extractor picks up the new
//! status automatically.
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
