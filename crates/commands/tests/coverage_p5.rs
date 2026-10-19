//! Cross-cutting command tests for task P5-18 (ADR 0049 rule 1).
//!
//! The commands here do not fit the per-feature test files: page
//! navigation without an active page, `view.zoom.fit` over page-sheet
//! cells that are not usable numbers, and the "connectors were not
//! re-routed" warning path shared by the editing commands, undo and
//! redo.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use ferrogram_commands::app::{AppCtx, DocumentSession, Settings};
use ferrogram_commands::builtins::registry;
use ferrogram_commands::{
    ArgValue, Args, CommandError, CommandId, Invocation, Outcome, Registry, Source,
};
use ferrogram_model::{
    Cell, CellName, CellValue, Connect, Document, PageId, Preserved, Row, Section, SectionName,
    Shape, ShapeId,
};

fn number_cell(name: &str, value: f64) -> Cell {
    Cell {
        name: CellName::new(name),
        formula: Some(format!("{value}")),
        value: Some(CellValue::Number(value)),
        unit: ferrogram_geom::Unit::Inches,
        preserved: Preserved::new(),
    }
}

/// One 2 x 1 shape at `(x, 0)` with one connection point at its centre.
fn box_shape(visio_id: i32, x: f64) -> Shape {
    let mut shape = Shape::new(ShapeId { index: 0 }, visio_id);
    shape.sheet.cells = vec![
        number_cell("Width", 2.0),
        number_cell("Height", 1.0),
        number_cell("PinX", x),
        number_cell("PinY", 0.0),
        number_cell("LocPinX", 1.0),
        number_cell("LocPinY", 0.5),
        number_cell("Angle", 0.0),
    ];
    let mut connection = Section::new(SectionName::from_static("Connection"));
    let mut row = Row::new();
    row.row_type = Some("Connection".to_owned());
    row.ix = Some(0);
    row.cells = vec![number_cell("X", 1.0), number_cell("Y", 0.5)];
    connection.rows.push(row);
    shape.sheet.sections.push(connection);
    shape
}

/// A connector box at `(x, 0)` with a finite two-point path.
fn connector_shape(visio_id: i32, x: f64) -> Shape {
    let mut shape = box_shape(visio_id, x);
    shape
        .sheet
        .sections
        .retain(|section| section.name.as_str() != "Connection");
    let mut geometry = Section::new(SectionName::from_static("Geometry"));
    for (ix, (kind, px, py)) in [("MoveTo", 0.0, 0.0), ("LineTo", 2.0, 0.0)]
        .into_iter()
        .enumerate()
    {
        let mut row = Row::new();
        row.row_type = Some(kind.to_owned());
        row.ix = Some(i32::try_from(ix).unwrap_or(0));
        row.cells = vec![number_cell("X", px), number_cell("Y", py)];
        geometry.rows.push(row);
    }
    shape.sheet.sections.push(geometry);
    shape
}

/// A page with two boxes and one connector; ids 1, 2 and 3.
fn fixture() -> (AppCtx, Registry, PageId) {
    let mut doc = Document::new();
    let page = doc.add_page("Page-1").expect("page");
    for shape in [
        box_shape(1, 1.0),
        box_shape(2, 5.0),
        connector_shape(3, 3.0),
    ] {
        doc.insert_shape(page, shape, None).expect("shape");
    }
    let mut ctx = AppCtx::new(Settings::default());
    ctx.open_document(DocumentSession::new(doc, None));
    (ctx, registry().expect("registry"), page)
}

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

fn select(ctx: &mut AppCtx, page: PageId, visio_ids: &[i32]) {
    let doc = &ctx.active_document().expect("session").document;
    let shapes: Vec<ShapeId> = visio_ids
        .iter()
        .filter_map(|id| doc.shape_by_visio_id(page, *id))
        .collect();
    ctx.selection.anchor = shapes.last().copied();
    ctx.selection.shapes = shapes;
    ctx.selection.page = Some(page);
}

fn glue_args(connector: i32, target: i32) -> Args {
    Args::new()
        .with("connector", ArgValue::Integer(i64::from(connector)))
        .with("target", ArgValue::Integer(i64::from(target)))
        .with("point", ArgValue::Integer(0))
}

#[test]
fn page_navigation_without_an_active_page_starts_at_an_end() {
    let (mut ctx, registry, _page) = fixture();
    ctx.active_document_mut().expect("session").active_page = None;
    let next = dispatch(&mut ctx, &registry, "page.next", Args::new()).expect("next");
    assert_eq!(next.message.as_deref(), Some("Page-1"));
    assert_eq!(
        ctx.active_document().expect("session").active_page,
        Some(PageId { index: 0 })
    );
    ctx.active_document_mut().expect("session").active_page = None;
    let previous = dispatch(&mut ctx, &registry, "page.previous", Args::new()).expect("previous");
    assert_eq!(previous.message.as_deref(), Some("Page-1"));
}

#[test]
fn zoom_fit_ignores_unusable_page_sheet_numbers() {
    let (mut ctx, registry, page) = fixture();
    {
        let document = &mut ctx.active_document_mut().expect("session").document;
        let page = document.pages.get_mut(page.index as usize).expect("page");
        page.sheet.cells = vec![
            Cell {
                name: CellName::new("PageWidth"),
                formula: Some("0".to_owned()),
                value: Some(CellValue::Number(0.0)),
                unit: ferrogram_geom::Unit::Inches,
                preserved: Preserved::new(),
            },
            Cell {
                name: CellName::new("PageHeight"),
                formula: Some("1/0".to_owned()),
                value: Some(CellValue::Error("#DIV/0!".to_owned())),
                unit: ferrogram_geom::Unit::Inches,
                preserved: Preserved::new(),
            },
        ];
    }
    let outcome = dispatch(
        &mut ctx,
        &registry,
        "view.zoom.fit",
        Args::new()
            .with("width", ArgValue::Number(612.0))
            .with("height", ArgValue::Number(792.0)),
    )
    .expect("fit");
    assert_eq!(outcome.message.as_deref(), Some("Zoom 100%"));
    let zoom = ctx.active_document().expect("session").view.zoom;
    assert!((zoom - 1.0).abs() < 1.0e-9, "{zoom}");
}

/// Pin the connector's transform so its path maps through infinity.
fn corrupt_connector_transform(ctx: &mut AppCtx, page: PageId) {
    let connector = ctx
        .active_document()
        .expect("session")
        .document
        .shape_by_visio_id(page, 3)
        .expect("connector");
    let shape = ctx
        .active_document_mut()
        .expect("session")
        .document
        .shape_mut(connector)
        .expect("connector");
    for cell in &mut shape.sheet.cells {
        match cell.name.as_str() {
            "PinX" => cell.value = Some(CellValue::Number(1.7e308)),
            "LocPinX" => cell.value = Some(CellValue::Number(-1.7e308)),
            _ => {}
        }
    }
}

#[test]
fn a_route_error_is_reported_by_reroute_all_and_never_panics() {
    let (mut ctx, registry, page) = fixture();
    dispatch(&mut ctx, &registry, "connector.glue_begin", glue_args(3, 1)).expect("glue");
    corrupt_connector_transform(&mut ctx, page);
    let error = dispatch(&mut ctx, &registry, "connector.reroute_all", Args::new())
        .expect_err("non-finite route");
    assert!(
        error.to_string().contains("cannot route connector 3"),
        "{error}"
    );
}

#[test]
fn an_edit_that_cannot_re_route_still_succeeds_with_one_warning() {
    let (mut ctx, registry, page) = fixture();
    dispatch(&mut ctx, &registry, "connector.glue_begin", glue_args(3, 1)).expect("glue");
    corrupt_connector_transform(&mut ctx, page);
    select(&mut ctx, page, &[2]);
    dispatch(
        &mut ctx,
        &registry,
        "shape.move",
        Args::new()
            .with("dx", ArgValue::Number(1.0))
            .with("dy", ArgValue::Number(0.5)),
    )
    .expect("move still succeeds");
    assert!(
        ctx.warnings.iter().any(|warning| warning
            .starts_with("connectors were not re-routed: cannot route connector 3:")),
        "{:?}",
        ctx.warnings
    );
    let doc = &ctx.active_document().expect("session").document;
    let pin = doc
        .shape_by_visio_id(page, 2)
        .and_then(|shape| doc.shape(shape))
        .and_then(|shape| {
            shape
                .sheet
                .cells
                .iter()
                .find(|cell| cell.name.as_str() == "PinX")
        })
        .and_then(|cell| match cell.value {
            Some(CellValue::Number(value)) => Some(value),
            _ => None,
        });
    assert_eq!(pin, Some(6.0));

    let warnings_before = ctx.warnings.len();
    dispatch(&mut ctx, &registry, "edit.delete", Args::new()).expect("delete still succeeds");
    assert_eq!(
        ctx.warnings.len(),
        warnings_before + 1,
        "{:?}",
        ctx.warnings
    );
    assert!(
        ctx.warnings
            .last()
            .is_some_and(|warning| warning.starts_with("connectors were not re-routed:")),
        "{:?}",
        ctx.warnings
    );

    let warnings_before = ctx.warnings.len();
    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(
        ctx.warnings.len(),
        warnings_before + 1,
        "{:?}",
        ctx.warnings
    );
    assert!(
        ctx.warnings
            .last()
            .is_some_and(|warning| warning.starts_with("connectors were not re-routed:")),
        "{:?}",
        ctx.warnings
    );
    assert_eq!(
        ctx.active_document().expect("session").document.page_of(
            ctx.active_document()
                .expect("session")
                .document
                .shape_by_visio_id(page, 2)
                .expect("shape 2")
        ),
        Some(page),
        "undo restored the deleted shape"
    );
}

#[test]
fn a_glue_revert_without_its_page_is_a_no_op() {
    let (mut ctx, registry, page) = fixture();
    dispatch(&mut ctx, &registry, "connector.glue_begin", glue_args(3, 1)).expect("glue");
    ctx.active_document_mut()
        .expect("session")
        .document
        .pages
        .clear();
    let outcome = dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(
        outcome.message.as_deref(),
        Some("Undo Glue connector begin")
    );
    assert!(
        ctx.active_document()
            .expect("session")
            .document
            .pages
            .is_empty()
    );
    let _ = page;
}

#[test]
fn a_connect_record_for_a_gone_connector_is_dropped_with_a_warning() {
    let (mut ctx, registry, page) = fixture();
    let mut stale = Connect::new();
    stale.from_sheet = 99;
    stale.from_cell = "BeginX".to_owned();
    stale.from_part = ferrogram_model::mutate::VIS_BEGIN;
    stale.to_sheet = 1;
    stale.to_cell = "PinX".to_owned();
    stale.to_part = ferrogram_model::mutate::VIS_WHOLE_SHAPE;
    ctx.active_document_mut()
        .expect("session")
        .document
        .add_connect(page, stale)
        .expect("connect");
    let outcome =
        dispatch(&mut ctx, &registry, "connector.reroute_all", Args::new()).expect("reroute");
    assert_eq!(outcome.message.as_deref(), Some("Re-routed 0 connectors"));
    assert!(
        ctx.warnings
            .iter()
            .any(|line| line.contains("connector 99 is gone")),
        "{:?}",
        ctx.warnings
    );
    assert!(
        ctx.active_document()
            .expect("session")
            .document
            .page(page)
            .expect("page")
            .connects
            .is_empty()
    );
}

#[test]
fn zoom_fit_names_a_missing_or_stale_active_page() {
    let (mut ctx, registry, _page) = fixture();
    let args = || {
        Args::new()
            .with("width", ArgValue::Number(612.0))
            .with("height", ArgValue::Number(792.0))
    };
    ctx.active_document_mut().expect("session").active_page = None;
    let error = dispatch(&mut ctx, &registry, "view.zoom.fit", args()).expect_err("no page");
    assert!(error.to_string().contains("no active page"), "{error}");
    ctx.active_document_mut().expect("session").active_page = Some(PageId { index: 9 });
    let error = dispatch(&mut ctx, &registry, "view.zoom.fit", args()).expect_err("stale page");
    assert!(error.to_string().contains("no active page"), "{error}");
}
