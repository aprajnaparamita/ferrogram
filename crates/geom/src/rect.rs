//! Axis-aligned rectangles, in inches.

use crate::point::Point;

/// An axis-aligned rectangle: the bounding box of `min` and `max`, in inches.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    /// The corner with the smaller coordinates.
    pub min: Point,
    /// The corner with the larger coordinates.
    pub max: Point,
}

impl Rect {
    /// Creates the rectangle spanning `min` and `max`.
    ///
    /// A rectangle keeps the bounding-box invariant `min.x <= max.x` and
    /// `min.y <= max.y`; callers that have two arbitrary corners build the
    /// rectangle with [`Rect::union`] instead.
    pub const fn new(min: Point, max: Point) -> Self {
        Self { min, max }
    }

    /// The width (`max.x - min.x`), in inches.
    pub const fn width(self) -> f64 {
        self.max.x - self.min.x
    }

    /// The height (`max.y - min.y`), in inches.
    pub const fn height(self) -> f64 {
        self.max.y - self.min.y
    }

    /// The smallest rectangle containing both rectangles.
    ///
    /// Also the right way to build a rectangle from two arbitrary corners.
    pub fn union(self, other: Rect) -> Rect {
        Rect {
            min: Point::new(self.min.x.min(other.min.x), self.min.y.min(other.min.y)),
            max: Point::new(self.max.x.max(other.max.x), self.max.y.max(other.max.y)),
        }
    }

    /// True when `p` lies inside the rectangle or on its edge.
    pub fn contains(self, p: Point) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_holds_its_corners() {
        let r = Rect::new(Point::new(0.0, 0.0), Point::new(2.0, 3.0));
        assert_eq!(r.min, Point::new(0.0, 0.0));
        assert_eq!(r.max, Point::new(2.0, 3.0));
    }

    #[test]
    fn width_and_height_span_the_corners() {
        let r = Rect::new(Point::new(-1.0, 2.0), Point::new(3.0, 5.0));
        assert_eq!(r.width(), 4.0);
        assert_eq!(r.height(), 3.0);
    }

    #[test]
    fn zero_rect_has_zero_extent() {
        let r = Rect::new(Point::new(7.0, -7.0), Point::new(7.0, -7.0));
        assert_eq!(r.width(), 0.0);
        assert_eq!(r.height(), 0.0);
    }

    #[test]
    fn union_spans_both_rectangles() {
        let a = Rect::new(Point::new(0.0, 0.0), Point::new(2.0, 3.0));
        let b = Rect::new(Point::new(-1.0, -2.0), Point::new(4.0, 1.0));
        assert_eq!(
            a.union(b),
            Rect::new(Point::new(-1.0, -2.0), Point::new(4.0, 3.0))
        );
    }

    #[test]
    fn union_with_self_is_the_same_rectangle() {
        let a = Rect::new(Point::new(0.0, 0.0), Point::new(2.0, 3.0));
        assert_eq!(a.union(a), a);
    }

    #[test]
    fn contains_is_inclusive_of_the_edges() {
        let r = Rect::new(Point::new(0.0, 0.0), Point::new(2.0, 3.0));
        assert!(r.contains(Point::new(1.0, 1.0)));
        assert!(r.contains(Point::new(0.0, 0.0)));
        assert!(r.contains(Point::new(2.0, 3.0)));
        assert!(!r.contains(Point::new(-0.5, 1.0)));
        assert!(!r.contains(Point::new(1.0, 3.5)));
    }
}
