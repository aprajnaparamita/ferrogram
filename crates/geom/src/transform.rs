//! Affine transforms for page and shape coordinates.

use crate::angle::Angle;
use crate::error::Error;
use crate::point::Point;

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

    /// A transform that translates by `t`, in inches.
    pub const fn translate(t: Point) -> Self {
        Self {
            m11: 1.0,
            m12: 0.0,
            m21: 0.0,
            m22: 1.0,
            dx: t.x,
            dy: t.y,
        }
    }

    /// A transform that rotates by `a` counterclockwise about the origin.
    ///
    /// Angles are positive counterclockwise, matching Visio's page axes
    /// (x right, y up).
    pub fn rotate(a: Angle) -> Self {
        let (sin, cos) = a.radians().sin_cos();
        Self {
            m11: cos,
            m12: -sin,
            m21: sin,
            m22: cos,
            dx: 0.0,
            dy: 0.0,
        }
    }

    /// A transform that scales the x axis by `sx` and the y axis by `sy`,
    /// about the origin.
    pub const fn scale(sx: f64, sy: f64) -> Self {
        Self {
            m11: sx,
            m12: 0.0,
            m21: 0.0,
            m22: sy,
            dx: 0.0,
            dy: 0.0,
        }
    }

    /// Maps a point through this transform.
    pub fn apply(self, p: Point) -> Point {
        Point::new(
            self.m11 * p.x + self.m12 * p.y + self.dx,
            self.m21 * p.x + self.m22 * p.y + self.dy,
        )
    }

    /// The composition `next` after `self`: the transform that applies `self`
    /// first, then `next`.
    pub fn then(self, next: Transform) -> Transform {
        Transform {
            m11: next.m11 * self.m11 + next.m12 * self.m21,
            m12: next.m11 * self.m12 + next.m12 * self.m22,
            m21: next.m21 * self.m11 + next.m22 * self.m21,
            m22: next.m21 * self.m12 + next.m22 * self.m22,
            dx: next.m11 * self.dx + next.m12 * self.dy + next.dx,
            dy: next.m21 * self.dx + next.m22 * self.dy + next.dy,
        }
    }

    /// The inverse transform: `self.invert()?.apply(self.apply(p)) == p`
    /// for every point `p` (up to `f64` rounding).
    ///
    /// Returns [`Error::Singular`] when the linear part has a zero or
    /// non-finite determinant. Never panics.
    pub fn invert(self) -> Result<Transform, Error> {
        let det = self.m11 * self.m22 - self.m12 * self.m21;
        if !det.is_finite() || det == 0.0 {
            return Err(Error::Singular);
        }
        let inv_det = 1.0 / det;
        let m11 = self.m22 * inv_det;
        let m12 = -self.m12 * inv_det;
        let m21 = -self.m21 * inv_det;
        let m22 = self.m11 * inv_det;
        Ok(Transform {
            m11,
            m12,
            m21,
            m22,
            dx: -(m11 * self.dx + m12 * self.dy),
            dy: -(m21 * self.dx + m22 * self.dy),
        })
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

    /// Relative tolerance for composed `f64` transforms.
    const EPS: f64 = 1.0e-9;

    fn assert_approx(got: f64, want: f64) {
        let tol = EPS * want.abs().max(1.0);
        assert!(
            (got - want).abs() <= tol,
            "got {got}, want {want} (tol {tol})"
        );
    }

    fn assert_point_approx(got: Point, want: Point) {
        assert_approx(got.x, want.x);
        assert_approx(got.y, want.y);
    }

    fn assert_transform_approx(got: Transform, want: Transform) {
        assert_approx(got.m11, want.m11);
        assert_approx(got.m12, want.m12);
        assert_approx(got.m21, want.m21);
        assert_approx(got.m22, want.m22);
        assert_approx(got.dx, want.dx);
        assert_approx(got.dy, want.dy);
    }

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

    #[test]
    fn identity_applies_as_the_identity() {
        let p = Point::new(1.25, -3.5);
        assert_eq!(Transform::identity().apply(p), p);
    }

    #[test]
    fn translate_shifts_the_point() {
        let t = Transform::translate(Point::new(2.0, -1.0));
        assert_eq!(t.apply(Point::new(1.0, 1.0)), Point::new(3.0, 0.0));
    }

    #[test]
    fn rotate_turns_counterclockwise() {
        let t = Transform::rotate(Angle::from_degrees(90.0));
        assert_point_approx(t.apply(Point::new(1.0, 0.0)), Point::new(0.0, 1.0));
        assert_point_approx(t.apply(Point::new(0.0, 1.0)), Point::new(-1.0, 0.0));
    }

    #[test]
    fn rotate_by_zero_is_the_identity() {
        assert_transform_approx(
            Transform::rotate(Angle::from_degrees(0.0)),
            Transform::identity(),
        );
    }

    #[test]
    fn scale_scales_each_axis() {
        let t = Transform::scale(2.0, -3.0);
        assert_eq!(t.apply(Point::new(1.5, 2.0)), Point::new(3.0, -6.0));
    }

    #[test]
    fn then_applies_self_first() {
        let self_t = Transform::translate(Point::new(1.0, 0.0));
        let next = Transform::scale(2.0, 2.0);
        let composed = self_t.then(next);
        let p = Point::new(3.0, 4.0);
        // translate first (4, 4), then scale (8, 8).
        assert_eq!(composed.apply(p), Point::new(8.0, 8.0));
        assert_eq!(composed.apply(p), next.apply(self_t.apply(p)));
    }

    #[test]
    fn then_is_not_commutative() {
        let a = Transform::translate(Point::new(1.0, 2.0));
        let b = Transform::scale(2.0, 3.0);
        assert_ne!(a.then(b), b.then(a));
    }

    #[test]
    fn then_with_identity_changes_nothing() {
        let t = Transform::rotate(Angle::from_degrees(30.0))
            .then(Transform::translate(Point::new(1.0, 2.0)));
        assert_transform_approx(t.then(Transform::identity()), t);
        assert_transform_approx(Transform::identity().then(t), t);
    }

    #[test]
    fn then_matches_sequential_apply() {
        let a = Transform::rotate(Angle::from_degrees(30.0)).then(Transform::scale(2.0, 0.5));
        let b = Transform::translate(Point::new(-3.0, 7.0)).then(Transform::scale(1.0, -1.0));
        let composed = a.then(b);
        for &p in &[
            Point::new(0.0, 0.0),
            Point::new(1.0, 2.0),
            Point::new(-4.5, 0.25),
        ] {
            assert_point_approx(composed.apply(p), b.apply(a.apply(p)));
        }
    }

    #[test]
    fn invert_undoes_apply() {
        let t = Transform::rotate(Angle::from_degrees(37.0))
            .then(Transform::scale(2.0, -3.0))
            .then(Transform::translate(Point::new(5.0, -6.0)));
        let inv = t.invert().expect("t is invertible");
        for &p in &[
            Point::new(0.0, 0.0),
            Point::new(1.0, 2.0),
            Point::new(-0.5, 4.25),
        ] {
            assert_point_approx(inv.apply(t.apply(p)), p);
            assert_point_approx(t.apply(inv.apply(p)), p);
        }
    }

    #[test]
    fn invert_of_a_translation_translates_back() {
        let inv = Transform::translate(Point::new(2.0, -3.0))
            .invert()
            .expect("translation is invertible");
        assert_eq!(inv, Transform::translate(Point::new(-2.0, 3.0)));
    }

    #[test]
    fn invert_of_a_scale_scales_back() {
        let inv = Transform::scale(2.0, -4.0)
            .invert()
            .expect("scale is invertible");
        assert_eq!(inv, Transform::scale(0.5, -0.25));
    }

    #[test]
    fn invert_of_the_identity_is_the_identity() {
        assert_eq!(Transform::identity().invert(), Ok(Transform::identity()));
    }

    #[test]
    fn invert_of_a_zero_scale_is_an_error() {
        assert_eq!(Transform::scale(0.0, 1.0).invert(), Err(Error::Singular));
        assert_eq!(Transform::scale(1.0, 0.0).invert(), Err(Error::Singular));
        let all_zero = Transform {
            m11: 0.0,
            m12: 0.0,
            m21: 0.0,
            m22: 0.0,
            dx: 1.0,
            dy: 2.0,
        };
        assert_eq!(all_zero.invert(), Err(Error::Singular));
    }

    #[test]
    fn invert_of_dependent_axes_is_an_error() {
        // rows proportional: determinant 2*3 - 4*6 == 0
        let dependent = Transform {
            m11: 2.0,
            m12: 4.0,
            m21: 3.0,
            m22: 6.0,
            dx: 0.0,
            dy: 0.0,
        };
        assert_eq!(dependent.invert(), Err(Error::Singular));
    }

    #[test]
    fn invert_of_a_non_finite_transform_is_an_error() {
        let nan = Transform::scale(f64::NAN, 1.0);
        assert_eq!(nan.invert(), Err(Error::Singular));
        let inf = Transform {
            m11: f64::INFINITY,
            m12: f64::INFINITY,
            m21: f64::INFINITY,
            m22: f64::INFINITY,
            dx: 0.0,
            dy: 0.0,
        };
        assert_eq!(inf.invert(), Err(Error::Singular));
    }

    // -- property tests ---------------------------------------------------

    use proptest::prelude::*;

    fn component() -> impl Strategy<Value = f64> {
        -10.0f64..10.0
    }

    fn translation() -> impl Strategy<Value = f64> {
        -100.0f64..100.0
    }

    fn any_transform() -> impl Strategy<Value = Transform> {
        (
            component(),
            component(),
            component(),
            component(),
            translation(),
            translation(),
        )
            .prop_map(|(m11, m12, m21, m22, dx, dy)| Transform {
                m11,
                m12,
                m21,
                m22,
                dx,
                dy,
            })
    }

    fn det(t: Transform) -> f64 {
        t.m11 * t.m22 - t.m12 * t.m21
    }

    proptest! {
        /// `invert(then(a, b))` round-trips: inverting the composition and
        /// applying it to the composed image returns the original point, and
        /// the inverse composition equals `invert(b)` then `invert(a)`.
        #[test]
        fn invert_of_then_round_trips(
            a in any_transform(),
            b in any_transform(),
            px in component(),
            py in component(),
        ) {
            let da = det(a);
            let db = det(b);
            prop_assume!(da.abs() > 0.5);
            prop_assume!(db.abs() > 0.5);
            let composed = a.then(b);
            prop_assume!(det(composed).abs() > 0.5);
            let inv = composed.invert().expect("composed is invertible");
            let p = Point::new(px, py);
            assert_point_approx(inv.apply(composed.apply(p)), p);
            assert_transform_approx(inv, b.invert().unwrap().then(a.invert().unwrap()));
        }

        /// `then` composes point mappings in the documented order.
        #[test]
        fn then_matches_sequential_apply_property(
            a in any_transform(),
            b in any_transform(),
            px in component(),
            py in component(),
        ) {
            let p = Point::new(px, py);
            assert_point_approx(a.then(b).apply(p), b.apply(a.apply(p)));
        }

        /// `invert` followed by `invert` returns the original transform.
        #[test]
        fn invert_is_an_involution(
            t in any_transform(),
        ) {
            prop_assume!(det(t).abs() > 0.5);
            let twice = t.invert().unwrap().invert().unwrap();
            assert_transform_approx(twice, t);
        }
    }
}
