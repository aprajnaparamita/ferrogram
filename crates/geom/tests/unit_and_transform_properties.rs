//! Property tests for unit conversion and transform algebra (task P2-10,
//! focus areas: exact conversion factors, error paths, transform algebra).
//!
//! The exact-factor tables and the error paths are tested next to the code
//! (`src/units.rs`); these properties pin the algebra: conversions are
//! inverses of each other, a kind rejects every other kind's units, and the
//! transform constructors compose as documented.

use core::f64::consts::FRAC_PI_2;

use ferrogram_geom::{Angle, Point, Transform, Unit, UnitKind};
use proptest::prelude::*;

/// Every unit with the canonical unit of its kind.
const UNITS: &[Unit] = &[
    Unit::Inches,
    Unit::Feet,
    Unit::Yards,
    Unit::Miles,
    Unit::Millimeters,
    Unit::Centimeters,
    Unit::Meters,
    Unit::Kilometers,
    Unit::Points,
    Unit::Picas,
    Unit::Ciceros,
    Unit::Angstroms,
    Unit::Nanometers,
    Unit::Microns,
    Unit::Millipoints,
    Unit::Degrees,
    Unit::Radians,
    Unit::Percent,
    Unit::Unitsless,
];

const KINDS: &[UnitKind] = &[
    UnitKind::Length,
    UnitKind::Angle,
    UnitKind::Ratio,
    UnitKind::Dimensionless,
];

fn any_unit() -> impl Strategy<Value = Unit> {
    proptest::sample::select(UNITS.to_vec())
}

fn any_value() -> impl Strategy<Value = f64> {
    -1.0e6f64..1.0e6
}

proptest::proptest! {
    /// `to_canonical` and `from_canonical` are inverses for every unit.
    #[test]
    fn conversions_round_trip(unit in any_unit(), value in any_value()) {
        let canonical = unit.to_canonical(value).expect("cannot fail");
        let back = unit.from_canonical(canonical).expect("cannot fail");
        let tolerance = 1.0e-9 * value.abs().max(1.0);
        proptest::prop_assert!((back - value).abs() <= tolerance, "{back} != {value}");
    }

    /// The kind-checked conversions convert within the kind and round-trip.
    #[test]
    fn kind_checked_conversions_round_trip(
        unit in any_unit(),
        value in any_value(),
    ) {
        let kind = unit.kind();
        let canonical = kind.to_canonical(unit, value).expect("kinds match");
        let back = kind.from_canonical(unit, canonical).expect("kinds match");
        let tolerance = 1.0e-9 * value.abs().max(1.0);
        proptest::prop_assert!((back - value).abs() <= tolerance, "{back} != {value}");
    }

    /// Translation composes as point addition.
    #[test]
    fn translations_add(a in any_value(), b in any_value(), px in any_value(), py in any_value()) {
        let p = Point::new(px, py);
        let first = Transform::translate(Point::new(a, b));
        let moved = first.apply(p);
        proptest::prop_assert!((moved.x - (px + a)).abs() <= 1.0e-9);
        proptest::prop_assert!((moved.y - (py + b)).abs() <= 1.0e-9);
    }

    /// `then` composes translations and scales componentwise.
    #[test]
    fn translations_and_scales_compose(
        ax in any_value(), ay in any_value(),
        bx in any_value(), by in any_value(),
        sx in 0.5f64..2.0, sy in 0.5f64..2.0,
    ) {
        let translate = Transform::translate(Point::new(ax, ay)).then(Transform::translate(Point::new(bx, by)));
        let sum = Transform::translate(Point::new(ax + bx, ay + by));
        proptest::prop_assert_eq!(translate, sum);
        let scale = Transform::scale(sx, sy).then(Transform::scale(sy, sx));
        let product = Transform::scale(sx * sy, sy * sx);
        proptest::prop_assert_eq!(scale, product);
    }

    /// `then` is associative (up to `f64` rounding).
    #[test]
    fn composition_is_associative(
        ax in any_value(), ay in any_value(),
        px in any_value(), py in any_value(),
        radians in -3.0f64..3.0,
    ) {
        let t1 = Transform::translate(Point::new(ax, ay));
        let t2 = Transform::scale(1.5, 0.5);
        let t3 = Transform::rotate(Angle::from_radians(radians));
        let left = t1.then(t2).then(t3);
        let right = t1.then(t2.then(t3));
        for (a, b) in [
            (left.m11, right.m11), (left.m12, right.m12),
            (left.m21, right.m21), (left.m22, right.m22),
            (left.dx, right.dx), (left.dy, right.dy),
        ] {
            proptest::prop_assert!((a - b).abs() <= 1.0e-9 * a.abs().max(1.0), "{a} != {b}");
        }
        // Both sides map a point the same way.
        let p = Point::new(px, py);
        let mapped_left = left.apply(p);
        let mapped_right = right.apply(p);
        proptest::prop_assert!((mapped_left.x - mapped_right.x).abs() <= 1.0e-6);
        proptest::prop_assert!((mapped_left.y - mapped_right.y).abs() <= 1.0e-6);
    }
}

#[test]
fn a_kind_rejects_every_other_kinds_units() {
    for kind in KINDS {
        for unit in UNITS {
            let outcome = kind.to_canonical(*unit, 1.0);
            if unit.kind() == *kind {
                assert!(outcome.is_ok(), "{kind:?} must accept {unit:?}");
            } else {
                assert_eq!(outcome, Err(ferrogram_geom::Error::WrongUnit));
            }
        }
    }
}

#[test]
fn rotation_turns_the_x_axis_up() {
    let rotated = Transform::rotate(Angle::from_degrees(90.0)).apply(Point::new(1.0, 0.0));
    assert!((rotated.x - 0.0).abs() <= 1.0e-12);
    assert!((rotated.y - 1.0).abs() <= 1.0e-12);
    // A right angle in radians is the same turn.
    let quarter = Transform::rotate(Angle::from_radians(FRAC_PI_2)).apply(Point::new(1.0, 0.0));
    assert!((quarter.y - 1.0).abs() <= 1.0e-12);
}
