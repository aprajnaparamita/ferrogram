//! Stencil drag: dropping a master from the Shapes window onto the page (ADR 0046).
//!
//! Task P5-06 implements the tool: the drop dispatches `insert.shape` with the pointer position as `PinX`/`PinY`, the master size (the P4-06 deferred item), snapping while dragging, and the optional glue arguments of ADR 0046. The Shapes window starts the drag (`crate::shapes_window`).
//!
//! The drag lives in [`ToolMachine`]'s `stencil` field; only this module
//! reads it.

use larrygram_commands::{AppCtx, ArgValue, Args, CommandId, Invocation, SessionId, Source};
use larrygram_geom::{Angle, Point, Rect};
use larrygram_model::PageId;
use larrygram_model::stencils::{self, MasterDef};
use larrygram_render::{Overlay, OverlayMark};

use super::{Modifiers, ToolEffect, ToolEvent};
use crate::UiError;
use crate::handles::{node_entries, scene_for};
use crate::snap::Snapper;
use crate::tools::ToolMachine;

/// The smallest size a `Shift`-drag draws, in inches (one screen pixel at
/// 96 ppi).
const MIN_SIZE_IN: f64 = 1.0 / 96.0;

/// One stencil drag.
#[derive(Debug, Clone, PartialEq)]
struct Drag {
    /// The stencil display name.
    stencil: String,
    /// The master's `NameU`.
    master: String,
    /// Where the `Shift`-drag started, or `None` for a plain drop.
    anchor: Option<Point>,
    /// The snapped current pointer position, in page inches.
    current: Point,
    /// The session, once an event saw one.
    session: Option<SessionId>,
    /// The page, once an event saw one.
    page: Option<PageId>,
    /// The zoom of this drag.
    zoom: f64,
}

/// The stencil tool's state inside a [`ToolMachine`].
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct State {
    drag: Option<Drag>,
}

/// Starts a drag of one catalog master (the Shapes window's drag start).
///
/// The shell calls this when a Shapes-window row drag begins; the events
/// then go to [`handle`]. A drag with an invalid stencil or master still
/// starts (the ghost falls back to a 1 inch box) and the release reports
/// the command error, never a panic.
pub fn begin_drag(state: &mut ToolMachine, stencil: &str, master: &str) {
    state.stencil.drag = Some(Drag {
        stencil: stencil.to_owned(),
        master: master.to_owned(),
        anchor: None,
        current: Point::new(0.0, 0.0),
        session: None,
        page: None,
        zoom: 1.0,
    });
}

/// Whether `machine` has a stencil drag in flight.
pub fn is_dragging(state: &ToolMachine) -> bool {
    state.stencil.drag.is_some()
}

/// Handles one event for this tool (owner: P5-06, ADR 0042).
pub fn handle(
    state: &mut ToolMachine,
    event: &ToolEvent,
    ctx: &larrygram_commands::AppCtx,
) -> Result<super::ToolEffect, UiError> {
    let Some(session) = ctx.active_document() else {
        state.stencil.drag = None;
        return Ok(ToolEffect::new());
    };
    let Some(page) = session.active_page else {
        state.stencil.drag = None;
        return Ok(ToolEffect::new());
    };
    let zoom = if session.view.zoom.is_finite() && session.view.zoom > 0.0 {
        session.view.zoom
    } else {
        1.0
    };
    match event {
        ToolEvent::PointerDown { pos, modifiers } => {
            down(state, ctx, session.id, page, *pos, *modifiers, zoom)
        }
        ToolEvent::PointerMove { pos, modifiers } => {
            moved(state, ctx, session.id, page, *pos, *modifiers)
        }
        ToolEvent::PointerUp { pos, modifiers } => {
            up(state, ctx, session.id, page, *pos, *modifiers)
        }
        ToolEvent::DoubleClick { .. } => Ok(ToolEffect::new()),
        ToolEvent::Cancel => {
            state.stencil.drag = None;
            Ok(ToolEffect::new())
        }
    }
}

/// The drag anchor, in page inches (the preview's start).
pub fn anchor(state: &ToolMachine) -> Option<Point> {
    state.stencil.drag.as_ref().map(|drag| drag.current)
}

#[allow(clippy::too_many_arguments)] // one event seam
fn down(
    machine: &mut ToolMachine,
    ctx: &AppCtx,
    session: SessionId,
    page: PageId,
    pos: Point,
    modifiers: Modifiers,
    zoom: f64,
) -> Result<ToolEffect, UiError> {
    let mut effect = ToolEffect::new();
    let Some(mut drag) = take_matching(machine, session, page) else {
        return Ok(effect);
    };
    if !finite(pos) {
        machine.stencil.drag = Some(drag);
        return Ok(effect);
    }
    let snapped = Snapper::default().snap_point(ctx, None, pos, zoom, modifiers.alt)?;
    drag.current = snapped.point;
    drag.anchor = if modifiers.shift {
        Some(snapped.point)
    } else {
        None
    };
    drag.zoom = zoom;
    effect.overlay = preview(&drag);
    machine.stencil.drag = Some(drag);
    Ok(effect)
}

#[allow(clippy::too_many_arguments)] // one event seam
fn moved(
    machine: &mut ToolMachine,
    ctx: &AppCtx,
    session: SessionId,
    page: PageId,
    pos: Point,
    modifiers: Modifiers,
) -> Result<ToolEffect, UiError> {
    let mut effect = ToolEffect::new();
    let Some(mut drag) = take_matching(machine, session, page) else {
        return Ok(effect);
    };
    if !finite(pos) {
        machine.stencil.drag = Some(drag);
        return Ok(effect);
    }
    let snapped = Snapper::default().snap_point(ctx, None, pos, drag.zoom, modifiers.alt)?;
    drag.current = snapped.point;
    effect.overlay = preview(&drag);
    machine.stencil.drag = Some(drag);
    Ok(effect)
}

#[allow(clippy::too_many_arguments)] // one event seam
fn up(
    machine: &mut ToolMachine,
    ctx: &AppCtx,
    session: SessionId,
    page: PageId,
    pos: Point,
    modifiers: Modifiers,
) -> Result<ToolEffect, UiError> {
    let mut effect = ToolEffect::new();
    let Some(mut drag) = take_matching(machine, session, page) else {
        return Ok(effect);
    };
    if !finite(pos) {
        return Ok(effect);
    }
    let snapped = Snapper::default().snap_point(ctx, None, pos, drag.zoom, modifiers.alt)?;
    drag.current = snapped.point;
    let Some(intent) = drop_intent(ctx, page, &drag, drag.zoom)? else {
        return Ok(effect);
    };
    effect.intents.push(intent);
    Ok(effect)
}

/// Takes the drag when it belongs to this session and page.
fn take_matching(machine: &mut ToolMachine, session: SessionId, page: PageId) -> Option<Drag> {
    let matches = machine.stencil.drag.as_ref().is_some_and(|drag| {
        drag.session.is_none_or(|value| value == session)
            && drag.page.is_none_or(|value| value == page)
    });
    if !matches {
        return None;
    }
    let mut drag = machine.stencil.drag.take()?;
    drag.session = Some(session);
    drag.page = Some(page);
    Some(drag)
}

/// The preview marks of one stencil drag.
fn preview(drag: &Drag) -> Overlay {
    let mut overlay = Overlay::new();
    let (width, height) = master_size(drag).unwrap_or((1.0, 1.0));
    match drag.anchor {
        Some(anchor) => {
            let rect = normalized(anchor, drag.current);
            overlay.marks.push(OverlayMark::RubberBand(rect));
        }
        None => {
            let half_w = width.max(MIN_SIZE_IN) * 0.5;
            let half_h = height.max(MIN_SIZE_IN) * 0.5;
            overlay.marks.push(OverlayMark::SelectionOutline {
                bounds: Rect::new(
                    Point::new(drag.current.x - half_w, drag.current.y - half_h),
                    Point::new(drag.current.x + half_w, drag.current.y + half_h),
                ),
                angle: Angle::from_radians(0.0),
            });
        }
    }
    overlay
}

/// The `insert.shape` intent of one drop (ADR 0046).
fn drop_intent(
    ctx: &AppCtx,
    page: PageId,
    drag: &Drag,
    zoom: f64,
) -> Result<Option<crate::Intent>, UiError> {
    let def = master_def(&drag.stencil, &drag.master);
    let mut args = Args::new()
        .with("stencil", ArgValue::Text(drag.stencil.clone()))
        .with("master", ArgValue::Text(drag.master.clone()))
        .with("x", ArgValue::Number(drag.current.x))
        .with("y", ArgValue::Number(drag.current.y));
    if let Some(anchor) = drag.anchor {
        let rect = normalized(anchor, drag.current);
        args.set("width", ArgValue::Number(rect.width().max(MIN_SIZE_IN)));
        args.set("height", ArgValue::Number(rect.height().max(MIN_SIZE_IN)));
    }
    if let Some(def) = def
        && !def.connection_points.is_empty()
        && let Some((target, point)) = nearest_target(ctx, page, drag.current, zoom)?
    {
        args.set("glue_to", ArgValue::Integer(i64::from(target)));
        args.set(
            "glue_point",
            ArgValue::Integer(i64::try_from(point).unwrap_or(i64::MAX)),
        );
    }
    let command = CommandId::parse("insert.shape").map_err(|error| UiError::Io {
        message: format!("the insert.shape id is not valid: {error}"),
    })?;
    Ok(Some(crate::Intent::new(
        command,
        Invocation::new(args, Source::Ui),
    )))
}

/// The catalog master the drag names, or `None` for a stale pick.
fn master_def(stencil: &str, master: &str) -> Option<MasterDef> {
    stencils::catalog()
        .into_iter()
        .find(|entry| entry.name == stencil)
        .and_then(|entry| {
            entry
                .masters
                .into_iter()
                .find(|def| def.name_u == master || def.name == master)
        })
}

/// The master size, when the pick is valid.
fn master_size(drag: &Drag) -> Option<(f64, f64)> {
    master_def(&drag.stencil, &drag.master).map(|def| (def.width, def.height))
}

/// The nearest connection point of an existing page shape within the snap
/// distance, as `(Visio id, connection row IX)`.
fn nearest_target(
    ctx: &AppCtx,
    page: PageId,
    point: Point,
    zoom: f64,
) -> Result<Option<(i32, usize)>, UiError> {
    let Some(session) = ctx.active_document() else {
        return Ok(None);
    };
    if session.active_page != Some(page) {
        return Ok(None);
    }
    let Some(scene) = scene_for(ctx)? else {
        return Ok(None);
    };
    let max_distance = Snapper::default().settings.distance_px
        / (if zoom.is_finite() && zoom > 0.0 {
            zoom
        } else {
            1.0
        })
        / crate::canvas::POINTS_PER_INCH;
    let mut best: Option<(i32, usize, f64)> = None;
    for entry in node_entries(&scene) {
        let Some(shape) = session.document.shape(entry.shape) else {
            continue;
        };
        for (ix, local) in session.document.connection_points_indexed(entry.shape) {
            let candidate = entry.transform.apply(local);
            let distance = candidate.sub(point).x.hypot(candidate.sub(point).y);
            if !distance.is_finite() || distance > max_distance {
                continue;
            }
            if best.is_none_or(|(_, _, best)| distance < best) {
                let ix = usize::try_from(ix).unwrap_or(usize::MAX);
                best = Some((shape.visio_id, ix, distance));
            }
        }
    }
    Ok(best.map(|(id, ix, _)| (id, ix)))
}

/// The axis-aligned rectangle between two points.
fn normalized(a: Point, b: Point) -> Rect {
    Rect::new(
        Point::new(a.x.min(b.x), a.y.min(b.y)),
        Point::new(a.x.max(b.x), a.y.max(b.y)),
    )
}

/// Whether both coordinates are finite.
fn finite(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;
    use larrygram_commands::{DocumentSession, Settings};
    use larrygram_model::{
        Cell, CellName, CellValue, Document, Page, PageId, Preserved, Row, Section, SectionName,
        Shape, ShapeId,
    };
    use larrygram_render::OverlayMark;

    fn cell(name: &str, value: f64) -> Cell {
        Cell {
            name: CellName::new(name),
            formula: Some(format!("{value}")),
            value: Some(CellValue::Number(value)),
            unit: larrygram_geom::Unit::Inches,
            preserved: Preserved::new(),
        }
    }

    fn fixture() -> AppCtx {
        let mut doc = Document::new();
        let mut shape = Shape::new(ShapeId { index: 0 }, 1);
        shape.sheet.cells = vec![
            cell("Width", 2.0),
            cell("Height", 1.0),
            cell("PinX", 3.0),
            cell("PinY", 3.0),
            cell("LocPinX", 1.0),
            cell("LocPinY", 0.5),
            cell("Angle", 0.0),
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
            row.ix = Some(ix as i32);
            row.cells = vec![cell("X", x), cell("Y", y)];
            geometry.rows.push(row);
        }
        shape.sheet.sections.push(geometry);
        let mut connection = Section::new(SectionName::from_static("Connection"));
        let mut row = Row::new();
        row.ix = Some(0);
        row.cells = vec![cell("X", 1.0), cell("Y", 0.5)];
        connection.rows.push(row);
        shape.sheet.sections.push(connection);
        let id = doc.add_shape(shape);
        let mut page = Page::new(PageId { index: 0 }, 1);
        page.name = "Page-1".to_owned();
        page.sheet.cells = vec![cell("PageWidth", 8.5), cell("PageHeight", 11.0)];
        page.shapes.push(id);
        doc.pages.push(page);
        let mut app = AppCtx::new(Settings::default());
        app.open_document(DocumentSession::new(doc, None));
        app
    }

    fn down(pos: Point, modifiers: Modifiers) -> ToolEvent {
        ToolEvent::PointerDown { pos, modifiers }
    }

    fn moved(pos: Point, modifiers: Modifiers) -> ToolEvent {
        ToolEvent::PointerMove { pos, modifiers }
    }

    fn up(pos: Point, modifiers: Modifiers) -> ToolEvent {
        ToolEvent::PointerUp { pos, modifiers }
    }

    #[test]
    fn a_plain_drop_emits_insert_shape_at_the_pointer() {
        let ctx = fixture();
        let mut machine = ToolMachine::new();
        begin_drag(&mut machine, "Basic Shapes", "Rectangle");
        assert!(is_dragging(&machine));
        let _ = handle(
            &mut machine,
            &down(Point::new(6.0, 8.0), Modifiers::none()),
            &ctx,
        )
        .expect("down");
        let effect = handle(
            &mut machine,
            &moved(Point::new(6.0, 8.0), Modifiers::none()),
            &ctx,
        )
        .expect("move");
        assert!(matches!(
            effect.overlay.marks.first(),
            Some(OverlayMark::SelectionOutline { .. })
        ));
        let effect = handle(
            &mut machine,
            &up(Point::new(6.0, 8.0), Modifiers::none()),
            &ctx,
        )
        .expect("up");
        assert_eq!(effect.intents.len(), 1);
        let intent = &effect.intents[0];
        assert_eq!(intent.command.as_str(), "insert.shape");
        assert_eq!(intent.invocation.args.text("stencil"), Some("Basic Shapes"));
        assert_eq!(intent.invocation.args.text("master"), Some("Rectangle"));
        assert_eq!(intent.invocation.args.number("x"), Some(6.0));
        assert_eq!(intent.invocation.args.number("y"), Some(8.0));
        assert_eq!(intent.invocation.args.number("width"), None);
        assert!(!is_dragging(&machine));
        assert_eq!(anchor(&machine), None);
    }

    #[test]
    fn a_shift_drag_sets_the_drawn_size() {
        let ctx = fixture();
        let mut machine = ToolMachine::new();
        begin_drag(&mut machine, "Basic Shapes", "Rectangle");
        let shift = Modifiers {
            shift: true,
            ..Modifiers::none()
        };
        let _ = handle(&mut machine, &down(Point::new(5.0, 6.0), shift), &ctx).expect("down");
        let effect = handle(&mut machine, &moved(Point::new(7.0, 8.0), shift), &ctx).expect("move");
        assert!(matches!(
            effect.overlay.marks.first(),
            Some(OverlayMark::RubberBand(_))
        ));
        let effect = handle(&mut machine, &up(Point::new(7.0, 8.0), shift), &ctx).expect("up");
        let args = &effect.intents[0].invocation.args;
        assert_eq!(args.number("width"), Some(2.0));
        assert_eq!(args.number("height"), Some(2.0));
        assert_eq!(args.number("x"), Some(7.0));
        assert_eq!(args.number("y"), Some(8.0));
    }

    #[test]
    fn a_drop_near_a_connection_point_carries_the_glue_arguments() {
        let ctx = fixture();
        let mut machine = ToolMachine::new();
        begin_drag(&mut machine, "Basic Shapes", "Rectangle");
        // The fixture's connection point is the shape's pin at (3, 3).
        let _ = handle(
            &mut machine,
            &down(Point::new(3.02, 3.02), Modifiers::none()),
            &ctx,
        )
        .expect("down");
        let effect = handle(
            &mut machine,
            &up(Point::new(3.02, 3.02), Modifiers::none()),
            &ctx,
        )
        .expect("up");
        let args = &effect.intents[0].invocation.args;
        assert_eq!(args.integer("glue_to"), Some(1));
        assert_eq!(args.integer("glue_point"), Some(0));
    }

    #[test]
    fn a_far_drop_has_no_glue_arguments() {
        let ctx = fixture();
        let mut machine = ToolMachine::new();
        begin_drag(&mut machine, "Basic Shapes", "Rectangle");
        let _ = handle(
            &mut machine,
            &down(Point::new(6.0, 8.0), Modifiers::none()),
            &ctx,
        )
        .expect("down");
        let effect = handle(
            &mut machine,
            &up(Point::new(6.0, 8.0), Modifiers::none()),
            &ctx,
        )
        .expect("up");
        assert_eq!(effect.intents[0].invocation.args.integer("glue_to"), None);
    }

    #[test]
    fn a_stale_pick_still_dispatches_and_lets_the_command_report_it() {
        let ctx = fixture();
        let mut machine = ToolMachine::new();
        begin_drag(&mut machine, "No Such Stencil", "No Such Master");
        let _ = handle(
            &mut machine,
            &down(Point::new(6.0, 8.0), Modifiers::none()),
            &ctx,
        )
        .expect("down");
        let effect = handle(
            &mut machine,
            &up(Point::new(6.0, 8.0), Modifiers::none()),
            &ctx,
        )
        .expect("up");
        assert_eq!(effect.intents.len(), 1);
        assert_eq!(
            effect.intents[0].invocation.args.text("stencil"),
            Some("No Such Stencil")
        );
    }

    #[test]
    fn events_without_a_drag_are_harmless() {
        let ctx = fixture();
        let mut machine = ToolMachine::new();
        assert!(
            handle(
                &mut machine,
                &down(Point::new(1.0, 1.0), Modifiers::none()),
                &ctx
            )
            .expect("down")
            .intents
            .is_empty()
        );
        assert_eq!(anchor(&machine), None);
    }

    #[test]
    fn cancel_drops_the_drag() {
        let ctx = fixture();
        let mut machine = ToolMachine::new();
        begin_drag(&mut machine, "Basic Shapes", "Rectangle");
        let _ = handle(
            &mut machine,
            &down(Point::new(6.0, 8.0), Modifiers::none()),
            &ctx,
        )
        .expect("down");
        let _ = handle(&mut machine, &ToolEvent::Cancel, &ctx).expect("cancel");
        assert!(!is_dragging(&machine));
    }

    #[test]
    fn the_master_size_of_a_known_pick_is_the_catalog_size() {
        let drag = Drag {
            stencil: "Basic Shapes".to_owned(),
            master: "Rectangle".to_owned(),
            anchor: None,
            current: Point::new(0.0, 0.0),
            session: None,
            page: None,
            zoom: 1.0,
        };
        let size = master_size(&drag).expect("size");
        let def = master_def("Basic Shapes", "Rectangle").expect("def");
        assert_eq!(size, (def.width, def.height));
    }

    #[test]
    fn a_non_finite_pointer_leaves_the_drag_intact() {
        let ctx = fixture();
        let mut machine = ToolMachine::new();
        begin_drag(&mut machine, "Basic Shapes", "Rectangle");
        let _ = handle(
            &mut machine,
            &down(Point::new(6.0, 8.0), Modifiers::none()),
            &ctx,
        )
        .expect("down");
        let _ = handle(
            &mut machine,
            &moved(Point::new(f64::NAN, 8.0), Modifiers::none()),
            &ctx,
        )
        .expect("move");
        assert!(is_dragging(&machine));
        let effect = handle(
            &mut machine,
            &up(Point::new(f64::NAN, 8.0), Modifiers::none()),
            &ctx,
        )
        .expect("up");
        assert!(effect.intents.is_empty());
    }
}
