//! Shared plumbing for the `larrygram-render` integration tests (task
//! P3-10): where the hand-authored fixtures and the fetched corpus live,
//! how to read them as documents, and how to build one scene per page.
//!
//! Fixtures under `corpus/fixtures/` are committed (our own work, ADR
//! 0005); corpus files under `corpus/vsdx/` are fetched and never
//! committed, so corpus-dependent tests skip with a message when the
//! directory is absent and fail instead when `LARRYGRAM_REQUIRE_CORPUS=1`
//! is set.

#![allow(dead_code)] // each test binary uses a different subset of the helpers

use std::path::{Path, PathBuf};

use larrygram_model::{Document, PageId};
use larrygram_render::{BuildOptions, Scene, build_scene};
use larrygram_vsdx::{Limits, read_document};

/// The `corpus/fixtures/` directory in the repository root (committed
/// hand-authored fixtures).
pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/fixtures")
}

/// The corpus directory with the real `.vsdx` packages (fetched).
pub fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("vsdx")
}

/// The `.vsdx` files of the corpus, sorted for a stable order.
pub fn corpus_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = match std::fs::read_dir(corpus_dir()) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "vsdx"))
            .collect(),
        Err(_) => Vec::new(),
    };
    files.sort();
    files
}

/// Fails the calling test when `LARRYGRAM_REQUIRE_CORPUS=1` is set and
/// the corpus is absent; returns `true` (the test returns early) when the
/// corpus is absent and the test may skip.
pub fn require_corpus_or(files: &[PathBuf]) -> bool {
    if files.is_empty() {
        assert!(
            std::env::var_os("LARRYGRAM_REQUIRE_CORPUS").is_none(),
            "corpus/vsdx is missing and LARRYGRAM_REQUIRE_CORPUS=1 is set"
        );
        eprintln!("skipping: corpus/vsdx is absent; run `cargo xtask corpus`");
        true
    } else {
        false
    }
}

/// The document of a committed fixture, e.g. `"render-fills.vsdx"`.
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test helper; tests may panic
pub fn fixture_document(name: &str) -> Document {
    let bytes =
        std::fs::read(fixtures_dir().join(name)).unwrap_or_else(|e| panic!("fixture {name}: {e}"));
    read_document(&bytes, &Limits::DEFAULT).expect("fixture reads as a document")
}

/// The document of one corpus file, or `None` when the corpus is absent.
pub fn corpus_document(name: &str) -> Option<Document> {
    let bytes = std::fs::read(corpus_dir().join(name)).ok()?;
    read_document(&bytes, &Limits::DEFAULT).ok()
}

/// Builds one scene per page of `doc`, in page order.
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test helper; tests may panic
pub fn page_scenes(doc: &Document, background: bool) -> Vec<Scene> {
    let options = BuildOptions {
        background,
        ..BuildOptions::new()
    };
    doc.pages
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let page = PageId {
                index: u32::try_from(index).expect("page index"),
            };
            build_scene(doc, page, &options).expect("builds")
        })
        .collect()
}
