//! Encoding model values back to the `V`, `U` and `E` attributes of a `Cell`
//! (task P2-07).
//!
//! This is the inverse of [`crate::read::value`]: the cached value is written
//! in canonical units (ADR 0011), so no conversion happens, and the `U` code
//! is restored from [`Cell::unit`](ferrogram_model::Cell). Only the codes that
//! map back to the same spelling are translated (the reader keeps every other
//! code in `Preserved`), so a write restores the file's own spelling.
//!
//! A cached value compares equal to its written attribute when the attribute
//! still spells it, under the same rules the reader parses with: a number may
//! have any spelling (`2.0` stays `2.0` when the value did not change), a
//! boolean may be `0`/`1` or `true`/`false` in any case, and a color may be
//! `#RGB` or `#RRGGBB` in any case. That keeps an unedited value byte-stable.

use ferrogram_geom::Unit;
use ferrogram_model::{CellValue, Color};

use crate::value_parse::parse_color;

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
        _ => None,
    }
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
    fn nan_and_infinity_keep_their_spelling() {
        assert_eq!(encode_value(&CellValue::Number(f64::NAN)), "NaN");
        assert_eq!(encode_value(&CellValue::Number(f64::INFINITY)), "inf");
        assert!(value_matches("NaN", &CellValue::Number(f64::NAN)));
        assert!(value_matches("inf", &CellValue::Number(f64::INFINITY)));
        assert!(value_matches("-inf", &CellValue::Number(f64::NEG_INFINITY)));
    }
}
