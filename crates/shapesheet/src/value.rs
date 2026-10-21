//! Evaluated values (ADR 0052).
//!
//! A formula evaluates to a [`Value`]: a number with a unit, a string, a
//! boolean, a colour, a list, or an **error value** such as `#VALUE!`
//! ([`ErrorCode`], "About Error Values"). Evaluation problems are values,
//! never Rust errors and never panics.
//!
//! The model stores a cached value and a unit spelling per cell (ADR
//! 0011): [`Value::to_cached`] and [`Value::from_cached`] are the bridge
//! (P2-14). Task P6-03 implements them.

use ferrogram_model::{CellValue, Color};

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
    /// `#VALUE!`, an argument or operand of the wrong type.
    Value,
    /// `#REF!`, a reference to a cell that does not exist.
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
    pub unit: ferrogram_geom::Unit,
}

impl Value {
    /// Maps an evaluated value to the model's cached form (P2-14).
    ///
    /// The stub returns [`Error::NotImplemented`] until task P6-03 lands.
    pub fn to_cached(&self) -> Result<CachedValue, Error> {
        let _ = self;
        Err(Error::NotImplemented("Value::to_cached"))
    }

    /// Maps a model cached value and its unit to an evaluated value
    /// (P2-14).
    ///
    /// The stub returns [`Error::NotImplemented`] until task P6-03 lands.
    pub fn from_cached(value: &CellValue, unit: ferrogram_geom::Unit) -> Result<Value, Error> {
        let _ = (value, unit);
        Err(Error::NotImplemented("Value::from_cached"))
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
    fn the_value_mapping_is_a_stub() {
        assert_eq!(
            Value::Bool(false).to_cached(),
            Err(Error::NotImplemented("Value::to_cached"))
        );
        assert_eq!(
            Value::from_cached(&CellValue::Number(1.0), ferrogram_geom::Unit::Inches),
            Err(Error::NotImplemented("Value::from_cached"))
        );
    }
}
