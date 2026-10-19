//! Connector routing through the public API (task P5-08, ADR 0045).

use ferrogram_geom::{Error, Point, RouteOptions, RouteStyle, route};
use proptest::prelude::*;

#[test]
fn straight_is_one_segment() {
    let start = Point::new(1.0, 2.0);
    let end = Point::new(4.0, -1.0);
    assert_eq!(
        route(start, end, &RouteOptions::default()).expect("route"),
        vec![start, end]
    );
}

#[test]
fn right_angle_is_horizontal_first_when_dx_is_at_least_dy() {
    let start = Point::new(0.0, 0.0);
    let end = Point::new(4.0, 1.0);
    assert_eq!(
        route(start, end, &RouteOptions::new(RouteStyle::RightAngle)).expect("route"),
        vec![start, Point::new(4.0, 0.0), end]
    );
}

#[test]
fn right_angle_is_vertical_first_when_dy_is_larger() {
    let start = Point::new(0.0, 0.0);
    let end = Point::new(1.0, 4.0);
    assert_eq!(
        route(start, end, &RouteOptions::new(RouteStyle::RightAngle)).expect("route"),
        vec![start, Point::new(0.0, 4.0), end]
    );
}

#[test]
fn degenerate_endpoints_return_the_two_points() {
    let p = Point::new(3.0, -2.0);
    for style in [RouteStyle::Straight, RouteStyle::RightAngle] {
        assert_eq!(
            route(p, p, &RouteOptions::new(style)).expect("route"),
            vec![p, p]
        );
    }
    // An axis-aligned pair has no room for an elbow.
    let start = Point::new(0.0, 0.0);
    for end in [Point::new(5.0, 0.0), Point::new(0.0, 5.0)] {
        assert_eq!(
            route(start, end, &RouteOptions::new(RouteStyle::RightAngle)).expect("route"),
            vec![start, end]
        );
    }
}

#[test]
fn non_finite_points_are_invalid_not_a_panic() {
    let fine = Point::new(0.0, 0.0);
    let bad = [
        Point::new(f64::NAN, 0.0),
        Point::new(0.0, f64::INFINITY),
        Point::new(f64::NEG_INFINITY, 2.0),
    ];
    for point in bad {
        for style in [RouteStyle::Straight, RouteStyle::RightAngle] {
            let options = RouteOptions::new(style);
            assert_eq!(route(fine, point, &options), Err(Error::InvalidArgument));
            assert_eq!(route(point, fine, &options), Err(Error::InvalidArgument));
        }
    }
}

proptest! {
    #[test]
    fn every_route_keeps_its_endpoints(
        x1 in -100.0f64..100.0,
        y1 in -100.0f64..100.0,
        x2 in -100.0f64..100.0,
        y2 in -100.0f64..100.0,
        right_angle in any::<bool>(),
    ) {
        let start = Point::new(x1, y1);
        let end = Point::new(x2, y2);
        let style = if right_angle { RouteStyle::RightAngle } else { RouteStyle::Straight };
        let points = route(start, end, &RouteOptions::new(style)).expect("finite route");
        prop_assert!(points.len() >= 2);
        prop_assert_eq!(points.first(), Some(&start));
        prop_assert_eq!(points.last(), Some(&end));
        if right_angle && points.len() == 3 {
            let elbow = points.get(1).copied().expect("elbow");
            // Adjacent segments are axis-aligned.
            prop_assert!(elbow.x == start.x || elbow.y == start.y);
            prop_assert!(elbow.x == end.x || elbow.y == end.y);
        }
        if !right_angle {
            prop_assert_eq!(points.len(), 2);
        }
    }
}
