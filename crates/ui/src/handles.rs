//! Selection handles: move, resize and rotate (ADR 0042).
//!
//! Handles live at screen-size anchors around the selection bounds; the
//! caller converts the tolerance from screen pixels with the zoom. The
//! pointer tool ([`crate::tools::pointer`], P5-05) owns the drag that
//! follows a hit. Task P5-05 implements the bodies.

use ferrogram_commands::AppCtx;
use ferrogram_geom::{Angle, Point, Rect};

use crate::UiError;

/// Which edge or corner a resize handle stretches (ADR 0042).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ResizeEdge {
    /// The top edge.
    North,
    /// The bottom edge.
    South,
    /// The right edge.
    East,
    /// The left edge.
    West,
    /// The top-right corner.
    NorthEast,
    /// The top-left corner.
    NorthWest,
    /// The bottom-right corner.
    SouthEast,
    /// The bottom-left corner.
    SouthWest,
}

/// What a handle does (ADR 0042).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HandleKind {
    /// Move the selection (the interior of the bounds).
    Move,
    /// Stretch one edge or corner.
    Resize(ResizeEdge),
    /// Rotate around the selection's pin (the handle above the bounds).
    Rotate,
}

/// The handle at `point` for one selected shape (page inches, `zoom` for
/// the screen-pixel tolerance of [`TOLERANCE_PX`]).
///
/// `None` when the point is on no handle (the caller hit-tests shapes
/// next). Task P5-05 implements the body.
pub fn handle_at(
    point: Point,
    bounds: Rect,
    angle: Angle,
    zoom: f64,
) -> Result<Option<HandleKind>, UiError> {
    let _ = (point, bounds, angle, zoom);
    Err(UiError::NotImplemented)
}

/// The handle anchor points of one selection: `(kind, point)` pairs in
/// page inches, rotated with `angle` (ADR 0042). Task P5-05 implements
/// the body.
pub fn handles_for(bounds: Rect, angle: Angle) -> Result<Vec<(HandleKind, Point)>, UiError> {
    let _ = (bounds, angle);
    Err(UiError::NotImplemented)
}

/// The union bounds of the selection in page inches (one shape: its
/// rotated bounds; several: the axis-aligned union). Task P5-05
/// implements the body.
pub fn selection_bounds(ctx: &AppCtx) -> Result<Rect, UiError> {
    let _ = ctx;
    Err(UiError::NotImplemented)
}

/// The screen-pixel hit tolerance of a handle (ADR 0042).
pub const TOLERANCE_PX: f64 = 6.0;

#[cfg(test)]
mod tests {
    //! Stub contract (P5-01); replace with real tests when implementing.
    use super::*;
    use ferrogram_commands::Settings;

    #[test]
    fn the_handle_helpers_are_not_implemented_stubs() {
        let bounds = Rect::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0));
        let angle = Angle::from_radians(0.0);
        assert!(matches!(
            handle_at(Point::new(0.5, 0.5), bounds, angle, 1.0),
            Err(UiError::NotImplemented)
        ));
        assert!(matches!(
            handles_for(bounds, angle),
            Err(UiError::NotImplemented)
        ));
        let ctx = AppCtx::new(Settings::default());
        assert!(matches!(
            selection_bounds(&ctx),
            Err(UiError::NotImplemented)
        ));
    }
}
