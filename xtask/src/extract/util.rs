//! Shared utilities for the extractors.
//!
//! See ADR 0009 for the contract. Holds the one YAML emitter
//! ([`emit_yaml`]), the one write-and-validate pipeline
//! ([`write_and_validate`]), the one atomic writer, the one HTML
//! entity decoder ([`unescape_entities`]) and the one source reader
//! ([`read_source`]). Nothing here panics; every failure is a message
//! or an [`ExtractError`].

#![allow(dead_code)] // some helpers are P1-03 building blocks not yet
// called by every extractor; the test suite
// covers them.

use std::path::{Path, PathBuf};

use serde_json::Value;
use serde_saphyr::SerializerOptions;

use super::schema::Schema;
use super::{ExtractError, ExtractResult};

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

/// Writes `text` to `path` atomically. The one atomic writer shared
/// with `crate::corpus`; temp names are unique per call (see
/// [`crate::util::unique_temp_path`]).
pub fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    crate::util::write_atomic_bytes(path, text.as_bytes())
}

/// Writes a generated file and validates it against
/// `ui-spec/schema.json`. The one write-and-validate pipeline every
/// extractor uses.
pub fn write_and_validate(schema: &Schema, path: &Path, text: &str) -> ExtractResult<()> {
    write_atomic(path, text).map_err(|source| ExtractError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    schema.validate_file(path)
}

/// The one YAML emitter. The emitter options below are the committed
/// style of every generated YAML file; change them here only.
pub fn emit_yaml(value: &Value) -> Result<String, String> {
    // serde-saphyr 1.x defaults differ from 0.0.16 in two ways that
    // affect the diff:
    //
    // - `compact_list_indent = true`: the list dash is put at the
    //   parent's indent. Disable to keep the older
    //   `tabs:\n  - bounds: null` style.
    // - `prefer_block_scalars = true`: long single-line strings and
    //   strings with newlines are written as block scalars
    //   (`| ... |`). Disable to keep plain (and quoted) inline
    //   scalars.
    let mut options = SerializerOptions::default();
    options.compact_list_indent = false;
    options.prefer_block_scalars = false;
    serde_saphyr::to_string_with_options(value, options).map_err(|e| format!("YAML serialize: {e}"))
}

/// Reads a source file under `reference/`. A missing file is
/// [`ExtractError::Missing`]; every other read failure (a directory
/// in the way, non-UTF-8 bytes, permissions) keeps the `io::Error`
/// text in [`ExtractError::Io`].
pub fn read_source(path: &Path) -> ExtractResult<String> {
    std::fs::read_to_string(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => ExtractError::Missing {
            path: path.to_path_buf(),
        },
        _ => ExtractError::Io {
            path: path.to_path_buf(),
            source: e.to_string(),
        },
    })
}

/// Decodes the HTML entities the sources use (`&amp;`, `&#39;`,
/// numeric codes). Unknown entities pass through unchanged. The one
/// entity decoder shared by every extractor.
pub fn unescape_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        let Some(semi) = rest[amp..].find(';').map(|e| amp + e) else {
            out.push_str(&rest[amp..]);
            return out;
        };
        let entity = &rest[amp + 1..semi];
        let decoded = match entity {
            "amp" => Some("&".to_string()),
            "lt" => Some("<".to_string()),
            "gt" => Some(">".to_string()),
            "quot" => Some("\"".to_string()),
            "apos" | "#39" | "#x27" | "#X27" => Some("'".to_string()),
            "nbsp" | "#160" => Some(" ".to_string()),
            _ => {
                let radix = if entity.starts_with("#x") || entity.starts_with("#X") {
                    16
                } else {
                    10
                };
                entity.strip_prefix('#').and_then(|d| {
                    let d = d
                        .strip_prefix('x')
                        .or_else(|| d.strip_prefix('X'))
                        .unwrap_or(d);
                    u32::from_str_radix(d, radix)
                        .ok()
                        .and_then(char::from_u32)
                        .map(|c| c.to_string())
                })
            }
        };
        match decoded {
            Some(s) => out.push_str(&s),
            None => {
                out.push('&');
                out.push_str(entity);
                out.push(';');
            }
        }
        rest = &rest[semi + 1..];
    }
    out.push_str(rest);
    out
}

/// Collapses whitespace runs to single spaces and trims.
pub fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
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
    use crate::util::test_support::TempDir;
    use std::collections::HashSet;

    #[test]
    fn is_readable_and_writable() {
        let tmp = TempDir::new("util-rw");
        assert!(is_writable_dir(tmp.path()));
        let f = tmp.join("hello.txt");
        std::fs::write(&f, b"hi").unwrap();
        assert!(is_readable(&f));
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
        let tmp = TempDir::new("util-atomic");
        let f = tmp.join("out.txt");
        write_atomic(&f, "v1").unwrap();
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "v1");
        write_atomic(&f, "v2").unwrap();
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "v2");
    }

    #[test]
    fn write_atomic_leaves_no_tmp() {
        let tmp = TempDir::new("util-atomic2");
        let f = tmp.join("out.txt");
        write_atomic(&f, "v1").unwrap();
        let names: HashSet<String> = std::fs::read_dir(tmp.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        assert!(names.contains("out.txt"));
        for n in &names {
            assert!(!n.ends_with(".tmp"), "leftover tmp: {n}");
        }
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

    #[test]
    fn unescape_decodes_known_entities_and_keeps_unknown() {
        assert_eq!(unescape_entities("a &amp; b"), "a & b");
        assert_eq!(unescape_entities("x &#39;y&#39;"), "x 'y'");
        assert_eq!(unescape_entities("&#65;"), "A");
        assert_eq!(unescape_entities("keep &unknown; as-is"), "keep &unknown; as-is");
        assert_eq!(unescape_entities("no entity"), "no entity");
    }

    #[test]
    fn collapse_squeezes_whitespace() {
        assert_eq!(collapse("  a\n b\t c  "), "a b c");
    }

    #[test]
    fn read_source_maps_missing_and_bad_reads() {
        let tmp = TempDir::new("util-read");
        let missing = tmp.join("gone.md");
        let err = read_source(&missing).unwrap_err();
        assert!(matches!(err, ExtractError::Missing { .. }), "{err}");

        // A directory in the way is an Io error, not Missing.
        std::fs::create_dir_all(tmp.join("adir")).unwrap();
        let err = read_source(&tmp.join("adir")).unwrap_err();
        assert!(matches!(err, ExtractError::Io { .. }), "{err}");
    }

    #[test]
    fn emit_yaml_keeps_the_committed_style() {
        let mut obj = serde_json::Map::new();
        obj.insert("a".into(), Value::String("plain".into()));
        let text = emit_yaml(&Value::Object(obj)).unwrap();
        assert!(text.contains("a: plain"), "{text}");
    }
}
