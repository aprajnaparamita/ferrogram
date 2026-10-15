//! ShapeSheet sections and rows.

use crate::cell::Cell;
use crate::name::SectionName;
use crate::raw::Preserved;

/// A ShapeSheet section: one named block of rows (ADR 0011).
///
/// A section can also carry `Cell` children outside its rows (`Geometry`
/// writes `NoFill`, `NoLine`, `NoShow`, `NoSnap` and `NoQuickDrag` there);
/// they are the `cells`, and a lookup matches them before the row cells.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// The section name, e.g. `"Geometry"` or `"Shape Transform"`.
    pub name: SectionName,
    /// The cells written outside the rows (`Cell` children of the `Section`
    /// element), in file order.
    pub cells: Vec<Cell>,
    /// The rows, in file order.
    pub rows: Vec<Row>,
    /// Unmapped attributes and children kept verbatim (ADR 0012).
    pub preserved: Preserved,
}

impl Section {
    /// Creates an empty section with the given name.
    pub fn new(name: SectionName) -> Self {
        Self {
            name,
            cells: Vec::new(),
            rows: Vec::new(),
            preserved: Preserved::new(),
        }
    }
}

/// A ShapeSheet row: one line of cells inside a section.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// The row name (`Row/@N`), e.g. `"Row_1"`, or `None`.
    pub name: Option<String>,
    /// The row type (`Row/@T`), e.g. `"MoveTo"`, or `None`.
    pub row_type: Option<String>,
    /// The row index (`Row/@IX`), or `None`.
    pub ix: Option<i32>,
    /// The cells, in file order.
    pub cells: Vec<Cell>,
    /// Unmapped attributes and children kept verbatim (ADR 0012).
    pub preserved: Preserved,
}

impl Row {
    /// Creates an empty row with no name, type or index.
    pub fn new() -> Self {
        Self {
            name: None,
            row_type: None,
            ix: None,
            cells: Vec::new(),
            preserved: Preserved::new(),
        }
    }

    /// Whether the row is deleted (`Row/@Del`), kept verbatim in
    /// [`Row::preserved`] because the read side maps only `N`, `T` and
    /// `IX`.
    ///
    /// A deleted row removes the inherited row of the same `IX` from the
    /// row union of a shape and its master chain (ADR 0019 amendment).
    pub fn is_deleted(&self) -> bool {
        self.preserved.attributes.iter().any(|attribute| {
            attribute.name.local == "Del" && matches!(attribute.value.trim(), "1" | "true" | "TRUE")
        })
    }
}

impl Default for Row {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::name::CellName;

    #[test]
    fn new_section_is_empty() {
        let s = Section::new(SectionName::from_static("Geometry"));
        assert_eq!(s.name, SectionName::from_static("Geometry"));
        assert!(s.cells.is_empty());
        assert!(s.rows.is_empty());
        assert!(s.preserved.children.is_empty());
    }

    #[test]
    fn section_holds_its_level_cells() {
        let mut s = Section::new(SectionName::from_static("Geometry"));
        s.cells.push(Cell::new(CellName::from_static("NoFill")));
        assert_eq!(s.cells.len(), 1);
        assert_eq!(s.cells[0].name, CellName::from_static("NoFill"));
    }

    #[test]
    fn new_row_has_no_name_type_or_index() {
        let r = Row::new();
        assert_eq!(r.name, None);
        assert_eq!(r.row_type, None);
        assert_eq!(r.ix, None);
        assert!(r.cells.is_empty());
        assert!(r.preserved.attributes.is_empty());
        assert_eq!(Row::default(), Row::new());
    }

    #[test]
    fn row_holds_its_cells() {
        let mut r = Row::new();
        r.row_type = Some("MoveTo".to_owned());
        r.ix = Some(1);
        r.cells.push(Cell::new(CellName::from_static("X")));
        assert_eq!(r.row_type.as_deref(), Some("MoveTo"));
        assert_eq!(r.cells.len(), 1);
    }
}
