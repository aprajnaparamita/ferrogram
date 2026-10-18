//! The pointer tool (P5-05).
//!
//! Select, multi-select, marquee, and move, resize and rotate through the
//! handles ([`crate::handles`], ADR 0041, ADR 0042).
//!
//! Task P5-05 implements the tool: a pointer down hit-tests the scene
//! (`ferrogram_render::hit`), shift toggles the selection, a drag moves or
//! transforms with snapping ([`crate::snap`], ADR 0043), and one drag
//! commits exactly one undo edit (ADR 0040).

use ferrogram_geom::Point;

use crate::UiError;

/// The pointer tool's drag state (owner: P5-05; opaque to the other tool
/// modules).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PointerState;

/// Handles one event for the pointer tool (owner: P5-05, ADR 0042).
///
/// The P5-01 stub returns `Err(UiError::NotImplemented)`; P5-05
/// implements it and dispatches to it from
/// [`super::ToolMachine::handle`].
pub fn handle(
    _state: &mut super::ToolMachine,
    _event: &super::ToolEvent,
    _ctx: &ferrogram_commands::AppCtx,
) -> Result<super::ToolEffect, UiError> {
    Err(UiError::NotImplemented)
}

/// The drag anchor, in page inches (cursor and overlay hint).
pub fn anchor(_state: &super::ToolMachine) -> Option<Point> {
    None
}
