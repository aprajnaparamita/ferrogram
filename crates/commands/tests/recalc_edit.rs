//! Edits with recalculation and undo (task P6-07, ADR 0056).
//!
//! The acceptance criteria, one test each:
//!
//! - one user action is one undo step including every recalculated
//!   cell, and undo restores the previous values by snapshot (an
//!   unknown function's cached value included);
//! - the ADR 0040 undo property holds for a resize with formulas (run,
//!   undo, compare; run, undo, redo, compare);
//! - resizing a master instance recomputes geometry through `recalc`
//!   and never through `Document::scale_geometry` (the rows keep their
//!   formulas);
//! - a `GUARD`-ed cell survives an indirect write with one `Guarded`
//!   warning and its formula unchanged (a direct edit replaces it: the
//!   `CellEdit::direct` unit test in `builtins/edit.rs`);
//! - a failing edit changes nothing and recalculates nothing (ADR 0039);
//! - connector re-routing skips a connector whose geometry rows carry
//!   formulas (ADR 0056's rule);
//! - constant geometry scales with the box while formula geometry
//!   recomputes (the writer split of ADR 0056; `dirty_cells` per edit
//!   type is pinned by the unit tests next to each edit type).

// The never-crash lints apply to non-test code only (clippy.toml).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod support;

use support::dispatch;

use larrygram_commands::app::{AppCtx, DocumentSession, Settings};
use larrygram_commands::builtins::registry;
use larrygram_commands::{ArgValue, Args, Registry, SessionId};
use larrygram_model::mutate::VIS_END;
use larrygram_model::{
    Cell, CellName, CellValue, Connect, Document, Master, MasterId, PageId, Preserved, Row,
    Section, SectionName, Shape, ShapeId, SheetRef,
};
use larrygram_shapesheet::CellAddress;

fn number_cell(name: &str, formula: &str, value: f64) -> Cell {
    Cell {
        name: CellName::new(name),
        formula: Some(formula.to_owned()),
        value: Some(CellValue::Number(value)),
        unit: larrygram_geom::Unit::Inches,
        preserved: Preserved::new(),
    }
}

fn cell_number(doc: &Document, shape: ShapeId, cell: &str) -> Option<f64> {
    doc.shape(shape)?
        .sheet
        .cells
        .iter()
        .find(|entry| entry.name.as_str() == cell)
        .and_then(|entry| match entry.value {
            Some(CellValue::Number(value)) => Some(value),
            _ => None,
        })
}

fn cell_formula(doc: &Document, shape: ShapeId, cell: &str) -> Option<String> {
    doc.shape(shape)?
        .sheet
        .cells
        .iter()
        .find(|entry| entry.name.as_str() == cell)
        .and_then(|entry| entry.formula.clone())
}

fn document(ctx: &AppCtx) -> Document {
    ctx.active_document().expect("document").document.clone()
}

fn select(ctx: &mut AppCtx, page: PageId, shapes: &[ShapeId]) {
    ctx.selection.shapes = shapes.to_vec();
    ctx.selection.anchor = shapes.last().copied();
    ctx.selection.page = Some(page);
}

fn args(pairs: &[(&str, f64)]) -> Args {
    let mut args = Args::new();
    for (name, value) in pairs {
        args.set(*name, ArgValue::Number(*value));
    }
    args
}

/// A document with one page and one 2 x 1 shape whose cells all carry
/// formulas: `PinX = Width*0.5` with a deliberately stale cached value,
/// `PinY = FOO(Width)` (an unknown function keeping its cached value)
/// and `Angle = GUARD(0)`. The geometry rows compute from `Width` and
/// `Height`.
fn fixture() -> (AppCtx, Registry, SessionId, PageId, ShapeId) {
    let mut doc = Document::new();
    let page = doc.add_page("Page-1").expect("page");
    let mut shape = Shape::new(ShapeId { index: 0 }, 1);
    shape.sheet.cells = vec![
        number_cell("Width", "2", 2.0),
        number_cell("Height", "1", 1.0),
        number_cell("PinX", "Width*0.5", 9.0),
        number_cell("PinY", "FOO(Width)", 42.0),
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
    shape.sheet.sections.push(geometry);
    let shape = doc.insert_shape(page, shape, None).expect("insert");
    let mut ctx = AppCtx::new(Settings::default());
    let index = ctx.open_document(DocumentSession::new(doc, None));
    let session = ctx.documents.get(index).expect("session").id;
    let registry = registry().expect("registry");
    (ctx, registry, session, page, shape)
}

/// Criterion 1: one user action is one undo step including every
/// recalculated cell, and undo restores the previous values by
/// snapshot — the recomputed `PinX` (stale 9.0) and the unknown
/// function's cached `PinY` (42.0) both come back unchanged.
#[test]
fn one_user_action_is_one_undo_step_including_the_recalculated_cells() {
    let (mut ctx, registry, _session, page, shape) = fixture();
    select(&mut ctx, page, &[shape]);
    let before = document(&ctx);

    dispatch(
        &mut ctx,
        &registry,
        "shape.set.width",
        args(&[("width", 4.0)]),
    )
    .expect("set width");
    let after = document(&ctx);
    assert_eq!(cell_number(&after, shape, "Width"), Some(4.0));
    assert_eq!(
        cell_number(&after, shape, "PinX"),
        Some(2.0),
        "the pass recomputed PinX from its formula"
    );
    assert_eq!(
        cell_number(&after, shape, "PinY"),
        Some(42.0),
        "an unknown function keeps its cached value"
    );
    let session = ctx.active_document().expect("document");
    assert_eq!(session.undo.len(), 1, "one user action, one undo step");
    assert_eq!(session.undo.undo_labels(), ["Set Width"]);
    assert!(
        ctx.warnings
            .iter()
            .any(|line| line.contains("unknown function FOO")),
        "{:?}",
        ctx.warnings
    );

    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    let restored = document(&ctx);
    assert_eq!(restored, before, "undo restores the document exactly");
    assert_eq!(
        cell_number(&restored, shape, "PinX"),
        Some(9.0),
        "the pre-edit (stale) value comes back unchanged"
    );
    assert_eq!(cell_number(&restored, shape, "PinY"), Some(42.0));
}

/// Criterion 2: the ADR 0040 undo property holds for a resize with
/// formulas (run, undo, compare documents; run, undo, redo, compare).
#[test]
fn the_undo_property_holds_for_a_resize_with_formulas() {
    let (mut ctx, registry, _session, page, shape) = fixture();
    select(&mut ctx, page, &[shape]);
    let before = document(&ctx);

    dispatch(
        &mut ctx,
        &registry,
        "shape.resize",
        args(&[("width", 4.0), ("height", 3.0)]),
    )
    .expect("resize");
    let after = document(&ctx);
    assert_ne!(before, after);

    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(document(&ctx), before, "run, undo: the document comes back");

    dispatch(&mut ctx, &registry, "edit.redo", Args::new()).expect("redo");
    assert_eq!(document(&ctx), after, "run, undo, redo: the same result");
    assert_eq!(
        cell_number(&document(&ctx), shape, "PinX"),
        Some(2.0),
        "the recomputation is part of the step"
    );
}

/// Criterion 3: resizing a master instance recomputes geometry through
/// `recalc` and never through `Document::scale_geometry` — the rows keep
/// the master's formulas (the model function would have materialised
/// scaled literals).
#[test]
fn resize_recomputes_a_master_instance_geometry_through_recalc() {
    let mut doc = Document::new();
    let page = doc.add_page("Page-1").expect("page");

    // The master: one shape with formula geometry.
    let mut master_shape = Shape::new(ShapeId { index: 0 }, 1);
    master_shape.sheet.cells = vec![
        number_cell("Width", "2", 2.0),
        number_cell("Height", "1", 1.0),
    ];
    let mut geometry = Section::new(SectionName::from_static("Geometry"));
    let mut row = Row::new();
    row.row_type = Some("LineTo".to_owned());
    row.ix = Some(0);
    row.cells = vec![
        number_cell("X", "Width*0.5", 1.0),
        number_cell("Y", "Height", 1.0),
    ];
    geometry.rows.push(row);
    master_shape.sheet.sections.push(geometry);
    let master_shape = doc.add_shape(master_shape);
    let mut master = Master::new(MasterId { index: 0 }, 1);
    master.name = "Formula Box".to_owned();
    master.name_u = "FormulaBox".to_owned();
    master.shapes = vec![master_shape];
    let master = doc.add_master(master).expect("master");

    // The instance: its own transform cells and `Inh` geometry rows
    // with the master-sized snapshot values (the corpus shape).
    let mut instance = Shape::new(ShapeId { index: 0 }, 2);
    instance.master = Some(master);
    instance.sheet.cells = vec![
        number_cell("Width", "2", 2.0),
        number_cell("Height", "1", 1.0),
        number_cell("PinX", "1", 1.0),
        number_cell("PinY", "1", 1.0),
    ];
    let mut geometry = Section::new(SectionName::from_static("Geometry"));
    let mut row = Row::new();
    row.row_type = Some("LineTo".to_owned());
    row.ix = Some(0);
    row.cells = vec![number_cell("X", "Inh", 7.7), number_cell("Y", "Inh", 8.8)];
    geometry.rows.push(row);
    instance.sheet.sections.push(geometry);
    let instance = doc.insert_shape(page, instance, None).expect("instance");

    let mut ctx = AppCtx::new(Settings::default());
    let index = ctx.open_document(DocumentSession::new(doc, None));
    ctx.documents.get_mut(index).expect("session").active_page = Some(page);
    let registry = registry().expect("registry");
    select(&mut ctx, page, &[instance]);
    let before = document(&ctx);

    dispatch(
        &mut ctx,
        &registry,
        "shape.resize",
        args(&[("width", 4.0), ("height", 3.0)]),
    )
    .expect("resize");
    let after = document(&ctx);
    let rows = &after
        .shape(instance)
        .expect("instance")
        .sheet
        .sections
        .first()
        .expect("geometry")
        .rows;
    let x = rows[0]
        .cells
        .iter()
        .find(|cell| cell.name.as_str() == "X")
        .expect("X");
    let y = rows[0]
        .cells
        .iter()
        .find(|cell| cell.name.as_str() == "Y")
        .expect("Y");
    assert_eq!(
        x.formula.as_deref(),
        Some("Width*0.5"),
        "the row keeps its formula: scale_geometry would have written a scaled literal"
    );
    assert_eq!(x.value, Some(CellValue::Number(2.0)), "computed at 4 in");
    assert_eq!(y.formula.as_deref(), Some("Height"));
    assert_eq!(y.value, Some(CellValue::Number(3.0)), "computed at 3 in");

    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(document(&ctx), before);
}

/// Criterion 4: a `GUARD`-ed cell survives an indirect write (here
/// `shape.set.angle`, a programmatic write) with one `Guarded` warning
/// and its formula unchanged. The direct-edit half of the criterion is
/// the `CellEdit::direct` unit test in `builtins/edit.rs`.
#[test]
fn a_guarded_cell_survives_an_indirect_write_with_one_warning() {
    let (mut ctx, registry, _session, page, shape) = fixture();
    select(&mut ctx, page, &[shape]);
    let before = document(&ctx);

    dispatch(
        &mut ctx,
        &registry,
        "shape.set.angle",
        args(&[("angle", 1.25)]),
    )
    .expect("set angle");
    let after = document(&ctx);
    assert_eq!(
        cell_formula(&after, shape, "Angle").as_deref(),
        Some("GUARD(0)"),
        "the formula is unchanged"
    );
    assert_eq!(cell_number(&after, shape, "Angle"), Some(0.0));
    let guarded: Vec<&String> = ctx
        .warnings
        .iter()
        .filter(|line| line.contains("Guarded"))
        .collect();
    assert_eq!(guarded.len(), 1, "{:?}", ctx.warnings);
    assert_eq!(
        ctx.active_document().expect("session").undo.len(),
        1,
        "the skipped step still records its undo entry"
    );

    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(document(&ctx), before);
}

/// Criterion 5 (ADR 0039): a failing edit changes nothing and
/// recalculates nothing. `insert.shape` with a glue whose target has no
/// connection points fails after the drop and rolls it back; the stale
/// dependent `PinX` of the fixture is never recomputed. (The strict
/// document-equality half of the criterion is the failing-`CellEdit`
/// unit test in `builtins/edit.rs`.)
#[test]
fn a_failing_edit_changes_nothing_and_recalculates_nothing() {
    let (mut ctx, registry, _session, page, _shape) = fixture();
    let target = {
        let session = ctx.active_document().expect("session");
        let doc = &session.document;
        doc.pages
            .get(page.index as usize)
            .and_then(|page| page.shapes.first().copied())
            .expect("fixture shape")
    };
    let target_visio = ctx
        .active_document()
        .expect("session")
        .document
        .shape(target)
        .expect("shape")
        .visio_id;
    let before = document(&ctx);
    let warnings = ctx.warnings.len();

    let mut args = Args::new();
    args.set("stencil", ArgValue::Text("Basic Shapes".to_owned()));
    args.set("master", ArgValue::Text("Rectangle".to_owned()));
    args.set("x", ArgValue::Number(4.0));
    args.set("y", ArgValue::Number(3.0));
    args.set("glue_to", ArgValue::Integer(i64::from(target_visio)));
    let error = dispatch(&mut ctx, &registry, "insert.shape", args)
        .expect_err("the glue target has no connection points");
    assert!(error.to_string().contains("connection points"), "{error}");

    // The drop is rolled back: the page holds the fixture shape only
    // and its sheet is untouched. (The materialised catalog master and
    // the detached instance stay in the arena — the P5-09 rule that a
    // comparison is over reachable state, not `Document` equality.)
    let after = document(&ctx);
    assert_eq!(
        after
            .pages
            .get(page.index as usize)
            .map(|entry| entry.shapes.clone()),
        Some(vec![target]),
        "a failing edit leaves no shape behind"
    );
    assert_eq!(
        after.shape(target).map(|shape| &shape.sheet),
        before.shape(target).map(|shape| &shape.sheet),
        "a failing edit changes nothing"
    );
    assert!(
        ctx.active_document().expect("session").undo.is_empty(),
        "no undo step"
    );
    assert_eq!(
        ctx.warnings.len(),
        warnings,
        "nothing was recalculated (no recalc warning)"
    );
    assert_eq!(
        cell_number(&document(&ctx), target, "PinX"),
        Some(9.0),
        "the stale dependent is untouched"
    );
}

/// Criterion 7 (ADR 0056's connector rule): re-routing skips a
/// connector whose geometry rows carry formulas; a connector with
/// constant rows keeps being routed.
#[test]
fn reroute_skips_a_connector_whose_geometry_rows_carry_formulas() {
    let mut doc = Document::new();
    let page = doc.add_page("Page-1").expect("page");
    let mut target = Shape::new(ShapeId { index: 0 }, 1);
    target.sheet.cells = vec![
        number_cell("Width", "2", 2.0),
        number_cell("Height", "1", 1.0),
        number_cell("PinX", "2", 2.0),
        number_cell("PinY", "2", 2.0),
    ];
    let mut connection = Section::new(SectionName::from_static("Connection"));
    let mut row = Row::new();
    row.row_type = Some("Connection".to_owned());
    row.ix = Some(0);
    row.cells = vec![number_cell("X", "1", 1.0), number_cell("Y", "0.5", 0.5)];
    connection.rows.push(row);
    target.sheet.sections.push(connection);
    let target = doc.insert_shape(page, target, None).expect("target");
    let target_visio = doc.shape(target).expect("shape").visio_id;

    // Two connectors glued to the target: one with formula geometry
    // (recalc owns it), one with constant rows (the router owns it).
    let mut formula_connector = Shape::new(ShapeId { index: 0 }, 2);
    formula_connector.sheet.cells = vec![number_cell("Width", "2", 2.0)];
    let mut geometry = Section::new(SectionName::from_static("Geometry"));
    let mut row = Row::new();
    row.row_type = Some("LineTo".to_owned());
    row.ix = Some(0);
    row.cells = vec![number_cell("X", "Width", 2.0), number_cell("Y", "0", 0.0)];
    geometry.rows.push(row);
    formula_connector.sheet.sections.push(geometry);
    let formula_connector = doc
        .insert_shape(page, formula_connector, None)
        .expect("connector");

    let mut constant_connector = Shape::new(ShapeId { index: 0 }, 3);
    constant_connector.sheet.cells = vec![number_cell("Width", "2", 2.0)];
    let mut geometry = Section::new(SectionName::from_static("Geometry"));
    let mut row = Row::new();
    row.row_type = Some("LineTo".to_owned());
    row.ix = Some(0);
    row.cells = vec![number_cell("X", "2", 2.0), number_cell("Y", "0", 0.0)];
    geometry.rows.push(row);
    constant_connector.sheet.sections.push(geometry);
    let constant_connector = doc
        .insert_shape(page, constant_connector, None)
        .expect("connector");

    for connector in [formula_connector, constant_connector] {
        let mut connect = Connect::new();
        connect.from_sheet = doc.shape(connector).expect("shape").visio_id;
        connect.from_cell = String::from("EndX");
        connect.from_part = VIS_END;
        connect.to_sheet = target_visio;
        connect.to_cell = String::from("Connections.X1");
        connect.to_part = larrygram_model::mutate::VIS_CONNECTION_POINT + 1;
        doc.add_connect(page, connect).expect("connect");
    }

    let mut ctx = AppCtx::new(Settings::default());
    let index = ctx.open_document(DocumentSession::new(doc, None));
    ctx.documents.get_mut(index).expect("session").active_page = Some(page);
    let registry = registry().expect("registry");
    let rows_of = |ctx: &AppCtx, shape: ShapeId| -> Vec<(Option<String>, Option<CellValue>)> {
        let doc = &ctx.active_document().expect("session").document;
        let section = &doc
            .shape(shape)
            .expect("shape")
            .sheet
            .sections
            .first()
            .expect("geometry");
        section
            .rows
            .iter()
            .flat_map(|row| row.cells.iter())
            .map(|cell| (cell.formula.clone(), cell.value.clone()))
            .collect()
    };
    let formula_rows = rows_of(&ctx, formula_connector);
    let constant_rows = rows_of(&ctx, constant_connector);

    select(&mut ctx, page, &[target]);
    dispatch(
        &mut ctx,
        &registry,
        "shape.move",
        args(&[("dx", 1.0), ("dy", 1.0)]),
    )
    .expect("move");

    assert_eq!(
        rows_of(&ctx, formula_connector),
        formula_rows,
        "a formula-driven connector is recalc's, not the router's"
    );
    assert_ne!(
        rows_of(&ctx, constant_connector),
        constant_rows,
        "the router still writes the constant-geometry connector"
    );
}

/// The ADR 0056 writer split in one resize: the constant rows scale
/// with the box, the formula rows recompute (and keep their formulas).
#[test]
fn a_resize_scales_constant_rows_and_recomputes_formula_rows() {
    let (mut ctx, registry, _session, page, shape) = fixture();
    {
        let doc = &mut ctx.active_document_mut().expect("session").document;
        let section = doc
            .shape_mut(shape)
            .expect("shape")
            .sheet
            .sections
            .first_mut()
            .expect("geometry");
        // Row 0 keeps its constant point, rows 1 and 2 their formulas.
        if let Some(row) = section.rows.get_mut(0) {
            for cell in &mut row.cells {
                cell.formula = Some(match cell.name.as_str() {
                    "X" => String::from("2"),
                    _ => String::from("1"),
                });
                cell.value = Some(CellValue::Number(match cell.name.as_str() {
                    "X" => 2.0,
                    _ => 1.0,
                }));
            }
        }
    }
    select(&mut ctx, page, &[shape]);
    dispatch(
        &mut ctx,
        &registry,
        "shape.resize",
        args(&[("width", 4.0), ("height", 3.0)]),
    )
    .expect("resize");
    let after = document(&ctx);
    let rows = &after
        .shape(shape)
        .expect("shape")
        .sheet
        .sections
        .first()
        .expect("geometry")
        .rows;
    let cell = |row: usize, name: &str| -> (Option<String>, Option<CellValue>) {
        let entry = rows[row]
            .cells
            .iter()
            .find(|cell| cell.name.as_str() == name)
            .expect("cell");
        (entry.formula.clone(), entry.value.clone())
    };
    assert_eq!(
        cell(0, "X"),
        (Some(String::from("4")), Some(CellValue::Number(4.0))),
        "a constant row scales with the box"
    );
    assert_eq!(
        cell(1, "X"),
        (Some(String::from("Width")), Some(CellValue::Number(4.0))),
        "a formula row recomputes and keeps its formula"
    );
    assert_eq!(
        cell(2, "Y"),
        (Some(String::from("Height")), Some(CellValue::Number(3.0)))
    );
}

/// The dirty set a recalc pass runs over is exposed on the step
/// ([`larrygram_commands::Edit::dirty_cells`]); the per-edit-type tests
/// live next to each type. This pins the composite aggregation the
/// multi-cell commands rely on.
#[test]
fn a_composite_edit_reports_every_step_cell() {
    use larrygram_commands::{CompositeEdit, Edit};

    let mut edit = CompositeEdit::new("two steps");
    assert!(edit.dirty_cells().is_empty());
    edit.push(Box::new(DirtyEdit(vec![CellAddress::new(
        SheetRef::Page(PageId { index: 0 }),
        CellName::new("A"),
    )])));
    edit.push(Box::new(DirtyEdit(vec![CellAddress::new(
        SheetRef::Page(PageId { index: 0 }),
        CellName::new("B"),
    )])));
    let names: Vec<&str> = edit
        .dirty_cells()
        .iter()
        .map(|address| address.cell.as_str())
        .collect();
    assert_eq!(names, ["A", "B"]);
}

/// One step reporting a fixed dirty set (the composite test above).
struct DirtyEdit(Vec<CellAddress>);

impl larrygram_commands::Edit for DirtyEdit {
    fn label(&self) -> &str {
        "dirty"
    }

    fn apply(&mut self, _ctx: &mut AppCtx) -> Result<(), larrygram_commands::CommandError> {
        Ok(())
    }

    fn revert(&mut self, _ctx: &mut AppCtx) -> Result<(), larrygram_commands::CommandError> {
        Ok(())
    }

    fn dirty_cells(&self) -> &[CellAddress] {
        &self.0
    }
}

/// [`larrygram_commands::recalc`] is the fill-in entry point task P6-08
/// consumes: a document with a formula and no cached value gains one,
/// with no undo entry.
#[test]
fn fill_missing_fills_the_holes_without_an_undo_entry() {
    let (mut ctx, _registry, session, _page, shape) = fixture();
    {
        let doc = &mut ctx.active_document_mut().expect("session").document;
        if let Some(entry) = doc.shape_mut(shape).and_then(|shape| {
            shape
                .sheet
                .cells
                .iter_mut()
                .find(|c| c.name.as_str() == "PinX")
        }) {
            entry.value = None;
        }
    }
    let report = larrygram_commands::recalc::fill_missing(&mut ctx, session).expect("fill");
    assert_eq!(report.updated, 1);
    assert_eq!(
        cell_number(&document(&ctx), shape, "PinX"),
        Some(1.0),
        "the hole is filled"
    );
    assert!(
        ctx.active_document().expect("session").undo.is_empty(),
        "the fill-in pass is not an edit"
    );
}

/// The one-warning spelling of a pass that cannot compute (the report
/// reaches the status bar and the report struct).
#[test]
fn recalc_after_edit_reports_the_warnings_it_could_not_compute() {
    let (mut ctx, _registry, session, page, shape) = fixture();
    let dirty = vec![CellAddress::new(
        SheetRef::Shape(shape),
        CellName::new("Width"),
    )];
    let report =
        larrygram_commands::recalc::recalc_after_edit(&mut ctx, session, &dirty).expect("recalc");
    assert!(!report.truncated);
    assert!(report.warnings.iter().any(|warning| {
        matches!(
            warning.kind,
            larrygram_shapesheet::RecalcWarningKind::UnknownFunction(_)
        )
    }));
    let _ = page;
}
