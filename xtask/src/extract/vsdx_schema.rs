//! MS-VSDX schema inventory extractor (P1-05).
//!
//! Reads the XSD embedded in
//! `reference/ms-vsdx/office-developer-client-docs/docs/visio/schema-mapvisio-xml.md`
//! (and the section/cell/complextype pages for the element-by-element
//! inventory) and produces `docs/vsdx-inventory.md`.
//!
//! Clean-room: the extractor never writes XSD prose. It writes only
//! derived facts (element name, type name, parent element, MVP status).
//! See AGENTS.md section 4 and KICKOFF.md "Clean-room".
//!
//! Status: stub. Returns [`ExtractError::Unimplemented`] with task id
//! `P1-05`.

use std::path::{Path, PathBuf};

use super::{ExtractError, ExtractResult};

/// Per-extractor entry point.
pub fn run(in_dir: &Path, out_dir: &Path) -> ExtractResult<Vec<PathBuf>> {
    let _ = (in_dir, out_dir);
    Err(ExtractError::Unimplemented { task: "P1-05" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_stub() {
        let err = run(Path::new("."), Path::new(".")).unwrap_err();
        assert!(matches!(err, ExtractError::Unimplemented { task: "P1-05" }));
    }
}
