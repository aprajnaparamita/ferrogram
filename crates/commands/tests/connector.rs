//! `connector.*` through the registry (task P5-08, ADR 0045).
//!
//! The exit-flowchart path is headless: a flow of shapes and five glued
//! connectors is built with `Source::Test` commands, a move re-routes the
//! affected connectors in the same undo step, and undo/redo restores and
//! re-applies the routed paths. No GUI is involved.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use ferrogram_commands::app::{AppCtx, DocumentSession, Settings};
use ferrogram_commands::builtins::registry;
use ferrogram_commands::{
    ArgValue, Args, CommandError, CommandId, Invocation, Outcome, Registry, Source,
};
use ferrogram_model::{
    Cell, CellName, CellValue, Document, Page, PageId, Preserved, Row, Section, SectionName, Shape,
    ShapeId,
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

/// A 1.5 x 0.75 box at `(2, y)` with two connection points (top and
/// bottom edge midpoints).
fn box_shape(visio_id: i32, y: f64) -> Shape {
    let mut shape = Shape::new(ShapeId { index: 0 }, visio_id);
    shape.sheet.cells = vec![
        cell("Width", 1.5),
        cell("Height", 0.75),
        cell("PinX", 2.0),
        cell("PinY", y),
        cell("LocPinX", 0.75),
        cell("LocPinY", 0.375),
        cell("Angle", 0.0),
    ];
    let mut connection = Section::new(SectionName::from_static("Connection"));
    for (ix, (x, y)) in [(0.75, 0.75), (0.75, 0.0)].into_iter().enumerate() {
        let mut row = Row::new();
        row.row_type = Some("Connection".to_owned());
        row.ix = Some(i32::try_from(ix).unwrap_or(0));
        row.cells = vec![cell("X", x), cell("Y", y)];
        connection.rows.push(row);
    }
    shape.sheet.sections.push(connection);
    shape
}

/// A connector instance at `(2, y)`; its transform cells are all the
/// path writer needs.
fn connector_shape(visio_id: i32, y: f64) -> Shape {
    let mut shape = Shape::new(ShapeId { index: 0 }, visio_id);
    shape.sheet.cells = vec![
        cell("Width", 1.5),
        cell("Height", 0.75),
        cell("PinX", 2.0),
        cell("PinY", y),
        cell("LocPinX", 0.75),
        cell("LocPinY", 0.375),
        cell("Angle", 0.0),
    ];
    shape
}

/// Six boxes at y = 7, 5.5, 4, 2.5, 1, -0.5 and five connectors between
/// them. Visio ids: boxes 1..=6, connectors 11..=15.
fn flow() -> (AppCtx, Registry, PageId) {
    let mut doc = Document::new();
    let mut page = Page::new(PageId { index: 0 }, 1);
    page.sheet.cells = vec![cell("PageWidth", 8.5), cell("PageHeight", 11.0)];
    for (index, y) in [7.0, 5.5, 4.0, 2.5, 1.0, -0.5].into_iter().enumerate() {
        let id = doc.add_shape(box_shape(i32::try_from(index).unwrap_or(0) + 1, y));
        page.shapes.push(id);
    }
    for (index, y) in [6.25, 4.75, 3.25, 1.75, 0.25].into_iter().enumerate() {
        let id = doc.add_shape(connector_shape(i32::try_from(index).unwrap_or(0) + 11, y));
        page.shapes.push(id);
    }
    let page_id = page.id;
    doc.pages.push(page);
    let mut ctx = AppCtx::new(Settings::default());
    ctx.open_document(DocumentSession::new(doc, None));
    (ctx, registry().expect("registry"), page_id)
}

/// One command the way the shell dispatches it: push the edit.
fn dispatch(
    ctx: &mut AppCtx,
    registry: &Registry,
    id: &str,
    args: Args,
) -> Result<Outcome, CommandError> {
    let ran_against = ctx.active_document().map(|session| session.id);
    let mut outcome = registry.execute(
        &CommandId::intern(id).expect("id"),
        ctx,
        Invocation::new(args, Source::Test),
    )?;
    if let Some(edit) = outcome.edit.take()
        && let Some(session) = ran_against.or_else(|| ctx.active_document().map(|s| s.id))
    {
        ctx.push_edit(session, edit)?;
    }
    Ok(outcome)
}

fn glue_args(connector: i32, target: i32, point: i32) -> Args {
    Args::new()
        .with("connector", ArgValue::Integer(i64::from(connector)))
        .with("target", ArgValue::Integer(i64::from(target)))
        .with("point", ArgValue::Integer(i64::from(point)))
}

/// Page-space path points of one connector (the inverse of
/// `write_connector_path`'s local mapping).
fn path(ctx: &AppCtx, connector: i32) -> Vec<ferrogram_geom::Point> {
    let doc = &ctx.active_document().expect("session").document;
    let shape = doc
        .shape_by_visio_id(PageId { index: 0 }, connector)
        .expect("connector");
    let geometry = doc.shape_geometry(shape).expect("geometry");
    let reader = |name: &str| -> f64 {
        doc.shape(shape)
            .and_then(|entry| {
                entry
                    .sheet
                    .cells
                    .iter()
                    .find(|cell| cell.name.as_str() == name)
            })
            .and_then(|cell| match &cell.value {
                Some(CellValue::Number(value)) => Some(*value),
                _ => None,
            })
            .unwrap_or(0.0)
    };
    let pin = ferrogram_geom::Point::new(reader("PinX"), reader("PinY"));
    let loc = ferrogram_geom::Point::new(reader("LocPinX"), reader("LocPinY"));
    geometry
        .paths
        .first()
        .map(|path| {
            path.rows
                .iter()
                .filter_map(|row| match row {
                    ferrogram_model::GeometryRow::MoveTo { x, y }
                    | ferrogram_model::GeometryRow::LineTo { x, y } => Some(
                        ferrogram_geom::Point::new(x - loc.x + pin.x, y - loc.y + pin.y),
                    ),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_five_connector_flow_is_built_headlessly() {
    let (mut ctx, registry, _page) = flow();
    for (index, connector) in [11, 12, 13, 14, 15].into_iter().enumerate() {
        let top = i32::try_from(index).unwrap_or(0) + 1;
        let bottom = top + 1;
        dispatch(
            &mut ctx,
            &registry,
            "connector.glue_begin",
            glue_args(connector, top, 1),
        )
        .expect("glue begin");
        dispatch(
            &mut ctx,
            &registry,
            "connector.glue_end",
            glue_args(connector, bottom, 0),
        )
        .expect("glue end");
    }
    let doc = &ctx.active_document().expect("session").document;
    let page = doc.page(PageId { index: 0 }).expect("page");
    assert_eq!(page.connects.len(), 10, "two glues per connector");
    for connector in [11, 12, 13, 14, 15] {
        let points = path(&ctx, connector);
        assert_eq!(points.len(), 2, "{connector}: {points:?}");
        assert!(
            points.first().map(|p| p.y) > points.last().map(|p| p.y),
            "{connector}: {points:?}"
        );
    }
    assert_eq!(path(&ctx, 11).first().map(|p| p.y), Some(6.625));
}

#[test]
fn a_move_reroutes_one_step_and_undo_redo_follow_it() {
    let (mut ctx, registry, page) = flow();
    for (index, connector) in [11, 12, 13, 14, 15].into_iter().enumerate() {
        let top = i32::try_from(index).unwrap_or(0) + 1;
        let bottom = top + 1;
        dispatch(
            &mut ctx,
            &registry,
            "connector.glue_begin",
            glue_args(connector, top, 1),
        )
        .expect("glue begin");
        dispatch(
            &mut ctx,
            &registry,
            "connector.glue_end",
            glue_args(connector, bottom, 0),
        )
        .expect("glue end");
    }
    let before: Vec<_> = [11, 12, 13, 14, 15]
        .into_iter()
        .map(|connector| path(&ctx, connector))
        .collect();
    let box_three = ctx
        .active_document()
        .expect("session")
        .document
        .shape_by_visio_id(page, 3)
        .expect("box 3");
    ctx.selection.shapes = vec![box_three];
    ctx.selection.anchor = Some(box_three);
    ctx.selection.page = Some(page);
    dispatch(
        &mut ctx,
        &registry,
        "shape.move",
        Args::new()
            .with("dx", ArgValue::Number(1.0))
            .with("dy", ArgValue::Number(0.0)),
    )
    .expect("move");
    let after: Vec<_> = [11, 12, 13, 14, 15]
        .into_iter()
        .map(|connector| path(&ctx, connector))
        .collect();
    assert_ne!(before[1], after[1], "connector 12 follows box 3");
    assert_ne!(before[2], after[2], "connector 13 follows box 3");
    assert_eq!(before[0], after[0], "connector 11 is unaffected");
    assert_eq!(before[3], after[3], "connector 14 is unaffected");

    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    let undone: Vec<_> = [11, 12, 13, 14, 15]
        .into_iter()
        .map(|connector| path(&ctx, connector))
        .collect();
    assert_eq!(undone, before);
    dispatch(&mut ctx, &registry, "edit.redo", Args::new()).expect("redo");
    let redone: Vec<_> = [11, 12, 13, 14, 15]
        .into_iter()
        .map(|connector| path(&ctx, connector))
        .collect();
    assert_eq!(redone, after);
}

#[test]
fn a_right_angle_style_applies_to_every_selected_connector() {
    let (mut ctx, registry, page) = flow();
    for (index, connector) in [11, 12, 13, 14, 15].into_iter().enumerate() {
        let top = i32::try_from(index).unwrap_or(0) + 1;
        let bottom = top + 1;
        dispatch(
            &mut ctx,
            &registry,
            "connector.glue_begin",
            glue_args(connector, top, 1),
        )
        .expect("glue begin");
        dispatch(
            &mut ctx,
            &registry,
            "connector.glue_end",
            glue_args(connector, bottom, 0),
        )
        .expect("glue end");
    }
    // Shift box 3 sideways so its two connectors have a real elbow.
    let box_three = ctx
        .active_document()
        .expect("session")
        .document
        .shape_by_visio_id(page, 3)
        .expect("box 3");
    ctx.selection.shapes = vec![box_three];
    ctx.selection.anchor = Some(box_three);
    ctx.selection.page = Some(page);
    dispatch(
        &mut ctx,
        &registry,
        "shape.move",
        Args::new()
            .with("dx", ArgValue::Number(1.0))
            .with("dy", ArgValue::Number(0.0)),
    )
    .expect("move box 3");
    let connectors: Vec<ShapeId> = [11, 12, 13, 14, 15]
        .into_iter()
        .filter_map(|connector| {
            ctx.active_document()
                .expect("session")
                .document
                .shape_by_visio_id(page, connector)
        })
        .collect();
    ctx.selection.shapes = connectors;
    ctx.selection.anchor = ctx.selection.shapes.last().copied();
    ctx.selection.page = Some(page);
    dispatch(
        &mut ctx,
        &registry,
        "connector.style",
        Args::new().with("style", ArgValue::Text("right_angle".to_owned())),
    )
    .expect("style");
    for connector in [11, 14] {
        assert_eq!(path(&ctx, connector).len(), 2, "{connector} is aligned");
    }
    for connector in [12, 13] {
        assert_eq!(path(&ctx, connector).len(), 3, "{connector} has an elbow");
    }
}

fn connector_shape_id(ctx: &AppCtx, visio_id: i32) -> ShapeId {
    ctx.active_document()
        .expect("session")
        .document
        .shape_by_visio_id(PageId { index: 0 }, visio_id)
        .expect("connector shape")
}

#[test]
fn unglue_and_style_report_missing_input_and_both_ends() {
    let (mut ctx, registry, page) = flow();
    dispatch(
        &mut ctx,
        &registry,
        "connector.glue_begin",
        glue_args(11, 1, 1),
    )
    .expect("glue begin");
    dispatch(
        &mut ctx,
        &registry,
        "connector.glue_end",
        glue_args(11, 2, 0),
    )
    .expect("glue end");

    // A missing `end` is a text prompt, not a silent no-op.
    let missing = dispatch(
        &mut ctx,
        &registry,
        "connector.unglue",
        Args::new().with("connector", ArgValue::Integer(11)),
    )
    .expect_err("missing end");
    assert!(matches!(
        missing,
        CommandError::NeedsInput(ferrogram_commands::NeededInput::Text { .. })
    ));

    let outcome = dispatch(
        &mut ctx,
        &registry,
        "connector.unglue",
        Args::new()
            .with("connector", ArgValue::Integer(11))
            .with("end", ArgValue::Text("end".to_owned())),
    )
    .expect("unglue end");
    assert_eq!(outcome.message.as_deref(), Some("Connector end unglued"));
    let ends = ctx
        .active_document()
        .expect("session")
        .document
        .connects_of(page, connector_shape_id(&ctx, 11))
        .expect("ends");
    assert!(ends.first().is_some_and(Option::is_some));
    assert!(ends.get(1).is_some_and(Option::is_none));

    // The straight style (the other message branch) on one shape.
    let connector = connector_shape_id(&ctx, 11);
    ctx.selection.shapes = vec![connector];
    ctx.selection.anchor = Some(connector);
    ctx.selection.page = Some(page);
    let outcome = dispatch(
        &mut ctx,
        &registry,
        "connector.style",
        Args::new().with("style", ArgValue::Text("straight".to_owned())),
    )
    .expect("straight style");
    assert_eq!(
        outcome.message.as_deref(),
        Some("Connector style: straight")
    );
    let missing =
        dispatch(&mut ctx, &registry, "connector.style", Args::new()).expect_err("missing style");
    assert!(matches!(
        missing,
        CommandError::NeedsInput(ferrogram_commands::NeededInput::Text { .. })
    ));
    let bad = dispatch(
        &mut ctx,
        &registry,
        "connector.style",
        Args::new().with("style", ArgValue::Flag(true)),
    )
    .expect_err("bad style type");
    assert!(bad.to_string().contains("straight or right_angle"), "{bad}");

    // The plural and singular re-route messages.
    let outcome =
        dispatch(&mut ctx, &registry, "connector.reroute_all", Args::new()).expect("reroute one");
    assert_eq!(outcome.message.as_deref(), Some("Re-routed 1 connector"));
    dispatch(
        &mut ctx,
        &registry,
        "connector.glue_end",
        glue_args(11, 2, 0),
    )
    .expect("glue end again");
    let outcome = dispatch(&mut ctx, &registry, "connector.reroute_all", Args::new())
        .expect("reroute one again");
    assert_eq!(outcome.message.as_deref(), Some("Re-routed 1 connector"));

    // A stale active page id is a named failure.
    ctx.active_document_mut().expect("session").active_page = Some(PageId { index: 9 });
    let error = dispatch(&mut ctx, &registry, "connector.reroute_all", Args::new())
        .expect_err("stale page");
    assert!(error.to_string().contains("unknown page id 9"), "{error}");
}

#[test]
fn stale_and_detached_glue_records_are_dropped() {
    let (mut ctx, registry, page) = flow();
    {
        let doc = &mut ctx.active_document_mut().expect("session").document;
        let mut stale_target = ferrogram_model::Connect::new();
        stale_target.from_sheet = 11;
        stale_target.from_cell = "BeginX".to_owned();
        stale_target.from_part = ferrogram_model::mutate::VIS_BEGIN;
        stale_target.to_sheet = 99;
        stale_target.to_cell = "PinX".to_owned();
        stale_target.to_part = ferrogram_model::mutate::VIS_WHOLE_SHAPE;
        doc.add_connect(page, stale_target).expect("connect");

        let mut stale_connector = ferrogram_model::Connect::new();
        stale_connector.from_sheet = 98;
        stale_connector.from_cell = "BeginX".to_owned();
        stale_connector.from_part = ferrogram_model::mutate::VIS_BEGIN;
        stale_connector.to_sheet = 1;
        stale_connector.to_cell = "PinX".to_owned();
        stale_connector.to_part = ferrogram_model::mutate::VIS_WHOLE_SHAPE;
        doc.add_connect(page, stale_connector).expect("connect");
    }
    dispatch(&mut ctx, &registry, "connector.reroute_all", Args::new()).expect("reroute");
    let doc = &ctx.active_document().expect("session").document;
    assert!(
        doc.page(page).expect("page").connects.is_empty(),
        "both bad records are gone"
    );
    assert!(
        ctx.warnings
            .iter()
            .any(|line| line.contains("connector 98 is gone")),
        "{:?}",
        ctx.warnings
    );
    assert!(
        ctx.warnings
            .iter()
            .any(|line| line.contains("connector 11 lost its glue to detached shape 99")),
        "{:?}",
        ctx.warnings
    );
}

#[test]
fn a_cyclic_connector_transform_is_reported() {
    let (mut ctx, registry, page) = flow();
    dispatch(
        &mut ctx,
        &registry,
        "connector.glue_begin",
        glue_args(11, 1, 1),
    )
    .expect("glue begin");
    let connector = connector_shape_id(&ctx, 11);
    if let Some(entry) = ctx
        .active_document_mut()
        .expect("session")
        .document
        .shape_mut(connector)
    {
        entry.parent = Some(connector);
    }
    dispatch(&mut ctx, &registry, "connector.reroute_all", Args::new()).expect("reroute");
    assert!(
        ctx.warnings
            .iter()
            .any(|line| line.contains("connector 11 has no usable transform")),
        "{:?}",
        ctx.warnings
    );
    let _ = page;
}

#[test]
fn every_geometry_row_kind_can_supply_a_current_endpoint() {
    let (mut ctx, registry, page) = flow();
    let variants: [&str; 10] = [
        "ArcTo",
        "EllipticalArcTo",
        "NurbsTo",
        "PolylineTo",
        "SplineStart",
        "SplineKnot",
        "Ellipse",
        "InfiniteLine",
        "RelLineTo",
        "Weird",
    ];
    for (offset, kind) in variants.into_iter().enumerate() {
        let mut shape = Shape::new(
            ShapeId { index: 0 },
            100 + i32::try_from(offset).unwrap_or(0),
        );
        shape.sheet.cells = vec![
            cell("Width", 1.5),
            cell("Height", 0.75),
            cell("PinX", 2.0),
            cell("PinY", 2.0),
            cell("LocPinX", 0.75),
            cell("LocPinY", 0.375),
            cell("Angle", 0.0),
        ];
        let mut section = Section::new(SectionName::from_static("Geometry"));
        for (ix, row_type) in ["MoveTo", kind].into_iter().enumerate() {
            let mut row = Row::new();
            row.row_type = Some(row_type.to_owned());
            row.ix = Some(i32::try_from(ix).unwrap_or(0));
            row.cells = vec![cell("X", 0.5), cell("Y", 0.25)];
            section.rows.push(row);
        }
        shape.sheet.sections.push(section);
        let id = ctx
            .active_document_mut()
            .expect("session")
            .document
            .insert_shape(page, shape, None)
            .expect("insert");
        let visio_id = shape_visio_id(&ctx, id);
        dispatch(
            &mut ctx,
            &registry,
            "connector.glue_begin",
            glue_args(visio_id, 1, 1),
        )
        .expect("glue begin");
        dispatch(&mut ctx, &registry, "connector.reroute_all", Args::new()).expect("reroute");
    }
}

fn shape_visio_id(ctx: &AppCtx, shape: ShapeId) -> i32 {
    ctx.active_document()
        .expect("session")
        .document
        .shape(shape)
        .expect("shape")
        .visio_id
}

#[test]
fn an_unknown_target_or_a_detached_connector_is_an_error() {
    let (mut ctx, registry, _page) = flow();
    let error = dispatch(
        &mut ctx,
        &registry,
        "connector.glue_begin",
        glue_args(11, 99, 0),
    )
    .expect_err("unknown target");
    assert!(
        error.to_string().contains("unknown target shape id 99"),
        "{error}"
    );
}

#[test]
fn a_cyclic_or_overflowing_target_degrades_to_a_warning() {
    // A self-parented target: its transform is a cycle.
    let (mut ctx, registry, page) = flow();
    dispatch(
        &mut ctx,
        &registry,
        "connector.glue_begin",
        glue_args(11, 1, 1),
    )
    .expect("glue begin");
    let box_one = ctx
        .active_document()
        .expect("session")
        .document
        .shape_by_visio_id(page, 1)
        .expect("box 1");
    if let Some(entry) = ctx
        .active_document_mut()
        .expect("session")
        .document
        .shape_mut(box_one)
    {
        entry.parent = Some(box_one);
    }
    dispatch(&mut ctx, &registry, "connector.reroute_all", Args::new()).expect("cycle reroute");

    // A connection point whose transform overflows, and a box whose
    // bounds overflow too: the outline fallback is unavailable.
    let (mut ctx, registry, page) = flow();
    let huge = {
        let mut shape = box_shape(50, 2.0);
        for cell in &mut shape.sheet.cells {
            if cell.name.as_str() == "PinX" {
                cell.value = Some(CellValue::Number(1.0e308));
            }
            if cell.name.as_str() == "Width" {
                cell.value = Some(CellValue::Number(1.0e308));
            }
            if cell.name.as_str() == "LocPinX" {
                cell.value = Some(CellValue::Number(0.0));
            }
        }
        for section in &mut shape.sheet.sections {
            if section.name.as_str() == "Connection"
                && let Some(row) = section.rows.first_mut()
                && let Some(cell) = row.cells.first_mut()
            {
                cell.value = Some(CellValue::Number(1.0e308));
            }
        }
        let doc = &mut ctx.active_document_mut().expect("session").document;
        doc.insert_shape(page, shape, None).expect("huge target")
    };
    let visio_id = shape_visio_id(&ctx, huge);
    dispatch(
        &mut ctx,
        &registry,
        "connector.glue_begin",
        glue_args(11, visio_id, 0),
    )
    .expect("glue huge");
    dispatch(&mut ctx, &registry, "connector.reroute_all", Args::new()).expect("overflow reroute");
    assert!(
        ctx.warnings
            .iter()
            .any(|line| line.contains("has no connection point")),
        "{:?}",
        ctx.warnings
    );
}

#[test]
fn malformed_connector_and_target_arguments_are_named() {
    let (mut ctx, registry, _page) = flow();
    let overflow = Args::new()
        .with("connector", ArgValue::Integer(i64::MAX))
        .with("target", ArgValue::Integer(1));
    let error = dispatch(&mut ctx, &registry, "connector.glue_begin", overflow).expect_err("range");
    assert!(error.to_string().contains("whole number"), "{error}");

    for name in ["connector", "target"] {
        let mut args = Args::new().with("connector", ArgValue::Integer(11));
        args.set(name, ArgValue::Flag(true));
        let error = dispatch(&mut ctx, &registry, "connector.glue_begin", args).expect_err(name);
        assert!(
            error.to_string().contains("whole number"),
            "{name}: {error}"
        );
    }

    let unglue = Args::new().with("connector", ArgValue::Flag(true));
    let error = dispatch(&mut ctx, &registry, "connector.unglue", unglue).expect_err("connector");
    assert!(error.to_string().contains("whole number"), "{error}");
}

#[test]
fn connector_commands_need_an_active_page_and_a_whole_point() {
    let (mut ctx, registry, _page) = flow();
    ctx.active_document_mut().expect("session").active_page = None;
    let error =
        dispatch(&mut ctx, &registry, "connector.reroute_all", Args::new()).expect_err("no page");
    assert!(error.to_string().contains("no active page"), "{error}");
    let error = dispatch(
        &mut ctx,
        &registry,
        "connector.glue_begin",
        glue_args(11, 1, 0),
    )
    .expect_err("no page");
    assert!(error.to_string().contains("no active page"), "{error}");

    let (mut ctx, registry, _page) = flow();
    let args = Args::new()
        .with("connector", ArgValue::Integer(11))
        .with("target", ArgValue::Integer(1))
        .with("point", ArgValue::Flag(true));
    let error = dispatch(&mut ctx, &registry, "connector.glue_begin", args).expect_err("point");
    assert!(error.to_string().contains("whole number"), "{error}");
}
