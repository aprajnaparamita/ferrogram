//! The recalc hook after an edit (ADR 0056).
//!
//! Every document change goes through [`AppCtx::push_edit`] (ADR 0040).
//! Phase 6 adds one step to that seam: recalculate the cells the edit
//! dirtied and their dependents (ADR 0055), and record the changed cells
//! in the same undo step so undo restores the old cached values.
//! Recalc never opens a second undo path and never fails the document:
//! a cell it cannot compute keeps its cached value and a warning is
//! recorded ([`RecalcReport`]).
//!
//! Task P6-07 implements this module.

use ferrogram_shapesheet::{CellAddress, RecalcWarning};

use crate::app::{AppCtx, SessionId};
use crate::error::CommandError;

/// What one recalc pass changed (ADR 0056).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RecalcReport {
    /// How many cells got a new cached value.
    pub updated: usize,
    /// What the pass could not do (unknown functions, cycles, budget).
    pub warnings: Vec<RecalcWarning>,
    /// True when the pass stopped at its budget and left cells stale.
    pub truncated: bool,
}

/// Recalculates the cells an edit dirtied and their dependents.
///
/// Runs after the edit's cells are written and before the undo step is
/// pushed; `session` names the document the edit changed. The stub
/// returns [`CommandError::Unsupported`] until task P6-07 lands.
pub fn recalc_after_edit(
    ctx: &mut AppCtx,
    session: SessionId,
    dirty: &[CellAddress],
) -> Result<RecalcReport, CommandError> {
    let _ = (ctx, session, dirty);
    Err(CommandError::Unsupported {
        id: String::from("recalc_after_edit"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{DocumentSession, Settings};
    use ferrogram_model::{CellName, Document, PageId, SheetRef};

    fn session(ctx: &mut AppCtx) -> SessionId {
        let session = DocumentSession::new(Document::new(), None);
        let id = session.id;
        ctx.open_document(session);
        id
    }

    #[test]
    fn recalc_after_edit_is_a_stub() {
        let mut ctx = AppCtx::new(Settings::default());
        let id = session(&mut ctx);
        let dirty = [CellAddress::new(
            SheetRef::Page(PageId { index: 0 }),
            CellName::from_static("Width"),
        )];
        assert_eq!(
            recalc_after_edit(&mut ctx, id, &dirty),
            Err(CommandError::Unsupported {
                id: String::from("recalc_after_edit")
            })
        );
    }

    #[test]
    fn a_report_starts_empty() {
        let report = RecalcReport::default();
        assert_eq!(report.updated, 0);
        assert!(report.warnings.is_empty());
        assert!(!report.truncated);
    }

    #[test]
    fn an_edit_reports_an_empty_dirty_set_by_default() {
        use crate::edit::CompositeEdit;
        use crate::undo::Edit;

        let edit = CompositeEdit::new("label");
        assert!(edit.dirty_cells().is_empty());
    }
}
