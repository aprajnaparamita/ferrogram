//! Cached-value reading with the Phase 3 default and warning policy
//! (ADR 0021).
//!
//! Phase 3 renders from cached values only: no formula evaluation. Every
//! cell the renderer reads goes through [`CellReader`], which resolves the
//! value through the inheritance lookup (ADR 0013) and, when the cached
//! value is missing, an error or of the wrong kind, substitutes the hard
//! default of the kind and records a [`CellWarning`]. A warning never fails
//! a render.

use crate::cell::{CellValue, Color};
use crate::document::Document;
use crate::error::Error;
use crate::id::{PageId, ShapeId};
use crate::name::CellName;

use ferrogram_geom::Angle;

/// The sheet a cell is read from (ADR 0021).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SheetRef {
    /// A shape's sheet.
    Shape(ShapeId),
    /// A page sheet.
    Page(PageId),
}

/// Why [`CellReader`] substituted a default (ADR 0021).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WarningKind {
    /// No source defines the value.
    Missing,
    /// The cached value is an error, e.g. `"#VALUE!"`.
    ErrorValue(String),
    /// The cached value has another kind than requested.
    KindMismatch,
}

/// One substituted cell value (ADR 0021).
#[derive(Debug, Clone, PartialEq)]
pub struct CellWarning {
    /// The sheet the cell was read from.
    pub sheet: SheetRef,
    /// The cell name.
    pub cell: CellName,
    /// Why the default was substituted.
    pub kind: WarningKind,
    /// The default that was used instead.
    pub substituted: CellValue,
}

/// Reads resolved cell values with the Phase 3 policy (ADR 0021).
///
/// Construct one reader per build (scene, layout) and collect its warnings
/// at the end. The reader borrows the document and changes nothing in it
/// (ADR 0013).
pub struct CellReader<'d> {
    doc: &'d Document,
    warnings: Vec<CellWarning>,
}

impl<'d> CellReader<'d> {
    /// A reader over `doc` with no warnings yet.
    pub fn new(doc: &'d Document) -> Self {
        Self {
            doc,
            warnings: Vec::new(),
        }
    }

    /// Reads a number in canonical units (inches, radians or fraction,
    /// ADR 0011).
    ///
    /// Returns [`Error::NotImplemented`] until the reading work lands
    /// (P3-02).
    pub fn number(&mut self, sheet: SheetRef, cell: &CellName) -> Result<f64, Error> {
        let _ = (sheet, cell);
        Err(Error::NotImplemented)
    }

    /// Reads an angle in radians (ADR 0011).
    ///
    /// Returns [`Error::NotImplemented`] until the reading work lands
    /// (P3-02).
    pub fn angle(&mut self, sheet: SheetRef, cell: &CellName) -> Result<Angle, Error> {
        let _ = (sheet, cell);
        Err(Error::NotImplemented)
    }

    /// Reads a boolean.
    ///
    /// Returns [`Error::NotImplemented`] until the reading work lands
    /// (P3-02).
    pub fn boolean(&mut self, sheet: SheetRef, cell: &CellName) -> Result<bool, Error> {
        let _ = (sheet, cell);
        Err(Error::NotImplemented)
    }

    /// Reads a string.
    ///
    /// Returns [`Error::NotImplemented`] until the reading work lands
    /// (P3-02).
    pub fn text(&mut self, sheet: SheetRef, cell: &CellName) -> Result<String, Error> {
        let _ = (sheet, cell);
        Err(Error::NotImplemented)
    }

    /// Reads a color, resolving palette and document-table indices
    /// (ADR 0019).
    ///
    /// Returns [`Error::NotImplemented`] until the reading work lands
    /// (P3-02).
    pub fn color(&mut self, sheet: SheetRef, cell: &CellName) -> Result<Color, Error> {
        let _ = (sheet, cell);
        Err(Error::NotImplemented)
    }

    /// Reads a numeric list (`LayerMember` and friends) as its numbers.
    ///
    /// Returns [`Error::NotImplemented`] until the reading work lands
    /// (P3-02).
    pub fn list(&mut self, sheet: SheetRef, cell: &CellName) -> Result<Vec<f64>, Error> {
        let _ = (sheet, cell);
        Err(Error::NotImplemented)
    }

    /// Reads a number from one row of a repeated section (ADR 0019).
    ///
    /// `row` is the row's position in the section. Returns
    /// [`Error::NotImplemented`] until the reading work lands (P3-02).
    pub fn row_number(
        &mut self,
        sheet: SheetRef,
        section: &str,
        row: usize,
        cell: &CellName,
    ) -> Result<f64, Error> {
        let _ = (sheet, section, row, cell);
        Err(Error::NotImplemented)
    }

    /// The warnings recorded so far.
    pub fn warnings(&self) -> &[CellWarning] {
        &self.warnings
    }

    /// The document being read.
    pub fn document(&self) -> &'d Document {
        self.doc
    }

    /// Takes the warnings recorded so far, leaving none.
    pub fn take_warnings(&mut self) -> Vec<CellWarning> {
        std::mem::take(&mut self.warnings)
    }
}

/// The fixed Visio palette (indices 0..23), our own transcription of
/// observed Visio output (ADR 0019).
///
/// Returns `None` for an index we do not know. The table lands with the
/// model work (P3-02); until then every index resolves to `None` and the
/// caller substitutes black with a warning (ADR 0021).
pub fn palette(index: u32) -> Option<Color> {
    let _ = index;
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    #[test]
    fn reader_starts_without_warnings() {
        let doc = Document::new();
        let reader = CellReader::new(&doc);
        assert!(reader.warnings().is_empty());
        assert!(std::ptr::eq(reader.document(), &doc));
    }

    #[test]
    fn take_warnings_drains_the_list() {
        let doc = Document::new();
        let mut reader = CellReader::new(&doc);
        reader.warnings.push(CellWarning {
            sheet: SheetRef::Shape(ShapeId { index: 0 }),
            cell: CellName::from_static("PinX"),
            kind: WarningKind::Missing,
            substituted: CellValue::Number(0.0),
        });
        assert_eq!(reader.warnings().len(), 1);
        let taken = reader.take_warnings();
        assert_eq!(taken.len(), 1);
        assert!(reader.warnings().is_empty());
    }

    #[test]
    fn sheet_refs_are_distinct() {
        assert_ne!(
            SheetRef::Shape(ShapeId { index: 1 }),
            SheetRef::Page(PageId { index: 1 })
        );
    }

    #[test]
    fn accessors_are_not_implemented_yet() {
        let doc = Document::new();
        let mut reader = CellReader::new(&doc);
        let sheet = SheetRef::Shape(ShapeId { index: 0 });
        let cell = CellName::from_static("PinX");
        assert_eq!(reader.number(sheet, &cell), Err(Error::NotImplemented));
        assert_eq!(reader.angle(sheet, &cell), Err(Error::NotImplemented));
        assert_eq!(reader.boolean(sheet, &cell), Err(Error::NotImplemented));
        assert_eq!(reader.text(sheet, &cell), Err(Error::NotImplemented));
        assert_eq!(reader.color(sheet, &cell), Err(Error::NotImplemented));
        assert_eq!(reader.list(sheet, &cell), Err(Error::NotImplemented));
        assert_eq!(
            reader.row_number(sheet, "Geometry", 0, &cell),
            Err(Error::NotImplemented)
        );
    }

    #[test]
    fn palette_is_empty_until_the_table_lands() {
        assert_eq!(palette(0), None);
        assert_eq!(palette(23), None);
    }
}
