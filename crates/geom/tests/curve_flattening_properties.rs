//! Property tests for curve flattening (task P3-03).
//!
//! Two properties (ADR 0022, P3-03 acceptance): the flattened cubics of a
//! circular arc stay within the tolerance of its exact circle, and the
//! conversion never panics (and always yields finite points) on random
//! bounded inputs of every curve kind.

use ferrogram_geom::{
    Angle, Arc, Cubic, Curve, EllipticalArc, Nurbs, NurbsPoint, Point, Polyline, Spline,
};
use proptest::prelude::*;

fn point(x: f64, y: f64) -> Point {
    Point::new(x, y)
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

fn assert_finite(cubics: &[Cubic]) {
    for (i, c) in cubics.iter().enumerate() {
        for q in [c.from, c.ctrl1, c.ctrl2, c.to] {
            assert!(q.x.is_finite() && q.y.is_finite(), "segment {i}: {q:?}");
        }
    }
}

proptest! {
    /// The flattened cubics of an arc stay within the tolerance of the
    /// exact circle through its start, end and apex, and the endpoints are
    /// exact.
    #[test]
    fn arc_flattening_stays_within_tolerance_of_its_exact_circle(
        sx in -100.0f64..100.0,
        sy in -100.0f64..100.0,
        bow in -50.0f64..50.0,
        tolerance in 1.0e-3f64..1.0,
    ) {
        prop_assume!(bow.abs() > 1.0e-3);
        let start = point(sx, sy);
        let end = point(sx + 10.0, sy);
        // The exact circle of the bow construction (see `Arc`).
        let chord2 = 100.0;
        let r_signed = 0.5 * bow + chord2 / (8.0 * bow);
        let radius = r_signed.abs();
        let center = point(sx + 5.0, sy + bow - r_signed);

        let curve = Curve::Arc(Arc { start, end, bow });
        let cubics = curve.to_cubics(tolerance).expect("never fails");
        prop_assert!(!cubics.is_empty());
        assert_finite(&cubics);
        prop_assert_eq!(cubics[0].from, start);
        prop_assert_eq!(cubics[cubics.len() - 1].to, end);

        let slack = tolerance * 1.0e-6 + 1.0e-9;
        for q in samples(&cubics, 16) {
            let d = (q.x - center.x).hypot(q.y - center.y) - radius;
            prop_assert!(
                d.abs() <= tolerance + slack,
                "{q:?} is {d} off the circle of radius {radius}"
            );
        }
    }

    /// Every curve kind converts on random bounded input: no panic, and
    /// every output point is finite.
    #[test]
    fn conversions_never_panic_on_random_bounded_inputs(
        kind in 0u8..7,
        points in proptest::collection::vec(-1.0e3f64..1.0e3, 0..12),
        more in proptest::collection::vec(-1.0e3f64..1.0e3, 0..12),
        bow in -100.0f64..100.0,
        ratio in -1.0f64..100.0,
        angle in -7.0f64..7.0,
        degree in 0u32..30,
        knots in proptest::collection::vec(-10.0f64..10.0, 0..16),
        weights in proptest::collection::vec(0.0f64..100.0, 0..12),
        tolerance in 1.0e-6f64..10.0,
    ) {
        let xy: Vec<Point> = points
            .iter()
            .step_by(2)
            .zip(points.iter().step_by(2).skip(1))
            .map(|(&x, &y)| point(x, y))
            .collect();
        let curve = match kind {
            0 => Curve::Arc(Arc {
                start: xy.first().copied().unwrap_or(point(0.0, 0.0)),
                end: xy.last().copied().unwrap_or(point(1.0, 0.0)),
                bow,
            }),
            1 => Curve::EllipticalArc(EllipticalArc {
                start: xy.first().copied().unwrap_or(point(0.0, 0.0)),
                end: xy.last().copied().unwrap_or(point(1.0, 0.0)),
                ctrl: xy.get(1).copied().unwrap_or(point(0.5, 1.0)),
                angle: Angle::from_radians(angle),
                ratio,
            }),
            2 => Curve::Quad {
                from: xy.first().copied().unwrap_or(point(0.0, 0.0)),
                ctrl: xy.get(1).copied().unwrap_or(point(0.5, 1.0)),
                to: xy.last().copied().unwrap_or(point(1.0, 0.0)),
            },
            3 => Curve::Cubic(Cubic {
                from: xy.first().copied().unwrap_or(point(0.0, 0.0)),
                ctrl1: xy.get(1).copied().unwrap_or(point(0.25, 1.0)),
                ctrl2: xy.get(2).copied().unwrap_or(point(0.75, 1.0)),
                to: xy.last().copied().unwrap_or(point(1.0, 0.0)),
            }),
            4 => Curve::Nurbs(Nurbs {
                degree,
                x_type: 1,
                y_type: 0,
                knot_last: knots.last().copied().unwrap_or(1.0),
                points: xy
                    .iter()
                    .enumerate()
                    .map(|(i, p)| NurbsPoint {
                        x: p.x,
                        y: p.y,
                        knot: knots.get(i).copied().unwrap_or(0.0),
                        weight: weights.get(i).copied().unwrap_or(1.0),
                    })
                    .collect(),
            }),
            5 => {
                // Sorted knots exercise the real path; the raw list
                // exercises the unsorted degrade.
                let mut sorted = knots.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                Curve::Spline(Spline {
                    degree,
                    ctrl: xy.clone(),
                    knots: if more.len() % 2 == 0 { sorted } else { knots.clone() },
                })
            }
            _ => {
                let mut pts = xy.clone();
                pts.extend(more.iter().step_by(2).map(|&x| point(x, 0.0)));
                Curve::Polyline(Polyline { points: pts })
            }
        };
        let cubics = curve.to_cubics(tolerance).expect("never fails");
        assert_finite(&cubics);
    }
}
