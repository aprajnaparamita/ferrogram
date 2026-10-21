//! Reference resolution and inheritance (ADR 0054).
//!
//! A formula's references resolve through the rungs of ADR 0013 (local,
//! master, style chain, section default, hard default), and an inherited
//! formula is evaluated against the sheet it is evaluated *for*: an
//! instance's `Width` in a master's geometry row is the instance's
//! width. [`ResolvedCell`] names where the formula and the cached value
//! came from, so the evaluator and the recalc pass can report provenance.
//!
//! Task P6-05 implements this module over `ferrogram_model`.

use ferrogram_model::{Document, SheetRef, ValueSource};

use crate::Error;
use crate::eval::{EvalContext, EvalWarning};
use crate::formula::CellRef;
use crate::value::Value;

/// One resolved cell: the formula that defines it and the value to use
/// (ADR 0054).
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedCell {
    /// The defining formula's source text, or `None` when no rung has one
    /// (a literal or an absent cell).
    pub formula: Option<String>,
    /// The value to use: the cached value of the defining rung, or the
    /// local snapshot of an `Inh` cell (ADR 0054 amendment), or `None`.
    pub value: Option<Value>,
    /// Which rung defines the cell.
    pub source: ValueSource,
}

/// Resolves references against a [`Document`] (ADR 0054).
///
/// The resolver is the [`EvalContext`] the evaluator runs against: it
/// reads cells with inheritance, applies basic `SETF` writes and collects
/// the structured warnings of ADR 0053.
pub struct DocumentResolver<'d> {
    document: &'d mut Document,
    warnings: Vec<EvalWarning>,
}

impl<'d> DocumentResolver<'d> {
    /// A resolver over `document`.
    pub fn new(document: &'d mut Document) -> Self {
        Self {
            document,
            warnings: Vec::new(),
        }
    }

    /// The document the resolver reads.
    pub fn document(&self) -> &Document {
        self.document
    }

    /// The warnings collected so far.
    pub fn warnings(&self) -> &[EvalWarning] {
        &self.warnings
    }

    /// The collected warnings, taking them out of the resolver.
    pub fn into_warnings(self) -> Vec<EvalWarning> {
        self.warnings
    }

    /// Resolves one referenced cell (ADR 0054).
    ///
    /// The stub returns [`Error::NotImplemented`] until task P6-05 lands.
    pub fn resolve(&mut self, sheet: SheetRef, reference: &CellRef) -> Result<ResolvedCell, Error> {
        let _ = (sheet, reference);
        Err(Error::NotImplemented("DocumentResolver::resolve"))
    }
}

impl EvalContext for DocumentResolver<'_> {
    fn cell_value(&mut self, sheet: SheetRef, reference: &CellRef) -> Result<Value, Error> {
        let _ = (sheet, reference);
        Err(Error::NotImplemented("DocumentResolver::cell_value"))
    }

    fn set_cell(
        &mut self,
        sheet: SheetRef,
        reference: &CellRef,
        value: Value,
    ) -> Result<(), Error> {
        let _ = (sheet, reference, value);
        Err(Error::NotImplemented("DocumentResolver::set_cell"))
    }

    fn warn(&mut self, warning: EvalWarning) {
        self.warnings.push(warning);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::EvalWarningKind;
    use ferrogram_model::PageId;

    fn sheet() -> SheetRef {
        SheetRef::Page(PageId { index: 0 })
    }

    #[test]
    fn a_resolver_holds_its_document_and_warnings() {
        let mut doc = Document::new();
        let mut resolver = DocumentResolver::new(&mut doc);
        assert!(resolver.warnings().is_empty());
        resolver.warn(EvalWarning {
            sheet: sheet(),
            kind: EvalWarningKind::Cycle,
        });
        assert_eq!(resolver.warnings().len(), 1);
        assert_eq!(resolver.into_warnings().len(), 1);
    }

    #[test]
    fn the_document_is_borrowable() {
        let mut doc = Document::new();
        let resolver = DocumentResolver::new(&mut doc);
        assert_eq!(resolver.document().pages.len(), 0);
    }

    #[test]
    fn resolution_is_a_stub() {
        let mut doc = Document::new();
        let mut resolver = DocumentResolver::new(&mut doc);
        assert_eq!(
            resolver.resolve(sheet(), &CellRef::new("PinX")),
            Err(Error::NotImplemented("DocumentResolver::resolve"))
        );
    }

    #[test]
    fn cell_access_is_a_stub() {
        let mut doc = Document::new();
        let mut resolver = DocumentResolver::new(&mut doc);
        let r = CellRef::new("PinX");
        assert_eq!(
            resolver.cell_value(sheet(), &r),
            Err(Error::NotImplemented("DocumentResolver::cell_value"))
        );
        assert_eq!(
            resolver.set_cell(sheet(), &r, Value::Bool(true)),
            Err(Error::NotImplemented("DocumentResolver::set_cell"))
        );
    }

    #[test]
    fn a_resolved_cell_holds_provenance() {
        let cell = ResolvedCell {
            formula: None,
            value: None,
            source: ValueSource::Local,
        };
        assert_eq!(cell.source, ValueSource::Local);
    }
}
