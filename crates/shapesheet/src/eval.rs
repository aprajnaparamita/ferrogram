//! The evaluator (ADR 0053).
//!
//! [`eval`] computes one expression against an [`EvalContext`], which is
//! the seam for reading and writing cells (ADR 0054) and for recording
//! the structured warnings an unknown function or a missing reference
//! produces. Evaluation never fails the document: a problem is a
//! [`Value::Err`] error value or a warning, never a panic.
//!
//! Task P6-04 implements [`eval`]; task P6-05 provides the model-backed
//! context ([`crate::DocumentResolver`]).

use ferrogram_model::SheetRef;

use crate::Error;
use crate::ast::Expr;
use crate::formula::{CellRef, Formula};
use crate::value::Value;

/// What a formula evaluation warned about (ADR 0053).
#[derive(Debug, Clone, PartialEq)]
pub struct EvalWarning {
    /// The sheet the formula being evaluated lives on.
    pub sheet: SheetRef,
    /// What happened.
    pub kind: EvalWarningKind,
}

/// The kind of an [`EvalWarning`] (ADR 0053).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum EvalWarningKind {
    /// A function the table does not implement; the cell keeps its cached
    /// value.
    UnknownFunction(String),
    /// A reference form the resolver cannot answer; the operand becomes
    /// `#REF!`.
    UnsupportedReference(String),
    /// A value had to be coerced to the type an operator needed and the
    /// conversion was lossy or defaulted (ADR 0052).
    Coercion(String),
    /// A computed number was not finite; it became `#NUM!` (P3-11).
    NonFinite,
    /// A dependency cycle was cut (ADR 0055).
    Cycle,
}

/// The cells an expression reads and writes while it evaluates (ADR 0054).
///
/// References are resolved with inheritance (ADR 0013): a formula a
/// master supplies evaluates against the sheet it is evaluated *for*.
pub trait EvalContext {
    /// The value of one referenced cell, resolved through the rungs of
    /// ADR 0013 and evaluated when its formula defines one.
    fn cell_value(&mut self, sheet: SheetRef, reference: &CellRef) -> Result<Value, Error>;

    /// Writes one referenced cell (basic `SETF`, ADR 0056).
    fn set_cell(&mut self, sheet: SheetRef, reference: &CellRef, value: Value)
    -> Result<(), Error>;

    /// Records one structured warning.
    fn warn(&mut self, warning: EvalWarning);
}

/// Evaluates one expression on `sheet`.
///
/// The stub returns [`Error::NotImplemented`] until task P6-04 lands.
/// Evaluation problems become [`Value::Err`] values; `Err` is reserved
/// for a broken context.
pub fn eval(expr: &Expr, sheet: SheetRef, ctx: &mut dyn EvalContext) -> Result<Value, Error> {
    let _ = (expr, sheet, ctx);
    Err(Error::NotImplemented("eval"))
}

/// Parses and evaluates one formula source on `sheet`.
///
/// The stub returns [`Error::NotImplemented`] until tasks P6-02 and
/// P6-04 land.
pub fn eval_formula(
    formula: &Formula,
    sheet: SheetRef,
    ctx: &mut dyn EvalContext,
) -> Result<Value, Error> {
    let _ = (formula, sheet, ctx);
    Err(Error::NotImplemented("eval_formula"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Span;
    use crate::ast::{ExprKind, Literal};

    fn sheet() -> SheetRef {
        SheetRef::Page(ferrogram_model::PageId { index: 0 })
    }

    struct NullContext;

    impl EvalContext for NullContext {
        fn cell_value(&mut self, _sheet: SheetRef, _reference: &CellRef) -> Result<Value, Error> {
            Err(Error::NotImplemented("EvalContext::cell_value"))
        }

        fn set_cell(
            &mut self,
            _sheet: SheetRef,
            _reference: &CellRef,
            _value: Value,
        ) -> Result<(), Error> {
            Err(Error::NotImplemented("EvalContext::set_cell"))
        }

        fn warn(&mut self, _warning: EvalWarning) {}
    }

    #[test]
    fn eval_is_a_stub() {
        let expr = Expr {
            span: Span::new(0, 1),
            kind: ExprKind::Literal(Literal::Bool(true)),
        };
        let mut ctx = NullContext;
        assert_eq!(
            eval(&expr, sheet(), &mut ctx),
            Err(Error::NotImplemented("eval"))
        );
    }

    #[test]
    fn eval_formula_is_a_stub() {
        let mut ctx = NullContext;
        assert_eq!(
            eval_formula(&Formula::new("1"), sheet(), &mut ctx),
            Err(Error::NotImplemented("eval_formula"))
        );
    }

    #[test]
    fn a_context_reports_unimplemented_access() {
        let mut ctx = NullContext;
        let r = CellRef::new("PinX");
        let sheet = sheet();
        assert_eq!(
            ctx.cell_value(sheet, &r),
            Err(Error::NotImplemented("EvalContext::cell_value"))
        );
        assert_eq!(
            ctx.set_cell(sheet, &r, Value::Bool(false)),
            Err(Error::NotImplemented("EvalContext::set_cell"))
        );
        // The warning sink records; a null context never warns by itself.
        ctx.warn(EvalWarning {
            sheet,
            kind: EvalWarningKind::UnknownFunction(String::from("THEMEVAL")),
        });
    }

    #[test]
    fn warning_kinds_exist() {
        let w = EvalWarning {
            sheet: sheet(),
            kind: EvalWarningKind::NonFinite,
        };
        assert_eq!(w.kind, EvalWarningKind::NonFinite);
        assert_ne!(EvalWarningKind::Cycle, EvalWarningKind::NonFinite);
    }
}
