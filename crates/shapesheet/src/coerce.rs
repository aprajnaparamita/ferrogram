//! Coercions between value kinds (ADR 0052).
//!
//! Visio converts an operand to the kind an operator or function needs,
//! and supplies the typed default of the target kind when it cannot
//! (`about-operators.md`: zero for numbers, `FALSE` for booleans, `""`
//! for strings). [`coerce`] is that one rule; the `as_*` readers are the
//! typed views callers use.
//!
//! Task P6-03 implements this module.

use crate::Error;
use crate::value::{Number, Value};
use ferrogram_model::Color;

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

/// Coerces `value` to `kind` (ADR 0052).
///
/// The stub returns [`Error::NotImplemented`] until task P6-03 lands.
pub fn coerce(value: &Value, kind: ValueKind) -> Result<Value, Error> {
    let _ = (value, kind);
    Err(Error::NotImplemented("coerce"))
}

/// Reads `value` as a number.
pub fn as_number(value: &Value) -> Result<Number, Error> {
    let _ = value;
    Err(Error::NotImplemented("as_number"))
}

/// Reads `value` as a string.
pub fn as_str(value: &Value) -> Result<String, Error> {
    let _ = value;
    Err(Error::NotImplemented("as_str"))
}

/// Reads `value` as a boolean.
pub fn as_bool(value: &Value) -> Result<bool, Error> {
    let _ = value;
    Err(Error::NotImplemented("as_bool"))
}

/// Reads `value` as a colour.
pub fn as_color(value: &Value) -> Result<Color, Error> {
    let _ = value;
    Err(Error::NotImplemented("as_color"))
}

/// Reads `value` as a list.
pub fn as_list(value: &Value) -> Result<Vec<Value>, Error> {
    let _ = value;
    Err(Error::NotImplemented("as_list"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Unit;

    #[test]
    fn coercions_are_stubs() {
        let v = Value::Bool(true);
        assert_eq!(
            coerce(&v, ValueKind::Number),
            Err(Error::NotImplemented("coerce"))
        );
        assert_eq!(as_number(&v), Err(Error::NotImplemented("as_number")));
        assert_eq!(as_str(&v), Err(Error::NotImplemented("as_str")));
        assert_eq!(as_bool(&v), Err(Error::NotImplemented("as_bool")));
        assert_eq!(as_color(&v), Err(Error::NotImplemented("as_color")));
        assert_eq!(as_list(&v), Err(Error::NotImplemented("as_list")));
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
