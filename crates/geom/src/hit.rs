//! Hit-test math: distances from a point to segments, path outlines and
//! rectangles (ADR 0041).
//!
//! The functions are pure geometry in inches (ADR 0011). The hit policy
//! (fill versus stroke, tolerance in screen pixels, z-order) lives in
//! `ferrogram-render::hit` (L3); the drag and snap callers live in
//! `crates/ui` (L6).
//!
//! Curve segments (quadratic and cubic Béziers) flatten to chords within
//! [`FLATTEN_TOLERANCE`] inches through [`Curve::to_cubics`] (the ADR
//! 0022 flattening), with bounded recursion. Non-finite input degrades,
//! never panics (AGENTS.md section 6): a non-finite point or segment
//! measures "no hit" (`f64::INFINITY` or `false`), the ADR 0021 policy
//! style.
//!
//! Part of Ferrogram. Alpha.

use crate::curve::{Cubic, Curve};
use crate::error::Error;
use crate::path::{Path, Segment};
use crate::point::Point;
use crate::rect::Rect;

/// The flattening tolerance of curve outlines, in inches: a quadratic or
/// cubic segment is replaced by chords that deviate from it by at most
/// this much (ADR 0022's flattening rule). One thousandth of an inch is
/// about a tenth of a pixel at 96 dpi, well below the smallest hit
/// tolerance the UI passes (4 px at 96 dpi are 0.042 in).
pub const FLATTEN_TOLERANCE: f64 = 0.001;

/// The depth cap of the cubic flattening recursion (AGENTS.md section 6:
/// bounded recursion). At most `2^MAX_FLATTEN_DEPTH` chords per cubic.
const MAX_FLATTEN_DEPTH: u32 = 12;

/// Distance from `point` to the segment `a`..`b`, in inches.
///
/// A degenerate segment (`a == b`) is a point distance. A non-finite
/// coordinate on either endpoint or the query point yields
/// `f64::INFINITY` (no hit); the caller clamps (ADR 0021 policy style).
pub fn distance_point_segment(point: Point, a: Point, b: Point) -> Result<f64, Error> {
    Ok(segment_distance(point, a, b))
}

/// Distance from `point` to the path outline, in inches: the minimum
/// distance over every segment and flattened curve of `path`.
///
/// An empty path (or one with only [`Segment::MoveTo`] segments) has no
/// outline; the result is `f64::INFINITY` then. A [`Segment::Close`]
/// contributes its closing segment to the outline distance. A degenerate
/// zero-length segment measures its point distance, as
/// [`distance_point_segment`] does. Non-finite coordinates are skipped
/// (no hit).
pub fn distance_point_path(point: Point, path: &Path) -> Result<f64, Error> {
    if !finite(point) {
        return Ok(f64::INFINITY);
    }
    let mut best = f64::INFINITY;
    let mut current: Option<Point> = None;
    let mut start: Option<Point> = None;
    for segment in path.segments() {
        match segment {
            Segment::MoveTo(p) => {
                current = finite(*p).then_some(*p);
                start = current;
            }
            Segment::LineTo(p) => {
                if let Some(from) = current {
                    if finite(*p) {
                        best = best.min(segment_distance(point, from, *p));
                        current = Some(*p);
                    } else {
                        current = None;
                    }
                }
            }
            Segment::QuadTo { ctrl, to } => {
                if let Some(from) = current {
                    if finite(*ctrl) && finite(*to) {
                        let curve = Curve::Quad {
                            from,
                            ctrl: *ctrl,
                            to: *to,
                        };
                        best = best.min(curve_distance(point, &curve)?);
                        current = Some(*to);
                    } else {
                        current = None;
                    }
                }
            }
            Segment::CubicTo { ctrl1, ctrl2, to } => {
                if let Some(from) = current {
                    if finite(*ctrl1) && finite(*ctrl2) && finite(*to) {
                        let curve = Curve::Cubic(Cubic {
                            from,
                            ctrl1: *ctrl1,
                            ctrl2: *ctrl2,
                            to: *to,
                        });
                        best = best.min(curve_distance(point, &curve)?);
                        current = Some(*to);
                    } else {
                        current = None;
                    }
                }
            }
            Segment::Close => {
                if let (Some(from), Some(to)) = (current, start) {
                    best = best.min(segment_distance(point, from, to));
                }
                current = start;
            }
        }
    }
    Ok(best)
}

/// Whether `point` lies inside the closed `path` (non-zero winding rule,
/// the same fill rule the backends draw with).
///
/// An open path is closed virtually for the test, as a fill does
/// (ADR 0022). A point exactly on the outline counts as inside. Curves
/// flatten within `FLATTEN_TOLERANCE` inches. A non-finite point or
/// path coordinate yields `false` for the query (no hit).
pub fn path_contains(path: &Path, point: Point) -> Result<bool, Error> {
    if !finite(point) {
        return Ok(false);
    }
    let mut winding: i32 = 0;
    let mut current: Option<Point> = None;
    let mut start: Option<Point> = None;
    // Every subpath closes (a fill closes each open subpath): the
    // closing edges are added at the next `MoveTo` and at the end.
    let mut edges: Vec<(Point, Point)> = Vec::new();
    for segment in path.segments() {
        match segment {
            Segment::MoveTo(p) => {
                if let (Some(from), Some(to)) = (current, start) {
                    push_edge(&mut edges, from, to);
                }
                current = finite(*p).then_some(*p);
                start = current;
            }
            Segment::LineTo(p) => {
                if let Some(from) = current {
                    if finite(*p) {
                        push_edge(&mut edges, from, *p);
                        current = Some(*p);
                    } else {
                        current = None;
                    }
                }
            }
            Segment::QuadTo { ctrl, to } => {
                if let Some(from) = current {
                    if finite(*ctrl) && finite(*to) {
                        let curve = Curve::Quad {
                            from,
                            ctrl: *ctrl,
                            to: *to,
                        };
                        push_chords(&mut edges, &curve)?;
                        current = Some(*to);
                    } else {
                        current = None;
                    }
                }
            }
            Segment::CubicTo { ctrl1, ctrl2, to } => {
                if let Some(from) = current {
                    if finite(*ctrl1) && finite(*ctrl2) && finite(*to) {
                        let curve = Curve::Cubic(Cubic {
                            from,
                            ctrl1: *ctrl1,
                            ctrl2: *ctrl2,
                            to: *to,
                        });
                        push_chords(&mut edges, &curve)?;
                        current = Some(*to);
                    } else {
                        current = None;
                    }
                }
            }
            Segment::Close => {
                if let (Some(from), Some(to)) = (current, start) {
                    push_edge(&mut edges, from, to);
                }
                current = start;
            }
        }
    }
    if let (Some(from), Some(to)) = (current, start) {
        push_edge(&mut edges, from, to);
    }
    for (a, b) in edges {
        if on_segment(point, a, b) {
            return Ok(true);
        }
        // Winding of the +x ray (the non-zero rule).
        if a.y <= point.y {
            if b.y > point.y && is_left(a, b, point) > 0.0 {
                winding += 1;
            }
        } else if b.y <= point.y && is_left(a, b, point) < 0.0 {
            winding -= 1;
        }
    }
    Ok(winding != 0)
}

/// Distance from `point` to the rectangle's boundary, in inches; 0 when
/// the point is inside (edges included).
///
/// The corners may come in any order (the rectangle is normalized to its
/// bounding box). A degenerate rectangle (a point or a segment) measures
/// to its degenerate boundary. A non-finite coordinate yields
/// `f64::INFINITY` (no hit).
pub fn distance_point_rect(point: Point, rect: &Rect) -> Result<f64, Error> {
    if !finite(point) || !finite(rect.min) || !finite(rect.max) {
        return Ok(f64::INFINITY);
    }
    let min = Point::new(rect.min.x.min(rect.max.x), rect.min.y.min(rect.max.y));
    let max = Point::new(rect.min.x.max(rect.max.x), rect.min.y.max(rect.max.y));
    if point.x >= min.x && point.x <= max.x && point.y >= min.y && point.y <= max.y {
        return Ok(0.0);
    }
    let corners = [min, Point::new(max.x, min.y), max, Point::new(min.x, max.y)];
    let mut best = f64::INFINITY;
    for (i, a) in corners.iter().enumerate() {
        let b = corners[(i + 1) % corners.len()];
        best = best.min(segment_distance(point, *a, b));
    }
    Ok(best)
}

// --- helpers ---------------------------------------------------------

/// True when both coordinates are finite.
fn finite(p: Point) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

/// The point `a + t * (b - a)`, in an overflow-free convex form.
fn lerp(a: Point, b: Point, t: f64) -> Point {
    Point::new(a.x * (1.0 - t) + b.x * t, a.y * (1.0 - t) + b.y * t)
}

/// The distance between two points, in inches; `f64::INFINITY` when the
/// difference is not finite.
fn point_distance(a: Point, b: Point) -> f64 {
    let d = a.sub(b);
    let d = d.x.hypot(d.y);
    if d.is_finite() { d } else { f64::INFINITY }
}

/// The distance from `point` to the segment `a`..`b`; `f64::INFINITY`
/// for non-finite input (no hit).
fn segment_distance(point: Point, a: Point, b: Point) -> f64 {
    if !finite(point) || !finite(a) || !finite(b) {
        return f64::INFINITY;
    }
    let ab = b.sub(a);
    let denom = ab.x * ab.x + ab.y * ab.y;
    if denom == 0.0 {
        return point_distance(point, a);
    }
    if !denom.is_finite() {
        // The squared length overflowed: the endpoint distances are the
        // best finite approximation (degradation, not an error).
        return point_distance(point, a).min(point_distance(point, b));
    }
    let t = ((point.x - a.x) * ab.x + (point.y - a.y) * ab.y) / denom;
    let t = t.clamp(0.0, 1.0);
    point_distance(point, lerp(a, b, t))
}

/// True when `point` lies exactly on the segment `a`..`b` (endpoints
/// included).
fn on_segment(point: Point, a: Point, b: Point) -> bool {
    let ab = b.sub(a);
    let ap = point.sub(a);
    let cross = ab.x * ap.y - ab.y * ap.x;
    if cross != 0.0 {
        return false;
    }
    let dot = ap.x * (point.x - b.x) + ap.y * (point.y - b.y);
    dot <= 0.0
}

/// The left-of test of the winding number: positive when `p` is left of
/// the directed line `a`..`b`.
fn is_left(a: Point, b: Point, p: Point) -> f64 {
    (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y)
}

/// Appends the edge `from`..`to`. A degenerate edge is a point: it
/// counts for the on-outline test and never for the winding number.
fn push_edge(edges: &mut Vec<(Point, Point)>, from: Point, to: Point) {
    edges.push((from, to));
}

/// Appends the flattened chords of `curve` as edges (ADR 0022).
fn push_chords(edges: &mut Vec<(Point, Point)>, curve: &Curve) -> Result<(), Error> {
    for chord in flatten_curve_chords(curve)? {
        push_edge(edges, chord.0, chord.1);
    }
    Ok(())
}

/// The distance from `point` to the flattened curve, in inches.
fn curve_distance(point: Point, curve: &Curve) -> Result<f64, Error> {
    let mut best = f64::INFINITY;
    for (a, b) in flatten_curve_chords(curve)? {
        best = best.min(segment_distance(point, a, b));
    }
    Ok(best)
}

/// The curve flattened to chords within [`FLATTEN_TOLERANCE`] inches
/// (ADR 0022): one chord pair per piece, in order.
fn flatten_curve_chords(curve: &Curve) -> Result<Vec<(Point, Point)>, Error> {
    let cubics = curve.to_cubics(FLATTEN_TOLERANCE)?;
    let mut chords = Vec::new();
    for cubic in cubics {
        let points = flatten_cubic(cubic);
        for pair in points.windows(2) {
            if let (Some(&a), Some(&b)) = (pair.first(), pair.get(1)) {
                chords.push((a, b));
            }
        }
    }
    Ok(chords)
}

/// The polyline of the flattened cubic: its start point and every chord
/// end point, each chord within [`FLATTEN_TOLERANCE`] inches of the
/// curve. A non-finite cubic flattens to nothing.
///
/// The recursion is bounded by [`MAX_FLATTEN_DEPTH`].
fn flatten_cubic(cubic: Cubic) -> Vec<Point> {
    let mut out = Vec::new();
    if finite(cubic.from) && finite(cubic.ctrl1) && finite(cubic.ctrl2) && finite(cubic.to) {
        out.push(cubic.from);
        flatten_into(cubic, 0, &mut out);
    }
    out
}

/// The de Casteljau halving of `cubic` until it is flat within
/// [`FLATTEN_TOLERANCE`], appending the chord end points to `out`.
fn flatten_into(cubic: Cubic, depth: u32, out: &mut Vec<Point>) {
    if depth >= MAX_FLATTEN_DEPTH || cubic_flat(cubic) <= FLATTEN_TOLERANCE {
        out.push(cubic.to);
        return;
    }
    let p01 = lerp(cubic.from, cubic.ctrl1, 0.5);
    let p12 = lerp(cubic.ctrl1, cubic.ctrl2, 0.5);
    let p23 = lerp(cubic.ctrl2, cubic.to, 0.5);
    let p012 = lerp(p01, p12, 0.5);
    let p123 = lerp(p12, p23, 0.5);
    let mid = lerp(p012, p123, 0.5);
    let left = Cubic {
        from: cubic.from,
        ctrl1: p01,
        ctrl2: p012,
        to: mid,
    };
    let right = Cubic {
        from: mid,
        ctrl1: p123,
        ctrl2: p23,
        to: cubic.to,
    };
    flatten_into(left, depth + 1, out);
    flatten_into(right, depth + 1, out);
}

/// How far the cubic's control points sit from its chord, in inches: an
/// upper bound on the chord deviation of the whole segment.
fn cubic_flat(cubic: Cubic) -> f64 {
    segment_distance(cubic.ctrl1, cubic.from, cubic.to).max(segment_distance(
        cubic.ctrl2,
        cubic.from,
        cubic.to,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Path {
        let mut path = Path::new();
        path.push(Segment::MoveTo(Point::new(0.0, 0.0)));
        path.push(Segment::LineTo(Point::new(1.0, 0.0)));
        path.push(Segment::LineTo(Point::new(1.0, 1.0)));
        path.push(Segment::LineTo(Point::new(0.0, 1.0)));
        path.push(Segment::Close);
        path
    }

    #[test]
    fn segment_distance_measures_the_projection() {
        let p = Point::new(0.5, 1.0);
        let d = distance_point_segment(p, Point::new(0.0, 0.0), Point::new(1.0, 0.0))
            .expect("finite input");
        assert_eq!(d, 1.0);
    }

    #[test]
    fn segment_distance_clamps_beyond_the_ends() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(1.0, 0.0);
        let d = distance_point_segment(Point::new(3.0, 0.0), a, b).expect("finite input");
        assert_eq!(d, 2.0);
        let d = distance_point_segment(Point::new(-1.0, 0.0), a, b).expect("finite input");
        assert_eq!(d, 1.0);
    }

    #[test]
    fn degenerate_segment_is_a_point_distance() {
        let a = Point::new(1.0, 1.0);
        let d = distance_point_segment(Point::new(1.0, 2.0), a, a).expect("finite input");
        assert_eq!(d, 1.0);
    }

    #[test]
    fn non_finite_input_measures_no_hit() {
        let nan = f64::NAN;
        assert_eq!(
            distance_point_segment(
                Point::new(nan, 0.0),
                Point::new(0.0, 0.0),
                Point::new(1.0, 0.0)
            ),
            Ok(f64::INFINITY)
        );
        assert_eq!(
            distance_point_segment(
                Point::new(0.0, 0.0),
                Point::new(nan, 0.0),
                Point::new(1.0, 0.0)
            ),
            Ok(f64::INFINITY)
        );
        assert_eq!(
            distance_point_path(Point::new(nan, nan), &square()),
            Ok(f64::INFINITY)
        );
        assert_eq!(path_contains(&square(), Point::new(nan, 0.0)), Ok(false));
        let rect = Rect::new(Point::new(nan, 0.0), Point::new(1.0, 1.0));
        assert_eq!(
            distance_point_rect(Point::new(0.5, 0.5), &rect),
            Ok(f64::INFINITY)
        );
    }

    #[test]
    fn huge_segments_degrade_instead_of_overflowing() {
        let big = 1.0e308;
        let d = distance_point_segment(
            Point::new(0.0, 0.0),
            Point::new(-big, 0.0),
            Point::new(big, 0.0),
        )
        .expect("finite input");
        assert!(d.is_finite());
    }

    #[test]
    fn empty_path_has_no_outline() {
        assert_eq!(
            distance_point_path(Point::new(0.0, 0.0), &Path::new()),
            Ok(f64::INFINITY)
        );
        let mut moves = Path::new();
        moves.push(Segment::MoveTo(Point::new(0.0, 0.0)));
        assert_eq!(
            distance_point_path(Point::new(0.0, 0.0), &moves),
            Ok(f64::INFINITY)
        );
    }

    #[test]
    fn degenerate_outline_measures_its_point() {
        let mut moves = Path::new();
        moves.push(Segment::MoveTo(Point::new(0.0, 0.0)));
        moves.push(Segment::Close);
        assert_eq!(
            distance_point_path(Point::new(0.0, 0.0), &moves),
            Ok(0.0),
            "a degenerate segment is a point distance"
        );
        assert_eq!(distance_point_path(Point::new(0.0, 1.0), &moves), Ok(1.0));
    }

    #[test]
    fn path_distance_measures_the_nearest_outline_segment() {
        let d = distance_point_path(Point::new(0.5, 0.5), &square()).expect("finite input");
        assert_eq!(d, 0.5);
        let d = distance_point_path(Point::new(2.0, 0.5), &square()).expect("finite input");
        assert_eq!(d, 1.0);
    }

    #[test]
    fn close_segment_counts_as_outline() {
        let mut open = Path::new();
        open.push(Segment::MoveTo(Point::new(0.0, 0.0)));
        open.push(Segment::LineTo(Point::new(1.0, 0.0)));
        open.push(Segment::LineTo(Point::new(1.0, 1.0)));
        // Near the virtual closing segment (0, 0)..(1, 1) only.
        let query = Point::new(0.6, 0.4);
        let d = distance_point_path(query, &open).expect("finite input");
        assert_eq!(d, 0.4);
        open.push(Segment::Close);
        let d = distance_point_path(query, &open).expect("finite input");
        assert!(d < 0.15, "distance {d}");
    }

    #[test]
    fn quad_segments_flatten_within_tolerance() {
        // The quadratic passes through (0.5, 0.5) at t = 0.5.
        let mut path = Path::new();
        path.push(Segment::MoveTo(Point::new(0.0, 0.0)));
        path.push(Segment::QuadTo {
            ctrl: Point::new(0.5, 1.0),
            to: Point::new(1.0, 0.0),
        });
        let d = distance_point_path(Point::new(0.5, 0.5), &path).expect("finite input");
        assert!(d <= FLATTEN_TOLERANCE, "distance {d}");
    }

    #[test]
    fn cubic_segments_flatten_within_tolerance() {
        let mut path = Path::new();
        path.push(Segment::MoveTo(Point::new(0.0, 0.0)));
        path.push(Segment::CubicTo {
            ctrl1: Point::new(0.0, 1.0),
            ctrl2: Point::new(1.0, 1.0),
            to: Point::new(1.0, 0.0),
        });
        // The cubic passes through its midpoint (0.5, 0.75) at t = 0.5.
        let d = distance_point_path(Point::new(0.5, 0.75), &path).expect("finite input");
        assert!(d <= FLATTEN_TOLERANCE, "distance {d}");
    }

    #[test]
    fn square_interior_is_contained() {
        assert_eq!(path_contains(&square(), Point::new(0.5, 0.5)), Ok(true));
        assert_eq!(path_contains(&square(), Point::new(1.5, 0.5)), Ok(false));
        assert_eq!(path_contains(&square(), Point::new(-0.5, 0.5)), Ok(false));
    }

    #[test]
    fn points_on_the_outline_count_as_inside() {
        assert_eq!(path_contains(&square(), Point::new(0.0, 0.5)), Ok(true));
        assert_eq!(path_contains(&square(), Point::new(1.0, 1.0)), Ok(true));
        assert_eq!(path_contains(&square(), Point::new(0.5, 0.0)), Ok(true));
    }

    #[test]
    fn open_paths_close_virtually() {
        let mut open = Path::new();
        open.push(Segment::MoveTo(Point::new(0.0, 0.0)));
        open.push(Segment::LineTo(Point::new(2.0, 0.0)));
        open.push(Segment::LineTo(Point::new(1.0, 2.0)));
        assert_eq!(path_contains(&open, Point::new(1.0, 0.5)), Ok(true));
        assert_eq!(path_contains(&open, Point::new(1.5, 1.5)), Ok(false));
    }

    #[test]
    fn subpaths_follow_the_non_zero_rule() {
        let mut with_hole = square();
        // The hole wound the other way is outside under the non-zero rule.
        with_hole.push(Segment::MoveTo(Point::new(0.25, 0.25)));
        with_hole.push(Segment::LineTo(Point::new(0.25, 0.75)));
        with_hole.push(Segment::LineTo(Point::new(0.75, 0.75)));
        with_hole.push(Segment::LineTo(Point::new(0.75, 0.25)));
        with_hole.push(Segment::Close);
        assert_eq!(path_contains(&with_hole, Point::new(0.5, 0.5)), Ok(false));
        assert_eq!(path_contains(&with_hole, Point::new(0.1, 0.1)), Ok(true));
    }

    #[test]
    fn empty_path_contains_nothing() {
        assert_eq!(path_contains(&Path::new(), Point::new(0.0, 0.0)), Ok(false));
    }

    #[test]
    fn rect_distance_is_zero_inside_and_measures_outside() {
        let rect = Rect::new(Point::new(0.0, 0.0), Point::new(2.0, 1.0));
        assert_eq!(distance_point_rect(Point::new(1.0, 0.5), &rect), Ok(0.0));
        assert_eq!(
            distance_point_rect(Point::new(0.0, 0.0), &rect),
            Ok(0.0),
            "edges are inside"
        );
        assert_eq!(distance_point_rect(Point::new(3.0, 0.5), &rect), Ok(1.0));
        assert_eq!(distance_point_rect(Point::new(1.0, -2.0), &rect), Ok(2.0));
    }

    #[test]
    fn rect_distance_normalizes_the_corners() {
        let flipped = Rect::new(Point::new(2.0, 1.0), Point::new(0.0, 0.0));
        assert_eq!(distance_point_rect(Point::new(1.0, 0.5), &flipped), Ok(0.0));
    }

    #[test]
    fn degenerate_rects_measure_to_their_boundary() {
        let point_rect = Rect::new(Point::new(1.0, 1.0), Point::new(1.0, 1.0));
        assert_eq!(
            distance_point_rect(Point::new(1.0, 2.0), &point_rect),
            Ok(1.0)
        );
        let line_rect = Rect::new(Point::new(0.0, 1.0), Point::new(2.0, 1.0));
        assert_eq!(
            distance_point_rect(Point::new(1.0, 1.0), &line_rect),
            Ok(0.0)
        );
        assert_eq!(
            distance_point_rect(Point::new(1.0, 2.0), &line_rect),
            Ok(1.0)
        );
    }

    #[test]
    fn the_flatten_tolerance_is_the_documented_value() {
        assert_eq!(FLATTEN_TOLERANCE, 0.001);
    }
}
