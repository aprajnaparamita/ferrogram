//! The recalculation pass (ADR 0055).
//!
//! [`recalc`] recomputes the cells a change dirtied and their transitive
//! dependents, and reports what it changed ([`CellUpdate`]) and what it
//! could not ([`RecalcWarning`]): an unknown function keeps the cached
//! value, a cut cycle keeps the values it found, a budget run-out leaves
//! the rest stale. A recalc never fails the document and never panics.
//!
//! Task P6-06 implements [`recalc`]; task P6-07 hooks it into the edit
//! seam (`ferrogram-commands`).

use ferrogram_model::{CellValue, Document};

use crate::Error;
use crate::graph::CellAddress;

/// How hard a recalc pass may work (ADR 0055).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecalcOptions {
    /// The most cell evaluations one pass may run. Exceeding it sets
    /// [`RecalcOutcome::truncated`] and leaves the rest stale.
    pub max_evaluations: u32,
    /// When true, a cell that inherits its formula gets its own copy of
    /// the evaluated formula and value (ADR 0056, resize).
    pub materialise_inherited: bool,
    /// When true, the pass recomputes only cells that have a formula and
    /// no cached value (ADR 0057, the fill-in pass after a read).
    pub fill_missing_only: bool,
}

impl RecalcOptions {
    /// The default budget of one pass (ADR 0055).
    pub const DEFAULT_MAX_EVALUATIONS: u32 = 100_000;

    /// Options with the default budget and no materialisation.
    pub const fn new() -> Self {
        Self {
            max_evaluations: Self::DEFAULT_MAX_EVALUATIONS,
            materialise_inherited: false,
            fill_missing_only: false,
        }
    }
}

impl Default for RecalcOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// One cell a recalc pass changed (ADR 0055).
#[derive(Debug, Clone, PartialEq)]
pub struct CellUpdate {
    /// The cell that was recomputed.
    pub target: CellAddress,
    /// The cached value before the pass, or `None`.
    pub before: Option<CellValue>,
    /// The cached value after the pass, or `None` when the cell keeps no
    /// value (an error value stays a value).
    pub after: Option<CellValue>,
}

/// What a recalc pass could not do (ADR 0053, ADR 0055).
#[derive(Debug, Clone, PartialEq)]
pub struct RecalcWarning {
    /// The cell the warning is about, or `None` for a whole-pass warning.
    pub target: Option<CellAddress>,
    /// What happened.
    pub kind: RecalcWarningKind,
}

/// The kind of a [`RecalcWarning`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum RecalcWarningKind {
    /// A function the table does not implement; the cell keeps its cached
    /// value.
    UnknownFunction(String),
    /// A reference the resolver could not answer; the cell evaluated to
    /// `#REF!`.
    MissingReference(String),
    /// A dependency cycle: the cells in it keep their cached values.
    Cycle(Vec<CellAddress>),
    /// The pass ran out of its evaluation budget and stopped.
    BudgetExceeded,
}

/// What one recalc pass did (ADR 0055).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RecalcOutcome {
    /// The cells the pass changed, in evaluation order.
    pub updates: Vec<CellUpdate>,
    /// What the pass could not do.
    pub warnings: Vec<RecalcWarning>,
    /// True when the pass stopped at its budget and left cells stale.
    pub truncated: bool,
}

/// Recalculates `dirty` and their transitive dependents in `document`.
///
/// The stub returns [`Error::NotImplemented`] until task P6-06 lands.
pub fn recalc(
    document: &mut Document,
    dirty: &[CellAddress],
    options: &RecalcOptions,
) -> Result<RecalcOutcome, Error> {
    let _ = (document, dirty, options);
    Err(Error::NotImplemented("recalc"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrogram_model::{CellName, PageId, SheetRef};

    fn address() -> CellAddress {
        CellAddress::new(
            SheetRef::Page(PageId { index: 0 }),
            CellName::from_static("Width"),
        )
    }

    #[test]
    fn options_carry_the_default_budget() {
        let o = RecalcOptions::new();
        assert_eq!(o.max_evaluations, RecalcOptions::DEFAULT_MAX_EVALUATIONS);
        assert!(!o.materialise_inherited);
        assert!(!o.fill_missing_only);
        assert_eq!(RecalcOptions::default(), o);
    }

    #[test]
    fn recalc_is_a_stub() {
        let mut doc = Document::new();
        assert_eq!(
            recalc(&mut doc, &[address()], &RecalcOptions::new()),
            Err(Error::NotImplemented("recalc"))
        );
    }

    #[test]
    fn an_outcome_reports_updates_warnings_and_truncation() {
        let outcome = RecalcOutcome {
            updates: vec![CellUpdate {
                target: address(),
                before: None,
                after: None,
            }],
            warnings: vec![RecalcWarning {
                target: Some(address()),
                kind: RecalcWarningKind::BudgetExceeded,
            }],
            truncated: true,
        };
        assert_eq!(outcome.updates.len(), 1);
        assert_eq!(outcome.warnings.len(), 1);
        assert!(outcome.truncated);
        assert_eq!(
            RecalcOutcome::default(),
            RecalcOutcome {
                updates: Vec::new(),
                warnings: Vec::new(),
                truncated: false,
            }
        );
    }

    #[test]
    fn warning_kinds_are_named() {
        let kinds = [
            RecalcWarningKind::UnknownFunction(String::from("THEMEVAL")),
            RecalcWarningKind::MissingReference(String::from("Sheet.9!PinX")),
            RecalcWarningKind::Cycle(vec![address()]),
            RecalcWarningKind::BudgetExceeded,
        ];
        assert_eq!(kinds.len(), 4);
    }
}
