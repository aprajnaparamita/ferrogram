//! Encoding model values back to the `V`, `U` and `E` attributes of a `Cell`
//! (task P2-07).
//!
//! This is the inverse of [`crate::read::value`]: the cached value is written
//! in canonical units (ADR 0011), so no conversion happens, and the `U` code
//! is restored from [`Cell::unit`](ferrogram_model::Cell). Only the codes that
//! map back to the same spelling are translated (the reader keeps every other
//! code in `Preserved`), so a write restores the file's own spelling.
//!
//! A value the model built (not one read from a file with its `U`) needs the
//! `U` the reader classifies it with (task P2-14): `U="BOOL"` for `Bool` and
//! `U="STR"` for a `Str` whose text would otherwise read as a number or a
//! colour. A [`CellValue::List`] is out of scope here: no `U` code maps back
//! to a list, so one is written as its raw text and reads back by shape; the
//! formula engine owns that kind.
//!
//! A cached value compares equal to its written attribute when the attribute
//! still spells it, under the same rules the reader parses with: a number may
//! have any spelling (`2.0` stays `2.0` when the value did not change), a
//! boolean may be `0`/`1` or `true`/`false` in any case, and a color may be
//! `#RGB` or `#RRGGBB` in any case. That keeps an unedited value byte-stable.

use ferrogram_geom::Unit;
use ferrogram_model::{CellValue, Color};

use crate::read::value::{Kind, kind};
use crate::value_parse::{Shaped, parse_color, shape};

/// The `U` code that `unit` spells, or `None` when the unit has no code
/// ([`Unit::Unitsless`] and codes the reader keeps preserved).
///
/// The mapping is the inverse of the reader's: every code that translates to
/// a [`Unit`] translates back to the same code.
pub(crate) fn unit_code(unit: Unit) -> Option<&'static str> {
    match unit {
        Unit::Inches => Some("IN"),
        Unit::Feet => Some("FT"),
        Unit::Yards => Some("YD"),
        Unit::Miles => Some("MI"),
        Unit::Millimeters => Some("MM"),
        Unit::Centimeters => Some("CM"),
        Unit::Meters => Some("M"),
        Unit::Kilometers => Some("KM"),
        Unit::Points => Some("PT"),
        Unit::Picas => Some("P"),
        Unit::Ciceros => Some("C"),
        Unit::Degrees => Some("DEG"),
        Unit::Radians => Some("RAD"),
        Unit::Percent => Some("PER"),
        // A unit with no code writes no `U` (decision, coverage audit minor
        // 8): the value reads back by its shape and a preserved spelling
        // stays in `Preserved`. Keep, not an error — an error would reject a
        // model the reader accepted. `every_unit_has_a_spelling_or_a_place`
        // lists the units this arm covers.
        _ => None,
    }
}

/// What the writer does with the `U` attribute of a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnitSpelling {
    /// The `U` already there declares the value's kind: keep it as written.
    Keep,
    /// Write this code.
    Code(&'static str),
    /// Write no `U`. A spelling the file used and the model kept is not
    /// removed by this; the caller checks [`Preserved`](ferrogram_model::Preserved).
    Absent,
}

/// What the writer does with the `U` attribute of a cell whose cached value
/// is `value` in `unit`, given the `U` spelling `current` (the file's
/// attribute or the preserved fragment's).
///
/// A value the reader would classify differently without a `U` gets the code
/// it needs ([`kind_code`]); a spelling that already declares the value's
/// kind is kept (`DATE` for a date string), so an unedited write changes
/// nothing. Otherwise the unit code wins, and a unit with no code writes no
/// `U`.
pub(crate) fn unit_spelling(
    value: Option<&CellValue>,
    unit: Unit,
    current: Option<&str>,
) -> UnitSpelling {
    if let Some(value) = value
        && let Some(code) = kind_code(value, unit)
    {
        return if declares_kind(current, value) {
            UnitSpelling::Keep
        } else {
            UnitSpelling::Code(code)
        };
    }
    match unit_code(unit) {
        Some(code) => UnitSpelling::Code(code),
        None => UnitSpelling::Absent,
    }
}

/// The `U` code a value's kind needs for the reader to classify it the same
/// way: `BOOL` for `Bool`, `STR` for a `Str` whose text would read as a
/// number or a colour, or whose unit would otherwise write a numeric code.
/// `None` when the value's own shape already reads back the same kind.
///
/// Kind fidelity wins over the unit for `Bool` and `Str`: the reader gives
/// neither kind a unit back. A [`CellValue::List`] has no code the reader
/// maps back to a list: out of scope (the formula engine owns it).
pub(crate) fn kind_code(value: &CellValue, unit: Unit) -> Option<&'static str> {
    match value {
        CellValue::Bool(_) => Some("BOOL"),
        CellValue::Str(text)
            if !matches!(shape(text), Shaped::Str(_)) || unit_code(unit).is_some() =>
        {
            Some("STR")
        }
        _ => None,
    }
}

/// Whether the `U` spelling `code` makes the reader classify the written `V`
/// of `value` as the value's kind (the rules of [`crate::read::value`]).
///
/// Only called for values whose [`kind_code`] fires (`Bool`, misreading
/// `Str`); without a `U` such a value never reads back the same kind, so
/// `None` never declares it.
fn declares_kind(code: Option<&str>, value: &CellValue) -> bool {
    let want = match value {
        CellValue::Bool(_) => Kind::Bool,
        CellValue::Str(_) => Kind::Str,
        _ => return false,
    };
    code.is_some_and(|code| kind(Some(code)) == want)
}

/// The `V` attribute value of a cached value.
///
/// Numbers are written in their shortest round-tripping spelling, booleans as
/// `0`/`1`, colors as `#rrggbb`, and strings as written. An error value is
/// written to `E`, never to `V` (the caller handles it).
pub(crate) fn encode_value(value: &CellValue) -> String {
    match value {
        CellValue::Number(number) => number_text(*number),
        CellValue::Str(text) | CellValue::List(text) | CellValue::Error(text) => text.clone(),
        CellValue::Bool(true) => "1".to_owned(),
        CellValue::Bool(false) => "0".to_owned(),
        CellValue::Color(color) => color_text(*color),
        // A future `CellValue` kind has no spelling here; the caller writes
        // the empty string rather than guessing at one.
        _ => String::new(),
    }
}

/// Whether `current` (a written `V` attribute) already spells `value`.
///
/// Equal spellings are never rewritten, so a round-trip keeps the file's own
/// numbers, boolean spellings and color case.
pub(crate) fn value_matches(current: &str, value: &CellValue) -> bool {
    if current == encode_value(value) {
        return true;
    }
    match value {
        CellValue::Number(number) => current.trim().parse::<f64>() == Ok(*number),
        CellValue::Bool(flag) => match current.trim() {
            "1" | "true" | "TRUE" => *flag,
            "0" | "false" | "FALSE" => !*flag,
            _ => false,
        },
        CellValue::Color(want) => parse_color(current).as_ref() == Some(want),
        _ => false,
    }
}

/// The shortest decimal spelling of a number that parses back to it.
fn number_text(number: f64) -> String {
    number.to_string()
}

/// The `#rrggbb` spelling of a color.
fn color_text(color: Color) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_codes_round_trip_the_reader_spellings() {
        for (code, unit) in [
            ("IN", Unit::Inches),
            ("FT", Unit::Feet),
            ("YD", Unit::Yards),
            ("MI", Unit::Miles),
            ("MM", Unit::Millimeters),
            ("CM", Unit::Centimeters),
            ("M", Unit::Meters),
            ("KM", Unit::Kilometers),
            ("PT", Unit::Points),
            ("P", Unit::Picas),
            ("C", Unit::Ciceros),
            ("DEG", Unit::Degrees),
            ("RAD", Unit::Radians),
            ("PER", Unit::Percent),
        ] {
            assert_eq!(unit_code(unit), Some(code));
        }
        assert_eq!(unit_code(Unit::Unitsless), None);
        assert_eq!(unit_code(Unit::Nanometers), None);
    }

    #[test]
    fn every_unit_has_a_spelling_or_a_place() {
        // Coverage (audit minor 8): the complete list of what the writer
        // does with every `Unit` variant. A variant with no code writes no
        // `U` (the `_ => None` fallback, where the keep-not-error decision
        // is recorded); this test lists them, so a new variant cannot ship
        // with a silent fallback.
        let spelled = [
            (Unit::Inches, "IN"),
            (Unit::Feet, "FT"),
            (Unit::Yards, "YD"),
            (Unit::Miles, "MI"),
            (Unit::Millimeters, "MM"),
            (Unit::Centimeters, "CM"),
            (Unit::Meters, "M"),
            (Unit::Kilometers, "KM"),
            (Unit::Points, "PT"),
            (Unit::Picas, "P"),
            (Unit::Ciceros, "C"),
            (Unit::Degrees, "DEG"),
            (Unit::Radians, "RAD"),
            (Unit::Percent, "PER"),
        ];
        for (unit, code) in spelled {
            assert_eq!(unit_code(unit), Some(code), "{unit:?} spells {code}");
        }
        let unspelled = [
            Unit::Angstroms,
            Unit::Nanometers,
            Unit::Microns,
            Unit::Millipoints,
            Unit::Unitsless,
        ];
        for unit in unspelled {
            assert_eq!(unit_code(unit), None, "{unit:?} writes no U");
        }
        // The two lists name every variant of the 19-variant `Unit` enum
        // (`#[non_exhaustive]`, so the match cannot check that itself).
        assert_eq!(spelled.len() + unspelled.len(), 19);
    }

    #[test]
    fn values_encode_to_their_attribute_spellings() {
        assert_eq!(encode_value(&CellValue::Number(2.0)), "2");
        assert_eq!(encode_value(&CellValue::Number(0.5)), "0.5");
        assert_eq!(encode_value(&CellValue::Bool(true)), "1");
        assert_eq!(encode_value(&CellValue::Bool(false)), "0");
        assert_eq!(encode_value(&CellValue::Str("Themed".to_owned())), "Themed");
        assert_eq!(encode_value(&CellValue::List("a;b".to_owned())), "a;b");
        assert_eq!(
            encode_value(&CellValue::Error("#VALUE!".to_owned())),
            "#VALUE!"
        );
        assert_eq!(
            encode_value(&CellValue::Color(Color::new(77, 77, 77))),
            "#4d4d4d"
        );
    }

    #[test]
    fn number_spellings_match_numerically() {
        assert!(value_matches("2.0", &CellValue::Number(2.0)));
        assert!(value_matches("2", &CellValue::Number(2.0)));
        assert!(value_matches(".50", &CellValue::Number(0.5)));
        assert!(value_matches(" 2 ", &CellValue::Number(2.0)));
        assert!(!value_matches("3", &CellValue::Number(2.0)));
    }

    #[test]
    fn boolean_spellings_match_the_reader_spellings() {
        assert!(value_matches("1", &CellValue::Bool(true)));
        assert!(value_matches("true", &CellValue::Bool(true)));
        assert!(value_matches("TRUE", &CellValue::Bool(true)));
        assert!(value_matches("0", &CellValue::Bool(false)));
        assert!(value_matches("false", &CellValue::Bool(false)));
        assert!(!value_matches("0", &CellValue::Bool(true)));
        assert!(!value_matches("2", &CellValue::Bool(true)));
    }

    #[test]
    fn color_spellings_match_in_any_case() {
        let white = Color::new(255, 255, 255);
        assert!(value_matches("#ffffff", &CellValue::Color(white)));
        assert!(value_matches("#FFFFFF", &CellValue::Color(white)));
        assert!(value_matches("#fff", &CellValue::Color(white)));
        assert!(!value_matches("#fffffe", &CellValue::Color(white)));
        assert!(!value_matches("Themed", &CellValue::Color(white)));
    }

    #[test]
    fn strings_match_exactly() {
        assert!(value_matches(
            "Themed",
            &CellValue::Str("Themed".to_owned())
        ));
        assert!(!value_matches("", &CellValue::Str("x".to_owned())));
        assert!(!value_matches("a;b", &CellValue::List("a".to_owned())));
        assert!(!value_matches("2", &CellValue::Str("2 ".to_owned())));
    }

    #[test]
    fn kind_codes_cover_bool_and_misreading_strings() {
        assert_eq!(
            kind_code(&CellValue::Bool(true), Unit::Unitsless),
            Some("BOOL")
        );
        // A string that reads back as a number or a colour without `U`.
        assert_eq!(
            kind_code(&CellValue::Str("2".to_owned()), Unit::Unitsless),
            Some("STR")
        );
        assert_eq!(
            kind_code(&CellValue::Str("#4d4d4d".to_owned()), Unit::Unitsless),
            Some("STR")
        );
        // A unit would write a numeric code over the string.
        assert_eq!(
            kind_code(&CellValue::Str("Themed".to_owned()), Unit::Inches),
            Some("STR")
        );
        assert_eq!(
            kind_code(&CellValue::Str("Themed".to_owned()), Unit::Unitsless),
            None
        );
        assert_eq!(kind_code(&CellValue::Number(2.0), Unit::Inches), None);
        assert_eq!(
            kind_code(&CellValue::Color(Color::new(0, 0, 0)), Unit::Unitsless),
            None
        );
        // `List` is out of scope: no code maps back to a list.
        assert_eq!(
            kind_code(&CellValue::List("a;b".to_owned()), Unit::Unitsless),
            None
        );
    }

    #[test]
    fn unit_spelling_prefers_the_kind_code() {
        assert_eq!(
            unit_spelling(Some(&CellValue::Bool(true)), Unit::Unitsless, None),
            UnitSpelling::Code("BOOL")
        );
        assert_eq!(
            unit_spelling(Some(&CellValue::Str("2".to_owned())), Unit::Unitsless, None),
            UnitSpelling::Code("STR")
        );
        assert_eq!(
            unit_spelling(Some(&CellValue::Number(2.0)), Unit::Inches, None),
            UnitSpelling::Code("IN")
        );
        assert_eq!(
            unit_spelling(Some(&CellValue::Number(2.0)), Unit::Unitsless, Some("IN")),
            UnitSpelling::Absent
        );
        assert_eq!(
            unit_spelling(None, Unit::Unitsless, None),
            UnitSpelling::Absent
        );
    }

    #[test]
    fn unit_spelling_keeps_a_spelling_that_declares_the_kind() {
        assert_eq!(
            unit_spelling(Some(&CellValue::Bool(true)), Unit::Unitsless, Some("BOOL")),
            UnitSpelling::Keep
        );
        // The file's `DATE` spelling stays; the kind reads back the same.
        assert_eq!(
            unit_spelling(
                Some(&CellValue::Str("2".to_owned())),
                Unit::Unitsless,
                Some("DATE")
            ),
            UnitSpelling::Keep
        );
        assert_eq!(
            unit_spelling(
                Some(&CellValue::Str("2".to_owned())),
                Unit::Unitsless,
                Some("STR")
            ),
            UnitSpelling::Keep
        );
        // A spelling that declares another kind does not survive a model
        // that took the value over.
        assert_eq!(
            unit_spelling(
                Some(&CellValue::Str("2".to_owned())),
                Unit::Unitsless,
                Some("FOO")
            ),
            UnitSpelling::Code("STR")
        );
        assert_eq!(
            unit_spelling(Some(&CellValue::Bool(true)), Unit::Unitsless, Some("DATE")),
            UnitSpelling::Code("BOOL")
        );
    }

    #[test]
    fn nan_and_infinity_keep_their_spelling() {
        assert_eq!(encode_value(&CellValue::Number(f64::NAN)), "NaN");
        assert_eq!(encode_value(&CellValue::Number(f64::INFINITY)), "inf");
        assert!(value_matches("NaN", &CellValue::Number(f64::NAN)));
        assert!(value_matches("inf", &CellValue::Number(f64::INFINITY)));
        assert!(value_matches("-inf", &CellValue::Number(f64::NEG_INFINITY)));
    }
}
