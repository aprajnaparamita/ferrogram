//! Connector routing (ADR 0045): straight and simple right-angle paths
//! between two points.
//!
//! The routing is our own, not Visio's: no obstacle avoidance in Phase 5,
//! and a deterministic elbow choice. The routing *policy* (when to
//! re-route, which endpoints are glued) lives in `crates/commands`
//! (L4); these functions are pure geometry in inches (ADR 0011).
//!
//! Part of Larrygram. Alpha.

use crate::error::Error;
use crate::point::Point;

/// How a connector path is routed (ADR 0045).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum RouteStyle {
    /// One straight segment.
    #[default]
    Straight,
    /// Segments parallel to the axes (a simple elbow, no obstacles).
    RightAngle,
}

/// Routing options (ADR 0045).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RouteOptions {
    /// The routing style.
    pub style: RouteStyle,
}

impl RouteOptions {
    /// Options with the given style.
    pub const fn new(style: RouteStyle) -> Self {
        Self { style }
    }
}

impl Default for RouteOptions {
    fn default() -> Self {
        Self::new(RouteStyle::Straight)
    }
}

/// The path points of a connector from `start` to `end`, in inches.
///
/// The result always begins with `start` and ends with `end`, and holds
/// at least those two points. A degenerate case (identical endpoints, or
/// an elbow that coincides with an endpoint) yields the two points and
/// nothing else; it is never an error and never a panic (AGENTS.md
/// section 6).
///
/// The right-angle elbow is deterministic (ADR 0045): horizontal first
/// when `|dx| >= |dy|`, vertical first otherwise. No point is repeated.
///
/// A non-finite endpoint is [`Error::InvalidArgument`]: it has no sensible
/// page position and would poison the written geometry.
pub fn route(start: Point, end: Point, options: &RouteOptions) -> Result<Vec<Point>, Error> {
    if !finite(start) || !finite(end) {
        return Err(Error::InvalidArgument);
    }
    match options.style {
        RouteStyle::Straight => Ok(vec![start, end]),
        RouteStyle::RightAngle => {
            let dx = (end.x - start.x).abs();
            let dy = (end.y - start.y).abs();
            let elbow = if dx >= dy {
                Point::new(end.x, start.y)
            } else {
                Point::new(start.x, end.y)
            };
            if same(elbow, start) || same(elbow, end) {
                Ok(vec![start, end])
            } else {
                Ok(vec![start, elbow, end])
            }
        }
    }
}

/// Whether both coordinates are finite.
fn finite(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

/// Exact point equality (the caller's coordinates are not rounded).
fn same(a: Point, b: Point) -> bool {
    a.x == b.x && a.y == b.y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_hold_their_style() {
        assert_eq!(RouteOptions::default().style, RouteStyle::Straight);
        let o = RouteOptions::new(RouteStyle::RightAngle);
        assert_eq!(o.style, RouteStyle::RightAngle);
    }

    #[test]
    fn a_straight_route_is_the_two_endpoints() {
        let start = Point::new(1.0, 2.0);
        let end = Point::new(4.0, -1.0);
        let points = route(start, end, &RouteOptions::default()).expect("route");
        assert_eq!(points, vec![start, end]);
    }

    #[test]
    fn a_right_angle_route_goes_horizontal_first_when_dx_is_larger() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(3.0, 1.0);
        let points = route(start, end, &RouteOptions::new(RouteStyle::RightAngle)).expect("route");
        assert_eq!(
            points,
            vec![start, Point::new(3.0, 0.0), end],
            "the elbow keeps the end x and the start y"
        );
    }

    #[test]
    fn a_right_angle_route_goes_vertical_first_when_dy_is_larger() {
        let start = Point::new(0.0, 0.0);
        let end = Point::new(1.0, 4.0);
        let points = route(start, end, &RouteOptions::new(RouteStyle::RightAngle)).expect("route");
        assert_eq!(
            points,
            vec![start, Point::new(0.0, 4.0), end],
            "the elbow keeps the start x and the end y"
        );
    }

    #[test]
    fn the_tie_goes_horizontal_first() {
        let start = Point::new(-2.0, 5.0);
        let end = Point::new(2.0, 1.0);
        let points = route(start, end, &RouteOptions::new(RouteStyle::RightAngle)).expect("route");
        assert_eq!(
            points,
            vec![start, Point::new(2.0, 5.0), end],
            "|dx| == |dy| uses the horizontal-first rule"
        );
    }

    #[test]
    fn a_degenerate_right_angle_route_has_no_repeated_elbow() {
        let options = RouteOptions::new(RouteStyle::RightAngle);
        let p = Point::new(1.5, -2.0);
        assert_eq!(route(p, p, &options).expect("route"), vec![p, p]);

        // A purely horizontal end makes the elbow the end point.
        let start = Point::new(0.0, 0.0);
        let end = Point::new(3.0, 0.0);
        assert_eq!(
            route(start, end, &options).expect("route"),
            vec![start, end]
        );

        // A purely vertical start/end makes the vertical-first elbow the
        // end point too.
        let end = Point::new(0.0, 3.0);
        assert_eq!(
            route(start, end, &options).expect("route"),
            vec![start, end]
        );
    }

    #[test]
    fn a_straight_degenerate_route_keeps_the_two_points() {
        let p = Point::new(2.0, 3.0);
        assert_eq!(
            route(p, p, &RouteOptions::default()).expect("route"),
            vec![p, p]
        );
    }

    #[test]
    fn a_non_finite_endpoint_is_invalid() {
        let fine = Point::new(0.0, 0.0);
        for bad in [
            Point::new(f64::NAN, 0.0),
            Point::new(0.0, f64::INFINITY),
            Point::new(f64::NEG_INFINITY, 0.0),
        ] {
            assert_eq!(
                route(fine, bad, &RouteOptions::default()),
                Err(Error::InvalidArgument)
            );
            assert_eq!(
                route(bad, fine, &RouteOptions::new(RouteStyle::RightAngle)),
                Err(Error::InvalidArgument)
            );
        }
    }

    #[test]
    fn every_route_starts_and_ends_at_its_endpoints() {
        let options = [RouteStyle::Straight, RouteStyle::RightAngle];
        for style in options {
            let start = Point::new(-1.25, 0.5);
            let end = Point::new(0.75, -2.5);
            let points = route(start, end, &RouteOptions::new(style)).expect("route");
            assert_eq!(points.first(), Some(&start));
            assert_eq!(points.last(), Some(&end));
            assert!(points.len() >= 2);
        }
    }
}
