//! ShapeSheet reference extractor (P1-04).
//!
//! Reads the markdown pages under
//! `reference/ms-vsdx/office-developer-client-docs/docs/visio/` and
//! produces `ui-spec/shapesheet.toml`. See ADR 0008 for the file
//! format and ADR 0009 for the extractor contract.
//!
//! The ShapeSheet reference directory is the same as the MS-VSDX
//! `docs/visio/` clone (one clone serves both extractors).
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
