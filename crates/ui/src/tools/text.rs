//! The text tool and the in-place editor hand-off (ADR 0044).
//!
//! Task P5-07 implements the tool: a click (or the pointer tool's double click) opens `crate::text_edit::TextEditor` over the shape's text block; a commit dispatches `shape.set.text`; the shortcut scope is `Text` while editing (ADR 0036).

use ferrogram_geom::Point;

use crate::UiError;

/// Handles one event for this tool (owner: P5-07, ADR 0042).
///
/// The P5-01 stub returns `Err(UiError::NotImplemented)`; P5-07
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
