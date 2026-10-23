//! Curves: circular arcs, elliptical arcs, Béziers, NURBS and splines.
//!
//! These are the mathematical forms of the Geometry rows (ADR 0022); the
//! typed rows live in `larrygram-model` and the renderer maps them to
//! [`Curve`] values. [`Curve::to_cubics`] is the single flattening point:
//! it approximates any curve with [`Cubic`] segments within a tolerance in
//! inches (ADR 0022).
//!
//! # Degenerate input
//!
//! Degenerate input degrades to the straight chord from the curve's start
//! to its end, or to nothing when that chord is not finite: a zero chord,
//! `bow = 0`, `ratio <= 0`, a knot list of the wrong length, a degree of 0
//! or above [`MAX_DEGREE`], too few control points, unsorted knots, a
//! zero-width knot domain, non-positive or non-finite weights, and
//! non-finite coordinates (ADR 0022). The conversion never panics and
//! never fails.
//!
//! # Knot vectors
//!
//! Visio stores one knot per control point plus a final knot (`knot_last`):
//! `n + 1` knots for `n` control points ([MS-VSDX] 2.4.2.7 `NURBSTo`,
//! 2.4.2.14 `SplineKnot`, 2.4.2.15 `SplineStart`: `SplineStart`'s `B`,
//! `A`, each `SplineKnot`'s `A`, then `SplineStart`'s `C`). The full knot
//! vector of a degree-`d` B-spline is that list followed by `d` more
//! copies of the last knot: the `DrawNURBS` remarks of the Visio
//! VBA/automation reference say the application supplies the missing
//! repeated knots at the tail of the conventional `n + degree + 1`
//! sequence (their example: the degree-2 list `0, 0, 0, 2, 5, 8` is read
//! as `0, 0, 0, 2, 5, 8, 8, 8`). The curve therefore runs over
//! `knots[d] ..= knot_last` and ends on its last control point. libvisio
//! pads the same way; that is corroboration only, not the source.
//!
//! The `DrawNURBS` remarks call a spline *periodic* when its first knot
//! repeats fewer than `degree + 1` times. We always read the stored list
//! as clamped (the padding above), so a periodic input draws as an open
//! curve that need not start at its first control point. No corpus file
//! exercises the case; it stays open until a file or the screenshot pass
//! needs it.

use core::f64::consts::{FRAC_PI_2, TAU};

use crate::angle::Angle;
use crate::error::Error;
use crate::point::Point;

/// The tolerance used when the caller passes `0`, a negative or a `NaN`
/// tolerance: 1e-9 inch (a nanoinch).
const MIN_TOLERANCE: f64 = 1.0e-9;

/// The largest degree of a B-spline (ADR 0022: an integer from 1 to 25).
const MAX_DEGREE: u32 = 25;

/// Hard cap on the number of [`Cubic`] segments one curve may produce, so
/// that an extreme input or tolerance cannot allocate without bound.
const MAX_SEGMENTS: usize = 4096;

/// The depth cap of the adaptive B-spline span flattening.
const MAX_DEPTH: u32 = 10;

/// Interior sample points per candidate piece of a B-spline span.
const SAMPLES: usize = 15;

/// Conservative bound on the radial error of one cubic approximation of a
/// circular arc piece: `error <= ARC_ERROR * sweep^6 * radius` for a piece
/// sweep of at most `pi / 2`. The measured constant is `1.82e-5`.
const ARC_ERROR: f64 = 2.0e-5;

/// A circular arc from `start` to `end` given by its bow (`ArcTo`).
///
/// The bow (sagitta) is the distance from the arc's midpoint to the
/// midpoint of the chord `start`..`end` (ADR 0022). A bow of zero is a
/// straight line; a negative bow curves to the other side of the chord.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arc {
    /// The start point.
    pub start: Point,
    /// The end point.
    pub end: Point,
    /// The bow (sagitta), in inches.
    pub bow: f64,
}

/// An elliptical arc from `start` to `end` through the control point
/// (`EllipticalArcTo`, `RelEllipticalArcTo`; ADR 0022).
///
/// The control point is a point on the arc. `angle` is the angle of the
/// ellipse's major axis from the parent's x axis and `ratio` is the
/// major/minor axis ratio (it need not exceed 1). The shape-sheet defaults
/// are `angle = 0` and `ratio = 1` when the row omits the cells.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EllipticalArc {
    /// The start point.
    pub start: Point,
    /// The end point.
    pub end: Point,
    /// A point on the arc, roughly halfway along it.
    pub ctrl: Point,
    /// The angle of the major axis.
    pub angle: Angle,
    /// The major/minor axis ratio.
    pub ratio: f64,
}

/// One control point of a [`Nurbs`] curve with its knot and weight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NurbsPoint {
    /// The x coordinate, in inches or a fraction of the width (`x_type`).
    pub x: f64,
    /// The y coordinate, in inches or a fraction of the height (`y_type`).
    pub y: f64,
    /// The knot of this control point.
    pub knot: f64,
    /// The weight of this control point.
    pub weight: f64,
}

/// A nonuniform rational B-spline (`NURBSTo`; ADR 0022).
///
/// `x_type`/`y_type` 0 means the control points are fractions of the
/// shape's width and height, 1 means local coordinates (the `NURBS()`
/// formula's `xType`/`yType`). `knot_last` is the last knot of the
/// formula.
///
/// The coordinates must already be local: [`Curve::to_cubics`] has no size
/// to scale fractions with, so the caller resolves `x_type`/`y_type`
/// first. The two flags are kept for diagnostics only.
#[derive(Debug, Clone, PartialEq)]
pub struct Nurbs {
    /// The degree of the spline.
    pub degree: u32,
    /// The `xType` of the `NURBS()` formula: 0 fraction, 1 local.
    pub x_type: u32,
    /// The `yType` of the `NURBS()` formula: 0 fraction, 1 local.
    pub y_type: u32,
    /// The last knot of the spline.
    pub knot_last: f64,
    /// The control points with their knots and weights, in order.
    ///
    /// The knot of the last point is the second-to-last knot of the
    /// spline; the last knot of the whole spline is [`Nurbs::knot_last`]
    /// (the `NURBS()` formula's `knotLast`).
    pub points: Vec<NurbsPoint>,
}

/// A B-spline (`SplineStart` + `SplineKnot` rows; ADR 0022).
#[derive(Debug, Clone, PartialEq)]
pub struct Spline {
    /// The degree of the spline (1 to 25).
    pub degree: u32,
    /// The control points, in order.
    pub ctrl: Vec<Point>,
    /// The knots, in order: one per control point plus one final knot
    /// (`ctrl.len() + 1` of them).
    pub knots: Vec<f64>,
}

/// A polyline (`PolylineTo`): straight segments through the points.
#[derive(Debug, Clone, PartialEq)]
pub struct Polyline {
    /// The points, in order.
    pub points: Vec<Point>,
}

/// A cubic Bézier segment (`RelCubBezTo`, and the output of
/// [`Curve::to_cubics`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cubic {
    /// The start point.
    pub from: Point,
    /// The first control point.
    pub ctrl1: Point,
    /// The second control point.
    pub ctrl2: Point,
    /// The end point.
    pub to: Point,
}

impl Cubic {
    /// Evaluates the cubic Bézier at `t`.
    ///
    /// `t` in `0..=1` is on the segment; other values extrapolate. A
    /// non-finite `t` or control point yields non-finite coordinates.
    pub fn eval(self, t: f64) -> Point {
        let m = 1.0 - t;
        let w0 = m * m * m;
        let w1 = 3.0 * m * m * t;
        let w2 = 3.0 * m * t * t;
        let w3 = t * t * t;
        Point::new(
            w0 * self.from.x + w1 * self.ctrl1.x + w2 * self.ctrl2.x + w3 * self.to.x,
            w0 * self.from.y + w1 * self.ctrl1.y + w2 * self.ctrl2.y + w3 * self.to.y,
        )
    }
}

/// A curve of one geometry row (ADR 0022).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Curve {
    /// A circular arc (`ArcTo`).
    Arc(Arc),
    /// An elliptical arc (`EllipticalArcTo`, `RelEllipticalArcTo`).
    EllipticalArc(EllipticalArc),
    /// A quadratic Bézier (`RelQuadBezTo`).
    Quad {
        /// The start point.
        from: Point,
        /// The control point.
        ctrl: Point,
        /// The end point.
        to: Point,
    },
    /// A cubic Bézier (`RelCubBezTo`).
    Cubic(Cubic),
    /// A nonuniform rational B-spline (`NURBSTo`).
    Nurbs(Nurbs),
    /// A B-spline (`SplineStart`, `SplineKnot`).
    Spline(Spline),
    /// A polyline (`PolylineTo`).
    Polyline(Polyline),
}

impl Curve {
    /// Approximates the curve with cubic Bézier segments.
    ///
    /// The approximation stays within `tolerance` inches of the true curve
    /// (ADR 0022). Degenerate input (zero chord, `bow = 0`, `ratio <= 0`,
    /// wrong knot or weight counts, non-finite values) degrades to the
    /// straight chord; it is not an error and never panics. A `tolerance`
    /// that is `0`, negative or `NaN` becomes the private `MIN_TOLERANCE`; an
    /// infinite tolerance asks for one segment per 90 degrees.
    ///
    /// The result is always `Ok` today: the `Result` is part of the
    /// interface fixed by P3-01 and reserves room for real errors without
    /// changing the signature.
    pub fn to_cubics(&self, tolerance: f64) -> Result<Vec<Cubic>, Error> {
        let tol = clean_tolerance(tolerance);
        Ok(match self {
            Curve::Arc(arc) => arc_cubics(*arc, tol),
            Curve::EllipticalArc(arc) => elliptical_cubics(*arc, tol),
            Curve::Quad { from, ctrl, to } => quad_cubics(*from, *ctrl, *to),
            Curve::Cubic(cubic) => cubic_cubics(*cubic),
            Curve::Nurbs(nurbs) => nurbs_cubics(nurbs, tol),
            Curve::Spline(spline) => spline_cubics(spline, tol),
            Curve::Polyline(polyline) => polyline_cubics(&polyline.points),
        })
    }
}

/// The tolerance to flatten with: [`MIN_TOLERANCE`] for a `NaN`, zero or
/// negative request, the value itself otherwise (including infinity).
fn clean_tolerance(tolerance: f64) -> f64 {
    if tolerance.is_nan() || tolerance <= 0.0 {
        MIN_TOLERANCE
    } else {
        tolerance
    }
}

/// True when both coordinates are finite.
fn finite(p: Point) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

/// The point `a + t * (b - a)`, in an overflow-free convex form.
fn lerp(a: Point, b: Point, t: f64) -> Point {
    Point::new(a.x * (1.0 - t) + b.x * t, a.y * (1.0 - t) + b.y * t)
}

/// The straight cubic from `from` to `to` (control points one third along).
fn chord(from: Point, to: Point) -> Cubic {
    Cubic {
        from,
        ctrl1: lerp(from, to, 1.0 / 3.0),
        ctrl2: lerp(from, to, 2.0 / 3.0),
        to,
    }
}

/// The straight chord, or nothing when an endpoint is not finite.
fn chord_or_empty(from: Point, to: Point) -> Vec<Cubic> {
    if finite(from) && finite(to) {
        vec![chord(from, to)]
    } else {
        Vec::new()
    }
}

/// The number of arc pieces for `sweep` radians of extent `radius` that
/// keeps the flattening error within `tol`, capped at [`MAX_SEGMENTS`].
fn piece_count(sweep: f64, tol: f64, radius: f64) -> usize {
    let delta = (tol / (ARC_ERROR * radius)).powf(1.0 / 6.0).min(FRAC_PI_2);
    let n = (sweep.abs() / delta).ceil().clamp(1.0, MAX_SEGMENTS as f64);
    n as usize
}

/// The signed sweep from `start` to `end` that passes through `mid` (three
/// angles on one circle). Positive is counterclockwise.
fn sweep_through(start: f64, mid: f64, end: f64) -> f64 {
    let ccw = (end - start).rem_euclid(TAU);
    let off = (start + 0.5 * ccw - mid).rem_euclid(TAU);
    let d_ccw = off.min(TAU - off);
    if d_ccw <= FRAC_PI_2 { ccw } else { ccw - TAU }
}

/// One cubic per piece of the circular arc at `theta` by `sweep` radians
/// (signed) around `center`, with radius `r`.
///
/// `radius` is the deviation extent the piece count is computed for: `r`
/// for a circle, the larger semi-axis for an ellipse (an affine map scales
/// the error by at most that factor).
fn circle_pieces(
    center: Point,
    r: f64,
    radius: f64,
    theta: f64,
    sweep: f64,
    tol: f64,
) -> Vec<Cubic> {
    let n = piece_count(sweep, tol, radius);
    let delta = sweep / n as f64;
    // The sweep sign lives in `dir`; the control offset is its magnitude.
    let k = (4.0 / 3.0) * (0.25 * delta).tan().abs();
    let dir = if sweep < 0.0 { -1.0 } else { 1.0 };
    let mut cubics = Vec::with_capacity(n);
    for i in 0..n {
        let a0 = theta + delta * i as f64;
        let a1 = a0 + delta;
        let p0 = Point::new(center.x + r * a0.cos(), center.y + r * a0.sin());
        let p3 = Point::new(center.x + r * a1.cos(), center.y + r * a1.sin());
        let t0 = Point::new(-a0.sin() * dir, a0.cos() * dir).scale(k * r);
        let t3 = Point::new(-a1.sin() * dir, a1.cos() * dir).scale(k * r);
        cubics.push(Cubic {
            from: p0,
            ctrl1: p0.add(t0),
            ctrl2: p3.sub(t3),
            to: p3,
        });
    }
    cubics
}

/// Flattens an [`Arc`], degrading to its chord (ADR 0022).
fn arc_cubics(arc: Arc, tol: f64) -> Vec<Cubic> {
    let (start, end, bow) = (arc.start, arc.end, arc.bow);
    if !finite(start) || !finite(end) || !bow.is_finite() {
        return chord_or_empty(start, end);
    }
    let d = end.sub(start);
    let chord2 = d.x * d.x + d.y * d.y;
    if chord2 == 0.0 || bow == 0.0 || !chord2.is_finite() {
        return chord_or_empty(start, end);
    }
    let len = chord2.sqrt();
    let r_signed = 0.5 * bow + chord2 / (8.0 * bow);
    if !r_signed.is_finite() {
        return chord_or_empty(start, end);
    }
    let r = r_signed.abs();
    let normal = Point::new(-d.y / len, d.x / len);
    let mid = lerp(start, end, 0.5);
    let apex = mid.add(normal.scale(bow));
    let center = apex.sub(normal.scale(r_signed));
    let bound = center.x.abs().max(center.y.abs()) + 2.0 * r;
    if !finite(center) || !bound.is_finite() {
        return chord_or_empty(start, end);
    }
    let theta_s = (start.y - center.y).atan2(start.x - center.x);
    let theta_m = (apex.y - center.y).atan2(apex.x - center.x);
    let theta_e = (end.y - center.y).atan2(end.x - center.x);
    let sweep = sweep_through(theta_s, theta_m, theta_e);
    let mut cubics = circle_pieces(center, r, r, theta_s, sweep, tol);
    if let Some(first) = cubics.first_mut() {
        first.from = start;
    }
    if let Some(last) = cubics.last_mut() {
        last.to = end;
    }
    cubics
}

/// Flattens an [`EllipticalArc`], degrading to its chord (ADR 0022).
///
/// In the major-axis frame the ellipse is the image of a circle of radius
/// `a` under `(x, y) -> (x, y / ratio)` shifted by the center; the arc is
/// flattened on that circle and the pieces are mapped back, which keeps
/// the error within `tol` because the map's gain is the larger semi-axis.
fn elliptical_cubics(arc: EllipticalArc, tol: f64) -> Vec<Cubic> {
    let (start, end, ctrl) = (arc.start, arc.end, arc.ctrl);
    let (ratio, angle) = (arc.ratio, arc.angle.radians());
    if !finite(start)
        || !finite(end)
        || !finite(ctrl)
        || !angle.is_finite()
        || !ratio.is_finite()
        || ratio <= 0.0
    {
        return chord_or_empty(start, end);
    }
    let d = end.sub(start);
    if d.x * d.x + d.y * d.y == 0.0 {
        return chord_or_empty(start, end);
    }
    let (cos, sin) = (angle.cos(), angle.sin());
    let rot = |p: Point| Point::new(p.x * cos + p.y * sin, -p.x * sin + p.y * cos);
    let (ps, pe, pg) = (rot(start), rot(end), rot(ctrl));
    let d2 = ratio * ratio;
    // The center (cx, cy) makes |q(p)| equal for the three points, where
    // `q(p) = (p.x - cx, ratio * (p.y - cy))`; two differences are linear.
    let (a11, a12) = (ps.x - pe.x, d2 * (ps.y - pe.y));
    let (a21, a22) = (pe.x - pg.x, d2 * (pe.y - pg.y));
    let b1 = 0.5 * ((ps.x * ps.x - pe.x * pe.x) + d2 * (ps.y * ps.y - pe.y * pe.y));
    let b2 = 0.5 * ((pe.x * pe.x - pg.x * pg.x) + d2 * (pe.y * pe.y - pg.y * pg.y));
    let det = a11 * a22 - a12 * a21;
    if det == 0.0 || !det.is_finite() {
        return chord_or_empty(start, end);
    }
    let cx = (b1 * a22 - a12 * b2) / det;
    let cy = (a11 * b2 - b1 * a21) / det;
    let a2 = (ps.x - cx) * (ps.x - cx) + d2 * (ps.y - cy) * (ps.y - cy);
    let b = a2.sqrt() / ratio;
    let bound = cx.abs() + cy.abs() + 4.0 * (a2.sqrt() + b);
    if a2 <= 0.0 || !a2.is_finite() || !bound.is_finite() {
        return chord_or_empty(start, end);
    }
    let a = a2.sqrt();
    let t_s = (ratio * (ps.y - cy)).atan2(ps.x - cx);
    let t_m = (ratio * (pg.y - cy)).atan2(pg.x - cx);
    let t_e = (ratio * (pe.y - cy)).atan2(pe.x - cx);
    let sweep = sweep_through(t_s, t_m, t_e);
    let map = |q: Point| {
        let p = Point::new(q.x + cx, q.y / ratio + cy);
        Point::new(p.x * cos - p.y * sin, p.x * sin + p.y * cos)
    };
    let pieces = circle_pieces(Point::new(0.0, 0.0), a, a.max(b), t_s, sweep, tol);
    let mut cubics: Vec<Cubic> = pieces
        .iter()
        .map(|c| Cubic {
            from: map(c.from),
            ctrl1: map(c.ctrl1),
            ctrl2: map(c.ctrl2),
            to: map(c.to),
        })
        .collect();
    if let Some(first) = cubics.first_mut() {
        first.from = start;
    }
    if let Some(last) = cubics.last_mut() {
        last.to = end;
    }
    cubics
}

/// The exact cubic of a [`Curve::Quad`], degrading to its chord.
fn quad_cubics(from: Point, ctrl: Point, to: Point) -> Vec<Cubic> {
    if finite(from) && finite(ctrl) && finite(to) {
        vec![Cubic {
            from,
            ctrl1: lerp(from, ctrl, 2.0 / 3.0),
            ctrl2: lerp(to, ctrl, 2.0 / 3.0),
            to,
        }]
    } else {
        chord_or_empty(from, to)
    }
}

/// The exact cubic of a [`Curve::Cubic`], degrading to its chord.
fn cubic_cubics(cubic: Cubic) -> Vec<Cubic> {
    if finite(cubic.from) && finite(cubic.ctrl1) && finite(cubic.ctrl2) && finite(cubic.to) {
        vec![cubic]
    } else {
        chord_or_empty(cubic.from, cubic.to)
    }
}

/// Straight segments through `points`, degrading to the chord when a point
/// is not finite. A polyline of fewer than two points holds no segment.
fn polyline_cubics(points: &[Point]) -> Vec<Cubic> {
    if points.len() < 2 {
        return Vec::new();
    }
    if points.iter().all(|p| finite(*p)) {
        points.windows(2).map(|w| chord(w[0], w[1])).collect()
    } else {
        chord_or_empty(points[0], points[points.len() - 1])
    }
}

/// Flattens a [`Nurbs`] through [`bspline_cubics`].
fn nurbs_cubics(nurbs: &Nurbs, tol: f64) -> Vec<Cubic> {
    let points: Vec<Point> = nurbs.points.iter().map(|p| Point::new(p.x, p.y)).collect();
    let weights: Vec<f64> = nurbs.points.iter().map(|p| p.weight).collect();
    let mut stored: Vec<f64> = nurbs.points.iter().map(|p| p.knot).collect();
    stored.push(nurbs.knot_last);
    bspline_cubics(&points, &weights, &stored, nurbs.degree, tol)
}

/// Flattens a [`Spline`] through [`bspline_cubics`], with unit weights.
fn spline_cubics(spline: &Spline, tol: f64) -> Vec<Cubic> {
    let weights = vec![1.0; spline.ctrl.len()];
    bspline_cubics(&spline.ctrl, &weights, &spline.knots, spline.degree, tol)
}

/// Flattens the B-spline of `degree` through `points` with `weights` and
/// the stored knot list (`points.len() + 1` knots, the last one being the
/// last knot of the spline).
///
/// Degrades to the chord from the first to the last control point, or to
/// nothing for fewer than two control points.
fn bspline_cubics(
    points: &[Point],
    weights: &[f64],
    stored: &[f64],
    degree: u32,
    tol: f64,
) -> Vec<Cubic> {
    if points.len() < 2 {
        return Vec::new();
    }
    match bspline_segments(points, weights, stored, degree, tol) {
        Some(cubics) => cubics,
        None => chord_or_empty(points[0], points[points.len() - 1]),
    }
}

/// The flattened B-spline, or [`None`] when the input is degenerate (the
/// caller degrades to the chord). `weights.len() == points.len()` and at
/// least two control points are invariants of the callers.
fn bspline_segments(
    points: &[Point],
    weights: &[f64],
    stored: &[f64],
    degree: u32,
    tol: f64,
) -> Option<Vec<Cubic>> {
    let n = points.len();
    let d = degree as usize;
    if degree == 0 || degree > MAX_DEGREE || n < d + 1 {
        return None;
    }
    if stored.len() != n + 1 {
        return None;
    }
    if !points.iter().all(|p| finite(*p)) {
        return None;
    }
    if !stored.iter().all(|k| k.is_finite()) || !weights.iter().all(|w| w.is_finite()) {
        return None;
    }
    if !weights.iter().all(|&w| w > 0.0) {
        return None;
    }
    if !points
        .iter()
        .zip(weights)
        .all(|(p, &w)| (w * p.x).is_finite() && (w * p.y).is_finite())
    {
        return None;
    }
    if !stored.windows(2).all(|k| k[0] <= k[1]) {
        return None;
    }
    // The full knot vector: the stored knots plus `d` copies of the last.
    let mut t = stored.to_vec();
    t.resize(stored.len() + d, stored[n]);
    // A run of equal knots longer than `d + 1` makes the basis vanish (it
    // also breaks the de Boor denominators), so the curve degenerates.
    let mut last = f64::NAN;
    let mut run = 0;
    for &knot in t.iter() {
        if knot == last {
            run += 1;
        } else {
            last = knot;
            run = 1;
        }
        if run > d + 1 {
            return None;
        }
    }
    let homo: Vec<(f64, f64, f64)> = points
        .iter()
        .zip(weights)
        .map(|(p, &w)| (w * p.x, w * p.y, w))
        .collect();
    // Split at every distinct knot value: the curve is only piecewise
    // smooth there. Each span flattens by bisection until its chord
    // deviation is within the tolerance (ADR 0022).
    let u_lo = t[d];
    let u_hi = t[n];
    let mut spans: Vec<(f64, f64)> = Vec::new();
    let mut a = u_lo;
    for &knot in t.iter() {
        if knot > a && knot < u_hi {
            spans.push((a, knot));
            a = knot;
        }
    }
    spans.push((a, u_hi));
    let mut cubics: Vec<Cubic> = Vec::new();
    for (a, b) in spans {
        let mut stack = vec![(a, b, 0u32)];
        while let Some((a, b, depth)) = stack.pop() {
            let pa = bspline_eval(&homo, &t, d, a);
            let pb = bspline_eval(&homo, &t, d, b);
            if cubics.len() >= MAX_SEGMENTS {
                cubics.push(chord(pa, pb));
                continue;
            }
            let mut dev: f64 = 0.0;
            for s in 1..=SAMPLES {
                let u = a + (b - a) * (s as f64 / (SAMPLES as f64 + 1.0));
                let p = bspline_eval(&homo, &t, d, u);
                dev = dev.max(distance_to_segment(p, pa, pb));
            }
            if dev <= tol || depth >= MAX_DEPTH {
                cubics.push(chord(pa, pb));
            } else {
                let m = a + 0.5 * (b - a);
                // LIFO: push the second half first so pieces come out in
                // parameter order.
                stack.push((m, b, depth + 1));
                stack.push((a, m, depth + 1));
            }
        }
    }
    Some(cubics)
}

/// De Boor's algorithm on the homogeneous control points `homo` (weights
/// folded in as the third coordinate), divided back to the plane at the
/// end. `t` is the full knot vector (`homo.len() + degree + 1` knots,
/// non-decreasing), and `u` lies in `t[degree] ..= t[homo.len()]`.
fn bspline_eval(homo: &[(f64, f64, f64)], t: &[f64], degree: usize, u: f64) -> Point {
    let n = homo.len();
    let u = u.clamp(t[degree], t[n]);
    let mut k = n - 1;
    for i in degree..n {
        if u < t[i + 1] {
            k = i;
            break;
        }
    }
    let mut work: Vec<(f64, f64, f64)> = homo[k - degree..=k].to_vec();
    for r in 1..=degree {
        for j in (r..=degree).rev() {
            let i = k - degree + j;
            // The denominator is positive: a zero one would need a run of
            // equal knots longer than `degree + 1`, which the caller
            // rejects.
            let alpha = (u - t[i]) / (t[i + degree - r + 1] - t[i]);
            let p0 = work[j - 1];
            let p1 = work[j];
            work[j] = (
                (1.0 - alpha) * p0.0 + alpha * p1.0,
                (1.0 - alpha) * p0.1 + alpha * p1.1,
                (1.0 - alpha) * p0.2 + alpha * p1.2,
            );
        }
    }
    let (x, y, w) = work[degree];
    Point::new(x / w, y / w)
}

/// The distance from `p` to the segment `a`..`b`.
fn distance_to_segment(p: Point, a: Point, b: Point) -> f64 {
    let ab = b.sub(a);
    let ap = p.sub(a);
    let len2 = ab.x * ab.x + ab.y * ab.y;
    if len2 == 0.0 {
        return ap.x.hypot(ap.y);
    }
    let t = ((ap.x * ab.x + ap.y * ab.y) / len2).clamp(0.0, 1.0);
    let q = lerp(a, b, t);
    (p.x - q.x).hypot(p.y - q.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn approx(got: f64, want: f64, tol: f64) {
        assert!((got - want).abs() <= tol, "{got} != {want} (tol {tol})");
    }

    /// Samples every cubic at `steps + 1` points.
    fn samples(cubics: &[Cubic], steps: usize) -> Vec<Point> {
        let mut out = Vec::new();
        for c in cubics {
            for i in 0..=steps {
                out.push(c.eval(i as f64 / steps as f64));
            }
        }
        out
    }

    /// Asserts every cubic is a straight chord from its `from` to its `to`.
    fn assert_straight(cubics: &[Cubic]) {
        for c in cubics {
            assert_eq!(c.ctrl1, lerp(c.from, c.to, 1.0 / 3.0));
            assert_eq!(c.ctrl2, lerp(c.from, c.to, 2.0 / 3.0));
        }
    }

    fn assert_all_finite(cubics: &[Cubic]) {
        for c in cubics {
            for q in [c.from, c.ctrl1, c.ctrl2, c.to] {
                assert!(finite(q), "not finite: {q:?}");
            }
        }
    }

    #[test]
    fn curve_variants_hold_their_data() {
        let p = Point::new(1.0, 2.0);
        let arc = Arc {
            start: p,
            end: Point::new(3.0, 4.0),
            bow: 0.5,
        };
        assert_eq!(arc.bow, 0.5);
        assert!(matches!(Curve::Arc(arc), Curve::Arc(_)));
        let ea = EllipticalArc {
            start: p,
            end: p,
            ctrl: p,
            angle: Angle::from_radians(1.0),
            ratio: 2.0,
        };
        assert_eq!(ea.ratio, 2.0);
        assert!(matches!(Curve::EllipticalArc(ea), Curve::EllipticalArc(_)));
        let quad = Curve::Quad {
            from: p,
            ctrl: p,
            to: p,
        };
        assert!(matches!(quad, Curve::Quad { .. }));
    }

    #[test]
    fn nurbs_spline_and_polyline_hold_their_data() {
        let n = Nurbs {
            degree: 3,
            x_type: 1,
            y_type: 0,
            knot_last: 2.0,
            points: vec![NurbsPoint {
                x: 0.0,
                y: 1.0,
                knot: 0.5,
                weight: 1.0,
            }],
        };
        assert_eq!(n.points.len(), 1);
        let s = Spline {
            degree: 2,
            ctrl: vec![Point::new(0.0, 0.0)],
            knots: vec![0.0, 1.0],
        };
        assert_eq!(s.degree, 2);
        let pl = Polyline {
            points: vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)],
        };
        assert_eq!(pl.points.len(), 2);
    }

    #[test]
    fn cubic_eval_hits_its_ends_and_midpoint() {
        let c = Cubic {
            from: p(0.0, 0.0),
            ctrl1: p(0.0, 1.0),
            ctrl2: p(1.0, 1.0),
            to: p(1.0, 0.0),
        };
        assert_eq!(c.eval(0.0), p(0.0, 0.0));
        assert_eq!(c.eval(1.0), p(1.0, 0.0));
        let mid = c.eval(0.5);
        approx(mid.x, 0.5, 1.0e-12);
        approx(mid.y, 0.75, 1.0e-12);
        let outside = c.eval(2.0);
        assert!(finite(outside));
    }

    #[test]
    fn cubic_to_cubics_is_exact() {
        let c = Cubic {
            from: p(0.0, 0.0),
            ctrl1: p(0.0, 1.0),
            ctrl2: p(1.0, 1.0),
            to: p(1.0, 0.0),
        };
        let cubics = Curve::Cubic(c).to_cubics(0.01).unwrap();
        assert_eq!(cubics, vec![c]);
    }

    #[test]
    fn quad_to_cubics_is_exact() {
        let curve = Curve::Quad {
            from: p(0.0, 0.0),
            ctrl: p(0.5, 1.0),
            to: p(1.0, 0.0),
        };
        let cubics = curve.to_cubics(1.0e-6).unwrap();
        assert_eq!(cubics.len(), 1);
        let c = cubics[0];
        assert_eq!(c.from, p(0.0, 0.0));
        assert_eq!(c.to, p(1.0, 0.0));
        approx(c.ctrl1.x, 1.0 / 3.0, 1.0e-12);
        approx(c.ctrl1.y, 2.0 / 3.0, 1.0e-12);
        approx(c.ctrl2.x, 2.0 / 3.0, 1.0e-12);
        approx(c.ctrl2.y, 2.0 / 3.0, 1.0e-12);
        // The cubic is the exact quadratic Bézier.
        for i in 1..8 {
            let t = i as f64 / 8.0;
            let m = 1.0 - t;
            let want = Point::new(2.0 * m * t * 0.5 + t * t, 2.0 * m * t * 1.0);
            let got = c.eval(t);
            approx(got.x, want.x, 1.0e-9);
            approx(got.y, want.y, 1.0e-9);
        }
    }

    #[test]
    fn quad_with_a_non_finite_control_point_degrades_to_the_chord() {
        let curve = Curve::Quad {
            from: p(0.0, 0.0),
            ctrl: p(f64::NAN, 1.0),
            to: p(1.0, 0.0),
        };
        let cubics = curve.to_cubics(0.01).unwrap();
        assert_eq!(cubics.len(), 1);
        assert_straight(&cubics);
    }

    #[test]
    fn cubic_with_a_non_finite_control_point_degrades_to_the_chord() {
        let c = Cubic {
            from: p(0.0, 0.0),
            ctrl1: p(f64::INFINITY, 1.0),
            ctrl2: p(1.0, 1.0),
            to: p(1.0, 0.0),
        };
        let cubics = Curve::Cubic(c).to_cubics(0.01).unwrap();
        assert_eq!(cubics.len(), 1);
        assert_straight(&cubics);
    }

    #[test]
    fn non_finite_endpoints_degrade_to_nothing() {
        let quad = Curve::Quad {
            from: p(f64::NAN, 0.0),
            ctrl: p(0.5, 1.0),
            to: p(1.0, 0.0),
        };
        assert!(quad.to_cubics(0.01).unwrap().is_empty());
        let cubic = Curve::Cubic(Cubic {
            from: p(0.0, 0.0),
            ctrl1: p(0.0, 1.0),
            ctrl2: p(1.0, 1.0),
            to: p(1.0, f64::NAN),
        });
        assert!(cubic.to_cubics(0.01).unwrap().is_empty());
    }

    #[test]
    fn polyline_becomes_straight_segments() {
        let curve = Curve::Polyline(Polyline {
            points: vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0)],
        });
        let cubics = curve.to_cubics(0.01).unwrap();
        assert_eq!(cubics.len(), 2);
        assert_straight(&cubics);
        assert_eq!(cubics[0].from, p(0.0, 0.0));
        assert_eq!(cubics[0].to, p(1.0, 0.0));
        assert_eq!(cubics[1].to, p(1.0, 1.0));
    }

    #[test]
    fn polyline_with_fewer_than_two_points_holds_nothing() {
        let empty = Curve::Polyline(Polyline { points: Vec::new() });
        assert!(empty.to_cubics(0.01).unwrap().is_empty());
        let one = Curve::Polyline(Polyline {
            points: vec![p(1.0, 2.0)],
        });
        assert!(one.to_cubics(0.01).unwrap().is_empty());
    }

    #[test]
    fn polyline_with_a_non_finite_point_degrades_to_the_chord() {
        let curve = Curve::Polyline(Polyline {
            points: vec![p(0.0, 0.0), p(f64::NAN, 5.0), p(1.0, 1.0)],
        });
        let cubics = curve.to_cubics(0.01).unwrap();
        assert_eq!(cubics.len(), 1);
        assert_straight(&cubics);
        assert_eq!(cubics[0].from, p(0.0, 0.0));
        assert_eq!(cubics[0].to, p(1.0, 1.0));
        let all_nan = Curve::Polyline(Polyline {
            points: vec![p(f64::NAN, 0.0), p(f64::NAN, 1.0)],
        });
        assert!(all_nan.to_cubics(0.01).unwrap().is_empty());
    }

    #[test]
    fn arc_zero_bow_is_the_chord() {
        let curve = Curve::Arc(Arc {
            start: p(0.0, 0.0),
            end: p(2.0, 1.0),
            bow: 0.0,
        });
        let cubics = curve.to_cubics(0.01).unwrap();
        assert_eq!(cubics.len(), 1);
        assert_straight(&cubics);
        assert_eq!(cubics[0].from, p(0.0, 0.0));
        assert_eq!(cubics[0].to, p(2.0, 1.0));
    }

    #[test]
    fn arc_bows_to_either_side_of_the_chord() {
        for (bow, sign) in [(1.0, 1.0), (-1.0, -1.0)] {
            let curve = Curve::Arc(Arc {
                start: p(0.0, 0.0),
                end: p(2.0, 0.0),
                bow,
            });
            let cubics = curve.to_cubics(0.01).unwrap();
            let apex = samples(&cubics, 16)
                .iter()
                .map(|q| q.y * sign)
                .fold(f64::NEG_INFINITY, f64::max);
            assert!(apex > 0.9, "bow {bow}: apex {apex}");
        }
    }

    #[test]
    fn arc_semicircle_stays_on_its_circle() {
        for bow in [1.0, -1.0] {
            let start = p(-1.0, 0.0);
            let end = p(1.0, 0.0);
            let curve = Curve::Arc(Arc { start, end, bow });
            let cubics = curve.to_cubics(0.01).unwrap();
            assert_eq!(cubics[0].from, start);
            assert_eq!(cubics[cubics.len() - 1].to, end);
            for q in samples(&cubics, 16) {
                approx(q.x.hypot(q.y), 1.0, 0.01);
            }
        }
    }

    #[test]
    fn arc_zero_chord_is_the_chord() {
        let curve = Curve::Arc(Arc {
            start: p(1.0, 1.0),
            end: p(1.0, 1.0),
            bow: 2.0,
        });
        let cubics = curve.to_cubics(0.01).unwrap();
        assert_eq!(cubics.len(), 1);
        assert_straight(&cubics);
    }

    #[test]
    fn arc_with_non_finite_input_degrades_to_the_chord() {
        let bad_bow = Curve::Arc(Arc {
            start: p(0.0, 0.0),
            end: p(1.0, 0.0),
            bow: f64::NAN,
        });
        assert_eq!(bad_bow.to_cubics(0.01).unwrap().len(), 1);
        let bad_start = Curve::Arc(Arc {
            start: p(f64::NAN, 0.0),
            end: p(1.0, 0.0),
            bow: 1.0,
        });
        assert!(bad_start.to_cubics(0.01).unwrap().is_empty());
    }

    #[test]
    fn arc_with_an_overflowing_radius_degrades_to_the_chord() {
        let curve = Curve::Arc(Arc {
            start: p(0.0, 0.0),
            end: p(1.0e154, 0.0),
            bow: 5.0e-324,
        });
        let cubics = curve.to_cubics(0.01).unwrap();
        assert_eq!(cubics.len(), 1);
        assert_straight(&cubics);
    }

    #[test]
    fn arc_with_an_overflowing_apex_degrades_to_the_chord() {
        let curve = Curve::Arc(Arc {
            start: p(0.0, 1.7e308),
            end: p(1.0, 1.7e308),
            bow: 1.7e308,
        });
        let cubics = curve.to_cubics(0.01).unwrap();
        assert_eq!(cubics.len(), 1);
        assert_straight(&cubics);
    }

    #[test]
    fn arc_with_an_overflowing_extent_degrades_to_the_chord() {
        let curve = Curve::Arc(Arc {
            start: p(0.0, 0.0),
            end: p(1.0, 0.0),
            bow: 1.7e308,
        });
        let cubics = curve.to_cubics(0.01).unwrap();
        assert_eq!(cubics.len(), 1);
        assert_straight(&cubics);
    }

    #[test]
    fn arc_with_a_tiny_tolerance_hits_the_segment_cap() {
        let curve = Curve::Arc(Arc {
            start: p(0.0, 0.0),
            end: p(1.0, 0.0),
            bow: 0.5,
        });
        let cubics = curve.to_cubics(1.0e-320).unwrap();
        assert_eq!(cubics.len(), MAX_SEGMENTS);
    }

    #[test]
    fn elliptical_arc_defaults_to_a_circle_through_the_control_point() {
        let (start, ctrl, end) = (p(0.0, 0.0), p(1.0, 1.0), p(2.0, 0.0));
        let curve = Curve::EllipticalArc(EllipticalArc {
            start,
            end,
            ctrl,
            angle: Angle::from_radians(0.0),
            ratio: 1.0,
        });
        let cubics = curve.to_cubics(0.01).unwrap();
        assert_eq!(cubics[0].from, start);
        assert_eq!(cubics[cubics.len() - 1].to, end);
        for q in samples(&cubics, 16) {
            approx((q.x - 1.0).hypot(q.y), 1.0, 0.01);
        }
    }

    #[test]
    fn elliptical_arc_with_angle_and_ratio_stays_on_its_ellipse() {
        // The ellipse x^2/4 + y^2 = 1, traced through (2, 0), (0, 1).
        let curve = Curve::EllipticalArc(EllipticalArc {
            start: p(2.0, 0.0),
            end: p(-2.0, 0.0),
            ctrl: p(0.0, 1.0),
            angle: Angle::from_radians(0.0),
            ratio: 2.0,
        });
        for q in samples(&curve.to_cubics(0.01).unwrap(), 16) {
            approx((q.x / 2.0).powi(2) + q.y.powi(2), 1.0, 0.01);
        }
        // The same ellipse with its major axis along y: x^2 + y^2/4 = 1.
        let rotated = Curve::EllipticalArc(EllipticalArc {
            start: p(0.0, 2.0),
            end: p(0.0, -2.0),
            ctrl: p(-1.0, 0.0),
            angle: Angle::from_radians(FRAC_PI_2),
            ratio: 2.0,
        });
        for q in samples(&rotated.to_cubics(0.01).unwrap(), 16) {
            approx(q.x.powi(2) + (q.y / 2.0).powi(2), 1.0, 0.01);
        }
    }

    #[test]
    fn elliptical_arc_with_a_non_positive_ratio_degrades_to_the_chord() {
        for ratio in [0.0, -1.0] {
            let curve = Curve::EllipticalArc(EllipticalArc {
                start: p(0.0, 0.0),
                end: p(2.0, 0.0),
                ctrl: p(1.0, 1.0),
                angle: Angle::from_radians(0.0),
                ratio,
            });
            let cubics = curve.to_cubics(0.01).unwrap();
            assert_eq!(cubics.len(), 1);
            assert_straight(&cubics);
        }
    }

    #[test]
    fn elliptical_arc_with_a_zero_chord_degrades_to_the_chord() {
        let curve = Curve::EllipticalArc(EllipticalArc {
            start: p(1.0, 1.0),
            end: p(1.0, 1.0),
            ctrl: p(1.0, 2.0),
            angle: Angle::from_radians(0.0),
            ratio: 1.0,
        });
        let cubics = curve.to_cubics(0.01).unwrap();
        assert_eq!(cubics.len(), 1);
        assert_straight(&cubics);
    }

    #[test]
    fn elliptical_arc_with_non_finite_input_degrades_to_the_chord() {
        let bad_ctrl = Curve::EllipticalArc(EllipticalArc {
            start: p(0.0, 0.0),
            end: p(2.0, 0.0),
            ctrl: p(1.0, f64::NAN),
            angle: Angle::from_radians(0.0),
            ratio: 1.0,
        });
        assert_eq!(bad_ctrl.to_cubics(0.01).unwrap().len(), 1);
        let bad_angle = Curve::EllipticalArc(EllipticalArc {
            start: p(0.0, 0.0),
            end: p(2.0, 0.0),
            ctrl: p(1.0, 1.0),
            angle: Angle::from_radians(f64::NAN),
            ratio: 1.0,
        });
        assert_eq!(bad_angle.to_cubics(0.01).unwrap().len(), 1);
        let bad_start = Curve::EllipticalArc(EllipticalArc {
            start: p(f64::NAN, 0.0),
            end: p(2.0, 0.0),
            ctrl: p(1.0, 1.0),
            angle: Angle::from_radians(0.0),
            ratio: 1.0,
        });
        assert!(bad_start.to_cubics(0.01).unwrap().is_empty());
    }

    #[test]
    fn elliptical_arc_with_a_collinear_control_point_degrades_to_the_chord() {
        let curve = Curve::EllipticalArc(EllipticalArc {
            start: p(0.0, 0.0),
            end: p(2.0, 0.0),
            ctrl: p(1.0, 0.0),
            angle: Angle::from_radians(0.0),
            ratio: 1.0,
        });
        let cubics = curve.to_cubics(0.01).unwrap();
        assert_eq!(cubics.len(), 1);
        assert_straight(&cubics);
    }

    #[test]
    fn elliptical_arc_with_an_extreme_ratio_degrades_to_the_chord() {
        let curve = Curve::EllipticalArc(EllipticalArc {
            start: p(0.0, 0.0),
            end: p(1.0, 0.0),
            ctrl: p(0.0, 1.0),
            angle: Angle::from_radians(0.0),
            ratio: 1.0e200,
        });
        let cubics = curve.to_cubics(0.01).unwrap();
        assert_eq!(cubics.len(), 1);
        assert_straight(&cubics);
    }

    #[test]
    fn elliptical_arc_with_a_near_collinear_control_point_degrades_to_the_chord() {
        // The fitted ellipse has a center around 5e199 and a radius above
        // the finite range: degrade instead of overflowing.
        let curve = Curve::EllipticalArc(EllipticalArc {
            start: p(0.0, 0.0),
            end: p(1.0, 0.0),
            ctrl: p(0.5, 1.0e-200),
            angle: Angle::from_radians(0.0),
            ratio: 1.0,
        });
        let cubics = curve.to_cubics(0.01).unwrap();
        assert_eq!(cubics.len(), 1);
        assert_straight(&cubics);
    }

    #[test]
    fn elliptical_arc_with_overflowing_squares_degrades_to_the_chord() {
        let curve = Curve::EllipticalArc(EllipticalArc {
            start: p(1.7e308, 0.0),
            end: p(1.7e308, 1.0),
            ctrl: p(0.0, 2.0),
            angle: Angle::from_radians(0.0),
            ratio: 1.0,
        });
        let cubics = curve.to_cubics(0.01).unwrap();
        assert_eq!(cubics.len(), 1);
        assert_straight(&cubics);
    }

    /// The quadratic Bézier through `q0`, `q1`, `q2` at `t`.
    fn quad_bezier(q0: Point, q1: Point, q2: Point, t: f64) -> Point {
        let m = 1.0 - t;
        Point::new(
            m * m * q0.x + 2.0 * m * t * q1.x + t * t * q2.x,
            m * m * q0.y + 2.0 * m * t * q1.y + t * t * q2.y,
        )
    }

    fn nurbs_data(weights: [f64; 3]) -> Nurbs {
        Nurbs {
            degree: 2,
            x_type: 1,
            y_type: 1,
            knot_last: 1.0,
            points: vec![
                NurbsPoint {
                    x: 0.0,
                    y: 0.0,
                    knot: 0.0,
                    weight: weights[0],
                },
                NurbsPoint {
                    x: 1.0,
                    y: 2.0,
                    knot: 0.0,
                    weight: weights[1],
                },
                NurbsPoint {
                    x: 2.0,
                    y: 0.0,
                    knot: 0.0,
                    weight: weights[2],
                },
            ],
        }
    }

    #[test]
    fn nurbs_degree_two_is_a_quadratic_bezier() {
        let cubics = Curve::Nurbs(nurbs_data([1.0, 1.0, 1.0]))
            .to_cubics(1.0e-3)
            .unwrap();
        assert_all_finite(&cubics);
        assert_eq!(cubics[0].from, p(0.0, 0.0));
        assert_eq!(cubics[cubics.len() - 1].to, p(2.0, 0.0));
        for q in samples(&cubics, 8) {
            let want = quad_bezier(p(0.0, 0.0), p(1.0, 2.0), p(2.0, 0.0), q.x / 2.0);
            approx(q.y, want.y, 1.0e-3);
        }
    }

    #[test]
    fn nurbs_with_rational_weights_is_a_rational_bezier() {
        let cubics = Curve::Nurbs(nurbs_data([1.0, 2.0, 1.0]))
            .to_cubics(1.0e-3)
            .unwrap();
        assert_all_finite(&cubics);
        for q in samples(&cubics, 8) {
            // Invert the rational quadratic numerically along x, then
            // compare the y coordinate.
            let x = q.x;
            let mut lo = 0.0;
            let mut hi = 1.0;
            for _ in 0..60 {
                let t = 0.5 * (lo + hi);
                let m = 1.0 - t;
                let w = m * m + 2.0 * m * t * 2.0 + t * t;
                let bx = (2.0 * m * t * 2.0 * 1.0 + t * t * 2.0) / w;
                if bx < x {
                    lo = t;
                } else {
                    hi = t;
                }
            }
            let t = 0.5 * (lo + hi);
            let m = 1.0 - t;
            let w = m * m + 4.0 * m * t + t * t;
            let want_y = (4.0 * m * t * 2.0) / w;
            approx(q.y, want_y, 2.0e-3);
        }
    }

    #[test]
    fn nurbs_with_a_non_positive_weight_degrades_to_the_chord() {
        for weight in [0.0, -1.0] {
            let cubics = Curve::Nurbs(nurbs_data([1.0, weight, 1.0]))
                .to_cubics(0.01)
                .unwrap();
            assert_eq!(cubics.len(), 1);
            assert_straight(&cubics);
        }
    }

    #[test]
    fn nurbs_with_an_overflowing_weight_degrades_to_the_chord() {
        let cubics = Curve::Nurbs(nurbs_data([1.0, 1.0e308, 1.0]))
            .to_cubics(0.01)
            .unwrap();
        assert_eq!(cubics.len(), 1);
        assert_straight(&cubics);
    }

    #[test]
    fn nurbs_with_unsorted_knots_degrades_to_the_chord() {
        let mut curve = nurbs_data([1.0, 1.0, 1.0]);
        curve.points[1].knot = 2.0;
        curve.knot_last = 1.0;
        let cubics = Curve::Nurbs(curve).to_cubics(0.01).unwrap();
        assert_eq!(cubics.len(), 1);
        assert_straight(&cubics);
    }

    #[test]
    fn nurbs_with_an_overlong_knot_run_degrades_to_the_chord() {
        // All knots equal: the basis vanishes over the whole domain.
        let mut curve = nurbs_data([1.0, 1.0, 1.0]);
        curve.knot_last = 0.0;
        let cubics = Curve::Nurbs(curve).to_cubics(0.01).unwrap();
        assert_eq!(cubics.len(), 1);
        assert_straight(&cubics);
    }

    #[test]
    fn nurbs_with_a_bad_degree_degrades_to_the_chord() {
        for degree in [0, 26] {
            let mut curve = nurbs_data([1.0, 1.0, 1.0]);
            curve.degree = degree;
            let cubics = Curve::Nurbs(curve).to_cubics(0.01).unwrap();
            assert_eq!(cubics.len(), 1);
            assert_straight(&cubics);
        }
        // A degree-3 spline needs at least four control points.
        let mut curve = nurbs_data([1.0, 1.0, 1.0]);
        curve.degree = 3;
        let cubics = Curve::Nurbs(curve).to_cubics(0.01).unwrap();
        assert_eq!(cubics.len(), 1);
        assert_straight(&cubics);
    }

    #[test]
    fn nurbs_with_non_finite_values_degrades_to_the_chord() {
        let mut bad_knot = nurbs_data([1.0, 1.0, 1.0]);
        bad_knot.points[1].knot = f64::NAN;
        assert_eq!(Curve::Nurbs(bad_knot).to_cubics(0.01).unwrap().len(), 1);
        let mut bad_point = nurbs_data([1.0, 1.0, 1.0]);
        bad_point.points[1].x = f64::INFINITY;
        assert_eq!(Curve::Nurbs(bad_point).to_cubics(0.01).unwrap().len(), 1);
        let mut bad_weight = nurbs_data([1.0, 1.0, 1.0]);
        bad_weight.points[1].weight = f64::INFINITY;
        assert_eq!(Curve::Nurbs(bad_weight).to_cubics(0.01).unwrap().len(), 1);
    }

    #[test]
    fn nurbs_with_fewer_than_two_points_holds_nothing() {
        let empty = Nurbs {
            degree: 2,
            x_type: 1,
            y_type: 1,
            knot_last: 1.0,
            points: Vec::new(),
        };
        assert!(Curve::Nurbs(empty).to_cubics(0.01).unwrap().is_empty());
        let mut one = nurbs_data([1.0, 1.0, 1.0]);
        one.points.truncate(1);
        assert!(Curve::Nurbs(one).to_cubics(0.01).unwrap().is_empty());
    }

    #[test]
    fn nurbs_fraction_flags_are_ignored() {
        let mut local = nurbs_data([1.0, 1.0, 1.0]);
        let mut fractions = local.clone();
        fractions.x_type = 0;
        fractions.y_type = 0;
        local.x_type = 1;
        local.y_type = 1;
        assert_eq!(
            Curve::Nurbs(local).to_cubics(0.01).unwrap(),
            Curve::Nurbs(fractions).to_cubics(0.01).unwrap()
        );
    }

    fn spline_curve(knots: Vec<f64>, degree: u32) -> Curve {
        Curve::Spline(Spline {
            degree,
            ctrl: vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0)],
            knots,
        })
    }

    #[test]
    fn spline_degree_one_is_a_polyline() {
        let curve = spline_curve(vec![0.0, 0.0, 1.0, 2.0], 1);
        let cubics = curve.to_cubics(0.01).unwrap();
        assert_all_finite(&cubics);
        assert_eq!(cubics[0].from, p(0.0, 0.0));
        assert_eq!(cubics[cubics.len() - 1].to, p(1.0, 1.0));
        // Every sample lies on the polyline (0,0)-(1,0)-(1,1).
        for q in samples(&cubics, 8) {
            let d = distance_to_segment(q, p(0.0, 0.0), p(1.0, 0.0)).min(distance_to_segment(
                q,
                p(1.0, 0.0),
                p(1.0, 1.0),
            ));
            assert!(d <= 0.01, "{q:?} is {d} off the polyline");
        }
    }

    #[test]
    fn spline_with_the_wrong_knot_count_degrades_to_the_chord() {
        let curve = spline_curve(vec![0.0, 1.0, 2.0], 1);
        let cubics = curve.to_cubics(0.01).unwrap();
        assert_eq!(cubics.len(), 1);
        assert_straight(&cubics);
        assert_eq!(cubics[0].from, p(0.0, 0.0));
        assert_eq!(cubics[0].to, p(1.0, 1.0));
    }

    #[test]
    fn spline_with_an_overlong_knot_run_degrades_to_the_chord() {
        // Five knots for four control points (degree 1), but the run of
        // 1s grows to four with the padded end knots: degenerate.
        let curve = Curve::Spline(Spline {
            degree: 1,
            ctrl: vec![p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0), p(2.0, 1.0)],
            knots: vec![0.0, 0.0, 1.0, 1.0, 1.0],
        });
        let cubics = curve.to_cubics(0.01).unwrap();
        assert_eq!(cubics.len(), 1);
        assert_straight(&cubics);
    }

    #[test]
    fn spline_with_a_tiny_tolerance_hits_the_caps() {
        let curve = Curve::Spline(Spline {
            degree: 3,
            ctrl: vec![
                p(0.0, 0.0),
                p(1.0, 2.0),
                p(2.0, -1.0),
                p(3.0, 3.0),
                p(4.0, -2.0),
                p(5.0, 1.0),
                p(6.0, -1.0),
                p(7.0, 0.0),
            ],
            knots: vec![0.0, 0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
        });
        let cubics = curve.to_cubics(1.0e-300).unwrap();
        assert_all_finite(&cubics);
        assert!(cubics.len() <= MAX_SEGMENTS + 32, "{}", cubics.len());
        assert!(cubics.len() > 32, "{}", cubics.len());
    }

    #[test]
    fn spline_through_one_point_stays_a_point() {
        let curve = Curve::Spline(Spline {
            degree: 1,
            ctrl: vec![p(3.0, 4.0), p(3.0, 4.0)],
            knots: vec![0.0, 0.0, 1.0],
        });
        let cubics = curve.to_cubics(0.01).unwrap();
        assert_all_finite(&cubics);
        for q in samples(&cubics, 8) {
            approx(q.x, 3.0, 1.0e-12);
            approx(q.y, 4.0, 1.0e-12);
        }
    }

    #[test]
    fn a_non_finite_or_tiny_tolerance_uses_the_minimum() {
        let curve = Curve::Arc(Arc {
            start: p(0.0, 0.0),
            end: p(1.0, 0.0),
            bow: 0.5,
        });
        let base = curve.to_cubics(MIN_TOLERANCE).unwrap();
        for tol in [f64::NAN, 0.0, -1.0] {
            assert_eq!(curve.to_cubics(tol).unwrap().len(), base.len());
        }
        // An infinite tolerance asks for one piece per 90 degrees.
        let loose = curve.to_cubics(f64::INFINITY).unwrap();
        assert_eq!(loose.len(), 2);
    }
}
