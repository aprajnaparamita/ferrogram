//! Snap math (ADR 0043): pick the nearest snap candidate within a
//! distance, and quantize to a grid.
//!
//! The policy (which candidates exist, their priority, the snap distance
//! in screen pixels) lives in `crates/ui::snap` (L6). These functions are
//! pure geometry in inches (ADR 0011). Task P5-03 implements the bodies.
//!
//! Part of Ferrogram. Alpha: the signatures are fixed by task P5-01 and
//! the bodies are `Err(Error::NotImplemented)` stubs.

use crate::error::Error;
use crate::point::Point;

/// What a snap candidate is (ADR 0043).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum SnapKind {
    /// A grid line.
    Grid,
    /// A shape edge or centre (alignment).
    Alignment,
    /// A connection point (glue); the highest priority (ADR 0043).
    ConnectionPoint,
}

/// One snap candidate in page coordinates, in inches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapCandidate {
    /// The candidate's kind (priority, ADR 0043).
    pub kind: SnapKind,
    /// The candidate's position, in inches.
    pub point: Point,
}

impl SnapCandidate {
    /// A candidate of the given kind at the given point.
    pub const fn new(kind: SnapKind, point: Point) -> Self {
        Self { kind, point }
    }
}

/// The outcome of a snap: the snapped point and the candidate kind that
/// won, if any (a snap out of range leaves the point unchanged and the
/// kind `None`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapResult {
    /// The point after snapping, in inches.
    pub point: Point,
    /// The winning candidate kind, or `None` when nothing was in range.
    pub kind: Option<SnapKind>,
}

impl SnapResult {
    /// A result with no snap applied.
    pub const fn none(point: Point) -> Self {
        Self { point, kind: None }
    }
}

/// Snaps `point` to the nearest candidate within `max_distance` inches.
///
/// Ties are broken by [`SnapKind`] priority: `ConnectionPoint`, then
/// `Alignment`, then `Grid` (ADR 0043), and by candidate order within a
/// kind. `max_distance` must be finite and non-negative; anything else is
/// an `Err` (P5-03 picks the variant).
pub fn snap_point(
    point: Point,
    candidates: &[SnapCandidate],
    max_distance: f64,
) -> Result<SnapResult, Error> {
    let _ = (point, candidates, max_distance);
    Err(Error::NotImplemented)
}

/// Snaps one axis value to the nearest candidate value within
/// `max_distance` inches (resize handles use one axis at a time).
///
/// The candidate order breaks ties: the first candidate within range wins.
pub fn snap_axis(
    value: f64,
    candidates: &[(SnapKind, f64)],
    max_distance: f64,
) -> Result<Option<(f64, SnapKind)>, Error> {
    let _ = (value, candidates, max_distance);
    Err(Error::NotImplemented)
}

/// Quantizes `value` to the nearest multiple of `spacing` inches.
///
/// `spacing` must be finite and positive; zero or a non-finite spacing is
/// an `Err` (P5-03 defines which variant). A value exactly between two
/// multiples rounds away from zero (deterministic, no banker's rounding).
pub fn snap_grid(value: f64, spacing: f64) -> Result<f64, Error> {
    let _ = (value, spacing);
    Err(Error::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_p5_01_stubs_are_not_implemented() {
        let p = Point::new(1.0, 2.0);
        let candidates = [SnapCandidate::new(SnapKind::Grid, p)];
        assert_eq!(snap_point(p, &candidates, 0.1), Err(Error::NotImplemented));
        assert_eq!(
            snap_axis(1.0, &[(SnapKind::Grid, 1.0)], 0.1),
            Err(Error::NotImplemented)
        );
        assert_eq!(snap_grid(1.0, 0.125), Err(Error::NotImplemented));
    }

    #[test]
    fn candidates_and_results_hold_their_data() {
        let c = SnapCandidate::new(SnapKind::ConnectionPoint, Point::new(1.0, 1.0));
        assert_eq!(c.kind, SnapKind::ConnectionPoint);
        assert_eq!(c.point, Point::new(1.0, 1.0));
        let r = SnapResult::none(Point::new(0.0, 0.0));
        assert_eq!(r.kind, None);
        assert!(SnapKind::ConnectionPoint > SnapKind::Alignment);
        assert!(SnapKind::Alignment > SnapKind::Grid);
    }
}
