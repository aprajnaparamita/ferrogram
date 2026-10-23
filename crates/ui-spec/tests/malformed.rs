//! Malformed-input and property tests for [`Loader`] (P1-08 goals 4
//! and 5).
//!
//! Two rules, checked for every input:
//!
//! - a malformed document produces a [`LoadError`], never a panic,
//! - pathological input (deep nesting, 10 MB scalars) finishes in
//!   well under the "few seconds" budget.
//!
//! The scratch workspaces are copies of the hand-written
//! `tests/fixtures/good` data set with one file replaced.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use larrygram_ui_spec::{LayoutSets, LoadError, Loader};
use proptest::prelude::*;

/// The good fixture workspace (the baseline documents).
fn baseline() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/good")
}

/// The four data files the loader reads, in read order.
const FILES: [&str; 4] = [
    "panel-2016.yaml",
    "panel-365.yaml",
    "commands.yaml",
    "shortcuts.yaml",
];

/// Builds a scratch workspace with the baseline documents and one
/// file replaced by `bytes`.
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test helper
fn workspace_with(file: &str, bytes: &[u8]) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "larrygram-ui-spec-malformed-{}-{n}",
        std::process::id()
    ));
    let ui = root.join("ui-spec");
    std::fs::create_dir_all(&ui).expect("scratch dir");
    for name in FILES {
        let source = baseline().join("ui-spec").join(name);
        if name == file {
            std::fs::write(ui.join(name), bytes).expect("scratch file");
        } else {
            std::fs::copy(&source, ui.join(name)).expect("baseline copy");
        }
    }
    root
}

/// Loads a scratch workspace whose `file` holds `bytes`. Returns the
/// load result and cleans the workspace up.
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test helper
fn load_with(file: &str, bytes: &[u8]) -> Result<LayoutSets, LoadError> {
    let root = workspace_with(file, bytes);
    let loader = Loader::new(&root).expect("the embedded schema compiles");
    let result = loader.load_all();
    let _ = std::fs::remove_dir_all(&root);
    result
}

/// Asserts that `bytes` in `file` yields a `LoadError` within the time
/// budget, never a panic.
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // test helper
fn assert_load_error(file: &str, bytes: &[u8], what: &str) {
    let started = Instant::now();
    let result = load_with(file, bytes);
    let took = started.elapsed();
    assert!(
        took < Duration::from_secs(5),
        "{what}: took {took:?}, must stay under a few seconds"
    );
    match result {
        Err(_) => {}
        Ok(_) => panic!("{what}: expected a LoadError, got Ok"),
    }
}

// ---- Goal 5: named malformed inputs ---------------------------------

#[test]
fn truncated_yaml_is_a_load_error() {
    assert_load_error("panel-2016.yaml", b"tabs: [1, 2", "truncated YAML");
    assert_load_error("commands.yaml", b"- fluent_ids: [", "truncated commands");
}

#[test]
fn wrong_top_level_type_is_a_load_error() {
    assert_load_error("panel-2016.yaml", b"42\n", "scalar instead of a layout");
    assert_load_error("commands.yaml", b"a map, not a table\n", "wrong table type");
    assert_load_error("shortcuts.yaml", b"{}\n", "map instead of a table");
}

#[test]
fn deep_nesting_is_a_load_error() {
    // Flow-style nesting far past any real document. The loader must
    // reject it, not blow the stack (P1-08 goal 5: recursion depth).
    for depth in [64usize, 1_000, 100_000] {
        let mut text = String::from("tabs: ");
        text.push_str(&"[".repeat(depth));
        text.push_str(&"]".repeat(depth));
        assert_load_error(
            "panel-2016.yaml",
            text.as_bytes(),
            &format!("nesting {depth}"),
        );
    }
}

#[test]
fn a_ten_mb_label_is_a_load_error() {
    // 10 MB of label text: schema rejection, quickly.
    let mut text = String::from("- fluent_ids: []\n  id: edit.undo\n  label: \"");
    text.push_str(&"A".repeat(10 * 1024 * 1024));
    text.push_str("\"\n  mapping_confidence: unknown\n  visuicmd: null\n");
    assert_load_error("commands.yaml", text.as_bytes(), "10 MB label");
}

#[test]
fn duplicate_keys_are_a_load_error() {
    let text = "\
- fluent_ids: []
  id: edit.undo
  id: edit.redo
  label: Undo
  mapping_confidence: unknown
  visuicmd: null
";
    assert_load_error("commands.yaml", text.as_bytes(), "duplicate keys");
}

#[test]
fn non_utf8_bytes_are_a_load_error() {
    assert_load_error("panel-2016.yaml", b"tabs: \xff\xfe", "non-UTF-8 bytes");
    assert_load_error("shortcuts.yaml", b"\x80\x81\x82", "non-UTF-8 bytes");
}

// ---- Goal 4: the loader never panics on arbitrary input --------------

proptest! {
    /// Arbitrary bytes in any of the four files: the loader answers
    /// (Ok or Err), it never panics.
    #[test]
    fn arbitrary_bytes_never_panic(
        file_index in 0..FILES.len(),
        data in proptest::collection::vec(any::<u8>(), 0..1024),
    ) {
        let _ = load_with(FILES[file_index], &data);
    }

    /// Arbitrary text in any of the four files: same contract.
    #[test]
    fn arbitrary_text_never_panics(file_index in 0..FILES.len(), text in ".*") {
        let _ = load_with(FILES[file_index], text.as_bytes());
    }

    /// Non-UTF-8 bytes always come back as an error (the files are
    /// read as UTF-8 text).
    #[test]
    fn non_utf8_bytes_are_always_an_error(
        file_index in 0..FILES.len(),
        data in proptest::collection::vec(any::<u8>(), 1..256)
            .prop_filter("valid UTF-8", |b| std::str::from_utf8(b).is_err()),
    ) {
        let result = load_with(FILES[file_index], &data);
        prop_assert!(result.is_err(), "non-UTF-8 bytes must not load");
    }
}
