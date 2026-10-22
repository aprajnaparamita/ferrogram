//! Fuzz target for the formula evaluator (task P6-11, ADR 0060).
//!
//! Any bytes go in and are read as lossy UTF-8 formula source, parsed
//! and then **evaluated** against a context that answers every
//! reference with `#REF!` and refuses no write. The contract (ADR 0052,
//! ADR 0053): a tree the parser accepts evaluates to a value or an
//! error value — never a panic, whatever the references name, whatever
//! the arguments are and whatever the numbers measure. An `Err` is a
//! valid outcome of `parse`; the evaluator's own errors are values.

#![no_main]

use ferrogram_model::{PageId, SheetRef};
use ferrogram_shapesheet::{CellRef, ErrorCode, EvalContext, EvalWarning, Error, Value, eval, parse};
use libfuzzer_sys::fuzz_target;

/// An [`EvalContext`] with no document behind it: every reference is
/// `#REF!` (ADR 0054's missing-reference rule) and every `SETF` write
/// is accepted and dropped.
#[derive(Debug, Default)]
struct NoDocument;

impl EvalContext for NoDocument {
    fn cell_value(&mut self, _sheet: SheetRef, _reference: &CellRef) -> Result<Value, Error> {
        Ok(Value::Err(ErrorCode::Ref))
    }

    fn set_cell(
        &mut self,
        _sheet: SheetRef,
        _reference: &CellRef,
        _value: Value,
    ) -> Result<(), Error> {
        Ok(())
    }

    fn warn(&mut self, _warning: EvalWarning) {}
}

fuzz_target!(|data: &[u8]| {
    let source = String::from_utf8_lossy(data);
    if let Ok(expr) = parse(&source) {
        let mut context = NoDocument;
        let sheet = SheetRef::Page(PageId { index: 0 });
        let _ = eval(&expr, sheet, &mut context);
    }
});
