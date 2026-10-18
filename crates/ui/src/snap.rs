//! Snapping policy (ADR 0043): which candidates exist, in what priority,
//! and the snap distance in screen pixels.
//!
//! The math lives in `ferrogram_geom::snap` (L0); this module gathers the
//! candidates from the document (grid lines, shape edges and centres,
//! connection points) and applies the priority of ADR 0043:
//! connection point, then alignment, then grid. Alt disables snapping.
//! Task P5-05 implements the bodies.

use ferrogram_commands::AppCtx;
use ferrogram_geom::{Point, SnapResult};
use ferrogram_model::ShapeId;

use crate::UiError;

/// The snapping settings (ADR 0043).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapSettings {
    /// The grid spacing, in inches (the canvas default: 1/8 in).
    pub grid: f64,
    /// The snap distance, in screen pixels (ADR 0043: 8 px).
    pub distance_px: f64,
    /// Snap to the grid.
    pub to_grid: bool,
    /// Snap to shape edges and centres (alignment).
    pub to_shapes: bool,
    /// Snap to connection points (glue candidates).
    pub to_connection_points: bool,
}

impl Default for SnapSettings {
    fn default() -> Self {
        Self {
            grid: 0.125,
            distance_px: 8.0,
            to_grid: true,
            to_shapes: true,
            to_connection_points: true,
        }
    }
}

/// The snapper used by the drag tools (ADR 0043).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Snapper {
    /// The snap settings.
    pub settings: SnapSettings,
}

impl Snapper {
    /// A snapper with the given settings.
    pub const fn new(settings: SnapSettings) -> Self {
        Self { settings }
    }

    /// Snaps a dragged selection's delta (relative offsets of the shapes
    /// are kept, ADR 0043).
    ///
    /// `zoom` converts the pixel snap distance to inches. `alt_held`
    /// disables snapping for this call (ADR 0043). Task P5-05 implements
    /// the body.
    pub fn snap_move(
        &self,
        ctx: &AppCtx,
        moving: &[ShapeId],
        delta: Point,
        zoom: f64,
        alt_held: bool,
    ) -> Result<SnapResult, UiError> {
        let _ = (ctx, moving, delta, zoom, alt_held);
        Err(UiError::NotImplemented)
    }

    /// Snaps one point (drawing tools, connector endpoints, drops).
    /// Task P5-05 implements the body.
    pub fn snap_point(
        &self,
        ctx: &AppCtx,
        point: Point,
        zoom: f64,
        alt_held: bool,
    ) -> Result<SnapResult, UiError> {
        let _ = (ctx, point, zoom, alt_held);
        Err(UiError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    //! Stub contract (P5-01); replace with real tests when implementing.
    use super::*;
    use ferrogram_commands::Settings;

    #[test]
    fn the_snapper_is_a_not_implemented_stub() {
        let snapper = Snapper::new(SnapSettings::default());
        assert_eq!(snapper, Snapper::default());
        let ctx = AppCtx::new(Settings::default());
        let point = Point::new(1.0, 1.0);
        assert!(matches!(
            snapper.snap_move(&ctx, &[ShapeId { index: 0 }], point, 1.0, false),
            Err(UiError::NotImplemented)
        ));
        assert!(matches!(
            snapper.snap_point(&ctx, point, 1.0, true),
            Err(UiError::NotImplemented)
        ));
    }
}
