//! ShapeSheet formula parser, evaluator, dependency graph and recalc.
//!
//! Phase 2 stored formulas as source strings in the model (ADR 0014);
//! Phase 6 turns them back into values. The pipeline:
//!
//! 1. [`parse`] turns a [`Formula`] into an [`Expr`] (ADR 0051),
//!    [`print()`] turns one back into source text.
//! 2. [`eval`] computes an expression against an [`EvalContext`], which
//!    resolves references with inheritance (ADR 0054) and applies basic
//!    `SETF` writes (ADR 0056). The result is a [`Value`] (ADR 0052);
//!    evaluation problems are error values and warnings, never failures.
//! 3. [`DepGraph`] answers what a change dirties (ADR 0055) and [`recalc`]
//!    recomputes those cells and their dependents, writing the results
//!    into the model's cached values.
//!
//! The crate never panics on input (AGENTS.md section 6): input length,
//! token count and nesting depth are bounded ([`MAX_SOURCE_LEN`],
//! [`MAX_TOKENS`], [`MAX_NESTING`]) and bad input is an [`Error`] with a
//! [`Span`].
//!
//! Layer L2 (ADR 0002): the engine runs on the document model.
//!
//! Part of Ferrogram. Alpha: function bodies marked "not implemented" are
//! stubs awaiting their implementing task (tasks P6-02..P6-06).

#![forbid(unsafe_code)]
#![deny(clippy::indexing_slicing)]

mod ast;
mod coerce;
mod error;
mod eval;
mod formula;
mod functions;
mod graph;
mod lexer;
mod parser;
mod printer;
mod recalc;
mod resolve;
mod units;
mod value;

pub use ast::{BinaryOp, Expr, ExprKind, Literal, NumberLiteral, UnaryOp};
pub use coerce::{Coerced, ValueKind, as_bool, as_color, as_list, as_number, as_str, coerce};
pub use error::{Error, Span, SyntaxKind};
pub use eval::{EvalContext, EvalWarning, EvalWarningKind, eval, eval_formula};
pub use formula::{CellRef, Formula};
pub use functions::{FUNCTIONS, FunctionDef, FunctionKind, is_supported, lookup_function};
pub use graph::{CellAddress, DepGraph};
pub use lexer::{MAX_NESTING, MAX_SOURCE_LEN, MAX_TOKENS, Token, TokenKind, tokenize};
pub use parser::{parse, parse_formula};
pub use printer::print;
pub use recalc::{
    CellUpdate, RecalcOptions, RecalcOutcome, RecalcWarning, RecalcWarningKind, recalc,
};
pub use resolve::{DocumentResolver, ResolvedCell};
pub use units::{Unit, UnitFamily, parse_unit};
pub use value::{CachedValue, ErrorCode, Number, Value};
