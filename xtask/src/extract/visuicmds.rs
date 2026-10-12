//! VisUICmds extractor (P1-04).
//!
//! Reads `reference/learn/visuicmds/visuicmds.md` and produces the
//! VisUICmds half of `ui-spec/commands.yaml`. P1-04 also reads
//! `reference/learn/enums/` to gather the four required enumerations
//! (`VisSectionIndices`, `VisRowIndices`, `VisCellIndices`,
//! `VisUnitCodes`).
//!
//! See ADR 0007 for the commands schema and ADR 0009 for the extractor
//! contract.
//!
//! Status: stub. Returns [`ExtractError::Unimplemented`] with task id
//! `P1-04`.

use std::path::{Path, PathBuf};

use super::{ExtractError, ExtractResult};

/// Per-extractor entry point. P1-04 implementation writes the
/// VisUICmds half of `ui-spec/commands.yaml`; it does **not** merge
/// with the Fluent half — `parity::run` (P1-03) does the merge.
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
