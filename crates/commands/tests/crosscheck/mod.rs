//! Shared helpers for the Phase 6 cross-check and exit-criterion tests
//! (task P6-10, ADR 0059 and ADR 0060).
//!
//! Both `tests/crosscheck.rs` and `tests/exit_resize.rs` include this
//! file with `#[path = "crosscheck/mod.rs"] mod helpers;`. It holds what
//! they share: the corpus discovery convention of the workspace (the
//! fetched corpus is never committed, ADR 0005; the tests self-skip
//! with a message and fail instead when `LARRYGRAM_REQUIRE_CORPUS=1`
//! is set) and the walk over every cell of a document, with the report
//! spelling of its address.
//!
//! Address spelling (the "cell address" of ADR 0059's failure message
//! and of the documented-exclusion matching): `page0!PinX`,
//! `shape1!Geometry2.X3`. The sheet is `page`/`shape` plus the arena
//! index; a repeated section carries its 1-based ordinal the way Visio
//! spells it in a reference (`Geometry1`, `Geometry2`, ...); a row cell
//! carries the row's `IX`, or its 1-based position when the row has no
//! `IX`.

#![allow(dead_code, clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use larrygram_model::{Cell, Document, RowTarget, Sheet, SheetRef};
use larrygram_shapesheet::CellAddress;
use larrygram_vsdx::{Limits, read_document};

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

/// The workspace's corpus convention: returns `true` when the calling
/// test must return early (the corpus is absent), printing why, and
/// failing instead when `LARRYGRAM_REQUIRE_CORPUS=1` makes the corpus
/// mandatory.
pub fn skip_without_corpus(files: &[PathBuf]) -> bool {
    if !files.is_empty() {
        return false;
    }
    eprintln!("skipping: corpus/vsdx is absent; run `cargo xtask corpus`");
    assert!(
        std::env::var_os("LARRYGRAM_REQUIRE_CORPUS").is_none(),
        "corpus/vsdx is missing and LARRYGRAM_REQUIRE_CORPUS=1 is set"
    );
    true
}

/// Reads one corpus package into the model, failing the test on a
/// malformed file (the corpus files must all read; `read_corpus.rs`
/// pins that).
pub fn read_corpus_file(path: &Path) -> Document {
    let bytes = std::fs::read(path).expect("corpus file reads");
    read_document(&bytes, &Limits::DEFAULT).expect("corpus file reads into a document")
}

/// One cell of a document with its engine address and report label.
pub struct LocatedCell<'a> {
    /// The engine's address for the cell (ADR 0055).
    pub address: CellAddress,
    /// The report spelling of the address (see the module docs).
    pub label: String,
    /// The cell itself.
    pub cell: &'a Cell,
}

/// Every cell of `document`, page sheets first and then every shape's
/// sheet (master shapes included), in the engine's order.
pub fn located_cells(document: &Document) -> Vec<LocatedCell<'_>> {
    let mut out = Vec::new();
    for page in &document.pages {
        collect_sheet(
            SheetRef::Page(page.id),
            &format!("page{}", page.id.index),
            &page.sheet,
            &mut out,
        );
    }
    for shape in &document.shapes {
        collect_sheet(
            SheetRef::Shape(shape.id),
            &format!("shape{}", shape.id.index),
            &shape.sheet,
            &mut out,
        );
    }
    out
}

/// The cells of one sheet, with addresses and labels.
fn collect_sheet<'a>(
    sheet: SheetRef,
    sheet_label: &str,
    data: &'a Sheet,
    out: &mut Vec<LocatedCell<'a>>,
) {
    // The 1-based ordinal of each repeated section name (`Geometry1`).
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    let mut ordinals: Vec<usize> = Vec::with_capacity(data.sections.len());
    for section in &data.sections {
        let count = seen.entry(section.name.as_str()).or_insert(0);
        *count = count.saturating_add(1);
        ordinals.push(*count);
    }
    for cell in &data.cells {
        out.push(LocatedCell {
            address: CellAddress::new(sheet, cell.name.clone()),
            label: format!("{sheet_label}!{}", cell.name.as_str()),
            cell,
        });
    }
    for (index, section) in data.sections.iter().enumerate() {
        let ordinal = ordinals.get(index).copied().unwrap_or(1);
        let section_label = format!("{}{ordinal}", section.name.as_str());
        for cell in &section.cells {
            let mut address = CellAddress::new(sheet, cell.name.clone());
            address.section = Some(section.name.clone());
            out.push(LocatedCell {
                address,
                label: format!("{sheet_label}!{section_label}.{}", cell.name.as_str()),
                cell,
            });
        }
        for (position, row) in section.rows.iter().enumerate() {
            let target = RowTarget {
                position: Some(position),
                ix: row.ix,
            };
            let row_label = row
                .ix
                .map(|ix| ix.to_string())
                .unwrap_or_else(|| position.saturating_add(1).to_string());
            for cell in &row.cells {
                out.push(LocatedCell {
                    address: CellAddress::in_row(
                        sheet,
                        section.name.clone(),
                        target,
                        cell.name.clone(),
                    ),
                    label: format!(
                        "{sheet_label}!{section_label}.{}{row_label}",
                        cell.name.as_str()
                    ),
                    cell,
                });
            }
        }
    }
}
