//! The drawing tools (P5-06): rectangle, ellipse and line (ADR 0042).
//!
//! Task P5-06 implements the tools: a drag previews the shape on the
//! overlay and commits one `insert.drawing.*` intent (undoable, ADR 0040)
//! with snapping applied (ADR 0043).
//!
//! The tool keeps its drag anchor in a thread-local slot keyed by the
//! [`ToolMachine`] address. `ToolMachine` owns the pointer tool's state
//! only (`crates/ui/src/tools/mod.rs` is outside this task's lane), so the
//! drawing tools cannot add a field to it; a UI process has one machine
//! and one pointer, which the key keeps honest across test threads. The
//! shell wiring for the tool buttons is requested in the task Result.

use std::cell::RefCell;

use ferrogram_commands::{AppCtx, ArgValue, Args, CommandId, Invocation, SessionId, Source};
use ferrogram_geom::{Point, Rect};
use ferrogram_model::PageId;
use ferrogram_render::{Overlay, OverlayMark};

use super::{Modifiers, Tool, ToolEffect, ToolEvent};
use crate::UiError;
use crate::snap::Snapper;
use crate::tools::ToolMachine;

/// The smallest shape a drag draws, in inches (one screen pixel at 96 ppi).
const MIN_SIZE_IN: f64 = 1.0 / 96.0;

/// One drawing drag.
#[derive(Debug, Clone, PartialEq)]
struct Drag {
    /// The machine that owns the drag (the thread-local state is shared
    /// by every machine on one thread).
    machine: usize,
    /// The active drawing tool.
    tool: Tool,
    /// The snapped drag anchor, in page inches.
    start: Point,
    /// The snapped current pointer position, in page inches.
    current: Point,
    /// The session of the drag.
    session: SessionId,
    /// The page of the drag.
    page: PageId,
    /// The zoom at the start (screen-pixel snapping).
    zoom: f64,
    /// Alt was held at the start (snapping off).
    alt: bool,
}

#[derive(Debug, Default)]
struct State {
    drag: Option<Drag>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// Runs `f` with the per-thread drawing state.
fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|cell| f(&mut cell.borrow_mut()))
}

/// The stable key of one machine for the thread-local state.
fn machine_key(state: &ToolMachine) -> usize {
    state as *const ToolMachine as usize
}

/// Handles one event for the active drawing tool (owner: P5-06, ADR 0042).
pub fn handle(
    state: &mut ToolMachine,
    event: &ToolEvent,
    ctx: &ferrogram_commands::AppCtx,
) -> Result<super::ToolEffect, UiError> {
    let tool = state.tool;
    if !matches!(tool, Tool::Rectangle | Tool::Ellipse | Tool::Line) {
        clear_machine(state);
        return Ok(ToolEffect::new());
    }
    clear_other_tool(state, tool);
    let Some(session) = ctx.active_document() else {
        clear_machine(state);
        return Ok(ToolEffect::new());
    };
    let Some(page) = session.active_page else {
        clear_machine(state);
        return Ok(ToolEffect::new());
    };
    let zoom = if session.view.zoom.is_finite() && session.view.zoom > 0.0 {
        session.view.zoom
    } else {
        1.0
    };
    let key = machine_key(state);
    match event {
        ToolEvent::PointerDown { pos, modifiers } => down(
            state, ctx, key, tool, session.id, page, *pos, *modifiers, zoom,
        ),
        ToolEvent::PointerMove { pos, modifiers } => {
            moved(state, ctx, key, tool, session.id, page, *pos, *modifiers)
        }
        ToolEvent::PointerUp { pos, modifiers } => {
            up(state, ctx, key, tool, session.id, page, *pos, *modifiers)
        }
        ToolEvent::DoubleClick { .. } => Ok(ToolEffect::new()),
        ToolEvent::Cancel => {
            clear_machine(state);
            Ok(ToolEffect::new())
        }
    }
}

/// The drag anchor, in page inches (the preview's first corner).
pub fn anchor(state: &ToolMachine) -> Option<Point> {
    let key = machine_key(state);
    with_state(|state| {
        state
            .drag
            .as_ref()
            .filter(|drag| drag.machine == key)
            .map(|drag| drag.start)
    })
}

/// Drops a drag left over from another drawing tool.
fn clear_other_tool(machine: &ToolMachine, tool: Tool) {
    let key = machine_key(machine);
    with_state(|state| {
        if state
            .drag
            .as_ref()
            .is_some_and(|drag| drag.machine == key && drag.tool != tool)
        {
            state.drag = None;
        }
    });
}

/// Drops the thread-local drag of `machine`.
fn clear_machine(machine: &ToolMachine) {
    let key = machine_key(machine);
    with_state(|state| {
        if state.drag.as_ref().is_some_and(|drag| drag.machine == key) {
            state.drag = None;
        }
    });
}

#[allow(clippy::too_many_arguments)] // one event seam
fn down(
    machine: &mut ToolMachine,
    ctx: &AppCtx,
    key: usize,
    tool: Tool,
    session: SessionId,
    page: PageId,
    pos: Point,
    modifiers: Modifiers,
    zoom: f64,
) -> Result<ToolEffect, UiError> {
    if !finite(pos) {
        clear_machine(machine);
        return Ok(ToolEffect::new());
    }
    let snapped = Snapper::default().snap_point(ctx, pos, zoom, modifiers.alt)?;
    let drag = Drag {
        machine: key,
        tool,
        start: snapped.point,
        current: snapped.point,
        session,
        page,
        zoom,
        alt: modifiers.alt,
    };
    let overlay = preview(&drag);
    with_state(|state| state.drag = Some(drag));
    Ok(ToolEffect {
        intents: Vec::new(),
        overlay,
    })
}

#[allow(clippy::too_many_arguments)] // one event seam
fn moved(
    machine: &mut ToolMachine,
    ctx: &AppCtx,
    key: usize,
    tool: Tool,
    session: SessionId,
    page: PageId,
    pos: Point,
    modifiers: Modifiers,
) -> Result<ToolEffect, UiError> {
    let mut effect = ToolEffect::new();
    let Some(mut drag) = take_matching(machine, key, tool, session, page) else {
        return Ok(effect);
    };
    if !finite(pos) {
        with_state(|state| state.drag = Some(drag));
        return Ok(effect);
    }
    let snapped = Snapper::default().snap_point(ctx, pos, drag.zoom, modifiers.alt)?;
    drag.current = snapped.point;
    effect.overlay = preview(&drag);
    with_state(|state| state.drag = Some(drag));
    Ok(effect)
}

#[allow(clippy::too_many_arguments)] // one event seam
fn up(
    machine: &mut ToolMachine,
    ctx: &AppCtx,
    key: usize,
    tool: Tool,
    session: SessionId,
    page: PageId,
    pos: Point,
    modifiers: Modifiers,
) -> Result<ToolEffect, UiError> {
    let mut effect = ToolEffect::new();
    let Some(mut drag) = take_matching(machine, key, tool, session, page) else {
        return Ok(effect);
    };
    if !finite(pos) {
        return Ok(effect);
    }
    let snapped = Snapper::default().snap_point(ctx, pos, drag.zoom, modifiers.alt)?;
    drag.current = snapped.point;
    match drawing_intent(tool, drag.start, drag.current)? {
        Some(intent) => effect.intents.push(intent),
        None => {
            // A click with no drag draws nothing; the overlay still
            // clears on this release.
        }
    }
    Ok(effect)
}

/// Takes the drag when it belongs to this machine, tool, session and page.
fn take_matching(
    machine: &ToolMachine,
    key: usize,
    tool: Tool,
    session: SessionId,
    page: PageId,
) -> Option<Drag> {
    let _ = machine;
    with_state(|state| {
        let matches = state.drag.as_ref().is_some_and(|drag| {
            drag.machine == key && drag.tool == tool && drag.session == session && drag.page == page
        });
        if matches { state.drag.take() } else { None }
    })
}

/// The preview marks of one drag (ADR 0042).
fn preview(drag: &Drag) -> Overlay {
    let mut overlay = Overlay::new();
    match drag.tool {
        Tool::Line if drag.start != drag.current => {
            overlay.marks.push(OverlayMark::ConnectorPreview {
                points: vec![drag.start, drag.current],
            });
        }
        Tool::Line => {}
        Tool::Rectangle | Tool::Ellipse => {
            let rect = normalized(drag.start, drag.current);
            if rect.width() > 0.0 || rect.height() > 0.0 {
                overlay.marks.push(OverlayMark::RubberBand(rect));
            }
        }
        _ => {}
    }
    overlay
}

/// The command intent of one completed drawing drag.
fn drawing_intent(
    tool: Tool,
    start: Point,
    current: Point,
) -> Result<Option<crate::Intent>, UiError> {
    let (id, args) = match tool {
        Tool::Rectangle | Tool::Ellipse => {
            let rect = normalized(start, current);
            if rect.width() < MIN_SIZE_IN && rect.height() < MIN_SIZE_IN {
                return Ok(None);
            }
            let id = if tool == Tool::Rectangle {
                "insert.drawing.rectangle"
            } else {
                "insert.drawing.ellipse"
            };
            (
                id,
                Args::new()
                    .with("x", ArgValue::Number(rect.min.x))
                    .with("y", ArgValue::Number(rect.min.y))
                    .with("width", ArgValue::Number(rect.width().max(MIN_SIZE_IN)))
                    .with("height", ArgValue::Number(rect.height().max(MIN_SIZE_IN))),
            )
        }
        Tool::Line => {
            let width = current.x - start.x;
            let height = current.y - start.y;
            if width.abs() < MIN_SIZE_IN && height.abs() < MIN_SIZE_IN {
                return Ok(None);
            }
            (
                "insert.drawing.line",
                Args::new()
                    .with("x", ArgValue::Number(start.x))
                    .with("y", ArgValue::Number(start.y))
                    .with("width", ArgValue::Number(width))
                    .with("height", ArgValue::Number(height)),
            )
        }
        _ => return Ok(None),
    };
    let command = CommandId::parse(id).map_err(|error| UiError::Io {
        message: format!("the drawing command id {id} is not valid: {error}"),
    })?;
    Ok(Some(crate::Intent::new(
        command,
        Invocation::new(args, Source::Ui),
    )))
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
    use ferrogram_commands::{DocumentSession, Settings};
    use ferrogram_model::{Cell, CellName, CellValue, Document, Page, PageId};

    fn fixture() -> AppCtx {
        let mut doc = Document::new();
        let mut page = Page::new(PageId { index: 0 }, 1);
        page.name = "Page-1".to_owned();
        page.sheet.cells = vec![
            Cell {
                name: CellName::new("PageWidth"),
                formula: Some("8.5".to_owned()),
                value: Some(CellValue::Number(8.5)),
                unit: ferrogram_geom::Unit::Inches,
                preserved: Default::default(),
            },
            Cell {
                name: CellName::new("PageHeight"),
                formula: Some("11".to_owned()),
                value: Some(CellValue::Number(11.0)),
                unit: ferrogram_geom::Unit::Inches,
                preserved: Default::default(),
            },
        ];
        doc.pages.push(page);
        let mut app = AppCtx::new(Settings::default());
        app.open_document(DocumentSession::new(doc, None));
        app
    }

    fn machine(tool: Tool) -> ToolMachine {
        let mut machine = ToolMachine::new();
        machine.tool = tool;
        machine
    }

    fn down(pos: Point) -> ToolEvent {
        ToolEvent::PointerDown {
            pos,
            modifiers: Modifiers::none(),
        }
    }

    fn moved(pos: Point) -> ToolEvent {
        ToolEvent::PointerMove {
            pos,
            modifiers: Modifiers::none(),
        }
    }

    fn up(pos: Point) -> ToolEvent {
        ToolEvent::PointerUp {
            pos,
            modifiers: Modifiers::none(),
        }
    }

    #[test]
    fn a_rectangle_drag_emits_one_insert_intent_and_a_preview() {
        let ctx = fixture();
        let mut machine = machine(Tool::Rectangle);
        let _ = handle(&mut machine, &down(Point::new(1.0, 1.0)), &ctx).expect("down");
        assert_eq!(anchor(&machine), Some(Point::new(1.0, 1.0)));
        let effect = handle(&mut machine, &moved(Point::new(3.0, 2.0)), &ctx).expect("move");
        assert!(matches!(
            effect.overlay.marks.first(),
            Some(OverlayMark::RubberBand(_))
        ));
        let effect = handle(&mut machine, &up(Point::new(3.0, 2.0)), &ctx).expect("up");
        assert_eq!(effect.intents.len(), 1);
        let intent = &effect.intents[0];
        assert_eq!(intent.command.as_str(), "insert.drawing.rectangle");
        assert_eq!(intent.invocation.args.number("x"), Some(1.0));
        assert_eq!(intent.invocation.args.number("width"), Some(2.0));
        assert_eq!(anchor(&machine), None);
    }

    #[test]
    fn a_rectangle_drag_is_normalized_in_every_direction() {
        let ctx = fixture();
        for (start, end) in [
            (Point::new(3.0, 2.0), Point::new(1.0, 1.0)),
            (Point::new(1.0, 2.0), Point::new(3.0, 1.0)),
            (Point::new(3.0, 1.0), Point::new(1.0, 2.0)),
        ] {
            let mut machine = machine(Tool::Rectangle);
            let _ = handle(&mut machine, &down(start), &ctx).expect("down");
            let effect = handle(&mut machine, &up(end), &ctx).expect("up");
            let args = &effect.intents[0].invocation.args;
            assert_eq!(args.number("x"), Some(1.0));
            assert_eq!(args.number("y"), Some(1.0));
            assert_eq!(args.number("width"), Some(2.0));
            assert_eq!(args.number("height"), Some(1.0));
        }
    }

    #[test]
    fn an_ellipse_drag_emits_the_ellipse_command() {
        let ctx = fixture();
        let mut machine = machine(Tool::Ellipse);
        let _ = handle(&mut machine, &down(Point::new(1.0, 1.0)), &ctx).expect("down");
        let effect = handle(&mut machine, &up(Point::new(2.0, 2.0)), &ctx).expect("up");
        assert_eq!(effect.intents.len(), 1);
        assert_eq!(effect.intents[0].command.as_str(), "insert.drawing.ellipse");
    }

    #[test]
    fn a_line_drag_keeps_the_start_and_the_signed_delta() {
        let ctx = fixture();
        let mut machine = machine(Tool::Line);
        let _ = handle(&mut machine, &down(Point::new(3.0, 2.0)), &ctx).expect("down");
        let effect = handle(&mut machine, &moved(Point::new(2.0, 1.0)), &ctx).expect("move");
        assert!(matches!(
            effect.overlay.marks.first(),
            Some(OverlayMark::ConnectorPreview { .. })
        ));
        let effect = handle(&mut machine, &up(Point::new(2.0, 1.0)), &ctx).expect("up");
        let args = &effect.intents[0].invocation.args;
        assert_eq!(effect.intents[0].command.as_str(), "insert.drawing.line");
        assert_eq!(args.number("x"), Some(3.0));
        assert_eq!(args.number("y"), Some(2.0));
        assert_eq!(args.number("width"), Some(-1.0));
        assert_eq!(args.number("height"), Some(-1.0));
    }

    #[test]
    fn a_click_draws_nothing_and_cancel_clears_the_drag() {
        let ctx = fixture();
        let mut machine = machine(Tool::Rectangle);
        let _ = handle(&mut machine, &down(Point::new(1.0, 1.0)), &ctx).expect("down");
        let effect = handle(&mut machine, &up(Point::new(1.0, 1.0)), &ctx).expect("up");
        assert!(effect.intents.is_empty());

        let _ = handle(&mut machine, &down(Point::new(1.0, 1.0)), &ctx).expect("down");
        let effect = handle(&mut machine, &ToolEvent::Cancel, &ctx).expect("cancel");
        assert!(effect.intents.is_empty());
        assert_eq!(anchor(&machine), None);
    }

    #[test]
    fn events_without_a_press_are_harmless() {
        let ctx = fixture();
        let mut machine = machine(Tool::Rectangle);
        assert!(
            handle(&mut machine, &moved(Point::new(1.0, 1.0)), &ctx)
                .expect("move")
                .intents
                .is_empty()
        );
        assert!(
            handle(&mut machine, &up(Point::new(1.0, 1.0)), &ctx)
                .expect("up")
                .intents
                .is_empty()
        );
        assert!(
            handle(
                &mut machine,
                &ToolEvent::DoubleClick {
                    pos: Point::new(1.0, 1.0)
                },
                &ctx
            )
            .expect("double")
            .intents
            .is_empty()
        );
    }

    #[test]
    fn a_non_finite_point_is_ignored_and_a_non_finite_anchor_draws_nothing() {
        let ctx = fixture();
        let mut machine = machine(Tool::Rectangle);
        let _ = handle(&mut machine, &down(Point::new(f64::NAN, 1.0)), &ctx).expect("down");
        assert_eq!(anchor(&machine), None);
        let effect = handle(&mut machine, &up(Point::new(2.0, 2.0)), &ctx).expect("up");
        assert!(effect.intents.is_empty());
    }

    #[test]
    fn the_pointer_and_connector_tools_leave_the_drawing_state_alone() {
        let ctx = fixture();
        let mut machine = machine(Tool::Pointer);
        assert!(
            handle(&mut machine, &down(Point::new(1.0, 1.0)), &ctx)
                .expect("down")
                .intents
                .is_empty()
        );
        assert_eq!(anchor(&machine), None);
    }

    #[test]
    fn a_tool_switch_drops_the_old_drag() {
        let ctx = fixture();
        let mut machine = machine(Tool::Rectangle);
        let _ = handle(&mut machine, &down(Point::new(1.0, 1.0)), &ctx).expect("down");
        machine.tool = Tool::Line;
        let effect = handle(&mut machine, &up(Point::new(2.0, 2.0)), &ctx).expect("up");
        assert!(effect.intents.is_empty());
        assert_eq!(anchor(&machine), None);
    }
}
