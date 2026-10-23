//! Coercions between value kinds (ADR 0052).
//!
//! Visio converts an operand to the kind an operator or function needs,
//! and supplies the typed default of the target kind when it cannot
//! (`about-operators.md`: zero for numbers, `FALSE` for booleans, `""`
//! for strings). [`coerce`] is that one rule; the `as_*` readers are the
//! typed views callers use.
//!
//! The documented conversions ("About Operators", ADR 0052, and the
//! model's own spelling of booleans as `0`/`1`, ADR 0021):
//!
//! - a numeric string to a number (`5 * "2"` is `10`) and a number to a
//!   string (`2 * 5 & " cents"` is `"10 cents"`);
//! - a number to a boolean (`0` is `FALSE`, any other magnitude `TRUE`)
//!   and a boolean to a number (`FALSE` is `0`, `TRUE` is `1`, the way
//!   the files write booleans);
//! - a boolean to a string (`TRUE`/`FALSE`, the spelling of the
//!   comparison results);
//! - every kind to a string through its own spelling (a colour as
//!   `#RRGGBB`, a list as its `;`-joined entries).
//!
//! Every other conversion falls back to the typed default of the target
//! kind (`0`, `""`, `FALSE`, black, the empty list) and reports it on
//! the `defaulted` field of `Coerced`, so the caller records one
//! [`crate::EvalWarningKind::Coercion`] warning (`5 * "sheep"` is `0`
//! with a `Coercion` warning). Coercion never fails and never panics.
//!
//! Two rules sit above the table: an **error value coerces to itself** in
//! every kind (the error propagates), and a **non-finite number is
//! `#NUM!`** (ADR 0052: a non-finite result is never a stored `inf`).

use crate::Error;
use crate::value::{ErrorCode, Number, Value, color_to_string, list_raw, number_to_string};
use larrygram_model::Color;

/// The kind a value is coerced to (ADR 0052).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ValueKind {
    /// A number.
    Number,
    /// A string.
    Str,
    /// A boolean.
    Bool,
    /// A colour.
    Color,
    /// A list.
    List,
}

/// What [`coerce`] produced (ADR 0052).
#[derive(Debug, Clone, PartialEq)]
pub struct Coerced {
    /// The value in the target kind: the conversion, or the typed default
    /// of the target kind.
    pub value: Value,
    /// True when the value could not be expressed in the target kind and
    /// the typed default was used (`5 * "sheep"` is `0`). The caller
    /// records one `crate::EvalWarningKind::Coercion` warning.
    pub defaulted: bool,
}

impl Coerced {
    /// A conversion of `value` that expressed the value.
    fn converted(value: Value) -> Self {
        Self {
            value,
            defaulted: false,
        }
    }

    /// The typed default of the target kind, flagged as defaulted.
    fn fallback(value: Value) -> Self {
        Self {
            value,
            defaulted: true,
        }
    }
}

/// The typed default of a target kind ("the typed equivalent of
/// nothing"): `0`, `""`, `FALSE`, black, the empty list.
fn typed_default(kind: ValueKind) -> Value {
    match kind {
        ValueKind::Number => Value::Number(Number::new(0.0, crate::units::Unit::Unitsless)),
        ValueKind::Str => Value::Str(String::new()),
        ValueKind::Bool => Value::Bool(false),
        ValueKind::Color => Value::Color(Color::new(0, 0, 0)),
        ValueKind::List => Value::List(Vec::new()),
    }
}

/// Coerces `value` to `kind` (ADR 0052).
///
/// The call always succeeds. It reports on the `defaulted` field of `Coerced` whether
/// the conversion expressed the value or fell back to the typed default;
/// only the second case earns a `Coercion` warning.
pub fn coerce(value: &Value, kind: ValueKind) -> Coerced {
    // An error value propagates unchanged in every kind.
    if let Value::Err(code) = value {
        return Coerced::converted(Value::Err(*code));
    }
    // A non-finite number is #NUM! at the source (ADR 0052, P3-11).
    if matches!(value, Value::Number(number) if !number.magnitude.is_finite()) {
        return Coerced::converted(Value::Err(ErrorCode::Num));
    }
    match kind {
        ValueKind::Number => to_number(value),
        ValueKind::Str => to_str(value),
        ValueKind::Bool => to_bool(value),
        ValueKind::Color => to_color(value),
        ValueKind::List => to_list(value),
    }
}

/// The value as a number: a numeric string parses, a boolean is `1`/`0`.
fn to_number(value: &Value) -> Coerced {
    match value {
        Value::Number(number) => Coerced::converted(Value::Number(*number)),
        Value::Str(text) => match text.trim().parse::<f64>() {
            Ok(magnitude) if magnitude.is_finite() => Coerced::converted(Value::Number(
                Number::new(magnitude, crate::units::Unit::Unitsless),
            )),
            Ok(_) => Coerced::converted(Value::Err(ErrorCode::Num)),
            Err(_) => Coerced::fallback(typed_default(ValueKind::Number)),
        },
        Value::Bool(flag) => Coerced::converted(Value::Number(Number::new(
            if *flag { 1.0 } else { 0.0 },
            crate::units::Unit::Unitsless,
        ))),
        _ => Coerced::fallback(typed_default(ValueKind::Number)),
    }
}

/// The value as a string: every kind has a spelling, a string is itself.
fn to_str(value: &Value) -> Coerced {
    let text = match value {
        Value::Number(number) => number_to_string(number.magnitude),
        Value::Str(text) => return Coerced::converted(Value::Str(text.clone())),
        Value::Bool(true) => String::from("TRUE"),
        Value::Bool(false) => String::from("FALSE"),
        Value::Color(color) => color_to_string(*color),
        Value::List(entries) => list_raw(entries),
        // `coerce` passes an error value through before this point.
        Value::Err(code) => code.as_str().to_owned(),
    };
    Coerced::converted(Value::Str(text))
}

/// The value as a boolean: `0` is `FALSE`, every other number `TRUE`.
fn to_bool(value: &Value) -> Coerced {
    match value {
        Value::Bool(flag) => Coerced::converted(Value::Bool(*flag)),
        Value::Number(number) => Coerced::converted(Value::Bool(number.magnitude != 0.0)),
        _ => Coerced::fallback(typed_default(ValueKind::Bool)),
    }
}

/// The value as a colour: only a colour expresses a colour.
fn to_color(value: &Value) -> Coerced {
    match value {
        Value::Color(color) => Coerced::converted(Value::Color(*color)),
        _ => Coerced::fallback(typed_default(ValueKind::Color)),
    }
}

/// The value as a list: only a list expresses a list.
fn to_list(value: &Value) -> Coerced {
    match value {
        Value::List(entries) => Coerced::converted(Value::List(entries.clone())),
        _ => Coerced::fallback(typed_default(ValueKind::List)),
    }
}

/// Reads `value` as a number.
///
/// The typed view of [`coerce`]: it succeeds when the conversion
/// expresses the value and returns [`Error::Conversion`] when coercion
/// would have to default, so a caller can choose its own default and
/// warning.
pub fn as_number(value: &Value) -> Result<Number, Error> {
    let coerced = coerce(value, ValueKind::Number);
    if coerced.defaulted {
        return Err(Error::Conversion("value is not a number"));
    }
    match coerced.value {
        Value::Number(number) => Ok(number),
        _ => Err(Error::Conversion("value is not a number")),
    }
}

/// Reads `value` as a string.
///
/// The typed view of [`coerce`]; see [`as_number`] for the error rule.
pub fn as_str(value: &Value) -> Result<String, Error> {
    let coerced = coerce(value, ValueKind::Str);
    if coerced.defaulted {
        return Err(Error::Conversion("value is not a string"));
    }
    match coerced.value {
        Value::Str(text) => Ok(text),
        _ => Err(Error::Conversion("value is not a string")),
    }
}

/// Reads `value` as a boolean.
///
/// The typed view of [`coerce`]; see [`as_number`] for the error rule.
pub fn as_bool(value: &Value) -> Result<bool, Error> {
    let coerced = coerce(value, ValueKind::Bool);
    if coerced.defaulted {
        return Err(Error::Conversion("value is not a boolean"));
    }
    match coerced.value {
        Value::Bool(flag) => Ok(flag),
        _ => Err(Error::Conversion("value is not a boolean")),
    }
}

/// Reads `value` as a colour.
///
/// The typed view of [`coerce`]; see [`as_number`] for the error rule.
pub fn as_color(value: &Value) -> Result<Color, Error> {
    let coerced = coerce(value, ValueKind::Color);
    if coerced.defaulted {
        return Err(Error::Conversion("value is not a colour"));
    }
    match coerced.value {
        Value::Color(color) => Ok(color),
        _ => Err(Error::Conversion("value is not a colour")),
    }
}

/// Reads `value` as a list.
///
/// The typed view of [`coerce`]; see [`as_number`] for the error rule.
pub fn as_list(value: &Value) -> Result<Vec<Value>, Error> {
    let coerced = coerce(value, ValueKind::List);
    if coerced.defaulted {
        return Err(Error::Conversion("value is not a list"));
    }
    match coerced.value {
        Value::List(entries) => Ok(entries),
        _ => Err(Error::Conversion("value is not a list")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Unit;

    fn number(magnitude: f64) -> Value {
        Value::Number(Number::new(magnitude, Unit::Unitsless))
    }

    #[test]
    fn every_target_kind_has_a_typed_default() {
        assert_eq!(typed_default(ValueKind::Number), number(0.0));
        assert_eq!(typed_default(ValueKind::Str), Value::Str(String::new()));
        assert_eq!(typed_default(ValueKind::Bool), Value::Bool(false));
        assert_eq!(
            typed_default(ValueKind::Color),
            Value::Color(Color::new(0, 0, 0))
        );
        assert_eq!(typed_default(ValueKind::List), Value::List(Vec::new()));
    }

    #[test]
    fn sheep_is_zero_with_a_defaulted_coercion() {
        let coerced = coerce(&Value::Str(String::from("sheep")), ValueKind::Number);
        assert_eq!(coerced.value, number(0.0));
        assert!(coerced.defaulted);
    }

    #[test]
    fn a_numeric_string_converts_silently() {
        let coerced = coerce(&Value::Str(String::from("2")), ValueKind::Number);
        assert_eq!(coerced.value, number(2.0));
        assert!(!coerced.defaulted);
        let coerced = coerce(&Value::Str(String::from(" -2.5 ")), ValueKind::Number);
        assert_eq!(coerced.value, number(-2.5));
        assert!(!coerced.defaulted);
    }

    #[test]
    fn a_number_to_a_string_prints_its_magnitude() {
        let coerced = coerce(&number(10.0), ValueKind::Str);
        assert_eq!(coerced.value, Value::Str(String::from("10")));
        assert!(!coerced.defaulted);
    }

    #[test]
    fn a_number_to_a_boolean_is_zero_is_false() {
        assert_eq!(
            coerce(&number(0.0), ValueKind::Bool).value,
            Value::Bool(false)
        );
        assert_eq!(
            coerce(&number(2.5), ValueKind::Bool).value,
            Value::Bool(true)
        );
        assert_eq!(
            coerce(&number(-1.0), ValueKind::Bool).value,
            Value::Bool(true)
        );
    }

    #[test]
    fn a_boolean_to_a_number_is_the_files_zero_one_spelling() {
        let coerced = coerce(&Value::Bool(true), ValueKind::Number);
        assert_eq!(coerced.value, number(1.0));
        assert!(!coerced.defaulted);
        let coerced = coerce(&Value::Bool(false), ValueKind::Number);
        assert_eq!(coerced.value, number(0.0));
        assert!(!coerced.defaulted);
    }

    #[test]
    fn every_kind_to_a_string_uses_its_own_spelling() {
        assert_eq!(
            coerce(&Value::Bool(true), ValueKind::Str).value,
            Value::Str(String::from("TRUE"))
        );
        assert_eq!(
            coerce(&Value::Color(Color::new(0x12, 0x34, 0x56)), ValueKind::Str).value,
            Value::Str(String::from("#123456"))
        );
        assert_eq!(
            coerce(
                &Value::List(vec![number(1.0), Value::Str(String::from("x"))]),
                ValueKind::Str
            )
            .value,
            Value::Str(String::from("1;x"))
        );
    }

    #[test]
    fn unconvertible_pairs_fall_back_to_the_typed_default() {
        let cases = [
            (Value::Str(String::from("sheep")), ValueKind::Number),
            (Value::Str(String::from("sheep")), ValueKind::Bool),
            (Value::Str(String::from("x")), ValueKind::Color),
            (number(1.0), ValueKind::Color),
            (number(1.0), ValueKind::List),
            (Value::Bool(true), ValueKind::Color),
            (Value::Bool(true), ValueKind::List),
            (Value::Color(Color::new(1, 2, 3)), ValueKind::Number),
            (Value::Color(Color::new(1, 2, 3)), ValueKind::Bool),
            (Value::List(Vec::new()), ValueKind::Number),
            (Value::List(Vec::new()), ValueKind::Bool),
            (Value::List(Vec::new()), ValueKind::Color),
        ];
        for (value, kind) in cases {
            let coerced = coerce(&value, kind);
            assert_eq!(coerced.value, typed_default(kind), "{value:?} to {kind:?}");
            assert!(coerced.defaulted, "{value:?} to {kind:?}");
        }
    }

    #[test]
    fn the_conversion_matrix_is_the_documented_one() {
        // The documented conversions: the same kind, number <-> boolean,
        // and every kind to a string through its own spelling.
        fn converts(value: &Value, kind: ValueKind) -> bool {
            kind_matches(value, kind)
                || kind == ValueKind::Str
                || matches!(
                    (value, kind),
                    (Value::Number(_), ValueKind::Bool) | (Value::Bool(_), ValueKind::Number)
                )
        }
        for value in [
            number(3.5),
            Value::Str(String::from("x")),
            Value::Bool(true),
            Value::Color(Color::new(1, 2, 3)),
            Value::List(vec![number(1.0)]),
        ] {
            for kind in [
                ValueKind::Number,
                ValueKind::Str,
                ValueKind::Bool,
                ValueKind::Color,
                ValueKind::List,
            ] {
                let coerced = coerce(&value, kind);
                assert_eq!(
                    !coerced.defaulted,
                    converts(&value, kind),
                    "{value:?} to {kind:?}"
                );
            }
        }
    }

    /// Whether `value` already is of `kind`.
    fn kind_matches(value: &Value, kind: ValueKind) -> bool {
        matches!(
            (value, kind),
            (Value::Number(_), ValueKind::Number)
                | (Value::Str(_), ValueKind::Str)
                | (Value::Bool(_), ValueKind::Bool)
                | (Value::Color(_), ValueKind::Color)
                | (Value::List(_), ValueKind::List)
        )
    }

    #[test]
    fn error_values_propagate_in_every_kind() {
        for kind in [
            ValueKind::Number,
            ValueKind::Str,
            ValueKind::Bool,
            ValueKind::Color,
            ValueKind::List,
        ] {
            let coerced = coerce(&Value::Err(ErrorCode::Div0), kind);
            assert_eq!(coerced.value, Value::Err(ErrorCode::Div0));
            assert!(!coerced.defaulted);
        }
    }

    #[test]
    fn a_non_finite_number_is_num_in_every_kind() {
        for magnitude in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            let value = Value::Number(Number::new(magnitude, Unit::Inches));
            for kind in [ValueKind::Number, ValueKind::Str, ValueKind::Bool] {
                let coerced = coerce(&value, kind);
                assert_eq!(
                    coerced.value,
                    Value::Err(ErrorCode::Num),
                    "{magnitude} to {kind:?}"
                );
                assert!(!coerced.defaulted);
            }
        }
    }

    #[test]
    fn a_numeric_string_that_overflows_is_num() {
        let coerced = coerce(&Value::Str(String::from("1e999")), ValueKind::Number);
        assert_eq!(coerced.value, Value::Err(ErrorCode::Num));
        assert!(!coerced.defaulted);
    }

    #[test]
    fn the_typed_views_succeed_only_on_real_conversions() {
        assert_eq!(
            as_number(&Value::Str(String::from("2"))),
            Ok(Number::new(2.0, Unit::Unitsless))
        );
        assert_eq!(
            as_number(&Value::Str(String::from("sheep"))),
            Err(Error::Conversion("value is not a number"))
        );
        assert_eq!(as_str(&number(10.0)), Ok(String::from("10")));
        assert_eq!(
            as_str(&Value::List(vec![number(1.0)])),
            Ok(String::from("1"))
        );
        assert_eq!(
            as_str(&Value::Err(ErrorCode::Ref)),
            Err(Error::Conversion("value is not a string"))
        );
        assert_eq!(as_bool(&number(0.0)), Ok(false));
        assert_eq!(
            as_bool(&Value::Str(String::from("x"))),
            Err(Error::Conversion("value is not a boolean"))
        );
        assert_eq!(
            as_color(&Value::Color(Color::new(1, 2, 3))),
            Ok(Color::new(1, 2, 3))
        );
        assert_eq!(
            as_color(&number(1.0)),
            Err(Error::Conversion("value is not a colour"))
        );
        assert_eq!(
            as_list(&Value::List(vec![number(1.0)])),
            Ok(vec![number(1.0)])
        );
        assert_eq!(
            as_list(&number(1.0)),
            Err(Error::Conversion("value is not a list"))
        );
    }

    #[test]
    fn the_typed_views_reject_error_values() {
        let error = Value::Err(ErrorCode::Value);
        assert_eq!(
            as_number(&error),
            Err(Error::Conversion("value is not a number"))
        );
        assert_eq!(
            as_str(&error),
            Err(Error::Conversion("value is not a string"))
        );
        assert_eq!(
            as_bool(&error),
            Err(Error::Conversion("value is not a boolean"))
        );
        assert_eq!(
            as_color(&error),
            Err(Error::Conversion("value is not a colour"))
        );
        assert_eq!(
            as_list(&error),
            Err(Error::Conversion("value is not a list"))
        );
    }

    #[test]
    fn the_typed_views_reject_non_finite_numbers() {
        let value = Value::Number(Number::new(f64::INFINITY, Unit::Inches));
        assert_eq!(
            as_number(&value),
            Err(Error::Conversion("value is not a number"))
        );
    }

    #[test]
    fn every_target_kind_exists() {
        let kinds = [
            ValueKind::Number,
            ValueKind::Str,
            ValueKind::Bool,
            ValueKind::Color,
            ValueKind::List,
        ];
        assert_eq!(kinds.len(), 5);
    }

    #[test]
    fn the_kinds_are_distinct() {
        assert_ne!(ValueKind::Number, ValueKind::Str);
        let n = Number::new(1.0, Unit::Unitsless);
        assert_eq!(n.magnitude, 1.0);
    }
}
