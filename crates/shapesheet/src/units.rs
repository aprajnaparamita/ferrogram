//! Units of measure in formulas (ADR 0052).
//!
//! The set is the ShapeSheet reference's "Units of measure" list
//! (`docs/sources.md` source 19): lengths, typographic units, angles,
//! ratios, durations, the fractional and compound spellings, and the
//! implicit units `DL`, `DP`, `DT`, `DA`, `DE`. Canonical units are the
//! model's (ADR 0011): lengths in inches, angles in radians, ratios as
//! fractions, durations in seconds.
//!
//! This is the unit set of *formula literals* and evaluated values. The
//! `U=` spelling a `.vsdx` cell writes is a different, smaller set and is
//! mapped by `ferrogram-vsdx`; [`Unit::to_geom`] is the bridge to the
//! model's `ferrogram_geom::Unit`.
//!
//! Spellings come in two alphabets ("About Units of Measure", "Universal
//! strings"): the localized spelling of the first table (`in`, `deg`) and
//! the universal one (`IN`, `DEG`). [`parse_unit`] accepts both spellings
//! in any case; [`Unit::spelling`] prints the universal one. The
//! multidimensional spellings (`SQ IN`, `HA`, `ACRES`) are **out of scope**
//! in this phase (ADR 0052: a [`crate::Value`] carries one unit, not a
//! dimension vector): [`parse_unit`] rejects them like any other unknown
//! spelling, with [`crate::SyntaxKind::InvalidUnit`].

use crate::Error;
use crate::Span;
use crate::SyntaxKind;

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

/// The spelling table: the universal spelling of every unit of
/// `about-units-of-measure.md` (source 19) and its unit.
///
/// [`parse_unit`] matches a spelling case-insensitively, so the localized
/// spellings of the first table (`in`, `deg`) parse through the same rows.
/// The symbols (`'`, `"`, `%`, `°`) have one spelling each. The implicit
/// units (`DL`..`DE`) are universal only and have no localized spelling.
const SPELLINGS: &[(&str, Unit)] = &[
    ("IN", Unit::Inches),
    ("FT", Unit::Feet),
    ("YD", Unit::Yards),
    ("MI", Unit::Miles),
    ("NM", Unit::NauticalMiles),
    ("MM", Unit::Millimeters),
    ("CM", Unit::Centimeters),
    ("M", Unit::Meters),
    ("KM", Unit::Kilometers),
    ("PT", Unit::Points),
    ("P", Unit::Picas),
    ("C", Unit::Ciceros),
    ("D", Unit::Didots),
    ("DEG", Unit::Degrees),
    ("RAD", Unit::Radians),
    ("'", Unit::Minutes),
    ("\"", Unit::Seconds),
    ("°", Unit::DegreeMinSec),
    ("%", Unit::Percent),
    ("EW", Unit::ElapsedWeeks),
    ("ED", Unit::ElapsedDays),
    ("EH", Unit::ElapsedHours),
    ("EM", Unit::ElapsedMinutes),
    ("ES", Unit::ElapsedSeconds),
    ("DATE", Unit::DateTime),
    ("IN_F", Unit::InchesFractional),
    ("MI_F", Unit::MilesFractional),
    ("FEET/INCH", Unit::FeetAndInches),
    ("PICAPOINTS", Unit::PicasAndPoints),
    ("CICERO/DIDOT", Unit::CicerosAndDidots),
    ("DL", Unit::ImplicitDrawing),
    ("DP", Unit::ImplicitPage),
    ("DT", Unit::ImplicitType),
    ("DA", Unit::ImplicitAngle),
    ("DE", Unit::ImplicitDuration),
];

/// The error a spelling that is not a unit produces.
///
/// The span is `0..len` of the spelling the caller passed; the parser maps
/// it onto the formula's own offsets (`Error::Syntax` is the caller path
/// the task P6-03 acceptance names for area spellings such as `SQ IN`).
fn invalid_unit(name: &str) -> Error {
    let len = u32::try_from(name.len()).unwrap_or(u32::MAX);
    Error::Syntax {
        span: Span::new(0, len),
        kind: SyntaxKind::InvalidUnit,
    }
}

/// Parses a unit spelling (`in`, `DEG`, `%`) as used in a formula.
///
/// The localized and the universal spelling of "About Units of Measure"
/// both parse, in any ASCII case. The empty spelling is
/// [`Unit::Unitsless`]: a plain number carries no suffix. Every other
/// spelling is [`crate::SyntaxKind::InvalidUnit`], including the
/// multidimensional ones (`SQ IN`, `HA`, `ACRES`), which are out of scope
/// in this phase (ADR 0052).
pub fn parse_unit(name: &str) -> Result<Unit, Error> {
    let name = name.trim();
    if name.is_empty() {
        return Ok(Unit::Unitsless);
    }
    for (spelling, unit) in SPELLINGS {
        if name.eq_ignore_ascii_case(spelling) {
            return Ok(*unit);
        }
    }
    Err(invalid_unit(name))
}

impl Unit {
    /// The family this unit measures.
    ///
    /// The compound spellings measure the family of their first component
    /// (`FEET/INCH` is a length; ADR 0052 puts the compound spellings in
    /// the length family), and the implicit units measure the family their
    /// display flag implies (`DL`, `DP`, `DT` length, `DA` angle, `DE`
    /// duration).
    pub fn family(self) -> Result<UnitFamily, Error> {
        let family = match self {
            Unit::Inches
            | Unit::Feet
            | Unit::Yards
            | Unit::Miles
            | Unit::NauticalMiles
            | Unit::Millimeters
            | Unit::Centimeters
            | Unit::Meters
            | Unit::Kilometers
            | Unit::InchesFractional
            | Unit::MilesFractional
            | Unit::FeetAndInches
            | Unit::PicasAndPoints
            | Unit::CicerosAndDidots
            | Unit::ImplicitDrawing
            | Unit::ImplicitPage
            | Unit::ImplicitType => UnitFamily::Length,
            Unit::Points | Unit::Picas | Unit::Ciceros | Unit::Didots => UnitFamily::Typographic,
            Unit::Degrees
            | Unit::Radians
            | Unit::Minutes
            | Unit::Seconds
            | Unit::DegreeMinSec
            | Unit::ImplicitAngle => UnitFamily::Angle,
            Unit::Percent => UnitFamily::Ratio,
            Unit::ElapsedWeeks
            | Unit::ElapsedDays
            | Unit::ElapsedHours
            | Unit::ElapsedMinutes
            | Unit::ElapsedSeconds
            | Unit::DateTime
            | Unit::ImplicitDuration => UnitFamily::Duration,
            Unit::Unitsless => UnitFamily::Dimensionless,
        };
        Ok(family)
    }

    /// The universal spelling of the unit (`IN`, `DEG`, `%`).
    ///
    /// [`Unit::Unitsless`] has no spelling and prints as the empty string,
    /// so `spelling()` and [`parse_unit`] round-trip for every unit.
    pub fn spelling(self) -> Result<&'static str, Error> {
        let spelling = match self {
            Unit::Inches => "IN",
            Unit::Feet => "FT",
            Unit::Yards => "YD",
            Unit::Miles => "MI",
            Unit::NauticalMiles => "NM",
            Unit::Millimeters => "MM",
            Unit::Centimeters => "CM",
            Unit::Meters => "M",
            Unit::Kilometers => "KM",
            Unit::Points => "PT",
            Unit::Picas => "P",
            Unit::Ciceros => "C",
            Unit::Didots => "D",
            Unit::Degrees => "DEG",
            Unit::Radians => "RAD",
            Unit::Minutes => "'",
            Unit::Seconds => "\"",
            Unit::DegreeMinSec => "°",
            Unit::Percent => "%",
            Unit::ElapsedWeeks => "EW",
            Unit::ElapsedDays => "ED",
            Unit::ElapsedHours => "EH",
            Unit::ElapsedMinutes => "EM",
            Unit::ElapsedSeconds => "ES",
            Unit::DateTime => "DATE",
            Unit::InchesFractional => "IN_F",
            Unit::MilesFractional => "MI_F",
            Unit::FeetAndInches => "FEET/INCH",
            Unit::PicasAndPoints => "PICAPOINTS",
            Unit::CicerosAndDidots => "CICERO/DIDOT",
            Unit::ImplicitDrawing => "DL",
            Unit::ImplicitPage => "DP",
            Unit::ImplicitType => "DT",
            Unit::ImplicitAngle => "DA",
            Unit::ImplicitDuration => "DE",
            Unit::Unitsless => "",
        };
        Ok(spelling)
    }

    /// The number of canonical units (inches, radians, fraction, seconds)
    /// one of this unit measures.
    ///
    /// Exact by definition where the factor is exact: 1 in = 72 pt,
    /// 1 ft = 12 in, 1 mi = 63 360 in (5 280 ft x 12), 1 deg = pi / 180
    /// rad, 1 % = 1/100. Metric lengths follow the international inch
    /// (1 in = 25.4 mm, exact), written once as an `f64`. The typographic
    /// non-inch units follow the didot point (0.376 mm), so ciceros are
    /// 12 didot points = 4.512 mm (the `ferrogram_geom` factor). A date
    /// serial counts days (1 = one day), so it converts at 86 400 s/day.
    /// The implicit units carry no scale of their own: internal units are
    /// the canonical ones (ADR 0052).
    const fn canonical_per_unit(self) -> f64 {
        match self {
            Unit::Inches => 1.0,
            Unit::Feet => 12.0,
            Unit::Yards => 36.0,
            Unit::Miles => 63_360.0,
            Unit::NauticalMiles => 185_200.0 / 2.54,
            Unit::Millimeters => 1.0 / 25.4,
            Unit::Centimeters => 1.0 / 2.54,
            Unit::Meters => 100.0 / 2.54,
            Unit::Kilometers => 100_000.0 / 2.54,
            Unit::Points => 1.0 / 72.0,
            Unit::Picas => 1.0 / 6.0,
            Unit::Ciceros => 4.512 / 25.4,
            Unit::Didots => 0.376 / 25.4,
            Unit::Degrees => core::f64::consts::PI / 180.0,
            Unit::Radians => 1.0,
            Unit::Minutes => core::f64::consts::PI / 10_800.0,
            Unit::Seconds => core::f64::consts::PI / 648_000.0,
            Unit::DegreeMinSec => core::f64::consts::PI / 180.0,
            Unit::Percent => 1.0 / 100.0,
            Unit::ElapsedWeeks => 604_800.0,
            Unit::ElapsedDays => 86_400.0,
            Unit::ElapsedHours => 3_600.0,
            Unit::ElapsedMinutes => 60.0,
            Unit::ElapsedSeconds => 1.0,
            Unit::DateTime => 86_400.0,
            Unit::InchesFractional => 1.0,
            Unit::MilesFractional => 63_360.0,
            Unit::FeetAndInches => 12.0,
            Unit::PicasAndPoints => 1.0 / 6.0,
            Unit::CicerosAndDidots => 4.512 / 25.4,
            Unit::ImplicitDrawing => 1.0,
            Unit::ImplicitPage => 1.0,
            Unit::ImplicitType => 1.0,
            Unit::ImplicitAngle => 1.0,
            Unit::ImplicitDuration => 1.0,
            Unit::Unitsless => 1.0,
        }
    }

    /// Converts a value in this unit to the family's canonical unit
    /// (inches for lengths and typographic lengths, radians for angles,
    /// fraction for ratios, seconds for durations, the plain number for
    /// [`Unit::Unitsless`]).
    ///
    /// This call cannot fail: the family comes from the unit itself. The
    /// result is [`crate::ErrorCode::Num`] territory at the caller's
    /// discretion; a non-finite `value` is converted as given (ADR 0052
    /// maps non-finite results to `#NUM!` where values are produced).
    pub fn to_canonical(self, value: f64) -> Result<f64, Error> {
        Ok(value * self.canonical_per_unit())
    }

    /// Converts a value in the family's canonical unit to this unit.
    ///
    /// The inverse of [`Unit::to_canonical`]: the round trip is exact for
    /// the exact factors and within `1e-12` relative for every unit
    /// (property test, ADR 0060).
    pub fn from_canonical(self, value: f64) -> Result<f64, Error> {
        Ok(value / self.canonical_per_unit())
    }

    /// The model unit this unit maps to (ADR 0011), or an error for the
    /// units the model does not carry.
    ///
    /// The model has no didots, nautical miles, arc minutes or seconds,
    /// no compound, fractional or implicit spellings, and no durations or
    /// dates (ADR 0052). [`crate::Value::to_cached`] falls back to the family's
    /// canonical unit for those; this function stays strict so a caller
    /// that needs the file's own spelling can say so.
    pub fn to_geom(self) -> Result<ferrogram_geom::Unit, Error> {
        let unit = match self {
            Unit::Inches => ferrogram_geom::Unit::Inches,
            Unit::Feet => ferrogram_geom::Unit::Feet,
            Unit::Yards => ferrogram_geom::Unit::Yards,
            Unit::Miles => ferrogram_geom::Unit::Miles,
            Unit::Millimeters => ferrogram_geom::Unit::Millimeters,
            Unit::Centimeters => ferrogram_geom::Unit::Centimeters,
            Unit::Meters => ferrogram_geom::Unit::Meters,
            Unit::Kilometers => ferrogram_geom::Unit::Kilometers,
            Unit::Points => ferrogram_geom::Unit::Points,
            Unit::Picas => ferrogram_geom::Unit::Picas,
            Unit::Ciceros => ferrogram_geom::Unit::Ciceros,
            Unit::Degrees => ferrogram_geom::Unit::Degrees,
            Unit::Radians => ferrogram_geom::Unit::Radians,
            Unit::Percent => ferrogram_geom::Unit::Percent,
            Unit::Unitsless => ferrogram_geom::Unit::Unitsless,
            _ => return Err(Error::Conversion("unit has no model spelling")),
        };
        Ok(unit)
    }

    /// The formula unit a model unit maps to.
    ///
    /// The model's angstroms, nanometers, micrometers and millipoints have
    /// no formula spelling ("About Units of Measure" has none), so they
    /// are [`Error::Conversion`] and never a silent scale change.
    pub fn from_geom(unit: ferrogram_geom::Unit) -> Result<Unit, Error> {
        let formula_unit = match unit {
            ferrogram_geom::Unit::Inches => Unit::Inches,
            ferrogram_geom::Unit::Feet => Unit::Feet,
            ferrogram_geom::Unit::Yards => Unit::Yards,
            ferrogram_geom::Unit::Miles => Unit::Miles,
            ferrogram_geom::Unit::Millimeters => Unit::Millimeters,
            ferrogram_geom::Unit::Centimeters => Unit::Centimeters,
            ferrogram_geom::Unit::Meters => Unit::Meters,
            ferrogram_geom::Unit::Kilometers => Unit::Kilometers,
            ferrogram_geom::Unit::Points => Unit::Points,
            ferrogram_geom::Unit::Picas => Unit::Picas,
            ferrogram_geom::Unit::Ciceros => Unit::Ciceros,
            ferrogram_geom::Unit::Degrees => Unit::Degrees,
            ferrogram_geom::Unit::Radians => Unit::Radians,
            ferrogram_geom::Unit::Percent => Unit::Percent,
            ferrogram_geom::Unit::Unitsless => Unit::Unitsless,
            _ => return Err(Error::Conversion("model unit has no formula spelling")),
        };
        Ok(formula_unit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every unit this module defines, for table-wide assertions.
    const ALL: &[Unit] = &[
        Unit::Inches,
        Unit::Feet,
        Unit::Yards,
        Unit::Miles,
        Unit::NauticalMiles,
        Unit::Millimeters,
        Unit::Centimeters,
        Unit::Meters,
        Unit::Kilometers,
        Unit::Points,
        Unit::Picas,
        Unit::Ciceros,
        Unit::Didots,
        Unit::Degrees,
        Unit::Radians,
        Unit::Minutes,
        Unit::Seconds,
        Unit::DegreeMinSec,
        Unit::Percent,
        Unit::ElapsedWeeks,
        Unit::ElapsedDays,
        Unit::ElapsedHours,
        Unit::ElapsedMinutes,
        Unit::ElapsedSeconds,
        Unit::DateTime,
        Unit::InchesFractional,
        Unit::MilesFractional,
        Unit::FeetAndInches,
        Unit::PicasAndPoints,
        Unit::CicerosAndDidots,
        Unit::ImplicitDrawing,
        Unit::ImplicitPage,
        Unit::ImplicitType,
        Unit::ImplicitAngle,
        Unit::ImplicitDuration,
        Unit::Unitsless,
    ];

    #[test]
    fn every_unit_has_a_spelling_that_parses_back() {
        for &unit in ALL {
            let spelling = unit.spelling().expect("spelling never fails");
            assert_eq!(parse_unit(spelling), Ok(unit), "spelling {spelling:?}");
        }
    }

    #[test]
    fn the_spelling_table_covers_every_unit() {
        for &unit in ALL {
            let known = SPELLINGS.iter().any(|(_, u)| *u == unit);
            assert!(
                known || unit == Unit::Unitsless,
                "no spelling row for {unit:?}"
            );
        }
    }

    #[test]
    fn spellings_are_distinct() {
        for (i, (spelling, _)) in SPELLINGS.iter().enumerate() {
            for (other, _) in SPELLINGS.iter().skip(i + 1) {
                assert!(
                    !spelling.eq_ignore_ascii_case(other),
                    "{spelling} == {other}"
                );
            }
        }
    }

    #[test]
    fn parsing_is_case_insensitive_and_trims() {
        assert_eq!(parse_unit("in"), Ok(Unit::Inches));
        assert_eq!(parse_unit("In"), Ok(Unit::Inches));
        assert_eq!(parse_unit("DEG"), Ok(Unit::Degrees));
        assert_eq!(parse_unit("deg"), Ok(Unit::Degrees));
        assert_eq!(parse_unit("  ft "), Ok(Unit::Feet));
        assert_eq!(parse_unit("feet/inch"), Ok(Unit::FeetAndInches));
    }

    #[test]
    fn the_empty_spelling_is_a_plain_number() {
        assert_eq!(parse_unit(""), Ok(Unit::Unitsless));
        assert_eq!(parse_unit("   "), Ok(Unit::Unitsless));
        assert_eq!(Unit::Unitsless.spelling(), Ok(""));
    }

    #[test]
    fn unknown_and_area_spellings_are_invalid_units() {
        for name in [
            "furlong", "SQ IN", "SQ. IN.", "IN^2", "HA", "ACRES", "HECTARES",
        ] {
            let error = parse_unit(name).expect_err("not a unit");
            match error {
                Error::Syntax { span, kind } => {
                    assert_eq!(kind, SyntaxKind::InvalidUnit);
                    assert_eq!(span, Span::new(0, name.len() as u32));
                }
                other => panic!("wrong error: {other:?}"),
            }
        }
    }

    #[test]
    fn families_match_the_adr_table() {
        for &unit in ALL {
            let family = unit.family().expect("family never fails");
            let want = match unit {
                Unit::Points | Unit::Picas | Unit::Ciceros | Unit::Didots => {
                    UnitFamily::Typographic
                }
                Unit::Degrees
                | Unit::Radians
                | Unit::Minutes
                | Unit::Seconds
                | Unit::DegreeMinSec
                | Unit::ImplicitAngle => UnitFamily::Angle,
                Unit::Percent => UnitFamily::Ratio,
                Unit::ElapsedWeeks
                | Unit::ElapsedDays
                | Unit::ElapsedHours
                | Unit::ElapsedMinutes
                | Unit::ElapsedSeconds
                | Unit::DateTime
                | Unit::ImplicitDuration => UnitFamily::Duration,
                Unit::Unitsless => UnitFamily::Dimensionless,
                _ => UnitFamily::Length,
            };
            assert_eq!(family, want, "{unit:?}");
        }
    }

    #[test]
    fn exact_factors_are_exact() {
        assert_eq!(Unit::Points.to_canonical(72.0), Ok(1.0));
        assert_eq!(Unit::Points.from_canonical(1.0), Ok(72.0));
        assert_eq!(Unit::Feet.to_canonical(1.0), Ok(12.0));
        assert_eq!(Unit::Miles.to_canonical(1.0), Ok(63_360.0));
        assert_eq!(
            Unit::Degrees.to_canonical(1.0),
            Ok(core::f64::consts::PI / 180.0)
        );
        assert_eq!(Unit::Percent.to_canonical(50.0), Ok(0.5));
        assert_eq!(Unit::ElapsedDays.to_canonical(1.0), Ok(86_400.0));
        assert_eq!(Unit::ImplicitDrawing.to_canonical(2.5), Ok(2.5));
        assert_eq!(Unit::Unitsless.to_canonical(0.25), Ok(0.25));
    }

    #[test]
    fn metric_lengths_follow_the_international_inch() {
        let inch = Unit::Millimeters
            .to_canonical(25.4)
            .expect("to_canonical never fails");
        assert!((inch - 1.0).abs() <= 1e-12);
    }

    #[test]
    fn geom_bridging_is_the_identity_on_the_shared_set() {
        for &unit in ALL {
            if let Ok(geom) = unit.to_geom() {
                assert_eq!(Unit::from_geom(geom), Ok(unit), "{unit:?}");
            }
        }
    }

    #[test]
    fn units_the_model_lacks_do_not_bridge() {
        for unit in [
            Unit::NauticalMiles,
            Unit::Didots,
            Unit::Minutes,
            Unit::Seconds,
            Unit::DegreeMinSec,
            Unit::ElapsedDays,
            Unit::DateTime,
            Unit::InchesFractional,
            Unit::FeetAndInches,
            Unit::ImplicitDrawing,
            Unit::ImplicitAngle,
            Unit::ImplicitDuration,
        ] {
            assert!(unit.to_geom().is_err(), "{unit:?} bridged");
        }
        for geom in [
            ferrogram_geom::Unit::Angstroms,
            ferrogram_geom::Unit::Nanometers,
            ferrogram_geom::Unit::Microns,
            ferrogram_geom::Unit::Millipoints,
        ] {
            assert!(Unit::from_geom(geom).is_err(), "{geom:?} bridged");
        }
    }
}
