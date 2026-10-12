//! Shared utilities for the extractors.
//!
//! P1-02 provides signatures and minimal helpers used by the
//! dispatcher; the heavy helpers (deterministic writers, sha256,
//! path validation) arrive with P1-03..P1-05.
//!
//! See ADR 0009 for the contract.

// Stub helpers used by the per-extractor modules once P1-03..P1-05
// land. Without this allow the bin build warns on the unused
// signatures.
#![allow(dead_code)]

use std::path::Path;

/// True when the path resolves to a file we can read.
pub fn is_readable(_path: &Path) -> bool {
    false
}

/// True when the path resolves to a directory we can list.
pub fn is_writable_dir(_path: &Path) -> bool {
    false
}

/// Joins a workspace-relative path under `base`, rejecting paths
/// that escape `base` (no `..`, no absolute components). P1-03
/// implementation.
pub fn safe_join(_base: &Path, _rel: &str) -> Option<std::path::PathBuf> {
    None
}

/// Writes `text` to `path` atomically (write to a temp file in the
/// same directory, then rename). P1-03 implementation; the P1-02
/// signature is here so callers can be written.
pub fn write_atomic(_path: &Path, _text: &str) -> Result<(), String> {
    Err("not implemented (P1-03)".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signatures_compile() {
        // The bodies are no-ops; this test just exercises the API.
        assert!(!is_readable(Path::new("/")));
        assert!(!is_writable_dir(Path::new("/")));
        assert!(safe_join(Path::new("/"), "../etc/passwd").is_none());
        assert!(write_atomic(Path::new("/tmp/x"), "x").is_err());
    }
}
