//! A recalc pass over the corpus (task P6-11).
//!
//! The Phase 6 pieces promise one thing about real files (ADR 0055,
//! ADR 0060): a pass over a corpus document terminates, reports what it
//! did, and reports the same thing on a second run (determinism). This
//! file walks every `corpus/vsdx` file twice and asserts it. The one
//! file ADR 0060 names for its cycle, `recursion-cycle.vsdx`, turns out
//! to hold no formulas at all (P6-06's Result: its cycle is in the
//! package relationships), so the file pins that a pass over it changes
//! nothing; the formula-cycle rule itself is pinned on a hand-built
//! cycle in `crates/shapesheet/tests/recalc_engine.rs`.
//!
//! The corpus is fetched, never committed (ADR 0005): the tests
//! self-skip with a message and fail instead when
//! `FERROGRAM_REQUIRE_CORPUS=1` is set, the workspace convention of the
//! `crosscheck` helpers.

// The never-crash lints apply to non-test code only (clippy.toml).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

#[path = "crosscheck/mod.rs"]
mod helpers;

use helpers::{LocatedCell, corpus_files, located_cells, read_corpus_file, skip_without_corpus};

use ferrogram_shapesheet::{CellAddress, RecalcOptions, recalc};

/// Every cell address of a document, in walk order.
fn addresses(cells: &[LocatedCell<'_>]) -> Vec<CellAddress> {
    cells.iter().map(|cell| cell.address.clone()).collect()
}

/// Every corpus file recalculates twice to the same report: the pass
/// terminates inside its budget and its updates and warnings are
/// reproducible (ADR 0055), whatever the file holds.
#[test]
fn every_corpus_file_recalculates_deterministically() {
    let files = corpus_files();
    if skip_without_corpus(&files) {
        return;
    }
    for path in &files {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let mut document = read_corpus_file(path);
        let dirty = addresses(&located_cells(&document));
        let first = recalc(&mut document, &dirty, &RecalcOptions::new()).expect("recalc");
        assert!(!first.truncated, "{name}: the corpus files fit the budget");

        let mut again = read_corpus_file(path);
        let second = recalc(&mut again, &dirty, &RecalcOptions::new()).expect("recalc");
        assert_eq!(second.updates, first.updates, "{name}: deterministic updates");
        assert_eq!(
            second.warnings, first.warnings,
            "{name}: deterministic warnings"
        );
    }
}

/// `recursion-cycle.vsdx` (ADR 0060's named cycle file) holds no
/// formulas: its cycle is in the package relationships (P6-06's
/// Result), so a pass over it changes nothing and warns nothing.
#[test]
fn the_relationship_cycle_file_holds_no_formulas() {
    let files = corpus_files();
    if skip_without_corpus(&files) {
        return;
    }
    let path = files
        .iter()
        .find(|path| {
            path.file_name()
                .is_some_and(|name| name == "recursion-cycle.vsdx")
        })
        .cloned()
        .expect("corpus/vsdx/recursion-cycle.vsdx (corpus/PROVENANCE.md)");
    let mut document = read_corpus_file(&path);
    let dirty = addresses(&located_cells(&document));
    let outcome = recalc(&mut document, &dirty, &RecalcOptions::new()).expect("recalc");
    assert_eq!(outcome.updates, Vec::new());
    assert_eq!(outcome.warnings, Vec::new());
    assert!(!outcome.truncated);
}
