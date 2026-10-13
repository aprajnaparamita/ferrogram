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
    pub const fn new(min: Point, max: Point) -> Self {
        Self { min, max }
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
}
