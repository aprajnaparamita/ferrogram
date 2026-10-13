//! Affine transforms for page and shape coordinates.

/// A 2D affine transform, in inches.
///
/// The transform maps `(x, y)` to
/// `(m11 * x + m12 * y + dx, m21 * x + m22 * y + dy)`.
/// This is the form Visio uses for shape transforms (`LocPinX`, `Width`,
/// `Angle`, flip flags are composed into one transform).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    /// The x coefficient of the input x.
    pub m11: f64,
    /// The x coefficient of the input y.
    pub m12: f64,
    /// The y coefficient of the input x.
    pub m21: f64,
    /// The y coefficient of the input y.
    pub m22: f64,
    /// The x translation, in inches.
    pub dx: f64,
    /// The y translation, in inches.
    pub dy: f64,
}

impl Transform {
    /// The identity transform.
    pub const fn identity() -> Self {
        Self {
            m11: 1.0,
            m12: 0.0,
            m21: 0.0,
            m22: 1.0,
            dx: 0.0,
            dy: 0.0,
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_the_identity_matrix() {
        let t = Transform::identity();
        assert_eq!(t.m11, 1.0);
        assert_eq!(t.m22, 1.0);
        assert_eq!(t.m12, 0.0);
        assert_eq!(t.m21, 0.0);
        assert_eq!(t.dx, 0.0);
        assert_eq!(t.dy, 0.0);
    }

    #[test]
    fn default_is_identity() {
        assert_eq!(Transform::default(), Transform::identity());
    }
}
