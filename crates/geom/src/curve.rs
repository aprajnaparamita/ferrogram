//! Curves: circular arcs, elliptical arcs, Béziers, NURBS and splines.
//!
//! These are the mathematical forms of the Geometry rows (ADR 0022); the
//! typed rows live in `ferrogram-model` and the renderer maps them to
//! [`Curve`] values. [`Curve::to_cubics`] is the single flattening point:
//! it approximates any curve with [`Cubic`] segments within a tolerance in
//! inches (ADR 0022).

use crate::angle::Angle;
use crate::error::Error;
use crate::point::Point;

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
    pub points: Vec<NurbsPoint>,
}

/// A B-spline (`SplineStart` + `SplineKnot` rows; ADR 0022).
#[derive(Debug, Clone, PartialEq)]
pub struct Spline {
    /// The degree of the spline (1 to 25).
    pub degree: u32,
    /// The control points, in order.
    pub ctrl: Vec<Point>,
    /// The knots, in order.
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
    /// straight chord; it is not an error and never panics.
    ///
    /// Returns [`Error::NotImplemented`] until the curve work lands
    /// (P3-03).
    pub fn to_cubics(&self, tolerance: f64) -> Result<Vec<Cubic>, Error> {
        let _ = tolerance;
        let _ = self;
        Err(Error::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn to_cubics_is_not_implemented_yet() {
        let c = Curve::Cubic(Cubic {
            from: Point::new(0.0, 0.0),
            ctrl1: Point::new(1.0, 1.0),
            ctrl2: Point::new(2.0, 1.0),
            to: Point::new(3.0, 0.0),
        });
        assert_eq!(c.to_cubics(0.01), Err(Error::NotImplemented));
    }
}
