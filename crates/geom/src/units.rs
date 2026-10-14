//! Visio units of measure (the `VisUnitTypes` enumeration).

use crate::error::Error;

/// The kind of quantity a unit measures.
///
/// Canonical units (ADR 0011) are one per kind: inches for
/// [`UnitKind::Length`], radians for [`UnitKind::Angle`], a fraction for
/// [`UnitKind::Ratio`] (`0.5` = 50%), and the plain number itself for
/// [`UnitKind::Dimensionless`].
///
/// A conversion given a unit outside the expected kind returns
/// [`Error::WrongUnit`], e.g. degrees where a length cell is expected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnitKind {
    /// A length. The canonical unit is inches.
    Length,
    /// An angle. The canonical unit is radians.
    Angle,
    /// A ratio. The canonical unit is a fraction (`0.5` = 50%).
    Ratio,
    /// A plain number. The canonical unit is the number itself.
    Dimensionless,
}

impl UnitKind {
    /// Converts `value`, measured in `unit`, to this kind's canonical unit.
    ///
    /// Returns [`Error::WrongUnit`] when `unit` measures a different kind.
    pub fn to_canonical(self, unit: Unit, value: f64) -> Result<f64, Error> {
        if unit.kind() != self {
            return Err(Error::WrongUnit);
        }
        unit.to_canonical(value)
    }

    /// Converts `value`, measured in this kind's canonical unit, to `unit`.
    ///
    /// Returns [`Error::WrongUnit`] when `unit` measures a different kind.
    pub fn from_canonical(self, unit: Unit, value: f64) -> Result<f64, Error> {
        if unit.kind() != self {
            return Err(Error::WrongUnit);
        }
        unit.from_canonical(value)
    }
}

/// A Visio unit of measure.
///
/// Cell values are stored in canonical units (ADR 0011): lengths in inches,
/// angles in radians, percentages as a fraction (`0.5` = 50%), and
/// [`Unit::Unitsless`] values as plain numbers. This enum is the unit a raw
/// file value was written in, kept so the value can be written back unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Unit {
    /// Inches (the canonical length unit).
    Inches,
    /// Feet.
    Feet,
    /// Yards.
    Yards,
    /// Miles.
    Miles,
    /// Millimeters.
    Millimeters,
    /// Centimeters.
    Centimeters,
    /// Meters.
    Meters,
    /// Kilometers.
    Kilometers,
    /// Points (1/72 inch).
    Points,
    /// Picas (1/6 inch).
    Picas,
    /// Ciceros (~4.5 mm).
    Ciceros,
    /// Angstroms.
    Angstroms,
    /// Nanometers.
    Nanometers,
    /// Micrometers.
    Microns,
    /// Millipoints (1/72000 inch).
    Millipoints,
    /// Degrees.
    Degrees,
    /// Radians (the canonical angle unit).
    Radians,
    /// Percent of a whole (`0.5` = 50%).
    Percent,
    /// A plain number with no unit.
    Unitsless,
}

impl Unit {
    /// The kind of quantity this unit measures.
    pub const fn kind(self) -> UnitKind {
        match self {
            Unit::Inches
            | Unit::Feet
            | Unit::Yards
            | Unit::Miles
            | Unit::Millimeters
            | Unit::Centimeters
            | Unit::Meters
            | Unit::Kilometers
            | Unit::Points
            | Unit::Picas
            | Unit::Ciceros
            | Unit::Angstroms
            | Unit::Nanometers
            | Unit::Microns
            | Unit::Millipoints => UnitKind::Length,
            Unit::Degrees | Unit::Radians => UnitKind::Angle,
            Unit::Percent => UnitKind::Ratio,
            Unit::Unitsless => UnitKind::Dimensionless,
        }
    }

    /// The number of canonical units in one of this unit.
    ///
    /// Exact by definition:
    ///
    /// - inch-multiples: feet 12, yards 36, miles 63 360 (5 280 ft x 12);
    /// - typographic lengths: points 1/72 inch, picas 1/6 inch,
    ///   millipoints 1/72000 inch;
    /// - metric lengths follow the international inch (1 in = 25.4 mm,
    ///   exact), so the factor is exact in real arithmetic and rounded once
    ///   when written as an `f64`;
    /// - [`Unit::Radians`] factor 1, percent factor 1/100,
    ///   [`Unit::Unitsless`] factor 1.
    ///
    /// Approximate:
    ///
    /// - degrees: the factor `pi / 180` cannot be exact in `f64` (same
    ///   rounding as [`f64::to_radians`]);
    /// - ciceros: 1 cicero = 12 didot points of 0.376 mm = 4.512 mm, so the
    ///   factor is `4.512 / 25.4` inch.
    const fn canonical_per_unit(self) -> f64 {
        match self {
            Unit::Inches => 1.0,
            Unit::Feet => 12.0,
            Unit::Yards => 36.0,
            Unit::Miles => 63_360.0,
            Unit::Millimeters => 1.0 / 25.4,
            Unit::Centimeters => 1.0 / 2.54,
            Unit::Meters => 100.0 / 2.54,
            Unit::Kilometers => 100_000.0 / 2.54,
            Unit::Points => 1.0 / 72.0,
            Unit::Picas => 1.0 / 6.0,
            Unit::Ciceros => 4.512 / 25.4,
            Unit::Angstroms => 1.0e-10 / 0.0254,
            Unit::Nanometers => 1.0e-9 / 0.0254,
            Unit::Microns => 1.0e-6 / 0.0254,
            Unit::Millipoints => 1.0 / 72_000.0,
            Unit::Degrees => core::f64::consts::PI / 180.0,
            Unit::Radians => 1.0,
            Unit::Percent => 1.0 / 100.0,
            Unit::Unitsless => 1.0,
        }
    }

    /// Converts `value`, measured in this unit, to the canonical unit of this
    /// unit's kind (inches for lengths, radians for angles, fraction for
    /// percent, the plain number for [`Unit::Unitsless`]).
    ///
    /// This call cannot fail: the kind comes from the unit itself. Use
    /// [`UnitKind::to_canonical`] to convert against the kind a cell expects
    /// and get [`Error::WrongUnit`] for a mismatch.
    pub fn to_canonical(self, value: f64) -> Result<f64, Error> {
        Ok(value * self.canonical_per_unit())
    }

    /// Converts `value`, measured in the canonical unit of this unit's kind,
    /// to this unit.
    ///
    /// This call cannot fail: the kind comes from the unit itself. Use
    /// [`UnitKind::from_canonical`] to convert against the kind a cell expects
    /// and get [`Error::WrongUnit`] for a mismatch.
    pub fn from_canonical(self, value: f64) -> Result<f64, Error> {
        Ok(value / self.canonical_per_unit())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Relative tolerance for `f64` conversions.
    const EPS: f64 = 1.0e-9;

    fn assert_approx(got: f64, want: f64) {
        let tol = EPS * want.abs().max(1.0);
        assert!(
            (got - want).abs() <= tol,
            "got {got}, want {want} (tol {tol})"
        );
    }

    /// Every unit with a known conversion: (unit, 1.0 of it in canonical).
    const KNOWN: &[(Unit, f64)] = &[
        (Unit::Inches, 1.0),
        (Unit::Feet, 12.0),
        (Unit::Yards, 36.0),
        (Unit::Miles, 63_360.0),
        (Unit::Millimeters, 1.0 / 25.4),
        (Unit::Centimeters, 1.0 / 2.54),
        (Unit::Meters, 100.0 / 2.54),
        (Unit::Kilometers, 100_000.0 / 2.54),
        (Unit::Points, 1.0 / 72.0),
        (Unit::Picas, 1.0 / 6.0),
        (Unit::Ciceros, 4.512 / 25.4),
        (Unit::Angstroms, 1.0e-10 / 0.0254),
        (Unit::Nanometers, 1.0e-9 / 0.0254),
        (Unit::Microns, 1.0e-6 / 0.0254),
        (Unit::Millipoints, 1.0 / 72_000.0),
        (Unit::Degrees, core::f64::consts::PI / 180.0),
        (Unit::Radians, 1.0),
        (Unit::Percent, 0.01),
        (Unit::Unitsless, 1.0),
    ];

    #[test]
    fn every_unit_has_a_known_factor() {
        for &(unit, canonical_per_unit) in KNOWN {
            assert_approx(
                unit.to_canonical(1.0).expect("to_canonical never fails"),
                canonical_per_unit,
            );
        }
    }

    #[test]
    fn every_unit_round_trips() {
        for &(unit, _) in KNOWN {
            for &x in &[0.0, 1.0, -2.5, 1234.5] {
                let canonical = unit.to_canonical(x).expect("to_canonical never fails");
                let back = unit
                    .from_canonical(canonical)
                    .expect("from_canonical never fails");
                assert_approx(back, x);
            }
        }
    }

    #[test]
    fn exact_factors_are_exact() {
        assert_eq!(Unit::Inches.to_canonical(2.5), Ok(2.5));
        assert_eq!(Unit::Feet.to_canonical(2.0), Ok(24.0));
        assert_eq!(Unit::Yards.to_canonical(2.0), Ok(72.0));
        assert_eq!(Unit::Miles.to_canonical(1.0), Ok(63_360.0));
        assert_eq!(Unit::Points.to_canonical(72.0), Ok(1.0));
        assert_eq!(Unit::Picas.to_canonical(6.0), Ok(1.0));
        assert_eq!(Unit::Millipoints.to_canonical(72_000.0), Ok(1.0));
        assert_eq!(Unit::Radians.to_canonical(2.5), Ok(2.5));
        assert_eq!(Unit::Unitsless.to_canonical(0.25), Ok(0.25));
    }

    #[test]
    fn metric_units_follow_the_international_inch() {
        assert_approx(Unit::Millimeters.to_canonical(25.4).unwrap(), 1.0);
        assert_approx(Unit::Centimeters.to_canonical(2.54).unwrap(), 1.0);
        assert_approx(Unit::Meters.to_canonical(0.0254).unwrap(), 1.0);
        assert_approx(Unit::Kilometers.to_canonical(0.0000254).unwrap(), 1.0);
    }

    #[test]
    fn percent_converts_to_a_fraction() {
        assert_eq!(Unit::Percent.to_canonical(50.0), Ok(0.5));
        assert_eq!(Unit::Percent.from_canonical(0.5), Ok(50.0));
    }

    #[test]
    fn degrees_convert_to_radians() {
        assert_approx(
            Unit::Degrees.to_canonical(180.0).unwrap(),
            core::f64::consts::PI,
        );
        assert_approx(
            Unit::Degrees.from_canonical(core::f64::consts::PI).unwrap(),
            180.0,
        );
    }

    #[test]
    fn from_canonical_divides_by_the_factor() {
        assert_eq!(Unit::Feet.from_canonical(24.0), Ok(2.0));
        assert_eq!(Unit::Miles.from_canonical(63_360.0), Ok(1.0));
    }

    #[test]
    fn kinds_match_the_categories() {
        for &(unit, _) in KNOWN {
            let kind = unit.kind();
            match unit {
                Unit::Degrees | Unit::Radians => assert_eq!(kind, UnitKind::Angle),
                Unit::Percent => assert_eq!(kind, UnitKind::Ratio),
                Unit::Unitsless => assert_eq!(kind, UnitKind::Dimensionless),
                _ => assert_eq!(kind, UnitKind::Length),
            }
        }
    }

    #[test]
    fn unit_kind_converts_within_the_kind() {
        assert_eq!(UnitKind::Length.to_canonical(Unit::Feet, 2.0), Ok(24.0));
        assert_eq!(UnitKind::Length.from_canonical(Unit::Feet, 24.0), Ok(2.0));
        assert_eq!(UnitKind::Angle.to_canonical(Unit::Radians, 0.0), Ok(0.0));
        assert_eq!(UnitKind::Ratio.to_canonical(Unit::Percent, 25.0), Ok(0.25));
        assert_eq!(
            UnitKind::Dimensionless.to_canonical(Unit::Unitsless, 7.0),
            Ok(7.0)
        );
    }

    #[test]
    fn wrong_unit_is_rejected_on_both_directions() {
        // Degrees where a length cell is expected (task P2-03).
        assert_eq!(
            UnitKind::Length.to_canonical(Unit::Degrees, 90.0),
            Err(Error::WrongUnit)
        );
        assert_eq!(
            UnitKind::Length.from_canonical(Unit::Degrees, 90.0),
            Err(Error::WrongUnit)
        );
        assert_eq!(
            UnitKind::Angle.to_canonical(Unit::Inches, 1.0),
            Err(Error::WrongUnit)
        );
        assert_eq!(
            UnitKind::Ratio.to_canonical(Unit::Unitsless, 1.0),
            Err(Error::WrongUnit)
        );
        assert_eq!(
            UnitKind::Dimensionless.to_canonical(Unit::Percent, 1.0),
            Err(Error::WrongUnit)
        );
    }

    #[test]
    fn units_are_distinct_values() {
        assert_ne!(Unit::Inches, Unit::Millimeters);
        assert_eq!(Unit::Degrees, Unit::Degrees);
    }
}
