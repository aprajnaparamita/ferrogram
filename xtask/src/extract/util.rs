//! Shared utilities for the extractors.
//!
//! See ADR 0009 for the contract. P1-03 fills the P1-02 stubs:
//! deterministic writer, sha256 helper, path validation.

#![allow(dead_code)] // some helpers are P1-03 building blocks not yet
// called by every extractor; the test suite
// covers them.

use std::path::{Path, PathBuf};

/// True when the path resolves to a file we can read.
pub fn is_readable(path: &Path) -> bool {
    path.is_file()
}

/// True when the path resolves to a directory we can list / create files in.
pub fn is_writable_dir(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    // Probe by trying to create and remove a temp file.
    let probe = path.join(".ferrogram-write-probe");
    match std::fs::write(&probe, b"x") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// Joins a workspace-relative path under `base`, rejecting paths
/// that escape `base` (no `..`, no absolute components). Returns
/// `None` on escape or on empty / non-relative input.
pub fn safe_join(base: &Path, rel: &str) -> Option<PathBuf> {
    if rel.is_empty() {
        return None;
    }
    if Path::new(rel).is_absolute() {
        return None;
    }
    if rel.contains("..") {
        return None;
    }
    Some(base.join(rel))
}

/// Writes `text` to `path` atomically (write to a temp file in the
/// same directory, then rename). The temp file name is random so
/// concurrent writes to the same path don't collide.
pub fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    use std::io::Write;

    let parent = path
        .parent()
        .ok_or_else(|| format!("path has no parent: {}", path.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;

    let file_name = path
        .file_name()
        .ok_or_else(|| format!("path has no file name: {}", path.display()))?
        .to_string_lossy()
        .into_owned();
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = parent.join(format!(".tmp.{pid}.{nanos}.{file_name}.tmp"));

    {
        let mut f = std::fs::File::create(&tmp)
            .map_err(|e| format!("cannot create {}: {e}", tmp.display()))?;
        f.write_all(text.as_bytes())
            .map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
        f.sync_all()
            .map_err(|e| format!("cannot sync {}: {e}", tmp.display()))?;
    }

    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("cannot rename to {}: {e}", path.display())
    })?;
    Ok(())
}

/// Computes the SHA-256 of `text` and returns the lowercase hex string.
pub fn sha256_hex(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    let digest = h.finalize();
    let mut out = String::with_capacity(64);
    for b in digest {
        use std::fmt::Write;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn is_readable_and_writable() {
        let tmp = std::env::temp_dir().join("ferrogram-util-test");
        std::fs::create_dir_all(&tmp).unwrap();
        assert!(is_writable_dir(&tmp));
        let f = tmp.join("hello.txt");
        std::fs::write(&f, b"hi").unwrap();
        assert!(is_readable(&f));
        let _ = std::fs::remove_file(&f);
        let _ = std::fs::remove_dir(&tmp);
    }

    #[test]
    fn safe_join_rejects_escape() {
        assert!(safe_join(Path::new("/work"), "../etc").is_none());
        assert!(safe_join(Path::new("/work"), "/abs").is_none());
        assert!(safe_join(Path::new("/work"), "").is_none());
        assert_eq!(
            safe_join(Path::new("/work"), "a/b"),
            Some(PathBuf::from("/work/a/b"))
        );
    }

    #[test]
    fn write_atomic_creates_and_overwrites() {
        let tmp = std::env::temp_dir().join("ferrogram-util-atomic");
        std::fs::create_dir_all(&tmp).unwrap();
        let f = tmp.join("out.txt");
        write_atomic(&f, "v1").unwrap();
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "v1");
        write_atomic(&f, "v2").unwrap();
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "v2");
        let _ = std::fs::remove_file(&f);
        let _ = std::fs::remove_dir(&tmp);
    }

    #[test]
    fn write_atomic_leaves_no_tmp() {
        let tmp = std::env::temp_dir().join("ferrogram-util-atomic2");
        std::fs::create_dir_all(&tmp).unwrap();
        let f = tmp.join("out.txt");
        write_atomic(&f, "v1").unwrap();
        let names: HashSet<String> = std::fs::read_dir(&tmp)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        assert!(names.contains("out.txt"));
        for n in &names {
            assert!(!n.ends_with(".tmp"), "leftover tmp: {n}");
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn sha256_known_value() {
        // Empty string SHA-256.
        assert_eq!(
            sha256_hex(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
