//! Connector routing (ADR 0045): straight and simple right-angle paths
//! between two points.
//!
//! The routing is our own, not Visio's: no obstacle avoidance in Phase 5,
//! and a deterministic elbow choice. The routing *policy* (when to
//! re-route, which endpoints are glued) lives in `crates/commands`
//! (L4); these functions are pure geometry in inches (ADR 0011). Task
//! P5-08 implements the bodies.
//!
//! Part of Ferrogram. Alpha: the signatures are fixed by task P5-01 and
//! the bodies are `Err(Error::NotImplemented)` stubs.

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
/// at least those two points. A degenerate case (identical endpoints)
/// yields the two points and nothing else; it is never an error and never
/// a panic (AGENTS.md section 6).
pub fn route(start: Point, end: Point, options: &RouteOptions) -> Result<Vec<Point>, Error> {
    let _ = (start, end, options);
    Err(Error::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_p5_01_stub_is_not_implemented() {
        let p = Point::new(0.0, 0.0);
        assert_eq!(
            route(p, Point::new(1.0, 1.0), &RouteOptions::default()),
            Err(Error::NotImplemented)
        );
    }

    #[test]
    fn options_hold_their_style() {
        assert_eq!(RouteOptions::default().style, RouteStyle::Straight);
        let o = RouteOptions::new(RouteStyle::RightAngle);
        assert_eq!(o.style, RouteStyle::RightAngle);
    }
}
