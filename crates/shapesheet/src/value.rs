//! Evaluated values (ADR 0052).
//!
//! A formula evaluates to a [`Value`]: a number with a unit, a string, a
//! boolean, a colour, a list, or an **error value** such as `#VALUE!`
//! ([`ErrorCode`], "About Error Values"). Evaluation problems are values,
//! never Rust errors and never panics.
//!
//! The model stores a cached value and a unit spelling per cell (ADR
//! 0011): [`Value::to_cached`] and [`Value::from_cached`] are the bridge
//! (P2-14). The rules they implement:
//!
//! - A number converts to the family's canonical unit (inches, radians,
//!   fraction, seconds) and records the unit the file would write (`U=`,
//!   [`Unit::to_geom`]) so a write restores it. A unit the model does not
//!   carry (arc minutes, durations, the implicit units) records the
//!   family's canonical unit instead: the spelling is lost, the value is
//!   not. [`Unit::from_geom`] maps the recorded unit back.
//! - A colour stores [`CellValue::Color`], a list stores its raw string
//!   ([`CellValue::List`], entries joined with `;`), an error value
//!   stores its `#...` text ([`CellValue::Error`]).
//! - A string, boolean, colour or list **drops the unit on write**: a
//!   unit on a non-numeric kind is meaningless and does not read back
//!   (the P2-14 fix).
//! - A non-finite magnitude never stores as `inf`: it is `#NUM!` in both
//!   directions (P3-11/P3-16, ADR 0052).

use larrygram_model::{CellValue, Color};

use crate::Error;
use crate::units::Unit;

/// A number with the unit it is measured in (ADR 0052).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Number {
    /// The magnitude.
    pub magnitude: f64,
    /// The unit the magnitude is in.
    pub unit: Unit,
}

impl Number {
    /// A number of `unit`.
    pub const fn new(magnitude: f64, unit: Unit) -> Self {
        Self { magnitude, unit }
    }
}

/// An error value (ADR 0052, "About Error Values").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ErrorCode {
    /// `#DIV/0!`, division by zero.
    Div0,
    /// `#VALUE!`, a wrongly typed operand or argument.
    Value,
    /// `#REF!`, a reference naming a cell the document does not hold.
    Ref,
    /// `#NUM!`, an invalid number (also a non-finite result, P3-11).
    Num,
    /// `#N/A!`, no available value (`NA()`).
    Na,
    /// `#DIM!`, a dimensional value out of range or misused.
    Dim,
}

impl ErrorCode {
    /// The spelling a cell caches (`#VALUE!`).
    pub const fn as_str(self) -> &'static str {
        match self {
            ErrorCode::Div0 => "#DIV/0!",
            ErrorCode::Value => "#VALUE!",
            ErrorCode::Ref => "#REF!",
            ErrorCode::Num => "#NUM!",
            ErrorCode::Na => "#N/A!",
            ErrorCode::Dim => "#DIM!",
        }
    }

    /// The error value a cached spelling names, or [`Error::Conversion`]
    /// for text that is none of the six (the reader keeps arbitrary text
    /// in the cell).
    pub fn from_text(text: &str) -> Result<ErrorCode, Error> {
        let text = text.trim();
        let code = match text {
            "#DIV/0!" => ErrorCode::Div0,
            "#VALUE!" => ErrorCode::Value,
            "#REF!" => ErrorCode::Ref,
            "#NUM!" => ErrorCode::Num,
            "#N/A!" => ErrorCode::Na,
            "#DIM!" => ErrorCode::Dim,
            _ => return Err(Error::Conversion("cached error value")),
        };
        Ok(code)
    }
}

/// An evaluated value (ADR 0052).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Value {
    /// A number with its unit.
    Number(Number),
    /// A string.
    Str(String),
    /// A boolean.
    Bool(bool),
    /// A colour.
    Color(Color),
    /// A list of values (a `List` cell caches it as one raw string).
    List(Vec<Value>),
    /// An error value.
    Err(ErrorCode),
}

/// What a [`Value`] writes into a `model::Cell` (ADR 0057, P2-14): the
/// cached value in canonical units and the unit spelling the cell keeps.
#[derive(Debug, Clone, PartialEq)]
pub struct CachedValue {
    /// The cached value, in canonical units (ADR 0011).
    pub value: CellValue,
    /// The unit the cell records (`U=` spelling source).
    pub unit: larrygram_geom::Unit,
}

/// The shortest decimal spelling of a number that parses back to the same
/// `f64` (`Rust`'s own [`f64`] display: `10`, `2.5`, `0.5`).
pub(crate) fn number_to_string(magnitude: f64) -> String {
    format!("{magnitude}")
}

/// The `#RRGGBB` spelling of a colour, the form a cell caches.
pub(crate) fn color_to_string(color: Color) -> String {
    format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b)
}

/// Parses a `#RGB` or `#RRGGBB` colour literal.
pub(crate) fn parse_color_literal(text: &str) -> Option<Color> {
    let hex = text.strip_prefix('#')?;
    match hex.len() {
        3 => {
            let r = hex_nibble(hex, 0)?;
            let g = hex_nibble(hex, 1)?;
            let b = hex_nibble(hex, 2)?;
            Some(Color::new(r * 17, g * 17, b * 17))
        }
        6 => {
            let r = hex_byte(hex, 0)?;
            let g = hex_byte(hex, 2)?;
            let b = hex_byte(hex, 4)?;
            Some(Color::new(r, g, b))
        }
        _ => None,
    }
}

/// One hex digit of `hex` at `at`.
fn hex_nibble(hex: &str, at: usize) -> Option<u8> {
    let digit = hex.get(at..at.checked_add(1)?)?;
    u8::from_str_radix(digit, 16).ok()
}

/// Two hex digits of `hex` at `at`, as a byte.
fn hex_byte(hex: &str, at: usize) -> Option<u8> {
    let digits = hex.get(at..at.checked_add(2)?)?;
    u8::from_str_radix(digits, 16).ok()
}

/// The raw spelling of one list entry (`;` separates entries).
///
/// Numbers spell their canonical magnitude; a nested list flattens into
/// the same spelling.
pub(crate) fn list_element_text(value: &Value) -> String {
    match value {
        Value::Number(number) => {
            if number.magnitude.is_finite() {
                let magnitude = number
                    .unit
                    .to_canonical(number.magnitude)
                    .unwrap_or(number.magnitude);
                number_to_string(magnitude)
            } else {
                ErrorCode::Num.as_str().to_owned()
            }
        }
        Value::Str(text) => text.clone(),
        Value::Bool(true) => String::from("TRUE"),
        Value::Bool(false) => String::from("FALSE"),
        Value::Color(color) => color_to_string(*color),
        Value::List(entries) => entries
            .iter()
            .map(list_element_text)
            .collect::<Vec<_>>()
            .join(";"),
        Value::Err(code) => code.as_str().to_owned(),
    }
}

/// The value one raw list entry spells.
///
/// The order of the rules is the reader's: an error spelling first (they
/// start with `#` like a colour literal but are not hex), then a boolean,
/// a colour, a number, and anything else is a string.
pub(crate) fn parse_list_element(entry: &str) -> Value {
    let entry = entry.trim();
    if entry.is_empty() {
        return Value::Str(String::new());
    }
    if let Ok(code) = ErrorCode::from_text(entry) {
        return Value::Err(code);
    }
    if entry.eq_ignore_ascii_case("TRUE") {
        return Value::Bool(true);
    }
    if entry.eq_ignore_ascii_case("FALSE") {
        return Value::Bool(false);
    }
    if let Some(color) = parse_color_literal(entry) {
        return Value::Color(color);
    }
    if let Ok(magnitude) = entry.parse::<f64>() {
        if magnitude.is_finite() {
            return Value::Number(Number::new(magnitude, Unit::Unitsless));
        }
        return Value::Err(ErrorCode::Num);
    }
    Value::Str(entry.to_owned())
}

/// The cached form of a list: its entries joined with `;`.
pub(crate) fn list_raw(entries: &[Value]) -> String {
    entries
        .iter()
        .map(list_element_text)
        .collect::<Vec<_>>()
        .join(";")
}

/// The evaluated form of a cached list raw string.
pub(crate) fn parse_list(raw: &str) -> Vec<Value> {
    if raw.is_empty() {
        return Vec::new();
    }
    raw.split(';').map(parse_list_element).collect()
}

/// The [`larrygram_geom::Unit`] a number's unit records in a cell: the
/// file's own unit when the model carries it, else the family's canonical
/// unit (inches, radians, percent), and the plain number for the
/// dimensionless and duration families (the model has no duration unit).
fn cached_unit(unit: Unit) -> larrygram_geom::Unit {
    unit.to_geom().unwrap_or_else(|_| match unit.family() {
        Ok(crate::units::UnitFamily::Length) | Ok(crate::units::UnitFamily::Typographic) => {
            larrygram_geom::Unit::Inches
        }
        Ok(crate::units::UnitFamily::Angle) => larrygram_geom::Unit::Radians,
        Ok(crate::units::UnitFamily::Ratio) => larrygram_geom::Unit::Percent,
        _ => larrygram_geom::Unit::Unitsless,
    })
}

impl Value {
    /// Maps an evaluated value to the model's cached form (P2-14).
    ///
    /// A number becomes [`CellValue::Number`] in canonical units and
    /// records the unit spelling of [`Unit::to_geom`]; every non-numeric
    /// kind records [`larrygram_geom::Unit::Unitsless`] (the P2-14 unit
    /// drop). A non-finite magnitude becomes [`CellValue::Error`] `#NUM!`.
    pub fn to_cached(&self) -> Result<CachedValue, Error> {
        let cached = match self {
            Value::Number(number) => {
                if !number.magnitude.is_finite() {
                    CachedValue {
                        value: CellValue::Error(ErrorCode::Num.as_str().to_owned()),
                        unit: larrygram_geom::Unit::Unitsless,
                    }
                } else {
                    let magnitude = number.unit.to_canonical(number.magnitude)?;
                    CachedValue {
                        value: CellValue::Number(magnitude),
                        unit: cached_unit(number.unit),
                    }
                }
            }
            Value::Str(text) => CachedValue {
                value: CellValue::Str(text.clone()),
                unit: larrygram_geom::Unit::Unitsless,
            },
            Value::Bool(flag) => CachedValue {
                value: CellValue::Bool(*flag),
                unit: larrygram_geom::Unit::Unitsless,
            },
            Value::Color(color) => CachedValue {
                value: CellValue::Color(*color),
                unit: larrygram_geom::Unit::Unitsless,
            },
            Value::List(entries) => CachedValue {
                value: CellValue::List(list_raw(entries)),
                unit: larrygram_geom::Unit::Unitsless,
            },
            Value::Err(code) => CachedValue {
                value: CellValue::Error(code.as_str().to_owned()),
                unit: larrygram_geom::Unit::Unitsless,
            },
        };
        Ok(cached)
    }

    /// Maps a model cached value and its unit to an evaluated value
    /// (P2-14).
    ///
    /// A [`CellValue::Number`] is in canonical units, so the magnitude is
    /// converted back into `unit` (`V='11.69' U='MM'` reads as 297 mm).
    /// A non-finite number reads as `#NUM!` and a cached error keeps its
    /// text. The unit of a non-numeric kind is ignored (the P2-14 drop).
    pub fn from_cached(value: &CellValue, unit: larrygram_geom::Unit) -> Result<Value, Error> {
        let value = match value {
            CellValue::Number(magnitude) => {
                if !magnitude.is_finite() {
                    Value::Err(ErrorCode::Num)
                } else {
                    let unit = Unit::from_geom(unit)?;
                    let magnitude = unit.from_canonical(*magnitude)?;
                    Value::Number(Number::new(magnitude, unit))
                }
            }
            CellValue::Str(text) => Value::Str(text.clone()),
            CellValue::Bool(flag) => Value::Bool(*flag),
            CellValue::Color(color) => Value::Color(*color),
            CellValue::List(raw) => Value::List(parse_list(raw)),
            CellValue::Error(text) => Value::Err(ErrorCode::from_text(text)?),
            // `CellValue` is non-exhaustive: a kind this crate does not
            // know is a failed conversion, never a panic.
            _ => return Err(Error::Conversion("cached value kind")),
        };
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_number_carries_its_unit() {
        let n = Number::new(2.0, Unit::Inches);
        assert_eq!(n.magnitude, 2.0);
        assert_eq!(n.unit, Unit::Inches);
    }

    #[test]
    fn error_codes_spell_their_cached_text() {
        assert_eq!(ErrorCode::Div0.as_str(), "#DIV/0!");
        assert_eq!(ErrorCode::Value.as_str(), "#VALUE!");
        assert_eq!(ErrorCode::Ref.as_str(), "#REF!");
        assert_eq!(ErrorCode::Num.as_str(), "#NUM!");
        assert_eq!(ErrorCode::Na.as_str(), "#N/A!");
        assert_eq!(ErrorCode::Dim.as_str(), "#DIM!");
    }

    #[test]
    fn error_spellings_read_back() {
        for code in [
            ErrorCode::Div0,
            ErrorCode::Value,
            ErrorCode::Ref,
            ErrorCode::Num,
            ErrorCode::Na,
            ErrorCode::Dim,
        ] {
            assert_eq!(ErrorCode::from_text(code.as_str()), Ok(code));
            assert_eq!(ErrorCode::from_text(code.as_str().trim()), Ok(code));
        }
    }

    #[test]
    fn unknown_error_text_is_a_conversion_error() {
        assert_eq!(
            ErrorCode::from_text("#NAME?"),
            Err(Error::Conversion("cached error value"))
        );
        assert_eq!(
            ErrorCode::from_text(""),
            Err(Error::Conversion("cached error value"))
        );
    }

    #[test]
    fn a_value_holds_every_kind() {
        let values = [
            Value::Number(Number::new(1.0, Unit::Unitsless)),
            Value::Str(String::from("x")),
            Value::Bool(true),
            Value::Color(Color::new(0, 0, 0)),
            Value::List(Vec::new()),
            Value::Err(ErrorCode::Value),
        ];
        assert_eq!(values.len(), 6);
    }

    #[test]
    fn numbers_convert_to_canonical_and_record_the_file_unit() {
        let cached = Value::Number(Number::new(72.0, Unit::Points))
            .to_cached()
            .expect("to_cached never fails");
        assert_eq!(cached.value, CellValue::Number(1.0));
        assert_eq!(cached.unit, larrygram_geom::Unit::Points);
    }

    #[test]
    fn a_number_the_model_has_no_unit_for_records_the_family_unit() {
        let cached = Value::Number(Number::new(1.0, Unit::Minutes))
            .to_cached()
            .expect("to_cached never fails");
        assert_eq!(
            cached.value,
            CellValue::Number(core::f64::consts::PI / 10_800.0)
        );
        assert_eq!(cached.unit, larrygram_geom::Unit::Radians);
    }

    #[test]
    fn a_cached_number_reads_back_in_its_recorded_unit() {
        // The corpus spelling: `PageWidth V='11.69291338582677' U='MM'`
        // is a 297 mm page written in inches (ADR 0011).
        let value = Value::from_cached(
            &CellValue::Number(11.69291338582677),
            larrygram_geom::Unit::Millimeters,
        )
        .expect("from_cached never fails here");
        match value {
            Value::Number(number) => {
                assert_eq!(number.unit, Unit::Millimeters);
                assert!((number.magnitude - 297.0).abs() <= 1e-6);
            }
            other => panic!("wrong kind: {other:?}"),
        }
    }

    #[test]
    fn non_finite_magnitudes_are_num_errors_in_both_directions() {
        let cached = Value::Number(Number::new(f64::INFINITY, Unit::Inches))
            .to_cached()
            .expect("to_cached never fails");
        assert_eq!(cached.value, CellValue::Error(String::from("#NUM!")));
        assert_eq!(cached.unit, larrygram_geom::Unit::Unitsless);
        assert_eq!(
            Value::from_cached(&CellValue::Number(f64::NAN), larrygram_geom::Unit::Inches),
            Ok(Value::Err(ErrorCode::Num))
        );
    }

    #[test]
    fn non_numeric_kinds_drop_the_unit_on_write() {
        for value in [
            Value::Str(String::from("x")),
            Value::Bool(true),
            Value::Color(Color::new(1, 2, 3)),
            Value::List(vec![Value::Number(Number::new(1.0, Unit::Unitsless))]),
            Value::Err(ErrorCode::Value),
        ] {
            let cached = value.to_cached().expect("to_cached never fails");
            assert_eq!(cached.unit, larrygram_geom::Unit::Unitsless, "{value:?}");
            // The test that reads it back (P2-14).
            let back = Value::from_cached(&cached.value, larrygram_geom::Unit::Inches)
                .expect("from_cached round-trips");
            assert_eq!(back, value, "{value:?}");
        }
    }

    #[test]
    fn an_unknown_cached_error_text_is_a_conversion_error() {
        assert_eq!(
            Value::from_cached(
                &CellValue::Error(String::from("#NAME?")),
                larrygram_geom::Unit::Inches
            ),
            Err(Error::Conversion("cached error value"))
        );
    }

    #[test]
    fn a_model_unit_without_a_formula_spelling_is_a_conversion_error() {
        assert_eq!(
            Value::from_cached(&CellValue::Number(1.0), larrygram_geom::Unit::Nanometers),
            Err(Error::Conversion("model unit has no formula spelling"))
        );
    }

    #[test]
    fn list_entries_round_trip_through_the_raw_string() {
        let entries = vec![
            Value::Number(Number::new(72.0, Unit::Points)),
            Value::Str(String::from("x")),
            Value::Bool(true),
            Value::Color(Color::new(0x12, 0x34, 0x56)),
            Value::Err(ErrorCode::Ref),
        ];
        let raw = list_raw(&entries);
        assert_eq!(raw, "1;x;TRUE;#123456;#REF!");
        let back = parse_list(&raw);
        assert_eq!(
            back,
            vec![
                Value::Number(Number::new(1.0, Unit::Unitsless)),
                Value::Str(String::from("x")),
                Value::Bool(true),
                Value::Color(Color::new(0x12, 0x34, 0x56)),
                Value::Err(ErrorCode::Ref),
            ]
        );
    }

    #[test]
    fn the_empty_list_spells_as_nothing() {
        assert_eq!(list_raw(&[]), "");
        assert_eq!(parse_list(""), Vec::new());
    }

    #[test]
    fn nested_lists_flatten() {
        let nested = Value::List(vec![
            Value::Str(String::from("a")),
            Value::List(vec![Value::Bool(false)]),
        ]);
        assert_eq!(list_element_text(&nested), "a;FALSE");
    }

    #[test]
    fn list_entries_that_look_like_something_else_are_strings() {
        assert_eq!(
            parse_list_element("sheep"),
            Value::Str(String::from("sheep"))
        );
        assert_eq!(parse_list_element(""), Value::Str(String::new()));
        assert_eq!(parse_list_element("#xyz"), Value::Str(String::from("#xyz")));
        assert_eq!(parse_list_element("1e999"), Value::Err(ErrorCode::Num));
    }

    #[test]
    fn colour_literals_come_in_three_and_six_digits() {
        assert_eq!(
            parse_color_literal("#123456"),
            Some(Color::new(0x12, 0x34, 0x56))
        );
        assert_eq!(
            parse_color_literal("#abc"),
            Some(Color::new(0xAA, 0xBB, 0xCC))
        );
        assert_eq!(parse_color_literal("#12345"), None);
        assert_eq!(parse_color_literal("123456"), None);
        assert_eq!(parse_color_literal("#gggggg"), None);
        assert_eq!(color_to_string(Color::new(0x12, 0x34, 0x56)), "#123456");
    }

    #[test]
    fn numbers_print_as_they_parse() {
        assert_eq!(number_to_string(10.0), "10");
        assert_eq!(number_to_string(2.5), "2.5");
        for magnitude in [0.0, -0.5, 1e-7, 1234.5678, 1e300] {
            let text = number_to_string(magnitude);
            assert_eq!(text.parse::<f64>(), Ok(magnitude), "{text}");
        }
    }

    #[test]
    fn hex_digits_use_get_never_indexing() {
        assert_eq!(hex_byte("12", 0), Some(0x12));
        assert_eq!(hex_byte("12", 5), None);
        assert_eq!(hex_byte("1", 0), None);
        assert_eq!(hex_nibble("F0", 0), Some(15));
        assert_eq!(hex_nibble("F", 1), None);
    }
}
