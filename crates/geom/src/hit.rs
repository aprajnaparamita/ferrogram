//! Hit-test math: distances from a point to segments, path outlines and
//! rectangles (ADR 0041).
//!
//! The functions are pure geometry in inches (ADR 0011). The hit policy
//! (fill versus stroke, tolerance in screen pixels, z-order) lives in
//! `ferrogram-render::hit` (L3); the drag and snap callers live in
//! `crates/ui` (L6). Task P5-03 implements the bodies.
//!
//! Part of Ferrogram. Alpha: the signatures are fixed by task P5-01 and
//! the bodies are `Err(Error::NotImplemented)` stubs.

use crate::error::Error;
use crate::path::Path;
use crate::point::Point;
use crate::rect::Rect;

/// Distance from `point` to the segment `a`..`b`, in inches.
///
/// A degenerate segment (`a == b`) is a point distance. P5-03 defines the
/// rule for non-finite input; the caller clamps (ADR 0021 policy style).
pub fn distance_point_segment(point: Point, a: Point, b: Point) -> Result<f64, Error> {
    let _ = (point, a, b);
    Err(Error::NotImplemented)
}

/// Distance from `point` to the path outline, in inches: the minimum
/// distance over every segment and flattened curve of `path`.
///
/// An empty path has no outline; the result is `f64::INFINITY` then.
pub fn distance_point_path(point: Point, path: &Path) -> Result<f64, Error> {
    let _ = (point, path);
    Err(Error::NotImplemented)
}

/// Whether `point` lies inside the closed `path` (non-zero winding rule,
/// the same fill rule the backends draw with).
///
/// An open path is closed virtually for the test, as a fill does
/// (ADR 0022).
pub fn path_contains(path: &Path, point: Point) -> Result<bool, Error> {
    let _ = (path, point);
    Err(Error::NotImplemented)
}

/// Distance from `point` to the rectangle's boundary, in inches; 0 when
/// the point is inside.
pub fn distance_point_rect(point: Point, rect: &Rect) -> Result<f64, Error> {
    let _ = (point, rect);
    Err(Error::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_p5_01_stubs_are_not_implemented() {
        let p = Point::new(0.0, 0.0);
        assert_eq!(
            distance_point_segment(p, Point::new(1.0, 0.0), Point::new(1.0, 1.0)),
            Err(Error::NotImplemented)
        );
        assert_eq!(
            distance_point_path(p, &Path::new()),
            Err(Error::NotImplemented)
        );
        assert_eq!(path_contains(&Path::new(), p), Err(Error::NotImplemented));
        let rect = Rect::new(Point::new(0.0, 0.0), Point::new(1.0, 1.0));
        assert_eq!(distance_point_rect(p, &rect), Err(Error::NotImplemented));
    }
}
