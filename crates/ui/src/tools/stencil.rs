//! Stencil drag: dropping a master from the Shapes window onto the page (ADR 0046).
//!
//! Task P5-06 implements the tool: the drop dispatches `insert.shape` with the pointer position as `PinX`/`PinY`, the master size (the P4-06 deferred item), snapping while dragging, and the optional glue arguments of ADR 0046. The Shapes window starts the drag (`crate::shapes_window`).

use ferrogram_geom::Point;

use crate::UiError;

/// Handles one event for this tool (owner: P5-06, ADR 0042).
///
/// The P5-01 stub returns `Err(UiError::NotImplemented)`; P5-06
/// implements it and P5-05 dispatches to it from
/// [`super::ToolMachine::handle`].
pub fn handle(
    _state: &mut super::ToolMachine,
    _event: &super::ToolEvent,
    _ctx: &ferrogram_commands::AppCtx,
) -> Result<super::ToolEffect, UiError> {
    Err(UiError::NotImplemented)
}

/// The drag anchor, in page inches (the preview's start).
pub fn anchor(_state: &super::ToolMachine) -> Option<Point> {
    None
}
