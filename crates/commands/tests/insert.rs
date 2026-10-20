//! Headless `insert.*` command tests (task P5-06, ADR 0049 rule 1).
//!
//! Every command runs through `Registry::execute` with `Source::Test`:
//! a master drop places one instance at the dropped pin with the master
//! size, a `Shift`-sized drop uses the drawn size, glue on drop writes one
//! `Connect` record, and each drawing command creates one shape with one
//! undo step. Missing or stale stencils are command errors, never a
//! crash.

// The never-crash lints apply to non-test code only (clippy.toml).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod support;

use support::dispatch;

use ferrogram_commands::app::{AppCtx, DocumentSession, Settings};
use ferrogram_commands::builtins::registry;
use ferrogram_commands::{
    ArgValue, Args, CommandError, CommandId, CommandStatus, Registry, SessionId,
};
use ferrogram_model::mutate::VIS_CONNECTION_POINT;
use ferrogram_model::stencils;
use ferrogram_model::{
    Cell, CellName, CellValue, Document, PageId, Preserved, Row, Section, SectionName, Shape,
    ShapeId,
};

/// The Phase 5 insert ids (task P5-06).
const IDS: &[&str] = &[
    "insert.shape",
    "insert.drawing.rectangle",
    "insert.drawing.ellipse",
    "insert.drawing.line",
    "insert.drawing.text_box",
];

/// The test that covers each id (ADR 0049 rule 1).
const COVERED: &[(&str, &str)] = &[
    ("insert.shape", "shape_drop_places_a_master_instance"),
    (
        "insert.drawing.rectangle",
        "each_drawing_command_creates_one_shape",
    ),
    (
        "insert.drawing.ellipse",
        "each_drawing_command_creates_one_shape",
    ),
    (
        "insert.drawing.line",
        "each_drawing_command_creates_one_shape",
    ),
    (
        "insert.drawing.text_box",
        "each_drawing_command_creates_one_shape",
    ),
];

fn number_cell(name: &str, value: f64) -> Cell {
    Cell {
        name: CellName::new(name),
        formula: Some(format!("{value}")),
        value: Some(CellValue::Number(value)),
        unit: ferrogram_geom::Unit::Inches,
        preserved: Preserved::new(),
    }
}

/// A 2 x 1 target shape at (2, 2) with one connection point at its pin.
fn target_shape() -> Shape {
    let mut shape = Shape::new(ShapeId { index: 0 }, 1);
    shape.name = "Target".to_owned();
    shape.name_u = "Target".to_owned();
    shape.sheet.cells = vec![
        number_cell("Width", 2.0),
        number_cell("Height", 1.0),
        number_cell("PinX", 2.0),
        number_cell("PinY", 2.0),
        number_cell("LocPinX", 1.0),
        number_cell("LocPinY", 0.5),
        number_cell("Angle", 0.0),
    ];
    let mut geometry = Section::new(SectionName::from_static("Geometry"));
    for (ix, (kind, x, y)) in [
        ("MoveTo", 0.0, 0.0),
        ("LineTo", 2.0, 0.0),
        ("LineTo", 2.0, 1.0),
        ("LineTo", 0.0, 1.0),
        ("LineTo", 0.0, 0.0),
    ]
    .into_iter()
    .enumerate()
    {
        let mut row = Row::new();
        row.row_type = Some(kind.to_owned());
        row.ix = Some(i32::try_from(ix).unwrap_or(0));
        row.cells = vec![number_cell("X", x), number_cell("Y", y)];
        geometry.rows.push(row);
    }
    shape.sheet.sections.push(geometry);
    let mut connection = Section::new(SectionName::from_static("Connection"));
    let mut row = Row::new();
    row.ix = Some(0);
    row.cells = vec![number_cell("X", 1.0), number_cell("Y", 0.5)];
    connection.rows.push(row);
    shape.sheet.sections.push(connection);
    shape
}

/// One page with the target shape.
fn fixture() -> (AppCtx, Registry, SessionId, PageId, ShapeId) {
    let mut doc = Document::new();
    let page = doc.add_page("Page-1").expect("page");
    let target = doc
        .insert_shape(page, target_shape(), None)
        .expect("target");
    let mut ctx = AppCtx::new(Settings::default());
    let index = ctx.open_document(DocumentSession::new(doc, None));
    let session = ctx.documents.get(index).expect("session").id;
    let registry = registry().expect("registry");
    (ctx, registry, session, page, target)
}

/// Dispatches one command and pushes the edit like the shell does.
/// One drop invocation.
fn drop_args(master: &str) -> Args {
    Args::new()
        .with("stencil", ArgValue::Text("Basic Shapes".to_owned()))
        .with("master", ArgValue::Text(master.to_owned()))
        .with("x", ArgValue::Number(4.0))
        .with("y", ArgValue::Number(3.0))
}

fn drawing_args(x: f64, y: f64, width: f64, height: f64) -> Args {
    Args::new()
        .with("x", ArgValue::Number(x))
        .with("y", ArgValue::Number(y))
        .with("width", ArgValue::Number(width))
        .with("height", ArgValue::Number(height))
}

fn page_shapes(ctx: &AppCtx, page: PageId) -> Vec<ShapeId> {
    ctx.active_document()
        .expect("document")
        .document
        .page(page)
        .expect("page")
        .shapes
        .clone()
}

fn direct_number(ctx: &AppCtx, shape: ShapeId, name: &str) -> Option<f64> {
    ctx.active_document()?
        .document
        .shape(shape)?
        .sheet
        .cells
        .iter()
        .find(|cell| cell.name.as_str() == name)
        .and_then(|cell| match cell.value {
            Some(CellValue::Number(value)) => Some(value),
            _ => None,
        })
}

#[test]
fn every_insert_command_is_registered_and_tested() {
    let registry = registry().expect("registry");
    for id in IDS {
        let command = CommandId::parse(id).unwrap_or_else(|error| panic!("{id}: {error}"));
        assert_eq!(
            registry.status(&command),
            CommandStatus::Implemented,
            "{id} must be implemented"
        );
        assert!(
            COVERED.iter().any(|(covered, _)| covered == id),
            "{id} has no test entry"
        );
    }
}

#[test]
fn shape_drop_places_a_master_instance() {
    let (mut ctx, registry, _session, page, target) = fixture();
    let before = page_shapes(&ctx, page).len();
    dispatch(&mut ctx, &registry, "insert.shape", drop_args("Rectangle")).expect("drop");
    let shapes = page_shapes(&ctx, page);
    assert_eq!(shapes.len(), before + 1);
    let shape = *shapes.last().expect("new shape");
    assert_ne!(shape, target);
    assert_eq!(direct_number(&ctx, shape, "PinX"), Some(4.0));
    assert_eq!(direct_number(&ctx, shape, "PinY"), Some(3.0));
    assert_eq!(direct_number(&ctx, shape, "Width"), Some(1.5));
    assert_eq!(direct_number(&ctx, shape, "Height"), Some(1.0));
    assert!(
        ctx.active_document()
            .expect("document")
            .document
            .shape(shape)
            .expect("shape")
            .master
            .is_some(),
        "the instance keeps its master link"
    );
    assert_eq!(ctx.selection.shapes, vec![shape]);
    assert_eq!(ctx.selection.anchor, Some(shape));
    assert_eq!(
        ctx.active_document().expect("document").changes,
        1,
        "one insertion bumps the change counter"
    );

    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(page_shapes(&ctx, page), vec![target], "one undo removes it");
    dispatch(&mut ctx, &registry, "edit.redo", Args::new()).expect("redo");
    assert_eq!(page_shapes(&ctx, page).len(), before + 1);
}

#[test]
fn a_sized_drop_uses_the_drawn_size_and_scales_the_geometry() {
    let (mut ctx, registry, _session, page, _target) = fixture();
    let mut args = drop_args("Rectangle");
    args.set("width", ArgValue::Number(3.0));
    args.set("height", ArgValue::Number(2.0));
    dispatch(&mut ctx, &registry, "insert.shape", args).expect("drop");
    let shape = *page_shapes(&ctx, page).last().expect("shape");
    assert_eq!(direct_number(&ctx, shape, "Width"), Some(3.0));
    assert_eq!(direct_number(&ctx, shape, "Height"), Some(2.0));
    let geometry = ctx
        .active_document()
        .expect("document")
        .document
        .shape_geometry(shape)
        .expect("geometry");
    let max_x = geometry
        .paths
        .iter()
        .flat_map(|path| path.rows.iter())
        .filter_map(|row| match row {
            ferrogram_model::GeometryRow::MoveTo { x, .. }
            | ferrogram_model::GeometryRow::LineTo { x, .. } => Some(*x),
            _ => None,
        })
        .fold(0.0_f64, f64::max);
    assert!((max_x - 3.0).abs() < 1.0e-9, "{max_x}");
}

#[test]
fn a_drop_near_a_connection_point_glues_once() {
    let (mut ctx, registry, _session, page, target) = fixture();
    let target_visio = ctx
        .active_document()
        .expect("document")
        .document
        .shape(target)
        .expect("target")
        .visio_id;
    let mut args = drop_args("Rectangle");
    args.set("x", ArgValue::Number(2.02));
    args.set("y", ArgValue::Number(2.02));
    args.set("glue_to", ArgValue::Integer(i64::from(target_visio)));
    args.set("glue_point", ArgValue::Integer(0));
    dispatch(&mut ctx, &registry, "insert.shape", args).expect("drop");
    let connects = ctx
        .active_document()
        .expect("document")
        .document
        .page(page)
        .expect("page")
        .connects
        .clone();
    assert_eq!(connects.len(), 1, "one Connect record");
    let connect = &connects[0];
    assert_eq!(connect.to_sheet, target_visio);
    assert_eq!(connect.to_part, VIS_CONNECTION_POINT);
    assert_eq!(connect.to_cell, "Connections.X1");
    assert!(connect.from_part >= VIS_CONNECTION_POINT);
    assert!(connect.from_cell.starts_with("Connections.X"));

    // Undo removes the shape and its glue together.
    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(
        ctx.active_document()
            .expect("document")
            .document
            .page(page)
            .expect("page")
            .connects
            .len(),
        0
    );
}

#[test]
fn each_drawing_command_creates_one_shape() {
    let (mut ctx, registry, _session, page, target) = fixture();
    let cases = [
        (
            "insert.drawing.rectangle",
            drawing_args(1.0, 2.0, 2.0, 1.0),
            true,
        ),
        (
            "insert.drawing.ellipse",
            drawing_args(1.0, 2.0, 2.0, 1.0),
            true,
        ),
        (
            "insert.drawing.line",
            drawing_args(1.0, 2.0, 2.0, -1.0),
            false,
        ),
        (
            "insert.drawing.text_box",
            drawing_args(1.0, 2.0, 2.0, 1.0),
            false,
        ),
    ];
    for (id, args, filled) in cases {
        let before = page_shapes(&ctx, page).len();
        dispatch(&mut ctx, &registry, id, args).expect("draw");
        let shapes = page_shapes(&ctx, page);
        assert_eq!(shapes.len(), before + 1, "{id} creates one shape");
        let shape = *shapes.last().expect("shape");
        if id == "insert.drawing.line" {
            assert_eq!(direct_number(&ctx, shape, "PinX"), Some(1.0));
            assert_eq!(direct_number(&ctx, shape, "PinY"), Some(2.0));
            assert_eq!(direct_number(&ctx, shape, "Width"), Some(2.0));
            assert_eq!(direct_number(&ctx, shape, "Height"), Some(1.0));
        } else {
            assert_eq!(direct_number(&ctx, shape, "PinX"), Some(2.0));
            assert_eq!(direct_number(&ctx, shape, "PinY"), Some(2.5));
            assert_eq!(direct_number(&ctx, shape, "Width"), Some(2.0));
            assert_eq!(direct_number(&ctx, shape, "Height"), Some(1.0));
        }
        assert_eq!(
            direct_number(&ctx, shape, "FillPattern"),
            Some(if filled { 1.0 } else { 0.0 })
        );
        dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
        assert_eq!(page_shapes(&ctx, page).len(), before, "{id} undoes once");
        assert!(
            !ctx.active_document().expect("document").undo.can_undo(),
            "{id}: one drawing step, undone in one command"
        );
    }
    assert_eq!(page_shapes(&ctx, page), vec![target]);
}

#[test]
fn a_shift_sized_drop_is_one_undo_step() {
    let (mut ctx, registry, _session, _page, _target) = fixture();
    let mut args = drop_args("Process");
    args.set("stencil", ArgValue::Text("Basic Flowchart".to_owned()));
    args.set("width", ArgValue::Number(2.0));
    args.set("height", ArgValue::Number(2.0));
    dispatch(&mut ctx, &registry, "insert.shape", args).expect("drop");
    let session = ctx.active_document().expect("document");
    assert_eq!(session.undo.len(), 1);
    assert_eq!(session.undo.undo_labels(), ["Insert Process"]);
}

#[test]
fn unknown_stencils_and_masters_are_command_errors() {
    let (mut ctx, registry, _session, page, _target) = fixture();
    let before = page_shapes(&ctx, page).len();
    let mut args = drop_args("Rectangle");
    args.set("stencil", ArgValue::Text("No Such Stencil".to_owned()));
    let error = dispatch(&mut ctx, &registry, "insert.shape", args).expect_err("stencil");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));

    let args = drop_args("No Such Master");
    let error = dispatch(&mut ctx, &registry, "insert.shape", args).expect_err("master");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));
    assert_eq!(page_shapes(&ctx, page).len(), before, "no shape was left");
}

#[test]
fn missing_arguments_ask_for_input() {
    let (mut ctx, registry, _session, _page, _target) = fixture();
    let error = dispatch(
        &mut ctx,
        &registry,
        "insert.shape",
        Args::new().with("stencil", ArgValue::Text("Basic Shapes".to_owned())),
    )
    .expect_err("missing master");
    assert!(matches!(error, CommandError::NeedsInput(_)));
}

#[test]
fn a_stale_glue_target_is_an_error_and_leaves_no_shape() {
    let (mut ctx, registry, _session, page, _target) = fixture();
    let before = page_shapes(&ctx, page).len();
    let mut args = drop_args("Rectangle");
    args.set("glue_to", ArgValue::Integer(999));
    args.set("glue_point", ArgValue::Integer(0));
    let error = dispatch(&mut ctx, &registry, "insert.shape", args).expect_err("glue target");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));
    assert_eq!(
        page_shapes(&ctx, page).len(),
        before,
        "the shape was rolled back"
    );
}

/// Task P5-19 coverage (with the decision it pins): a drop of a
/// document master with no `Connection` rows cannot make the glue the
/// caller asked for, so the drop **fails** with the typed error and the
/// shape is rolled back. The alternative (warn and skip the glue) would
/// silently ignore the request; a foreign master with the catalog's
/// `NameU` is exactly the read-file case of the audit.
#[test]
fn a_master_without_connection_rows_cannot_glue_on_drop() {
    let (mut ctx, registry, _session, page, target) = fixture();
    let name_u = stencils::catalog()
        .into_iter()
        .find(|entry| entry.name == "Basic Shapes")
        .and_then(|entry| {
            entry
                .masters
                .into_iter()
                .find(|definition| definition.name == "Rectangle")
        })
        .map(|definition| definition.name_u)
        .expect("catalog master");
    {
        let doc = &mut ctx.active_document_mut().expect("document").document;
        let mut content = Shape::new(ShapeId { index: 0 }, 55);
        content.name_u = name_u.to_owned();
        content.sheet.cells = vec![
            number_cell("Width", 1.0),
            number_cell("Height", 1.0),
            number_cell("PinX", 0.0),
            number_cell("PinY", 0.0),
        ];
        let content = doc.add_shape(content);
        let mut master = ferrogram_model::Master::new(ferrogram_model::MasterId { index: 0 }, 0);
        master.name_u = name_u.to_owned();
        master.shapes = vec![content];
        doc.add_master(master).expect("master");
    }
    let before = page_shapes(&ctx, page).len();
    let target_visio = ctx
        .active_document()
        .expect("document")
        .document
        .shape(target)
        .expect("target")
        .visio_id;
    let mut args = drop_args("Rectangle");
    args.set("glue_to", ArgValue::Integer(i64::from(target_visio)));
    args.set("glue_point", ArgValue::Integer(0));
    let error = dispatch(&mut ctx, &registry, "insert.shape", args)
        .expect_err("no connection points to glue");
    assert!(
        error.to_string().contains("no connection points to glue"),
        "{error}"
    );
    assert_eq!(
        page_shapes(&ctx, page).len(),
        before,
        "the drop was rolled back"
    );
}

#[test]
fn an_out_of_range_glue_point_is_an_error_and_leaves_no_shape() {
    let (mut ctx, registry, _session, page, target) = fixture();
    let before = page_shapes(&ctx, page).len();
    let target_visio = ctx
        .active_document()
        .expect("document")
        .document
        .shape(target)
        .expect("target")
        .visio_id;
    let mut args = drop_args("Rectangle");
    args.set("glue_to", ArgValue::Integer(i64::from(target_visio)));
    args.set("glue_point", ArgValue::Integer(7));
    let error = dispatch(&mut ctx, &registry, "insert.shape", args).expect_err("glue point");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));
    assert_eq!(page_shapes(&ctx, page).len(), before);
}

#[test]
fn a_negative_size_and_a_zero_line_are_invalid() {
    let (mut ctx, registry, _session, page, _target) = fixture();
    let before = page_shapes(&ctx, page).len();
    let mut args = drop_args("Rectangle");
    args.set("width", ArgValue::Number(-1.0));
    args.set("height", ArgValue::Number(1.0));
    let error = dispatch(&mut ctx, &registry, "insert.shape", args).expect_err("negative");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));

    let error = dispatch(
        &mut ctx,
        &registry,
        "insert.drawing.line",
        drawing_args(1.0, 1.0, 0.0, 0.0),
    )
    .expect_err("zero line");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));
    assert_eq!(page_shapes(&ctx, page).len(), before);
}

#[test]
fn a_master_name_accepts_the_display_spelling_too() {
    let (mut ctx, registry, _session, page, _target) = fixture();
    let mut args = drop_args("Rectangle");
    args.set("master", ArgValue::Text("Rectangle".to_owned()));
    dispatch(&mut ctx, &registry, "insert.shape", args).expect("drop");
    assert_eq!(page_shapes(&ctx, page).len(), 2);
    let _ = stencils::catalog();
}

#[test]
fn a_second_drop_reuses_the_materialised_master() {
    let (mut ctx, registry, _session, page, _target) = fixture();
    dispatch(&mut ctx, &registry, "insert.shape", drop_args("Rectangle")).expect("first");
    let masters = ctx
        .active_document()
        .expect("document")
        .document
        .masters
        .len();
    dispatch(&mut ctx, &registry, "insert.shape", drop_args("Rectangle")).expect("second");
    assert_eq!(
        ctx.active_document()
            .expect("document")
            .document
            .masters
            .len(),
        masters,
        "the catalog master is materialised once"
    );
    assert_eq!(page_shapes(&ctx, page).len(), 3);
}

#[test]
fn a_drop_without_an_explicit_glue_point_chooses_the_nearest_pair() {
    let (mut ctx, registry, _session, page, target) = fixture();
    let target_visio = ctx
        .active_document()
        .expect("document")
        .document
        .shape(target)
        .expect("target")
        .visio_id;
    let mut args = drop_args("Rectangle");
    args.set("x", ArgValue::Number(2.02));
    args.set("y", ArgValue::Number(2.02));
    args.set("glue_to", ArgValue::Integer(i64::from(target_visio)));
    dispatch(&mut ctx, &registry, "insert.shape", args).expect("drop");
    let connects = ctx
        .active_document()
        .expect("document")
        .document
        .page(page)
        .expect("page")
        .connects
        .clone();
    assert_eq!(connects.len(), 1);
    assert!(connects[0].from_part >= VIS_CONNECTION_POINT);
    assert_eq!(connects[0].to_part, VIS_CONNECTION_POINT);
}

#[test]
fn a_glue_point_without_a_target_is_invalid() {
    let (mut ctx, registry, _session, page, _target) = fixture();
    let before = page_shapes(&ctx, page).len();
    let mut args = drop_args("Rectangle");
    args.set("glue_point", ArgValue::Integer(0));
    let error = dispatch(&mut ctx, &registry, "insert.shape", args).expect_err("no target");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));
    assert_eq!(page_shapes(&ctx, page).len(), before);
}

#[test]
fn a_glue_target_without_connection_points_is_an_error() {
    let (mut ctx, registry, _session, page, _target) = fixture();
    let plain = {
        let document = &mut ctx.active_document_mut().expect("document").document;
        let mut shape = Shape::new(ShapeId { index: 0 }, 0);
        shape.name = "Plain".to_owned();
        shape.name_u = "Plain".to_owned();
        shape.sheet.cells = vec![
            number_cell("Width", 1.0),
            number_cell("Height", 1.0),
            number_cell("PinX", 6.0),
            number_cell("PinY", 6.0),
            number_cell("LocPinX", 0.5),
            number_cell("LocPinY", 0.5),
        ];
        document.insert_shape(page, shape, None).expect("plain")
    };
    let plain_visio = ctx
        .active_document()
        .expect("document")
        .document
        .shape(plain)
        .expect("plain")
        .visio_id;
    let before = page_shapes(&ctx, page).len();
    let mut args = drop_args("Rectangle");
    args.set("glue_to", ArgValue::Integer(i64::from(plain_visio)));
    let error = dispatch(&mut ctx, &registry, "insert.shape", args).expect_err("no points");
    assert!(matches!(error, CommandError::Failed { .. }));
    assert_eq!(page_shapes(&ctx, page).len(), before);
}

#[test]
fn malformed_arguments_are_invalid_without_a_shape() {
    let (mut ctx, registry, _session, page, _target) = fixture();
    let before = page_shapes(&ctx, page).len();

    let mut args = drop_args("Rectangle");
    args.set("width", ArgValue::Number(1.0));
    let error = dispatch(&mut ctx, &registry, "insert.shape", args).expect_err("half size");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));

    let mut args = drop_args("Rectangle");
    args.set("stencil", ArgValue::Text(String::new()));
    let error = dispatch(&mut ctx, &registry, "insert.shape", args).expect_err("empty stencil");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));

    let mut args = drop_args("Rectangle");
    args.set("stencil", ArgValue::Integer(1));
    let error = dispatch(&mut ctx, &registry, "insert.shape", args).expect_err("stencil kind");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));

    let mut args = drop_args("Rectangle");
    args.set("glue_to", ArgValue::Text("one".to_owned()));
    let error = dispatch(&mut ctx, &registry, "insert.shape", args).expect_err("glue kind");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));

    let mut args = drop_args("Rectangle");
    args.set("glue_to", ArgValue::Integer(1));
    args.set("glue_point", ArgValue::Text("first".to_owned()));
    let error = dispatch(&mut ctx, &registry, "insert.shape", args).expect_err("point kind");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));

    let error = dispatch(
        &mut ctx,
        &registry,
        "insert.drawing.rectangle",
        drawing_args(1.0, 1.0, -2.0, 1.0),
    )
    .expect_err("negative drawing size");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));

    assert_eq!(page_shapes(&ctx, page).len(), before);
}

#[test]
fn a_drop_onto_an_overflowing_target_point_never_glues() {
    let (mut ctx, registry, _session, page, _target) = fixture();
    let huge = {
        let document = &mut ctx.active_document_mut().expect("document").document;
        let mut shape = target_shape();
        for cell in &mut shape.sheet.cells {
            if cell.name.as_str() == "PinX" {
                cell.value = Some(CellValue::Number(1.7e308));
            }
        }
        for section in &mut shape.sheet.sections {
            if section.name.as_str() == "Connection"
                && let Some(row) = section.rows.first_mut()
                && let Some(cell) = row.cells.first_mut()
            {
                cell.value = Some(CellValue::Number(1.7e308));
            }
        }
        document.insert_shape(page, shape, None).expect("huge")
    };
    let huge_visio = ctx
        .active_document()
        .expect("document")
        .document
        .shape(huge)
        .expect("huge")
        .visio_id;
    let before = page_shapes(&ctx, page).len();
    let mut args = drop_args("Rectangle");
    args.set("glue_to", ArgValue::Integer(i64::from(huge_visio)));
    let error = dispatch(&mut ctx, &registry, "insert.shape", args).expect_err("no finite point");
    assert!(
        error.to_string().contains("no finite connection point"),
        "{error}"
    );
    assert_eq!(
        page_shapes(&ctx, page).len(),
        before,
        "the failed glue rolled the drop back"
    );
}

#[test]
fn a_stale_active_page_is_named_before_any_shape_is_inserted() {
    let (mut ctx, registry, _session, page, _target) = fixture();
    let before = page_shapes(&ctx, page).len();
    ctx.active_document_mut().expect("document").active_page = Some(PageId { index: 9 });
    let error = dispatch(
        &mut ctx,
        &registry,
        "insert.drawing.rectangle",
        drawing_args(1.0, 1.0, 1.0, 1.0),
    )
    .expect_err("stale page");
    assert!(error.to_string().contains("unknown page id 9"), "{error}");
    assert_eq!(page_shapes(&ctx, page).len(), before);

    let error = dispatch(&mut ctx, &registry, "insert.shape", drop_args("Rectangle"))
        .expect_err("stale page");
    assert!(error.to_string().contains("unknown page id 9"), "{error}");
    assert_eq!(page_shapes(&ctx, page).len(), before);
}

#[test]
fn out_of_range_glue_ids_and_rows_are_invalid() {
    let (mut ctx, registry, _session, page, _target) = fixture();
    let before = page_shapes(&ctx, page).len();
    let mut args = drop_args("Rectangle");
    args.set("glue_to", ArgValue::Integer(i64::MAX));
    let error = dispatch(&mut ctx, &registry, "insert.shape", args).expect_err("id range");
    assert!(
        error.to_string().contains("glue_to is out of range"),
        "{error}"
    );

    let mut args = drop_args("Rectangle");
    args.set("glue_to", ArgValue::Integer(1));
    args.set("glue_point", ArgValue::Integer(-1));
    let error = dispatch(&mut ctx, &registry, "insert.shape", args).expect_err("negative row");
    assert!(
        error.to_string().contains("non-negative row index"),
        "{error}"
    );
    assert_eq!(page_shapes(&ctx, page).len(), before);
}

#[test]
fn every_insert_number_names_the_argument_it_rejects() {
    let (mut ctx, registry, _session, _page, _target) = fixture();
    let mut bad = |id: &str, set: &[(&str, f64)], want: &str| {
        let mut args = drawing_args(1.0, 1.0, 1.0, 1.0);
        if id == "insert.shape" {
            args = drop_args("Rectangle");
        }
        for (name, value) in set {
            args.set(*name, ArgValue::Number(*value));
        }
        let error = dispatch(&mut ctx, &registry, id, args).expect_err(id);
        assert!(error.to_string().contains(want), "{id}: {error}");
    };
    bad("insert.shape", &[("x", f64::NAN)], "x must be finite");
    bad("insert.shape", &[("y", f64::INFINITY)], "y must be finite");
    bad(
        "insert.shape",
        &[("width", f64::INFINITY)],
        "width must be finite",
    );
    bad(
        "insert.shape",
        &[("height", f64::NEG_INFINITY)],
        "height must be finite",
    );
    bad(
        "insert.drawing.rectangle",
        &[("x", f64::NAN)],
        "x must be finite",
    );
    bad(
        "insert.drawing.rectangle",
        &[("y", f64::INFINITY)],
        "y must be finite",
    );
    bad(
        "insert.drawing.rectangle",
        &[("width", f64::INFINITY)],
        "width must be finite",
    );
    bad(
        "insert.drawing.rectangle",
        &[("height", f64::NAN)],
        "height must be finite",
    );
    bad(
        "insert.drawing.line",
        &[("width", f64::INFINITY)],
        "width must be finite",
    );
    bad(
        "insert.drawing.line",
        &[("height", f64::NAN)],
        "height must be finite",
    );
}

#[test]
fn an_insert_without_an_active_page_is_an_error() {
    let (mut ctx, registry, _session, page, _target) = fixture();
    let before = page_shapes(&ctx, page).len();
    ctx.active_document_mut().expect("document").active_page = None;
    for (id, args) in [
        ("insert.shape", drop_args("Rectangle")),
        ("insert.drawing.ellipse", drawing_args(1.0, 1.0, 1.0, 1.0)),
    ] {
        let error = dispatch(&mut ctx, &registry, id, args).expect_err(id);
        assert!(
            error.to_string().contains("no active page"),
            "{id}: {error}"
        );
    }
    assert_eq!(page_shapes(&ctx, page).len(), before);
}
