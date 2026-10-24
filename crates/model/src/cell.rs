//! ShapeSheet cells: formula, cached value and unit (ADR 0011).

use larrygram_geom::Unit;

use crate::name::CellName;
use crate::raw::Preserved;

/// An RGB color as stored in a cached color value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    /// Red, 0 to 255.
    pub r: u8,
    /// Green, 0 to 255.
    pub g: u8,
    /// Blue, 0 to 255.
    pub b: u8,
}

impl Color {
    /// Creates a color from its red, green and blue components.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

/// The cached value of a ShapeSheet cell.
///
/// Numbers are in canonical units (ADR 0011): inches for lengths, radians for
/// angles, fraction for percentages. The unit the file wrote is kept on the
/// cell.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum CellValue {
    /// A numeric value, in canonical units.
    Number(f64),
    /// A string value.
    Str(String),
    /// A boolean value.
    Bool(bool),
    /// A color value.
    Color(Color),
    /// A list value, kept as its raw string.
    List(String),
    /// A cached error value, kept as its raw string (e.g. `"#VALUE!"`).
    Error(String),
}

/// A ShapeSheet cell: its formula, its cached value and its unit, kept
/// separately (ADR 0011).
///
/// Phase 2 stores the formula as its source string; formula parsing belongs
/// to `larrygram-shapesheet` (ADR 0014).
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// The cell name, e.g. `"PinX"`.
    pub name: CellName,
    /// The formula source text, without the leading `=`, or `None` when the
    /// cell has no formula (a literal value).
    pub formula: Option<String>,
    /// The value cached in the file, in canonical units, or `None` when the
    /// file cached nothing.
    pub value: Option<CellValue>,
    /// The unit the file's cached value was written in.
    pub unit: Unit,
    /// Unmapped attributes and children of the `Cell` element kept verbatim
    /// (ADR 0012).
    pub preserved: Preserved,
}

impl Cell {
    /// Creates a cell with a name and no formula, value or unit.
    pub fn new(name: CellName) -> Self {
        Self {
            name,
            formula: None,
            value: None,
            unit: Unit::Unitsless,
            preserved: Preserved::new(),
        }
    }

    /// Replaces the formula and returns the previous one.
    ///
    /// Returning the previous value is what makes undo possible in Phase 4
    /// (ADR 0011): a command records the returned value as its inverse.
    pub fn set_formula(&mut self, formula: Option<String>) -> Option<String> {
        std::mem::replace(&mut self.formula, formula)
    }

    /// Replaces the cached value and returns the previous one.
    pub fn set_value(&mut self, value: Option<CellValue>) -> Option<CellValue> {
        std::mem::replace(&mut self.value, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_constructor_holds_its_components() {
        let c = Color::new(1, 2, 3);
        assert_eq!((c.r, c.g, c.b), (1, 2, 3));
    }

    #[test]
    fn new_cell_is_empty() {
        let c = Cell::new(CellName::from_static("PinX"));
        assert_eq!(c.name, CellName::from_static("PinX"));
        assert_eq!(c.formula, None);
        assert_eq!(c.value, None);
        assert_eq!(c.unit, Unit::Unitsless);
    }

    #[test]
    fn set_formula_returns_the_previous_formula() {
        let mut c = Cell::new(CellName::from_static("PinX"));
        assert_eq!(c.set_formula(Some("0".to_owned())), None);
        assert_eq!(c.set_formula(Some("1".to_owned())), Some("0".to_owned()));
        assert_eq!(c.set_formula(None), Some("1".to_owned()));
    }

    #[test]
    fn set_value_returns_the_previous_value() {
        let mut c = Cell::new(CellName::from_static("PinX"));
        assert_eq!(c.set_value(Some(CellValue::Number(1.0))), None);
        assert_eq!(
            c.set_value(Some(CellValue::Bool(true))),
            Some(CellValue::Number(1.0))
        );
        assert_eq!(c.set_value(None), Some(CellValue::Bool(true)));
    }

    #[test]
    fn cell_values_compare_by_value() {
        assert_eq!(CellValue::Number(1.0), CellValue::Number(1.0));
        assert_ne!(CellValue::Number(1.0), CellValue::Str("x".to_owned()));
    }
}
