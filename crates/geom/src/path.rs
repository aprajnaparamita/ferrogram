//! Paths: sequences of move, line, curve and close segments.
//!
//! A [`Path`] is the scene-graph primitive (ADR 0020): shape geometry and
//! glyph outlines are built as paths in canonical units (inches, ADR 0011).
//! Curves (arcs, NURBS, splines) convert to [`Cubic`](crate::Cubic)
//! segments through [`Curve::to_cubics`](crate::Curve::to_cubics)
//! (ADR 0022).

use crate::point::Point;

/// One segment of a [`Path`].
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Segment {
    /// Starts a new subpath at the point.
    MoveTo(Point),
    /// A straight line segment to the point.
    LineTo(Point),
    /// A quadratic Bézier segment with one control point.
    QuadTo {
        /// The control point.
        ctrl: Point,
        /// The end point.
        to: Point,
    },
    /// A cubic Bézier segment with two control points.
    CubicTo {
        /// The first control point.
        ctrl1: Point,
        /// The second control point.
        ctrl2: Point,
        /// The end point.
        to: Point,
    },
    /// Closes the current subpath with a straight line to its start.
    Close,
}

/// A path of segments, in canonical units (inches, ADR 0011).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Path {
    segments: Vec<Segment>,
}

impl Path {
    /// An empty path.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a segment.
    pub fn push(&mut self, segment: Segment) {
        self.segments.push(segment);
    }

    /// The segments, in order.
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// Whether the path has no segments.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_path_is_empty() {
        let p = Path::new();
        assert!(p.is_empty());
        assert!(p.segments().is_empty());
    }

    #[test]
    fn push_appends_in_order() {
        let mut p = Path::new();
        p.push(Segment::MoveTo(Point::new(0.0, 0.0)));
        p.push(Segment::LineTo(Point::new(1.0, 2.0)));
        p.push(Segment::Close);
        assert_eq!(p.segments().len(), 3);
        assert_eq!(p.segments()[0], Segment::MoveTo(Point::new(0.0, 0.0)));
        assert_eq!(p.segments()[1], Segment::LineTo(Point::new(1.0, 2.0)));
        assert_eq!(p.segments()[2], Segment::Close);
        assert!(!p.is_empty());
    }

    #[test]
    fn curve_segments_hold_their_points() {
        let q = Segment::QuadTo {
            ctrl: Point::new(1.0, 1.0),
            to: Point::new(2.0, 0.0),
        };
        let c = Segment::CubicTo {
            ctrl1: Point::new(1.0, 1.0),
            ctrl2: Point::new(2.0, 1.0),
            to: Point::new(3.0, 0.0),
        };
        let mut p = Path::new();
        p.push(q);
        p.push(c);
        assert_eq!(p.segments().len(), 2);
    }
}
