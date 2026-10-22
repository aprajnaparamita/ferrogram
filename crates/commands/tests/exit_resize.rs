//! The Phase 6 exit criterion (task P6-10, ADR 0060): resizing
//! master-based shapes from the corpus recomputes geometry correctly
//! for supported functions (KICKOFF's exit line).
//!
//! The criterion in code: read a corpus file, resize its shape through
//! the model mutation API (`shape.resize`, which writes `Width`/`Height`
//! through [`CellEdit`](ferrogram_commands::builtins) and lets
//! `push_edit` recalculate the dirty cells with `materialise_inherited`,
//! ADR 0056), run recalc, and assert the geometry rows equal
//! **hand-computed** expectations — the master's formulas evaluated at
//! the new size, written here as literals. No cached value from the file
//! takes part in an expectation.
//!
//! ## Deviation from the task's file list (reported in the Result)
//!
//! The task names `blue-box.vsdx` and `qs-box.vsdx`; neither holds a
//! master instance (both page shapes carry no `Master` attribute — their
//! one master, "Dynamic connector", is unused). Their resize case runs
//! as written (the recomputed transform cells and the `Rel*` geometry
//! rows that ADR 0056 keeps), and the master-based case of the exit
//! criterion runs on `testfile5.vsdx`, whose page instance inherits a
//! master whose geometry rows are formulas (`Width*1`, `Height*1`, ...):
//! exactly "the master's formulas evaluated at the new size".

// The never-crash lints apply to non-test code only (clippy.toml).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

#[path = "crosscheck/mod.rs"]
mod helpers;

mod support;

use ferrogram_commands::app::{AppCtx, DocumentSession, Settings};
use ferrogram_commands::builtins::registry;
use ferrogram_commands::{ArgValue, Args, Registry, SessionId};
use ferrogram_model::{Cell, CellValue, Document, ShapeId, Sheet, SheetRef};
use ferrogram_shapesheet::{
    CellAddress, CellRef, DepGraph, DocumentEngine, Error, Evaluation, Formula, FormulaEngine,
    RecalcOptions, RecalcOutcome, recalc_with,
};
use support::dispatch;

use helpers::{corpus_dir, read_corpus_file};

/// The new width of every resize below: 4 in, so the hand-computed
/// expectations halve and double cleanly.
const WIDTH: f64 = 4.0;

/// The new height of every resize below: 2 in.
const HEIGHT: f64 = 2.0;

// ---------------------------------------------------------------------------
// Reading the result back.
// ---------------------------------------------------------------------------

/// The document the context currently holds.
fn document(ctx: &AppCtx) -> Document {
    ctx.active_document().expect("document").document.clone()
}

/// Selects `shape` on `page` the way the shell does.
fn select(ctx: &mut AppCtx, page: ferrogram_model::PageId, shapes: &[ShapeId]) {
    ctx.selection.shapes = shapes.to_vec();
    ctx.selection.anchor = shapes.last().copied();
    ctx.selection.page = Some(page);
}

/// Opens `name` from the corpus and returns the context, registry and
/// session, with the page's first shape selected.
fn open(
    name: &str,
) -> (
    AppCtx,
    Registry,
    SessionId,
    ferrogram_model::PageId,
    ShapeId,
) {
    let document = read_corpus_file(&corpus_dir().join(name));
    let page = document.pages.first().expect("one page").id;
    let shape = document
        .page(page)
        .and_then(|page| page.shapes.first().copied())
        .expect("one page shape");
    let mut ctx = AppCtx::new(Settings::default());
    let index = ctx.open_document(DocumentSession::new(document, None));
    ctx.documents.get_mut(index).expect("session").active_page = Some(page);
    select(&mut ctx, page, &[shape]);
    let session = ctx.documents.get(index).expect("session").id;
    let registry = registry().expect("registry");
    (ctx, registry, session, page, shape)
}

/// Resizes the selection to [`WIDTH`] by [`HEIGHT`].
fn resize(ctx: &mut AppCtx, registry: &Registry) {
    let mut args = Args::new();
    args.set("width", ArgValue::Number(WIDTH));
    args.set("height", ArgValue::Number(HEIGHT));
    dispatch(ctx, registry, "shape.resize", args).expect("resize");
}

/// A direct cell of `shape` (`PinX`).
fn direct<'a>(doc: &'a Document, shape: ShapeId, name: &str) -> &'a Cell {
    doc.shape(shape)
        .expect("shape")
        .sheet
        .cells
        .iter()
        .find(|cell| cell.name.as_str() == name)
        .unwrap_or_else(|| panic!("{name}"))
}

/// The `Geometry` section of `sheet` with this 1-based ordinal (the
/// report's `Geometry1` spelling, [`helpers`]).
fn geometry_section(sheet: &Sheet, ordinal: usize) -> &ferrogram_model::Section {
    let mut seen = 0usize;
    for section in &sheet.sections {
        if section.name.as_str().eq_ignore_ascii_case("Geometry") {
            seen = seen.saturating_add(1);
            if seen == ordinal {
                return section;
            }
        }
    }
    panic!("no Geometry section {ordinal}");
}

/// A row cell of `shape` (`Geometry` section `section`, row `row`).
fn row_cell<'a>(
    doc: &'a Document,
    shape: ShapeId,
    section: usize,
    row: usize,
    name: &str,
) -> &'a Cell {
    let sheet: &Sheet = &doc.shape(shape).expect("shape").sheet;
    let section = geometry_section(sheet, section);
    section
        .rows
        .get(row)
        .and_then(|found| found.cells.iter().find(|cell| cell.name.as_str() == name))
        .unwrap_or_else(|| panic!("{name} in section {} row {row}", section.name.as_str()))
}

/// Asserts that `cell` holds `expected` as a number and keeps `formula`.
fn assert_number(cell: &Cell, formula: &str, expected: f64) {
    assert_eq!(
        cell.formula.as_deref(),
        Some(formula),
        "{} keeps its formula",
        cell.name.as_str()
    );
    match &cell.value {
        Some(CellValue::Number(value)) => {
            assert!(
                (value - expected).abs() <= 1e-9,
                "{}: {value} != {expected}",
                cell.name.as_str()
            );
        }
        other => panic!("{}: {other:?}", cell.name.as_str()),
    }
}

/// Asserts that `cell` holds `expected` as a number and keeps no
/// formula (a constant row the rescaler owns).
fn assert_constant(cell: &Cell, expected: f64) {
    assert_eq!(
        cell.formula,
        None,
        "{} stays a constant",
        cell.name.as_str()
    );
    match &cell.value {
        Some(CellValue::Number(value)) => {
            assert!(
                (value - expected).abs() <= 1e-9,
                "{}: {value} != {expected}",
                cell.name.as_str()
            );
        }
        other => panic!("{}: {other:?}", cell.name.as_str()),
    }
}

// ---------------------------------------------------------------------------
// The named files of the task: `blue-box.vsdx` and `qs-box.vsdx`.
// ---------------------------------------------------------------------------

/// `blue-box.vsdx`: the page shape recomputes its formula cells at the
/// new size and its `Rel*` geometry rows keep their values (ADR 0056's
/// documented rule for relative rows).
///
/// Hand-computed expectations: `LocPinX = Width*0.5` at 4 in is 2 in and
/// `LocPinY = Height*0.5` at 2 in is 1 in; the relative rows outline the
/// box with the constants `0` and `1` whatever the size.
#[test]
fn blue_box_resize_recomputes_its_formula_cells_at_the_new_size() {
    let (mut ctx, registry, _session, page, shape) = open("blue-box.vsdx");
    resize(&mut ctx, &registry);
    let doc = document(&ctx);

    assert_number(direct(&doc, shape, "Width"), "4", WIDTH);
    assert_number(direct(&doc, shape, "Height"), "2", HEIGHT);
    assert_number(direct(&doc, shape, "LocPinX"), "Width*0.5", 2.0);
    assert_number(direct(&doc, shape, "LocPinY"), "Height*0.5", 1.0);

    for (row, x, y) in [
        (0usize, 0.0, 0.0),
        (1, 1.0, 0.0),
        (2, 1.0, 1.0),
        (3, 0.0, 1.0),
        (4, 0.0, 0.0),
    ] {
        assert_constant(row_cell(&doc, shape, 1, row, "X"), x);
        assert_constant(row_cell(&doc, shape, 1, row, "Y"), y);
    }
    let _ = page;
}

/// `qs-box.vsdx`: the same shape geometry with the quick-style cells;
/// the same hand-computed expectations as [`blue_box_resize_recomputes_its_formula_cells_at_the_new_size`].
#[test]
fn qs_box_resize_recomputes_its_formula_cells_at_the_new_size() {
    let (mut ctx, registry, _session, page, shape) = open("qs-box.vsdx");
    resize(&mut ctx, &registry);
    let doc = document(&ctx);

    assert_number(direct(&doc, shape, "LocPinX"), "Width*0.5", 2.0);
    assert_number(direct(&doc, shape, "LocPinY"), "Height*0.5", 1.0);

    for (row, x, y) in [
        (0usize, 0.0, 0.0),
        (1, 1.0, 0.0),
        (2, 1.0, 1.0),
        (3, 0.0, 1.0),
        (4, 0.0, 0.0),
    ] {
        assert_constant(row_cell(&doc, shape, 1, row, "X"), x);
        assert_constant(row_cell(&doc, shape, 1, row, "Y"), y);
    }
    let _ = page;
}

// ---------------------------------------------------------------------------
// The exit criterion itself: a master-based corpus shape.
// ---------------------------------------------------------------------------

/// `testfile5.vsdx`: the page shape is an instance of the `Process`
/// master, whose geometry rows are formulas; the instance carries its
/// own `Inh` rows (the corpus case of P6-07's open issue 2). Resizing it
/// materialises the master's formulas at the new size.
///
/// Hand-computed expectations, from the master's rows:
///
/// | row (`IX`) | master formula | at 4 x 2 in |
/// |---|---|---|
/// | 2 `X` | `Width*1` | 4 |
/// | 3 `X` | `Width*1` | 4 |
/// | 3 `Y` | `Height*1` | 2 |
/// | 4 `Y` | `Height*1` | 2 |
///
/// and the transform cells the master defines: `LocPinX = Width*0.5` is
/// 2 and `LocPinY = Height*0.5` is 1. The file's own cached values (the
/// master-sized snapshot: 3.05 x 1.83 in) take no part in these.
#[test]
fn a_master_instance_recomputes_the_master_geometry_rows_at_the_new_size() {
    let (mut ctx, registry, _session, page, shape) = open("testfile5.vsdx");
    let master = document(&ctx).shape(shape).expect("shape").master;
    assert!(master.is_some(), "the page shape is a master instance");
    resize(&mut ctx, &registry);
    let doc = document(&ctx);

    // The instance's own geometry rows: the `Inh` cells materialise to
    // the master's formulas, evaluated against the instance's size.
    assert_number(row_cell(&doc, shape, 1, 0, "X"), "Width*1", 4.0);
    assert_number(row_cell(&doc, shape, 1, 1, "X"), "Width*1", 4.0);
    assert_number(row_cell(&doc, shape, 1, 1, "Y"), "Height*1", 2.0);
    assert_number(row_cell(&doc, shape, 1, 2, "Y"), "Height*1", 2.0);

    // The transform cells the master's formulas define.
    assert_number(direct(&doc, shape, "LocPinX"), "Width*0.5", 2.0);
    assert_number(direct(&doc, shape, "LocPinY"), "Height*0.5", 1.0);
    let _ = page;
}

// ---------------------------------------------------------------------------
// The evaluation count of the resize case (ADR 0055's budget).
// ---------------------------------------------------------------------------

/// A [`FormulaEngine`] that counts the cell evaluations of one pass.
struct CountingEngine {
    /// How many cells the pass evaluated.
    evaluations: u32,
}

impl FormulaEngine for CountingEngine {
    fn references(&mut self, formula: &Formula) -> Result<Vec<CellRef>, Error> {
        DocumentEngine.references(formula)
    }

    fn evaluate(
        &mut self,
        document: &mut Document,
        formula: &Formula,
        sheet: SheetRef,
    ) -> Result<Evaluation, Error> {
        self.evaluations = self.evaluations.saturating_add(1);
        DocumentEngine.evaluate(document, formula, sheet)
    }
}

/// The evaluation count of the exit criterion's resize, against
/// [`RecalcOptions::DEFAULT_MAX_EVALUATIONS`] (ADR 0055 leaves the
/// constant open; the Result of task P6-10 reports the measured number).
#[test]
fn the_resize_case_runs_far_below_the_evaluation_budget() {
    let (mut ctx, registry, session, _page, _shape) = open("testfile5.vsdx");
    let mut args = Args::new();
    args.set("width", ArgValue::Number(WIDTH));
    args.set("height", ArgValue::Number(HEIGHT));
    // `Registry::execute`, not `dispatch`: the dispatch seam pushes the
    // edit and runs the pass itself (`registry.rs`), which hides the
    // step's dirty cells. Here the run stops after `apply` and the pass
    // is replayed below with a counting engine.
    let outcome = registry
        .execute(
            &ferrogram_commands::CommandId::intern("shape.resize").expect("id"),
            &mut ctx,
            ferrogram_commands::Invocation::new(args, ferrogram_commands::Source::Test),
        )
        .expect("resize");

    // The exact dirty set the resize wrote, replayed on the document it
    // left behind: the same pass the edit runs (ADR 0056).
    let dirty: Vec<CellAddress> = outcome
        .edit
        .as_ref()
        .expect("one undoable edit")
        .dirty_cells()
        .to_vec();
    let mut document = ctx
        .documents
        .iter()
        .find(|slot| slot.id == session)
        .expect("session")
        .document
        .clone();
    let mut graph = DepGraph::build(&document).expect("graph");
    let mut engine = CountingEngine { evaluations: 0 };
    let outcome: RecalcOutcome = recalc_with(
        &mut document,
        &dirty,
        &RecalcOptions {
            materialise_inherited: true,
            ..RecalcOptions::new()
        },
        &mut graph,
        &mut engine,
    )
    .expect("recalc");
    assert!(!outcome.truncated, "the pass stays inside its budget");
    println!(
        "resize evaluations: {} (DEFAULT_MAX_EVALUATIONS {})",
        engine.evaluations,
        RecalcOptions::DEFAULT_MAX_EVALUATIONS
    );
    assert!(
        u64::from(engine.evaluations) <= u64::from(RecalcOptions::DEFAULT_MAX_EVALUATIONS),
        "{} evaluations exceed the budget",
        engine.evaluations
    );
    // ADR 0055: "tens of nodes, not thousands" for one shape's resize.
    assert!(
        engine.evaluations <= 200,
        "{} evaluations for one shape's resize",
        engine.evaluations
    );
}
