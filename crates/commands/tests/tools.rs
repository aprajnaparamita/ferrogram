//! Tool-choice commands and the one-step connector drag (task P5-17).
//!
//! Every tool id selects a tool without adding an undo step or bumping
//! the change counter; the aliases name the same tool. The connector
//! drag command inserts, glues and routes in one undo step.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod support;

use support::dispatch;

use ferrogram_commands::app::{ActiveTool, AppCtx, DocumentSession, Settings};
use ferrogram_commands::builtins::registry;
use ferrogram_commands::{ArgValue, Args, CommandError, CommandId, Invocation, Registry, Source};
use ferrogram_geom::Point;
use ferrogram_model::{
    Cell, CellName, CellValue, Connect, Document, Page, PageId, Preserved, Row, Section,
    SectionName, Shape, ShapeId,
};

fn cell(name: &str, value: f64) -> Cell {
    Cell {
        name: CellName::new(name),
        formula: Some(format!("{value}")),
        value: Some(CellValue::Number(value)),
        unit: ferrogram_geom::Unit::Inches,
        preserved: Preserved::new(),
    }
}

fn box_shape(visio_id: i32, pin: (f64, f64)) -> Shape {
    let mut shape = Shape::new(ShapeId { index: 0 }, visio_id);
    shape.sheet.cells = vec![
        cell("Width", 2.0),
        cell("Height", 1.0),
        cell("PinX", pin.0),
        cell("PinY", pin.1),
        cell("LocPinX", 1.0),
        cell("LocPinY", 0.5),
        cell("Angle", 0.0),
    ];
    let mut connection = Section::new(SectionName::from_static("Connection"));
    for (ix, (x, y)) in [(1.0, 1.0), (1.0, 0.0)].into_iter().enumerate() {
        let mut row = Row::new();
        row.row_type = Some("Connection".to_owned());
        row.ix = Some(i32::try_from(ix).unwrap_or(0));
        row.cells = vec![cell("X", x), cell("Y", y)];
        connection.rows.push(row);
    }
    shape.sheet.sections.push(connection);
    shape
}

fn fixture() -> (AppCtx, Registry) {
    let mut doc = Document::new();
    let mut page = Page::new(PageId { index: 0 }, 1);
    page.sheet.cells = vec![cell("PageWidth", 8.5), cell("PageHeight", 11.0)];
    let left = doc.add_shape(box_shape(1, (2.0, 3.0)));
    let right = doc.add_shape(box_shape(2, (7.0, 3.0)));
    page.shapes.push(left);
    page.shapes.push(right);
    doc.pages.push(page);
    let mut ctx = AppCtx::new(Settings::default());
    ctx.open_document(DocumentSession::new(doc, None));
    (ctx, registry().expect("registry"))
}

fn empty_ctx() -> AppCtx {
    AppCtx::new(Settings::default())
}

#[test]
fn every_tool_id_selects_its_tool_without_an_edit() {
    let (mut ctx, registry) = fixture();
    for (id, tool) in [
        ("tools.pointer", ActiveTool::Pointer),
        ("tools.rectangle", ActiveTool::Rectangle),
        ("shape.rectangle", ActiveTool::Rectangle),
        ("tools.ellipse", ActiveTool::Ellipse),
        ("shape.ellipse", ActiveTool::Ellipse),
        ("shape.line", ActiveTool::Line),
        ("tools.connector", ActiveTool::Connector),
        ("tools.text", ActiveTool::Text),
    ] {
        ctx.active_tool = ActiveTool::Pointer;
        let before = ctx.active_document().expect("session").changes;
        let outcome = dispatch(&mut ctx, &registry, id, Args::new()).expect(id);
        assert_eq!(ctx.active_tool, tool, "{id}");
        assert!(outcome.edit.is_none(), "{id} is not an edit");
        let session = ctx.active_document().expect("session");
        assert_eq!(session.changes, before, "{id} does not bump changes");
        assert!(session.undo.is_empty(), "{id} adds no undo step");
        assert_eq!(ctx.active_tool(), tool);
    }
}

#[test]
fn tool_ids_are_disabled_without_a_document() {
    let mut ctx = empty_ctx();
    let registry = registry().expect("registry");
    for id in [
        "tools.pointer",
        "tools.rectangle",
        "shape.rectangle",
        "tools.ellipse",
        "shape.ellipse",
        "shape.line",
        "tools.connector",
        "tools.text",
    ] {
        let command = CommandId::intern(id).expect("id");
        let enabled = registry.enabled(&command, &ctx);
        assert!(!enabled.value, "{id}");
        assert_eq!(enabled.reason.as_deref(), Some("no open document"));
        let error = registry
            .execute(&command, &mut ctx, Invocation::empty(Source::Test))
            .expect_err(id);
        assert!(
            matches!(error, CommandError::NotEnabled { .. })
                || error.to_string().contains("no open document"),
            "{id}: {error}"
        );
    }
    assert_eq!(ctx.active_tool, ActiveTool::Pointer, "no tool changed");
}

/// The connector path points in page inches (the inverse of the
/// writer's local points under the shape transform).
fn connector_page_points(ctx: &AppCtx, visio_id: i32) -> Vec<Point> {
    let session = ctx.active_document().expect("session");
    let page = session.active_page.expect("page");
    let shape = session
        .document
        .shape_by_visio_id(page, visio_id)
        .expect("connector");
    let entity = session.document.shape(shape).expect("shape");
    let number = |name: &str| {
        entity
            .sheet
            .cells
            .iter()
            .find(|cell| cell.name.as_str() == name)
            .and_then(|cell| match cell.value {
                Some(CellValue::Number(value)) => Some(value),
                _ => None,
            })
            .unwrap_or(0.0)
    };
    let offset = Point::new(
        number("PinX") - number("LocPinX"),
        number("PinY") - number("LocPinY"),
    );
    session
        .document
        .shape_geometry(shape)
        .expect("geometry")
        .paths
        .first()
        .expect("path")
        .rows
        .iter()
        .filter_map(|row| match row {
            ferrogram_model::GeometryRow::MoveTo { x, y }
            | ferrogram_model::GeometryRow::LineTo { x, y } => {
                Some(Point::new(x + offset.x, y + offset.y))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn a_connector_glued_at_one_end_reaches_the_free_drop_point_in_one_undo_step() {
    let (mut ctx, registry) = fixture();
    let before = {
        let session = ctx.active_document().expect("session");
        session
            .document
            .page(PageId { index: 0 })
            .expect("page")
            .shapes
            .len()
    };
    let args = Args::new()
        .with("stencil", ArgValue::Text("Basic Flowchart".to_owned()))
        .with("master", ArgValue::Text("Connector".to_owned()))
        .with("x", ArgValue::Number(2.0))
        .with("y", ArgValue::Number(3.0))
        .with("begin_target", ArgValue::Integer(1))
        .with("begin_point", ArgValue::Integer(0))
        .with("end_x", ArgValue::Number(6.0))
        .with("end_y", ArgValue::Number(3.0));
    dispatch(&mut ctx, &registry, "connector.draw", args).expect("connector");

    let connector = ctx.selection.anchor.expect("the new connector is selected");
    let visio_id = ctx
        .active_document()
        .expect("session")
        .document
        .shape(connector)
        .expect("connector")
        .visio_id;
    let points = connector_page_points(&ctx, visio_id);
    let end = points.last().copied().expect("a routed path");
    assert!(
        (end.x - 6.0).abs() < 1.0e-9 && (end.y - 3.0).abs() < 1.0e-9,
        "the free end reaches the drop point: {end:?}"
    );
    assert_eq!(
        ctx.active_document().expect("session").undo.len(),
        1,
        "one drag, one undo step"
    );

    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    let after = {
        let session = ctx.active_document().expect("session");
        session
            .document
            .page(PageId { index: 0 })
            .expect("page")
            .shapes
            .len()
    };
    assert_eq!(after, before, "one undo removes the whole connector");
    assert!(ctx.active_document().expect("session").undo.can_redo());
}

#[test]
fn a_connector_with_a_bad_target_is_rolled_back() {
    let (mut ctx, registry) = fixture();
    let before = {
        let session = ctx.active_document().expect("session");
        session
            .document
            .page(PageId { index: 0 })
            .expect("page")
            .shapes
            .len()
    };
    let args = Args::new()
        .with("stencil", ArgValue::Text("Basic Flowchart".to_owned()))
        .with("master", ArgValue::Text("Connector".to_owned()))
        .with("x", ArgValue::Number(2.0))
        .with("y", ArgValue::Number(3.0))
        .with("begin_target", ArgValue::Integer(99))
        .with("end_x", ArgValue::Number(6.0))
        .with("end_y", ArgValue::Number(3.0));
    let error = dispatch(&mut ctx, &registry, "connector.draw", args).expect_err("bad target");
    assert!(error.to_string().contains("unknown glue target"), "{error}");
    let session = ctx.active_document().expect("session");
    assert_eq!(
        session
            .document
            .page(PageId { index: 0 })
            .expect("page")
            .shapes
            .len(),
        before,
        "the failed drag leaves no shape"
    );
    assert!(session.undo.is_empty());
}

#[test]
fn free_end_arguments_must_be_complete() {
    let (mut ctx, registry) = fixture();
    let args = Args::new()
        .with("stencil", ArgValue::Text("Basic Flowchart".to_owned()))
        .with("master", ArgValue::Text("Connector".to_owned()))
        .with("x", ArgValue::Number(2.0))
        .with("y", ArgValue::Number(3.0))
        .with("end_x", ArgValue::Number(6.0));
    let error =
        dispatch(&mut ctx, &registry, "connector.draw", args).expect_err("incomplete free point");
    assert!(error.to_string().contains("end_y is required"), "{error}");
}

#[test]
fn a_connector_with_two_free_ends_uses_both_given_points() {
    let (mut ctx, registry) = fixture();
    let args = Args::new()
        .with("stencil", ArgValue::Text("Basic Flowchart".to_owned()))
        .with("master", ArgValue::Text("Connector".to_owned()))
        .with("x", ArgValue::Number(1.0))
        .with("y", ArgValue::Number(4.0))
        .with("begin_x", ArgValue::Number(1.0))
        .with("begin_y", ArgValue::Number(4.0))
        .with("end_x", ArgValue::Number(6.0))
        .with("end_y", ArgValue::Number(2.0));
    dispatch(&mut ctx, &registry, "connector.draw", args).expect("connector");
    let connector = ctx.selection.anchor.expect("selected");
    let visio_id = ctx
        .active_document()
        .expect("session")
        .document
        .shape(connector)
        .expect("shape")
        .visio_id;
    let points = connector_page_points(&ctx, visio_id);
    assert_eq!(points.first(), Some(&Point::new(1.0, 4.0)));
    assert_eq!(points.last(), Some(&Point::new(6.0, 2.0)));
    assert_eq!(ctx.active_document().expect("session").undo.len(), 1);
}

#[test]
fn a_free_begin_can_pair_with_a_glued_end() {
    let (mut ctx, registry) = fixture();
    let args = Args::new()
        .with("stencil", ArgValue::Text("Basic Flowchart".to_owned()))
        .with("master", ArgValue::Text("Connector".to_owned()))
        .with("x", ArgValue::Number(1.0))
        .with("y", ArgValue::Number(4.0))
        .with("begin_x", ArgValue::Number(1.0))
        .with("begin_y", ArgValue::Number(4.0))
        .with("end_target", ArgValue::Integer(2))
        .with("end_point", ArgValue::Integer(0));
    dispatch(&mut ctx, &registry, "connector.draw", args).expect("connector");
    let connector = ctx.selection.anchor.expect("selected");
    let visio_id = ctx
        .active_document()
        .expect("session")
        .document
        .shape(connector)
        .expect("shape")
        .visio_id;
    let points = connector_page_points(&ctx, visio_id);
    assert_eq!(points.first(), Some(&Point::new(1.0, 4.0)));
    // The right box's point 0 is local (1, 1) with its local pin at
    // (1, 0.5), so page (7, 3.5).
    assert_eq!(points.last(), Some(&Point::new(7.0, 3.5)));
}

#[test]
fn malformed_connector_arguments_are_errors() {
    let (mut ctx, registry) = fixture();
    let missing = Args::new()
        .with("master", ArgValue::Text("Connector".to_owned()))
        .with("x", ArgValue::Number(1.0))
        .with("y", ArgValue::Number(1.0));
    let error = dispatch(&mut ctx, &registry, "connector.draw", missing).expect_err("stencil");
    assert!(matches!(error, CommandError::NeedsInput(_)), "{error}");

    let point_without_target = Args::new()
        .with("stencil", ArgValue::Text("Basic Flowchart".to_owned()))
        .with("master", ArgValue::Text("Connector".to_owned()))
        .with("x", ArgValue::Number(1.0))
        .with("y", ArgValue::Number(1.0))
        .with("begin_point", ArgValue::Integer(0));
    let error = dispatch(&mut ctx, &registry, "connector.draw", point_without_target)
        .expect_err("begin point");
    assert!(error.to_string().contains("begin_point needs"), "{error}");

    let end_point_without_target = Args::new()
        .with("stencil", ArgValue::Text("Basic Flowchart".to_owned()))
        .with("master", ArgValue::Text("Connector".to_owned()))
        .with("x", ArgValue::Number(1.0))
        .with("y", ArgValue::Number(1.0))
        .with("end_point", ArgValue::Integer(0));
    let error = dispatch(
        &mut ctx,
        &registry,
        "connector.draw",
        end_point_without_target,
    )
    .expect_err("end point");
    assert!(error.to_string().contains("end_point needs"), "{error}");

    let only_y = Args::new()
        .with("stencil", ArgValue::Text("Basic Flowchart".to_owned()))
        .with("master", ArgValue::Text("Connector".to_owned()))
        .with("x", ArgValue::Number(1.0))
        .with("y", ArgValue::Number(1.0))
        .with("begin_y", ArgValue::Number(3.0));
    let error = dispatch(&mut ctx, &registry, "connector.draw", only_y).expect_err("only y");
    assert!(error.to_string().contains("begin_x is required"), "{error}");

    let negative = Args::new()
        .with("stencil", ArgValue::Text("Basic Flowchart".to_owned()))
        .with("master", ArgValue::Text("Connector".to_owned()))
        .with("x", ArgValue::Number(1.0))
        .with("y", ArgValue::Number(1.0))
        .with("end_target", ArgValue::Integer(2))
        .with("end_point", ArgValue::Integer(-1));
    let error =
        dispatch(&mut ctx, &registry, "connector.draw", negative).expect_err("negative point");
    assert!(
        error.to_string().contains("must not be negative"),
        "{error}"
    );
}

#[test]
fn insert_shape_rejects_a_non_finite_size_and_a_pointless_glue_target() {
    let (mut ctx, registry) = fixture();
    let infinite = Args::new()
        .with("stencil", ArgValue::Text("Basic Shapes".to_owned()))
        .with("master", ArgValue::Text("Rectangle".to_owned()))
        .with("x", ArgValue::Number(1.0))
        .with("y", ArgValue::Number(1.0))
        .with("width", ArgValue::Number(f64::INFINITY));
    let error = dispatch(&mut ctx, &registry, "insert.shape", infinite).expect_err("size");
    assert!(error.to_string().contains("must be finite"), "{error}");

    // A target with no Connection rows is an error after the insert, and
    // the command rolls the new shape back out.
    let before = {
        let session = ctx.active_document().expect("session");
        session
            .document
            .page(PageId { index: 0 })
            .expect("page")
            .shapes
            .len()
    };
    let target = {
        let session = ctx.active_document_mut().expect("session");
        let target = session
            .document
            .add_shape(Shape::new(ShapeId { index: 0 }, 3));
        if let Some(page) = session.document.pages.get_mut(0) {
            page.shapes.push(target);
        }
        target
    };
    let _ = target;
    let args = Args::new()
        .with("stencil", ArgValue::Text("Basic Shapes".to_owned()))
        .with("master", ArgValue::Text("Rectangle".to_owned()))
        .with("x", ArgValue::Number(1.0))
        .with("y", ArgValue::Number(1.0))
        .with("glue_to", ArgValue::Integer(3));
    let error = dispatch(&mut ctx, &registry, "insert.shape", args).expect_err("no points");
    assert!(
        error.to_string().contains("no connection points"),
        "{error}"
    );
    let after = {
        let session = ctx.active_document().expect("session");
        session
            .document
            .page(PageId { index: 0 })
            .expect("page")
            .shapes
            .len()
    };
    assert_eq!(after, before + 1, "only the target was added");
}

#[test]
fn edit_select_rejects_an_unknown_mode() {
    let (mut ctx, registry) = fixture();
    let args = Args::new().with("mode", ArgValue::Text("bogus".to_owned()));
    let error = dispatch(&mut ctx, &registry, "edit.select", args).expect_err("mode");
    assert!(
        error.to_string().contains("mode must be replace"),
        "{error}"
    );
}

#[test]
fn page_moves_from_no_active_page_pick_the_first_and_last_page() {
    let (mut ctx, registry) = fixture();
    {
        let session = ctx.active_document_mut().expect("session");
        session.active_page = None;
        session.document.add_page("Page-2").expect("second page");
    }
    dispatch(&mut ctx, &registry, "page.next", Args::new()).expect("next");
    assert_eq!(
        ctx.active_document().expect("session").active_page,
        Some(PageId { index: 0 })
    );
    {
        let session = ctx.active_document_mut().expect("session");
        session.active_page = None;
    }
    dispatch(&mut ctx, &registry, "page.previous", Args::new()).expect("previous");
    assert_eq!(
        ctx.active_document().expect("session").active_page,
        Some(PageId { index: 1 })
    );
}

#[test]
fn a_reroute_route_failure_is_a_warning_not_an_edit_failure() {
    let (mut ctx, registry) = fixture();
    let (connector, target) = {
        let session = ctx.active_document_mut().expect("session");
        let mut connector = Shape::new(ShapeId { index: 0 }, 3);
        connector.sheet.cells = vec![
            cell("Width", 1.5),
            cell("Height", 0.75),
            cell("PinX", 1.0e308),
            cell("PinY", 3.0),
            cell("LocPinX", -1.0e308),
            cell("LocPinY", 0.0),
            cell("Angle", 0.0),
        ];
        let mut geometry = Section::new(SectionName::from_static("Geometry"));
        for (ix, (kind, x, y)) in [("MoveTo", 0.0, 0.0), ("LineTo", 1.5, 0.75)]
            .into_iter()
            .enumerate()
        {
            let mut row = Row::new();
            row.row_type = Some(kind.to_owned());
            row.ix = Some(i32::try_from(ix).unwrap_or(0));
            row.cells = vec![cell("X", x), cell("Y", y)];
            geometry.rows.push(row);
        }
        connector.sheet.sections.push(geometry);
        let connector = session.document.add_shape(connector);
        if let Some(page) = session.document.pages.get_mut(0) {
            page.shapes.push(connector);
            let mut connect = Connect::new();
            connect.from_sheet = 3;
            connect.from_cell = "BeginX".to_owned();
            connect.from_part = ferrogram_model::mutate::VIS_BEGIN;
            connect.to_sheet = 1;
            connect.to_cell = "PinX".to_owned();
            connect.to_part = ferrogram_model::mutate::VIS_WHOLE_SHAPE;
            page.connects.push(connect);
        }
        let target = session
            .document
            .shape_by_visio_id(PageId { index: 0 }, 1)
            .expect("target");
        (connector, target)
    };
    let _ = connector;
    ctx.selection.shapes = vec![target];
    ctx.selection.anchor = Some(target);
    let args = Args::new()
        .with("dx", ArgValue::Number(0.5))
        .with("dy", ArgValue::Number(0.0));
    dispatch(&mut ctx, &registry, "shape.move", args).expect("move");
    assert!(ctx.active_document().expect("session").undo.can_undo());
    assert!(
        ctx.warnings
            .iter()
            .any(|warning| warning.contains("connectors were not re-routed")),
        "{:?}",
        ctx.warnings
    );
}

#[test]
fn delete_with_no_active_page_skips_the_glue_warning_scan() {
    let (mut ctx, registry) = fixture();
    let target = {
        let session = ctx.active_document().expect("session");
        session
            .document
            .shape_by_visio_id(PageId { index: 0 }, 1)
            .expect("shape")
    };
    {
        let session = ctx.active_document_mut().expect("session");
        session.active_page = None;
    }
    ctx.selection.shapes = vec![target];
    ctx.selection.anchor = Some(target);
    dispatch(&mut ctx, &registry, "edit.delete", Args::new()).expect("delete");
    assert!(ctx.active_document().expect("session").undo.can_undo());
}
