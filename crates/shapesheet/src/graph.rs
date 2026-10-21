//! The dependency graph and recalc order (ADR 0055).
//!
//! One node is one cell ([`CellAddress`]); a formula's references are its
//! dependencies. The graph answers "what must be recomputed after this
//! change" ([`DepGraph::evaluation_order`]) and is where a reference
//! cycle is detected. The recalc pass itself is [`crate::recalc`].
//!
//! Task P6-06 implements this module.

use ferrogram_model::{CellName, Document, RowTarget, SectionName, SheetRef};

use crate::Error;

/// The address of one cell: the sheet, the section and row it lives in
/// and its name (ADR 0055).
///
/// A row is named the way the model names it ([`RowTarget`]): by `IX` or
/// by position, because a formula can address a row either way. The type
/// is `Clone`, not `Copy` ([`CellName`] holds a `Cow`); the graph may add
/// hashing when it needs map keys (task P6-06).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellAddress {
    /// The sheet the cell is on.
    pub sheet: SheetRef,
    /// The section, or `None` for a sheet's direct cells.
    pub section: Option<SectionName>,
    /// The row in a repeated section, or `None`.
    pub row: Option<RowTarget>,
    /// The cell name.
    pub cell: CellName,
}

impl CellAddress {
    /// The cell `cell` on `sheet`, outside any section.
    pub const fn new(sheet: SheetRef, cell: CellName) -> Self {
        Self {
            sheet,
            section: None,
            row: None,
            cell,
        }
    }

    /// The same cell in `section`, in `row`.
    pub const fn in_row(
        sheet: SheetRef,
        section: SectionName,
        row: RowTarget,
        cell: CellName,
    ) -> Self {
        Self {
            sheet,
            section: Some(section),
            row: Some(row),
            cell,
        }
    }
}

/// The dependency graph of a document's formulas (ADR 0055).
#[derive(Debug, Clone, PartialEq)]
pub struct DepGraph {
    _private: (),
}

impl Default for DepGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl DepGraph {
    /// An empty graph; [`DepGraph::build`] fills one in from a document.
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Builds the graph for `document`.
    ///
    /// The stub returns [`Error::NotImplemented`] until task P6-06 lands.
    pub fn build(document: &Document) -> Result<DepGraph, Error> {
        let _ = document;
        Err(Error::NotImplemented("DepGraph::build"))
    }

    /// The cells `cell`'s formula reads.
    ///
    /// The stub returns [`Error::NotImplemented`] until task P6-06 lands.
    pub fn dependencies(&self, cell: &CellAddress) -> Result<Vec<CellAddress>, Error> {
        let _ = cell;
        Err(Error::NotImplemented("DepGraph::dependencies"))
    }

    /// The cells whose formulas read `cell`.
    ///
    /// The stub returns [`Error::NotImplemented`] until task P6-06 lands.
    pub fn dependents(&self, cell: &CellAddress) -> Result<Vec<CellAddress>, Error> {
        let _ = cell;
        Err(Error::NotImplemented("DepGraph::dependents"))
    }

    /// The order to evaluate `dirty` and their transitive dependents in
    /// (dependencies first, ADR 0055).
    ///
    /// The stub returns [`Error::NotImplemented`] until task P6-06 lands.
    pub fn evaluation_order(&self, dirty: &[CellAddress]) -> Result<Vec<CellAddress>, Error> {
        let _ = dirty;
        Err(Error::NotImplemented("DepGraph::evaluation_order"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrogram_model::PageId;

    fn address() -> CellAddress {
        CellAddress::new(
            SheetRef::Page(PageId { index: 0 }),
            CellName::from_static("PinX"),
        )
    }

    #[test]
    fn an_address_holds_its_parts() {
        let a = address();
        assert_eq!(a.cell.as_str(), "PinX");
        assert_eq!(a.section, None);
        assert_eq!(a.row, None);
    }

    #[test]
    fn an_address_can_name_a_row_cell() {
        let a = CellAddress::in_row(
            SheetRef::Page(PageId { index: 0 }),
            SectionName::from_static("Geometry"),
            RowTarget::at(1),
            CellName::from_static("X"),
        );
        assert_eq!(a.section, Some(SectionName::from_static("Geometry")));
        assert_eq!(a.row, Some(RowTarget::at(1)));
        assert_eq!(a.cell.as_str(), "X");
    }

    #[test]
    fn graph_operations_are_stubs() {
        let g = DepGraph::new();
        assert_eq!(DepGraph::default(), g);
        let doc = Document::new();
        assert_eq!(
            DepGraph::build(&doc),
            Err(Error::NotImplemented("DepGraph::build"))
        );
        assert_eq!(
            g.dependencies(&address()),
            Err(Error::NotImplemented("DepGraph::dependencies"))
        );
        assert_eq!(
            g.dependents(&address()),
            Err(Error::NotImplemented("DepGraph::dependents"))
        );
        assert_eq!(
            g.evaluation_order(&[address()]),
            Err(Error::NotImplemented("DepGraph::evaluation_order"))
        );
    }
}
