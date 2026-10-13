//! Formula source text and cell references.

use std::fmt;

/// The source text of a ShapeSheet formula, without the leading `=`.
///
/// Phase 2 stores and writes formulas as text (ADR 0014); [`parse`] turns one
/// into an [`Expr`] once the formula engine lands.
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

/// A reference to one cell, e.g. `Sheet.4!PinX` or `Char.Size`.
///
/// Every part is kept as written; the parser resolves sheet names, section
/// names and row names against the document.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct CellRef {
    /// The sheet part (`Sheet.4`), or `None` for the current sheet.
    pub sheet: Option<String>,
    /// The section part (`Geometry`), or `None`.
    pub section: Option<String>,
    /// The row part (`Row_2`), or `None`.
    pub row: Option<String>,
    /// The cell name (`PinX`).
    pub cell: String,
}

impl CellRef {
    /// Creates a cell reference to `cell` with no sheet, section or row.
    pub fn new(cell: impl Into<String>) -> Self {
        Self {
            sheet: None,
            section: None,
            row: None,
            cell: cell.into(),
        }
    }
}

/// A parsed formula (opaque until the formula engine lands).
///
/// The AST shape is decided by the task that implements [`parse`]; nothing
/// outside this crate may match on it before then.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    _inner: (),
}

/// Parses one formula source text into an expression tree.
pub fn parse(source: &str) -> Result<Expr, crate::Error> {
    let _ = source;
    Err(crate::Error::NotImplemented("parse"))
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
        assert_eq!(r.cell, "PinX");
        assert_eq!(r.sheet, None);
        assert_eq!(r.section, None);
        assert_eq!(r.row, None);
        assert_eq!(CellRef::default(), CellRef::new(""));
    }

    #[test]
    fn parse_is_a_stub() {
        assert_eq!(parse("1+1"), Err(crate::Error::NotImplemented("parse")));
    }
}
