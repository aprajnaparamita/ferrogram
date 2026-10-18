//! The dynamic connector tool (ADR 0045).
//!
//! Task P5-08 implements the tool: a drag previews the routed path (`ferrogram_geom::routing`), snaps and glues the ends to connection points or shape outlines, and commits one `insert.shape` intent plus the glue records.

use ferrogram_geom::Point;

use crate::UiError;

/// Handles one event for this tool (owner: P5-08, ADR 0042).
///
/// The P5-01 stub returns `Err(UiError::NotImplemented)`; P5-08
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
