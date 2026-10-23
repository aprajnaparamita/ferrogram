//! Formula source text and cell references (ADR 0051).
//!
//! A formula is stored as its source string (ADR 0014); [`crate::parse`]
//! turns one into an [`crate::Expr`] and [`crate::print`] turns an
//! expression back into source text. A [`CellRef`] keeps every part of a
//! reference **as written**: the parser does not decide what `Prop.Cost`
//! or `Scratch.B3` means, the resolver does (ADR 0054), because the rule
//! depends on the section the name points at.

use std::fmt;

/// The source text of a ShapeSheet formula, without the leading `=`.
///
/// Phase 2 stores and writes formulas as text (ADR 0014); [`crate::parse`]
/// turns one into an [`crate::Expr`] once the formula engine lands.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Formula(String);

impl Formula {
    /// Creates a formula from its source text.
    pub fn new(source: impl Into<String>) -> Self {
        Self(source.into())
    }

    /// The source text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Formula {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A cell reference, e.g. `Sheet.4!PinX`, `ThePage!User.Anchor_Point`
/// or `Geometry1.X2`.
///
/// Every part is kept **as written** (ADR 0051): the sheet parts before
/// the `!` separators, the section part before the first `.`, and the
/// dotted name parts after it. Which part names a row, a column or the
/// default cell of a row is a resolution rule (ADR 0054), not a parsing
/// rule: `Char.Size` is the `Size` column of the `Character` section,
/// while `Prop.Cost` is the `Cost` row's default cell.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct CellRef {
    /// The sheet parts before the `!` separators, in order
    /// (`Sheet.4`, `ThePage`, `Wheel`, `Hub`).
    pub sheets: Vec<String>,
    /// The section part (`Geometry1`, `Char`, `Prop`), or `None` for a
    /// plain cell name such as `Width`.
    pub section: Option<String>,
    /// The dotted name parts after the section, in order and as written
    /// (`X2`, `Size`, `Cost`, `Font[3]`, `A5`).
    pub names: Vec<String>,
}

impl CellRef {
    /// Creates a reference to one local cell name (`PinX`).
    pub fn new(cell: impl Into<String>) -> Self {
        Self {
            sheets: Vec::new(),
            section: None,
            names: vec![cell.into()],
        }
    }

    /// The name parts joined as written (`Geometry1.X2`, `PinX`).
    ///
    /// Returns `None` when the reference names no cell at all, which a
    /// parser rejects but the value type can hold.
    pub fn name(&self) -> Option<String> {
        if self.names.is_empty() {
            return None;
        }
        Some(self.names.join("."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formula_holds_its_source() {
        let f = Formula::new("0.5 in");
        assert_eq!(f.as_str(), "0.5 in");
        assert_eq!(f.to_string(), "0.5 in");
    }

    #[test]
    fn cell_ref_is_bare_by_default() {
        let r = CellRef::new("PinX");
        assert_eq!(r.names, vec![String::from("PinX")]);
        assert_eq!(r.sheets, Vec::<String>::new());
        assert_eq!(r.section, None);
        assert_eq!(
            CellRef::default(),
            CellRef {
                sheets: Vec::new(),
                section: None,
                names: Vec::new()
            }
        );
    }

    #[test]
    fn cell_ref_keeps_every_part_as_written() {
        let r = CellRef {
            sheets: vec![String::from("Sheet.4")],
            section: Some(String::from("Geometry1")),
            names: vec![String::from("X2")],
        };
        assert_eq!(r.sheets, vec![String::from("Sheet.4")]);
        assert_eq!(r.section.as_deref(), Some("Geometry1"));
        assert_eq!(r.name().as_deref(), Some("X2"));
    }

    #[test]
    fn a_reference_with_no_names_has_no_name() {
        let r = CellRef::default();
        assert_eq!(r.name(), None);
    }

    #[test]
    fn dotted_names_join_as_written() {
        let r = CellRef {
            sheets: Vec::new(),
            section: Some(String::from("Prop")),
            names: vec![String::from("Cost"), String::from("Value")],
        };
        assert_eq!(r.name().as_deref(), Some("Cost.Value"));
    }
}
