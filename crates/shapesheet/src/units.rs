//! Units of measure in formulas (ADR 0052).
//!
//! The set is the ShapeSheet reference's "Units of measure" list
//! (`docs/sources.md` source 19): lengths, typographic units, angles,
//! ratios, durations, the fractional and compound spellings, and the
//! implicit units `DL`, `DP`, `DT`, `DA`, `DE`. Canonical units are the
//! model's (ADR 0011): lengths in inches, angles in radians, ratios as
//! fractions.
//!
//! This is the unit set of *formula literals* and evaluated values. The
//! `U=` spelling a `.vsdx` cell writes is a different, smaller set and is
//! mapped by `ferrogram-vsdx`; [`Unit::to_geom`] is the bridge to the
//! model's `ferrogram_geom::Unit`.
//!
//! Task P6-03 implements this module.

use crate::Error;

/// A unit of measure a formula can carry (ADR 0052).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Unit {
    /// `in`, inches.
    Inches,
    /// `ft`, feet.
    Feet,
    /// `yd`, yards.
    Yards,
    /// `mi`, miles.
    Miles,
    /// `nm`, nautical miles.
    NauticalMiles,
    /// `mm`, millimeters.
    Millimeters,
    /// `cm`, centimeters.
    Centimeters,
    /// `m`, meters.
    Meters,
    /// `km`, kilometers.
    Kilometers,
    /// `pt`, points (1/72 inch).
    Points,
    /// `p`, picas (1/6 inch).
    Picas,
    /// `c`, ciceros.
    Ciceros,
    /// `d`, didots.
    Didots,
    /// `deg`, degrees.
    Degrees,
    /// `rad`, radians.
    Radians,
    /// `'`, minutes of arc.
    Minutes,
    /// `"`, seconds of arc.
    Seconds,
    /// `°`, degrees-minutes-seconds.
    DegreeMinSec,
    /// `%`, percent of a whole.
    Percent,
    /// `ew`, elapsed weeks.
    ElapsedWeeks,
    /// `ed`, elapsed days.
    ElapsedDays,
    /// `eh`, elapsed hours.
    ElapsedHours,
    /// `em`, elapsed minutes.
    ElapsedMinutes,
    /// `es`, elapsed seconds.
    ElapsedSeconds,
    /// `date`, a date or time value.
    DateTime,
    /// `IN_F`, inches in fractions (DrawingScale only).
    InchesFractional,
    /// `MI_F`, miles in fractions (DrawingScale only).
    MilesFractional,
    /// `FEET/INCH`, feet and inches.
    FeetAndInches,
    /// `PICAPOINTS`, picas and points.
    PicasAndPoints,
    /// `CICERO/DIDOT`, ciceros and didots.
    CicerosAndDidots,
    /// `DL`, a value in internal (drawing) units, displayed in the
    /// drawing units of the page.
    ImplicitDrawing,
    /// `DP`, internal units displayed in page units.
    ImplicitPage,
    /// `DT`, internal units displayed in type units.
    ImplicitType,
    /// `DA`, internal units displayed in angle units.
    ImplicitAngle,
    /// `DE`, internal units displayed in duration units.
    ImplicitDuration,
    /// No unit: a plain number.
    Unitsless,
}

/// The kind of quantity a [`Unit`] measures (ADR 0052).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UnitFamily {
    /// A length (inches canonical).
    Length,
    /// A typographic length (points canonical for display, inches for
    /// computation).
    Typographic,
    /// An angle (radians canonical, ADR 0011).
    Angle,
    /// A ratio (fraction canonical).
    Ratio,
    /// A duration (seconds canonical).
    Duration,
    /// A plain number.
    Dimensionless,
}

/// Parses a unit spelling (`in`, `DEG`, `%`) as used in a formula.
///
/// Returns `None` for a spelling this crate does not know; task P6-03
/// implements the table and decides the case-folding rule.
pub fn parse_unit(name: &str) -> Result<Unit, Error> {
    let _ = name;
    Err(Error::NotImplemented("parse_unit"))
}

impl Unit {
    /// The family this unit measures.
    pub fn family(self) -> Result<UnitFamily, Error> {
        let _ = self;
        Err(Error::NotImplemented("Unit::family"))
    }

    /// The universal spelling of the unit (`in`, `DEG`, `%`).
    pub fn spelling(self) -> Result<&'static str, Error> {
        let _ = self;
        Err(Error::NotImplemented("Unit::spelling"))
    }

    /// Converts a value in this unit to the family's canonical unit
    /// (inches, radians, fraction, seconds).
    pub fn to_canonical(self, value: f64) -> Result<f64, Error> {
        let _ = (self, value);
        Err(Error::NotImplemented("Unit::to_canonical"))
    }

    /// Converts a value in the family's canonical unit to this unit.
    pub fn from_canonical(self, value: f64) -> Result<f64, Error> {
        let _ = (self, value);
        Err(Error::NotImplemented("Unit::from_canonical"))
    }

    /// The model unit this unit maps to (ADR 0011), or `None` for the
    /// units the model does not carry (durations, dates, implicit units).
    pub fn to_geom(self) -> Result<ferrogram_geom::Unit, Error> {
        let _ = self;
        Err(Error::NotImplemented("Unit::to_geom"))
    }

    /// The formula unit a model unit maps to.
    pub fn from_geom(unit: ferrogram_geom::Unit) -> Result<Unit, Error> {
        let _ = unit;
        Err(Error::NotImplemented("Unit::from_geom"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_unit_is_a_stub() {
        assert_eq!(parse_unit("in"), Err(Error::NotImplemented("parse_unit")));
    }

    #[test]
    fn unit_operations_are_stubs() {
        assert_eq!(
            Unit::Inches.family(),
            Err(Error::NotImplemented("Unit::family"))
        );
        assert_eq!(
            Unit::Inches.spelling(),
            Err(Error::NotImplemented("Unit::spelling"))
        );
        assert_eq!(
            Unit::Inches.to_canonical(1.0),
            Err(Error::NotImplemented("Unit::to_canonical"))
        );
        assert_eq!(
            Unit::Inches.from_canonical(1.0),
            Err(Error::NotImplemented("Unit::from_canonical"))
        );
        assert_eq!(
            Unit::Inches.to_geom(),
            Err(Error::NotImplemented("Unit::to_geom"))
        );
        assert_eq!(
            Unit::from_geom(ferrogram_geom::Unit::Inches),
            Err(Error::NotImplemented("Unit::from_geom"))
        );
    }

    #[test]
    fn the_unit_set_covers_every_family() {
        let units = [
            Unit::Inches,
            Unit::Points,
            Unit::Degrees,
            Unit::Percent,
            Unit::ElapsedDays,
            Unit::ImplicitDrawing,
            Unit::Unitsless,
        ];
        assert_eq!(units.len(), 7);
    }

    #[test]
    fn the_families_are_distinct() {
        assert_ne!(UnitFamily::Length, UnitFamily::Typographic);
        assert_ne!(UnitFamily::Angle, UnitFamily::Duration);
    }
}
