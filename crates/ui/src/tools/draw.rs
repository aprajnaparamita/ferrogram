//! The drawing tools (P5-06): rectangle, ellipse and line (ADR 0042).
//!
//! Task P5-06 implements the tools: a drag previews the shape on the
//! overlay and commits one `insert.drawing.*` intent (undoable, ADR 0040)
//! with snapping applied (ADR 0043).

use ferrogram_geom::Point;

use crate::UiError;

/// Handles one event for the active drawing tool (owner: P5-06, ADR 0042).
///
/// The P5-01 stub returns `Err(UiError::NotImplemented)`.
pub fn handle(
    _state: &mut super::ToolMachine,
    _event: &super::ToolEvent,
    _ctx: &ferrogram_commands::AppCtx,
) -> Result<super::ToolEffect, UiError> {
    Err(UiError::NotImplemented)
}

/// The drag anchor, in page inches (the preview's first corner).
pub fn anchor(_state: &super::ToolMachine) -> Option<Point> {
    None
}
