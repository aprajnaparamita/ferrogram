//! The ADR 0040 undo property over the recalculating commands (task
//! P6-11).
//!
//! `recalc_edit.rs` pins one resize by hand. This file extends the
//! property to every recalc-affecting command the acceptance criteria
//! name — **resize, move, a cell edit and a `SETF`-carrying cell** —
//! and then drives random sequences of them:
//!
//! - run a command, `edit.undo`, and the document (including every
//!   recalculated value, a `GUARD`-ed cell and an unknown function's
//!   cached value) is exactly what it was;
//! - run, `edit.undo`, `edit.redo`, and the document is exactly the
//!   post-run one again;
//! - a whole random sequence rewound leaves the starting document.
//!
//! The fixture has two shapes: one whose transform and geometry cells
//! carry formulas (one stale, one unknown-function, one `GUARD`ed) and
//! one carrying a `SETF` formula that writes a sibling cell when its
//! shape resizes. A panic is the bug (AGENTS.md section 6).

// The never-crash lints apply to non-test code only (clippy.toml).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod support;

use support::dispatch;

use ferrogram_commands::app::{AppCtx, DocumentSession, Settings};
use ferrogram_commands::builtins::registry;
use ferrogram_commands::{ArgValue, Args, Registry};
use ferrogram_model::{
    Cell, CellName, CellValue, Document, PageId, Preserved, Row, Section, SectionName, Shape,
    ShapeId,
};

/// A number cell with `formula` and a cached `value`.
fn number_cell(name: &str, formula: &str, value: f64) -> Cell {
    Cell {
        name: CellName::new(name),
        formula: Some(formula.to_owned()),
        value: Some(CellValue::Number(value)),
        unit: ferrogram_geom::Unit::Inches,
        preserved: Preserved::new(),
    }
}

/// Two shapes on one page: `formulas` (transform and geometry, one cell
/// stale, one with an unknown function, one `GUARD`ed) and `setf` (a
/// `SETF` formula that writes its own `PinX` when the shape resizes,
/// plus a cell that reads the first shape across the page).
fn fixture() -> (AppCtx, Registry, PageId, ShapeId, ShapeId) {
    let mut doc = Document::new();
    let page = doc.add_page("Page-1").expect("page");

    let mut formulas = Shape::new(ShapeId { index: 0 }, 1);
    formulas.sheet.cells = vec![
        number_cell("Width", "2", 2.0),
        number_cell("Height", "1", 1.0),
        number_cell("PinX", "Width*0.5", 9.0),
        number_cell("PinY", "FOO(Height)", 42.0),
        number_cell("Angle", "GUARD(0)", 0.0),
    ];
    let mut geometry = Section::new(SectionName::from_static("Geometry"));
    for (ix, kind, x, y) in [
        (0, "MoveTo", "0", "0"),
        (1, "LineTo", "Width", "0"),
        (2, "LineTo", "Width", "Height"),
    ] {
        let mut row = Row::new();
        row.row_type = Some(kind.to_owned());
        row.ix = Some(ix);
        row.cells = vec![number_cell("X", x, 2.0), number_cell("Y", y, 1.0)];
        geometry.rows.push(row);
    }
    formulas.sheet.sections.push(geometry);
    let formulas = doc.insert_shape(page, formulas, None).expect("insert");

    let mut setf = Shape::new(ShapeId { index: 0 }, 2);
    setf.sheet.cells = vec![
        number_cell("Width", "2", 2.0),
        number_cell("PinX", "1", 1.0),
        number_cell("Sync", "SETF(PinX,Width*3)", 0.0),
        number_cell("Follow", "Sheet.1!PinX+1", 10.0),
    ];
    let setf = doc.insert_shape(page, setf, None).expect("insert");

    let mut ctx = AppCtx::new(Settings::default());
    let index = ctx.open_document(DocumentSession::new(doc, None));
    ctx.documents.get_mut(index).expect("session").active_page = Some(page);
    let registry = registry().expect("registry");
    (ctx, registry, page, formulas, setf)
}

/// The document as it stands.
fn document(ctx: &AppCtx) -> Document {
    ctx.active_document().expect("document").document.clone()
}

/// Selects `shape` as the command subject.
fn select(ctx: &mut AppCtx, page: PageId, shape: ShapeId) {
    ctx.selection.shapes = vec![shape];
    ctx.selection.anchor = Some(shape);
    ctx.selection.page = Some(page);
}

/// Number arguments.
fn args(pairs: &[(&str, f64)]) -> Args {
    let mut args = Args::new();
    for (name, value) in pairs {
        args.set(*name, ArgValue::Number(*value));
    }
    args
}

/// The cached number of a shape's direct cell.
fn number_of(doc: &Document, shape: ShapeId, cell: &str) -> Option<f64> {
    let entry = doc
        .shape(shape)?
        .sheet
        .cells
        .iter()
        .find(|entry| entry.name.as_str() == cell)?;
    match entry.value {
        Some(CellValue::Number(number)) => Some(number),
        _ => None,
    }
}

/// The formula text of a shape's direct cell.
fn formula_of(doc: &Document, shape: ShapeId, cell: &str) -> Option<String> {
    doc.shape(shape)?
        .sheet
        .cells
        .iter()
        .find(|entry| entry.name.as_str() == cell)
        .and_then(|entry| entry.formula.clone())
}

/// Runs `id` with `args` on `shape` and asserts the ADR 0040 property
/// around it: run, undo, compare; run, undo, redo, compare. Returns
/// what the run produced.
fn round_trip(
    ctx: &mut AppCtx,
    registry: &Registry,
    page: PageId,
    shape: ShapeId,
    id: &str,
    args: Args,
) -> Document {
    let before = document(ctx);
    dispatch(ctx, registry, id, args).expect("dispatch");
    let after = document(ctx);
    dispatch(ctx, registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(document(ctx), before, "{id}: undo restores the document");
    dispatch(ctx, registry, "edit.redo", Args::new()).expect("redo");
    assert_eq!(document(ctx), after, "{id}: redo repeats the run");
    dispatch(ctx, registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(document(ctx), before, "{id}: and back again");
    dispatch(ctx, registry, "edit.redo", Args::new()).expect("redo");
    select(ctx, page, shape);
    after
}

/// Resize: `Width`/`Height` are literals, `PinX` and the geometry rows
/// recompute and undo restores the stale values the fixture started
/// with.
#[test]
fn a_resize_is_undone_with_its_recalculated_cells() {
    let (mut ctx, registry, page, formulas, _setf) = fixture();
    select(&mut ctx, page, formulas);
    let after = round_trip(
        &mut ctx,
        &registry,
        page,
        formulas,
        "shape.resize",
        args(&[("width", 4.0), ("height", 3.0)]),
    );
    assert_eq!(number_of(&after, formulas, "PinX"), Some(2.0));
    let restored = document(&ctx);
    assert_eq!(number_of(&restored, formulas, "PinX"), Some(2.0));
    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    let back = document(&ctx);
    assert_eq!(
        number_of(&back, formulas, "PinX"),
        Some(9.0),
        "the pre-edit stale value comes back by snapshot"
    );
    assert_eq!(number_of(&back, formulas, "PinY"), Some(42.0));
}

/// Move: `PinX`/`PinY` become literals, the dependent cells of the
/// other shape recompute (`Follow` reads `Sheet.1!PinX`) and undo
/// restores every one of them.
#[test]
fn a_move_is_undone_with_its_recalculated_cells() {
    let (mut ctx, registry, page, formulas, setf) = fixture();
    select(&mut ctx, page, formulas);
    let before = document(&ctx);
    let after = round_trip(
        &mut ctx,
        &registry,
        page,
        formulas,
        "shape.move",
        args(&[("dx", 1.5), ("dy", -0.5)]),
    );
    assert_eq!(
        number_of(&after, setf, "Follow"),
        number_of(&before, formulas, "PinX").map(|pin| pin + 1.5 + 1.0),
        "the cross-shape dependent recomputed at the new PinX"
    );
    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(document(&ctx), before);
    assert_eq!(
        number_of(&document(&ctx), setf, "Follow"),
        Some(10.0),
        "the dependent's pre-edit value is back"
    );
}

/// A cell edit (the `CellEdit`-backed `shape.set.width`) recalculates
/// its dependents inside the same undo step.
#[test]
fn a_cell_edit_is_undone_with_its_recalculated_cells() {
    let (mut ctx, registry, page, formulas, _setf) = fixture();
    select(&mut ctx, page, formulas);
    let after = round_trip(
        &mut ctx,
        &registry,
        page,
        formulas,
        "shape.set.width",
        args(&[("width", 6.0)]),
    );
    assert_eq!(number_of(&after, formulas, "Width"), Some(6.0));
    assert_eq!(
        number_of(&after, formulas, "PinX"),
        Some(3.0),
        "PinX recomputed from its formula"
    );
    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    let back = document(&ctx);
    assert_eq!(number_of(&back, formulas, "Width"), Some(2.0));
    assert_eq!(number_of(&back, formulas, "PinX"), Some(9.0));
}

/// A `SETF`-carrying cell: resizing its shape evaluates `Sync`, which
/// writes `PinX` as a literal (ADR 0056), and undo puts back the
/// target's formula **and** value by snapshot.
#[test]
fn a_setf_write_is_undone_with_the_command_that_triggered_it() {
    let (mut ctx, registry, page, _formulas, setf) = fixture();
    select(&mut ctx, page, setf);
    let after = round_trip(
        &mut ctx,
        &registry,
        page,
        setf,
        "shape.resize",
        args(&[("width", 4.0), ("height", 1.0)]),
    );
    assert_eq!(
        number_of(&after, setf, "PinX"),
        Some(12.0),
        "the SETF write landed at the new Width"
    );
    assert_eq!(
        formula_of(&after, setf, "PinX").as_deref(),
        Some("12"),
        "the write replaces the target's formula with the literal"
    );
    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    let back = document(&ctx);
    assert_eq!(
        formula_of(&back, setf, "PinX").as_deref(),
        Some("1"),
        "the target's formula comes back by snapshot"
    );
    assert_eq!(number_of(&back, setf, "PinX"), Some(1.0));
    assert_eq!(
        number_of(&back, setf, "Width"),
        Some(2.0),
        "the command's own write is part of the same step"
    );
}

// ---------------------------------------------------------------------------
// The property over random sequences
// ---------------------------------------------------------------------------

use proptest::prelude::*;

/// One command the property drives: the command id and its arguments,
/// chosen from `pick`.
#[derive(Debug, Clone)]
struct Op {
    id: &'static str,
    shape: u8,
    args: Vec<(&'static str, f64)>,
}

/// The operations the fixture supports: every recalc-affecting command
/// over both shapes (neither shape is ever deleted, so every run
/// applies).
fn op_strategy() -> impl Strategy<Value = Op> {
    let ids: Vec<&'static str> = vec![
        "shape.resize",
        "shape.move",
        "shape.set.width",
        "shape.set.height",
        "shape.set.pin",
        "shape.set.angle",
    ];
    (
        any::<u8>(),
        -5.0..5.0,
        -5.0..5.0,
        proptest::sample::select(ids),
    )
        .prop_map(|(shape, a, b, id): (u8, f64, f64, &'static str)| {
            let args = match id {
                "shape.resize" => vec![("width", 1.0 + a.abs()), ("height", 1.0 + b.abs())],
                "shape.move" => vec![("dx", a), ("dy", b)],
                "shape.set.width" => vec![("width", 1.0 + a.abs())],
                "shape.set.height" => vec![("height", 1.0 + b.abs())],
                "shape.set.pin" => vec![("x", a), ("y", b)],
                _ => vec![("angle", a)],
            };
            Op {
                id,
                shape: shape % 2,
                args,
            }
        })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    /// Any sequence of the recalc-affecting commands: every step undoes
    /// and redoes to the same documents (the ADR 0040 property over
    /// recalculated values), and rewinding the whole sequence restores
    /// the fixture exactly.
    #[test]
    fn random_sequences_undo_to_the_same_documents(ops in proptest::collection::vec(op_strategy(), 1..8)) {
        let (mut ctx, registry, page, formulas, setf) = fixture();
        let start = document(&ctx);
        let mut applied = 0;
        for op in &ops {
            let shape = if op.shape == 0 { formulas } else { setf };
            select(&mut ctx, page, shape);
            let before = document(&ctx);
            if dispatch(&mut ctx, &registry, op.id, args(&op.args)).is_err() {
                continue;
            }
            applied += 1;
            let after = document(&ctx);
            dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
            prop_assert_eq!(document(&ctx), before, "{}: undo", op.id);
            dispatch(&mut ctx, &registry, "edit.redo", Args::new()).expect("redo");
            prop_assert_eq!(document(&ctx), after, "{}: redo", op.id);
        }
        for _ in 0..applied {
            dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
        }
        prop_assert_eq!(document(&ctx), start, "the whole sequence rewinds");
    }
}
