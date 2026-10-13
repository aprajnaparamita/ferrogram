//! Visio units of measure (the `VisUnitTypes` enumeration).

use crate::error::Error;

/// A Visio unit of measure.
///
/// Cell values are stored in canonical units (ADR 0011): lengths in inches,
/// angles in degrees, percentages as a fraction (`0.5` = 50%), and
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
    /// Ciceros (~4.51 mm).
    Ciceros,
    /// Angstroms.
    Angstroms,
    /// Nanometers.
    Nanometers,
    /// Micrometers.
    Microns,
    /// Millipoints (1/72000 inch).
    Millipoints,
    /// Degrees (the canonical angle unit).
    Degrees,
    /// Radians.
    Radians,
    /// Percent of a whole (`0.5` = 50%).
    Percent,
    /// A plain number with no unit.
    Unitsless,
}

impl Unit {
    /// Converts `value`, measured in this unit, to the canonical unit
    /// (inches for lengths, degrees for angles, fraction for percent).
    pub fn to_canonical(self, value: f64) -> Result<f64, Error> {
        let _ = value;
        Err(Error::NotImplemented("Unit::to_canonical"))
    }

    /// Converts `value`, measured in the canonical unit, to this unit.
    pub fn from_canonical(self, value: f64) -> Result<f64, Error> {
        let _ = value;
        Err(Error::NotImplemented("Unit::from_canonical"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions_are_stubs() {
        assert_eq!(
            Unit::Feet.to_canonical(1.0),
            Err(Error::NotImplemented("Unit::to_canonical"))
        );
        assert_eq!(
            Unit::Feet.from_canonical(12.0),
            Err(Error::NotImplemented("Unit::from_canonical"))
        );
    }

    #[test]
    fn units_are_distinct_values() {
        assert_ne!(Unit::Inches, Unit::Millimeters);
        assert_eq!(Unit::Degrees, Unit::Degrees);
    }
}
