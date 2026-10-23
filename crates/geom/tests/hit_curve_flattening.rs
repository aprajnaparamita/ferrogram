//! Curve outlines hit within tolerance: every curve family flattens
//! through `Curve::to_cubics` (ADR 0022) and the hit math measures the
//! flattened chords, so a point on the true curve is within tolerance of
//! the path outline (ADR 0041). One test per curve family.

use core::f64::consts::FRAC_PI_4;

use larrygram_geom::{
    Arc, Cubic, Curve, EllipticalArc, FLATTEN_TOLERANCE, Nurbs, NurbsPoint, Path, Point, Polyline,
    Segment, Spline, distance_point_path,
};

/// The flattening tolerance the tests build their paths with, in inches.
const TOLERANCE: f64 = FLATTEN_TOLERANCE;

/// The path of the curve flattened to cubics (what `render::build` does
/// with the row curves).
#[allow(clippy::expect_used)] // test helper; a failing build is a test failure
fn curve_path(curve: &Curve) -> Path {
    let cubics = curve.to_cubics(TOLERANCE).expect("to_cubics is always Ok");
    let mut path = Path::new();
    let mut started = false;
    for cubic in &cubics {
        if !started {
            path.push(Segment::MoveTo(cubic.from));
            started = true;
        }
        path.push(Segment::CubicTo {
            ctrl1: cubic.ctrl1,
            ctrl2: cubic.ctrl2,
            to: cubic.to,
        });
    }
    path
}

/// The point on the true curve must be within tolerance of the outline.
/// The bound covers the two flattening stages (curve to cubics, cubics to
/// chords).
#[allow(clippy::expect_used)] // test helper; a failing build is a test failure
fn assert_on_curve(curve: &Curve, point: Point) {
    let path = curve_path(curve);
    assert!(!path.is_empty(), "the curve flattened to nothing");
    let distance = distance_point_path(point, &path).expect("finite input");
    assert!(
        distance <= 3.0 * TOLERANCE,
        "a point on the curve measured {distance} from the outline"
    );
}

#[test]
fn arc_hits_within_tolerance() {
    // The arc through the apex: chord midpoint + bow along the normal.
    // The circle is centred at (0.5, 0) with radius 0.5.
    let curve = Curve::Arc(Arc {
        start: Point::new(0.0, 0.0),
        end: Point::new(1.0, 0.0),
        bow: 0.5,
    });
    assert_on_curve(&curve, Point::new(0.5, 0.5));
    // An interior point of a flattened piece (45 degrees up), not a
    // piece boundary.
    let (sin, cos) = FRAC_PI_4.sin_cos();
    assert_on_curve(&curve, Point::new(0.5 + 0.5 * cos, 0.5 * sin));
}

#[test]
fn elliptical_arc_hits_within_tolerance() {
    // The upper unit semicircle; its control point is on the arc.
    let curve = Curve::EllipticalArc(EllipticalArc {
        start: Point::new(1.0, 0.0),
        end: Point::new(-1.0, 0.0),
        ctrl: Point::new(0.0, 1.0),
        angle: larrygram_geom::Angle::from_radians(0.0),
        ratio: 1.0,
    });
    assert_on_curve(&curve, Point::new(0.0, 1.0));
}

#[test]
fn quad_hits_within_tolerance() {
    let curve = Curve::Quad {
        from: Point::new(0.0, 0.0),
        ctrl: Point::new(0.5, 1.0),
        to: Point::new(1.0, 0.0),
    };
    // The quadratic midpoint at t = 0.5.
    assert_on_curve(&curve, Point::new(0.5, 0.5));
}

#[test]
fn cubic_hits_within_tolerance() {
    let curve = Curve::Cubic(Cubic {
        from: Point::new(0.0, 0.0),
        ctrl1: Point::new(0.0, 1.0),
        ctrl2: Point::new(1.0, 1.0),
        to: Point::new(1.0, 0.0),
    });
    // The cubic midpoint at t = 0.5.
    assert_on_curve(&curve, Point::new(0.5, 0.75));
}

#[test]
fn nurbs_hits_within_tolerance() {
    // A clamped quadratic B-spline with unit weights is the quadratic
    // Bézier through (0, 0), (0, 1), (1, 0); its t = 0.5 point is
    // (0.25, 0.5).
    let curve = Curve::Nurbs(Nurbs {
        degree: 2,
        x_type: 1,
        y_type: 1,
        knot_last: 1.0,
        points: vec![
            NurbsPoint {
                x: 0.0,
                y: 0.0,
                knot: 0.0,
                weight: 1.0,
            },
            NurbsPoint {
                x: 0.0,
                y: 1.0,
                knot: 0.0,
                weight: 1.0,
            },
            NurbsPoint {
                x: 1.0,
                y: 0.0,
                knot: 0.0,
                weight: 1.0,
            },
        ],
    });
    assert_on_curve(&curve, Point::new(0.25, 0.5));
}

#[test]
fn spline_hits_within_tolerance() {
    // The same clamped quadratic as a `Spline` row pair.
    let curve = Curve::Spline(Spline {
        degree: 2,
        ctrl: vec![
            Point::new(0.0, 0.0),
            Point::new(0.0, 1.0),
            Point::new(1.0, 0.0),
        ],
        knots: vec![0.0, 0.0, 0.0, 1.0],
    });
    assert_on_curve(&curve, Point::new(0.25, 0.5));
}

#[test]
fn polyline_hits_within_tolerance() {
    let curve = Curve::Polyline(Polyline {
        points: vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(1.0, 1.0),
        ],
    });
    assert_on_curve(&curve, Point::new(0.5, 0.0));
}

#[test]
fn a_point_off_the_outline_misses_with_a_tight_tolerance() {
    let curve = Curve::Arc(Arc {
        start: Point::new(0.0, 0.0),
        end: Point::new(1.0, 0.0),
        bow: 0.5,
    });
    let path = curve_path(&curve);
    // Well above the apex of the arc.
    let distance = distance_point_path(Point::new(0.5, 0.9), &path).expect("finite input");
    assert!(distance > 0.3, "distance {distance}");
}

#[test]
fn a_rotated_curve_hits_the_same_way() {
    // With `ratio` 1 the elliptical arc is the circle through its three
    // points whatever the major-axis angle; `ctrl` is on the arc.
    let curve = Curve::EllipticalArc(EllipticalArc {
        start: Point::new(0.5, -0.5),
        end: Point::new(-0.5, 0.5),
        ctrl: Point::new(-0.5, -0.5),
        angle: larrygram_geom::Angle::from_radians(FRAC_PI_4),
        ratio: 1.0,
    });
    assert_on_curve(&curve, Point::new(-0.5, -0.5));
}
