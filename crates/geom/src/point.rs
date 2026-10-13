//! Points and sizes, in inches.

/// A point in page coordinates, in inches (ADR 0011).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    /// The x coordinate, in inches.
    pub x: f64,
    /// The y coordinate, in inches.
    pub y: f64,
}

impl Point {
    /// Creates a point at `(x, y)`, in inches.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// A width and a height, in inches.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    /// The width, in inches.
    pub width: f64,
    /// The height, in inches.
    pub height: f64,
}

impl Size {
    /// Creates a size of `width` by `height`, in inches.
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_constructor_holds_its_values() {
        let p = Point::new(1.5, -2.0);
        assert_eq!(p.x, 1.5);
        assert_eq!(p.y, -2.0);
    }

    #[test]
    fn size_constructor_holds_its_values() {
        let s = Size::new(3.0, 4.0);
        assert_eq!(s.width, 3.0);
        assert_eq!(s.height, 4.0);
    }

    #[test]
    fn defaults_are_zero() {
        assert_eq!(Point::default(), Point::new(0.0, 0.0));
        assert_eq!(Size::default(), Size::new(0.0, 0.0));
    }
}
