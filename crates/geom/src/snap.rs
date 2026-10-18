//! Snap math (ADR 0043): pick the nearest snap candidate within a
//! distance, and quantize to a grid.
//!
//! The policy (which candidates exist, their priority, the snap distance
//! in screen pixels) lives in `crates/ui::snap` (L6). These functions are
//! pure geometry in inches (ADR 0011).
//!
//! A `max_distance` or `spacing` argument that is negative, zero where a
//! positive value is required, or not finite is [`Error::WrongUnit`]:
//! the closest variant of the crate error for "this value is not a valid
//! distance" (the error type is fixed by task P5-01; the P5-03 Result
//! asks for an `Error::InvalidArgument`). Non-finite coordinates never
//! panic: they are skipped or leave the value unchanged (ADR 0021
//! policy style).
//!
//! Part of Ferrogram. Alpha.

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
/// Across kinds the higher [`SnapKind`] wins even when it is slightly
/// farther (still within `max_distance`): `ConnectionPoint`, then
/// `Alignment`, then `Grid` (ADR 0043). Within one kind the nearest
/// candidate wins; equal distances keep the earlier candidate (candidate
/// order breaks ties). A candidate out of range leaves the point
/// unchanged and the kind `None`.
///
/// `max_distance` must be finite and non-negative; anything else is an
/// [`Error::WrongUnit`]. Candidates with non-finite coordinates are
/// skipped; a non-finite `point` never snaps.
pub fn snap_point(
    point: Point,
    candidates: &[SnapCandidate],
    max_distance: f64,
) -> Result<SnapResult, Error> {
    let max_distance = checked_distance(max_distance)?;
    if !finite(point) {
        return Ok(SnapResult::none(point));
    }
    // The nearest candidate of each kind (the first of equal distance).
    let mut best: [Option<SnapCandidate>; 3] = [None, None, None];
    for candidate in candidates {
        if !finite(candidate.point) {
            continue;
        }
        let delta = point.sub(candidate.point);
        let distance = delta.x.hypot(delta.y);
        if !distance.is_finite() || distance > max_distance {
            continue;
        }
        let slot = kind_slot(candidate.kind);
        let replace = match best[slot] {
            None => true,
            Some(current) => {
                let delta = point.sub(current.point);
                distance < delta.x.hypot(delta.y)
            }
        };
        if replace {
            best[slot] = Some(*candidate);
        }
    }
    // The highest kind with a candidate in range wins (ADR 0043).
    let winner = best[2].or(best[1]).or(best[0]);
    Ok(match winner {
        Some(candidate) => SnapResult {
            point: candidate.point,
            kind: Some(candidate.kind),
        },
        None => SnapResult::none(point),
    })
}

/// Snaps one axis value to the nearest candidate value within
/// `max_distance` inches (resize handles use one axis at a time).
///
/// The nearest candidate wins; equal distances keep the earlier one: the
/// candidate order breaks ties, the first candidate within range wins
/// the tie. Unlike [`snap_point`] there is no kind priority here: the
/// resize caller folds its candidates into one ordered list.
///
/// `max_distance` must be finite and non-negative; anything else is an
/// [`Error::WrongUnit`]. Candidates with non-finite values are skipped;
/// a non-finite `value` never snaps.
pub fn snap_axis(
    value: f64,
    candidates: &[(SnapKind, f64)],
    max_distance: f64,
) -> Result<Option<(f64, SnapKind)>, Error> {
    let max_distance = checked_distance(max_distance)?;
    if !value.is_finite() {
        return Ok(None);
    }
    let mut best: Option<(f64, SnapKind, f64)> = None;
    for (kind, candidate) in candidates {
        if !candidate.is_finite() {
            continue;
        }
        let distance = (value - candidate).abs();
        if !distance.is_finite() || distance > max_distance {
            continue;
        }
        let replace = match best {
            None => true,
            Some((_, _, best_distance)) => distance < best_distance,
        };
        if replace {
            best = Some((*candidate, *kind, distance));
        }
    }
    Ok(best.map(|(candidate, kind, _)| (candidate, kind)))
}

/// Quantizes `value` to the nearest multiple of `spacing` inches.
///
/// `spacing` must be finite and positive; zero or a non-finite spacing is
/// [`Error::WrongUnit`]. A value exactly between two multiples rounds
/// away from zero (deterministic, no banker's rounding). A non-finite
/// `value` stays unchanged.
pub fn snap_grid(value: f64, spacing: f64) -> Result<f64, Error> {
    if !spacing.is_finite() || spacing <= 0.0 {
        return Err(Error::WrongUnit);
    }
    if !value.is_finite() {
        return Ok(value);
    }
    let steps = value / spacing;
    if !steps.is_finite() {
        // The division overflowed: snapping cannot improve the value.
        return Ok(value);
    }
    // `f64::round` rounds half-way cases away from zero.
    let snapped = steps.round() * spacing;
    Ok(if snapped.is_finite() { snapped } else { value })
}

// --- helpers ---------------------------------------------------------

/// Validates a snap distance (see the function contracts).
fn checked_distance(max_distance: f64) -> Result<f64, Error> {
    if !max_distance.is_finite() || max_distance < 0.0 {
        return Err(Error::WrongUnit);
    }
    Ok(max_distance)
}

/// True when both coordinates are finite.
fn finite(p: Point) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

/// The slot of `best` for a kind: grid, alignment, connection point.
const fn kind_slot(kind: SnapKind) -> usize {
    match kind {
        SnapKind::Grid => 0,
        SnapKind::Alignment => 1,
        SnapKind::ConnectionPoint => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(kind: SnapKind, x: f64, y: f64) -> SnapCandidate {
        SnapCandidate::new(kind, Point::new(x, y))
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

    #[test]
    fn snap_point_takes_the_nearest_candidate_of_one_kind() {
        let candidates = [
            candidate(SnapKind::Grid, 0.05, 0.0),
            candidate(SnapKind::Grid, 0.02, 0.0),
        ];
        let result = snap_point(Point::new(0.0, 0.0), &candidates, 0.1).expect("valid args");
        assert_eq!(result.point, Point::new(0.02, 0.0));
        assert_eq!(result.kind, Some(SnapKind::Grid));
    }

    #[test]
    fn snap_point_prefers_the_higher_kind_across_kinds() {
        // A slightly farther connection point beats the grid (ADR 0043).
        let candidates = [
            candidate(SnapKind::Grid, 0.01, 0.0),
            candidate(SnapKind::ConnectionPoint, 0.03, 0.0),
        ];
        let result = snap_point(Point::new(0.0, 0.0), &candidates, 0.1).expect("valid args");
        assert_eq!(result.point, Point::new(0.03, 0.0));
        assert_eq!(result.kind, Some(SnapKind::ConnectionPoint));
    }

    #[test]
    fn snap_point_breaks_distance_ties_by_kind_then_order() {
        let candidates = [
            candidate(SnapKind::Grid, 0.05, 0.0),
            candidate(SnapKind::Alignment, 0.05, 0.0),
        ];
        let result = snap_point(Point::new(0.0, 0.0), &candidates, 0.1).expect("valid args");
        assert_eq!(result.kind, Some(SnapKind::Alignment));

        let candidates = [
            candidate(SnapKind::Grid, 0.05, 0.0),
            candidate(SnapKind::Grid, -0.05, 0.0),
        ];
        let result = snap_point(Point::new(0.0, 0.0), &candidates, 0.1).expect("valid args");
        assert_eq!(result.point, Point::new(0.05, 0.0), "first wins the tie");
    }

    #[test]
    fn snap_point_leaves_the_point_unchanged_out_of_range() {
        let candidates = [candidate(SnapKind::ConnectionPoint, 1.0, 1.0)];
        let point = Point::new(0.0, 0.0);
        let result = snap_point(point, &candidates, 0.1).expect("valid args");
        assert_eq!(result, SnapResult::none(point));
    }

    #[test]
    fn snap_point_at_zero_distance_only_takes_exact_matches() {
        let candidates = [candidate(SnapKind::Grid, 0.0, 0.0)];
        let result = snap_point(Point::new(0.0, 0.0), &candidates, 0.0).expect("valid args");
        assert_eq!(result.kind, Some(SnapKind::Grid));
        let result = snap_point(Point::new(1.0, 0.0), &candidates, 0.0).expect("valid args");
        assert_eq!(result.kind, None);
    }

    #[test]
    fn snap_point_skips_non_finite_candidates_and_points() {
        let nan = f64::NAN;
        let candidates = [
            candidate(SnapKind::Grid, nan, 0.0),
            candidate(SnapKind::Alignment, 0.05, 0.0),
        ];
        let result = snap_point(Point::new(0.0, 0.0), &candidates, 0.1).expect("valid args");
        assert_eq!(result.kind, Some(SnapKind::Alignment));
        let result = snap_point(Point::new(nan, 0.0), &candidates, 0.1).expect("valid args");
        assert_eq!(result.kind, None);
    }

    #[test]
    fn snap_point_rejects_a_bad_distance() {
        let candidates = [candidate(SnapKind::Grid, 0.0, 0.0)];
        let point = Point::new(0.0, 0.0);
        assert_eq!(snap_point(point, &candidates, -0.1), Err(Error::WrongUnit));
        assert_eq!(
            snap_point(point, &candidates, f64::NAN),
            Err(Error::WrongUnit)
        );
        assert_eq!(
            snap_point(point, &candidates, f64::INFINITY),
            Err(Error::WrongUnit)
        );
    }

    #[test]
    fn snap_axis_takes_the_nearest_value() {
        let candidates = [
            (SnapKind::Alignment, 0.05),
            (SnapKind::Grid, 0.02),
            (SnapKind::Grid, 0.09),
        ];
        let result = snap_axis(0.0, &candidates, 0.1).expect("valid args");
        assert_eq!(result, Some((0.02, SnapKind::Grid)));
    }

    #[test]
    fn snap_axis_breaks_ties_by_candidate_order() {
        let candidates = [(SnapKind::Grid, 0.05), (SnapKind::Alignment, -0.05)];
        let result = snap_axis(0.0, &candidates, 0.1).expect("valid args");
        assert_eq!(result, Some((0.05, SnapKind::Grid)), "first wins the tie");
    }

    #[test]
    fn snap_axis_has_no_kind_priority() {
        // The nearest wins even when a higher kind is in range: the
        // resize caller orders its own candidates.
        let candidates = [(SnapKind::Grid, 0.02), (SnapKind::ConnectionPoint, 0.03)];
        let result = snap_axis(0.0, &candidates, 0.1).expect("valid args");
        assert_eq!(result, Some((0.02, SnapKind::Grid)));
    }

    #[test]
    fn snap_axis_returns_none_out_of_range() {
        let candidates = [(SnapKind::Grid, 1.0)];
        assert_eq!(snap_axis(0.0, &candidates, 0.1), Ok(None));
        assert_eq!(snap_axis(f64::NAN, &candidates, 0.1), Ok(None));
        assert_eq!(snap_axis(0.0, &[(SnapKind::Grid, f64::NAN)], 0.1), Ok(None));
    }

    #[test]
    fn snap_axis_rejects_a_bad_distance() {
        let candidates = [(SnapKind::Grid, 0.0)];
        assert_eq!(snap_axis(0.0, &candidates, -1.0), Err(Error::WrongUnit));
        assert_eq!(
            snap_axis(0.0, &candidates, f64::INFINITY),
            Err(Error::WrongUnit)
        );
    }

    #[test]
    fn snap_grid_quantizes_to_the_spacing() {
        assert_eq!(snap_grid(0.1, 0.125), Ok(0.125));
        assert_eq!(snap_grid(0.06, 0.125), Ok(0.0));
        assert_eq!(snap_grid(-0.2, 0.125), Ok(-0.25));
        assert_eq!(snap_grid(1000000.0, 1.0), Ok(1000000.0));
    }

    #[test]
    fn snap_grid_rounds_half_way_away_from_zero() {
        assert_eq!(snap_grid(0.0625, 0.125), Ok(0.125));
        assert_eq!(snap_grid(-0.0625, 0.125), Ok(-0.125));
        assert_eq!(snap_grid(0.1875, 0.125), Ok(0.25));
        assert_eq!(snap_grid(-0.1875, 0.125), Ok(-0.25));
    }

    #[test]
    fn snap_grid_rejects_a_bad_spacing() {
        assert_eq!(snap_grid(1.0, 0.0), Err(Error::WrongUnit));
        assert_eq!(snap_grid(1.0, -0.125), Err(Error::WrongUnit));
        assert_eq!(snap_grid(1.0, f64::NAN), Err(Error::WrongUnit));
        assert_eq!(snap_grid(1.0, f64::INFINITY), Err(Error::WrongUnit));
    }

    #[test]
    fn snap_grid_keeps_non_finite_values() {
        let nan = snap_grid(f64::NAN, 0.125).expect("valid spacing");
        assert!(nan.is_nan());
        assert_eq!(snap_grid(f64::INFINITY, 0.125), Ok(f64::INFINITY));
        assert_eq!(
            snap_grid(1.0e308, 1.0e-300),
            Ok(1.0e308),
            "overflow degrades"
        );
    }
}
