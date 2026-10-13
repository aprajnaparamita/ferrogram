//! ShapeSheet formula lexer, parser, evaluator, dependency graph and recalc.
//!
//! Phase 2 stores formulas as source strings in the model (ADR 0014); this
//! crate's Phase 2 work is only the formula text type and the cell reference
//! type the parser will produce. Parsing and evaluation land with the formula
//! engine in a later phase.
//!
//! Part of Ferrogram. Alpha: function bodies here are `Err` stubs until their
//! implementing task (task P2-01 defines the signatures).

#![forbid(unsafe_code)]
#![deny(clippy::indexing_slicing)]

mod error;
mod formula;

pub use error::Error;
pub use formula::{CellRef, Expr, Formula, parse};
