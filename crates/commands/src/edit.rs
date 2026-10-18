//! Composite edits: one drag is one undo step (ADR 0040).
//!
//! A command that builds its undo step out of several smaller edits (a
//! multi-shape move, a group, a paste) wraps them in [`CompositeEdit`]:
//! undo reverts them in reverse order and redo applies them in order. The
//! individual edit types (`CellEdit`, `StructuralEdit`) are task P5-04's
//! implementation detail and stay in `crates/commands::builtins::edit`.
//!
//! Task P5-04 implements the bodies. The bodies are
//! [`CommandError::failed`] "not implemented" errors until then.

use crate::app::AppCtx;
use crate::error::CommandError;
use crate::undo::Edit;

/// Several edits as one undoable step (ADR 0040).
#[derive(Default)]
pub struct CompositeEdit {
    /// The undo menu label ("Move 3 shapes").
    pub label: String,
    /// The steps, in apply order (undo runs them backwards).
    pub edits: Vec<Box<dyn Edit>>,
}

impl CompositeEdit {
    /// An empty composite with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            edits: Vec::new(),
        }
    }
}

impl Edit for CompositeEdit {
    fn label(&self) -> &str {
        &self.label
    }

    /// Re-applies every step in order (redo).
    fn apply(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError> {
        let _ = ctx;
        Err(CommandError::failed(
            "not implemented: CompositeEdit::apply",
        ))
    }

    /// Reverts every step in reverse order (undo).
    fn revert(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError> {
        let _ = ctx;
        Err(CommandError::failed(
            "not implemented: CompositeEdit::revert",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppCtx, Settings};

    #[test]
    fn composites_hold_their_label_and_steps() {
        let mut edit = CompositeEdit::new("Move 3 shapes");
        assert_eq!(edit.label(), "Move 3 shapes");
        assert!(edit.edits.is_empty());
        let mut ctx = AppCtx::new(Settings::default());
        assert!(edit.apply(&mut ctx).is_err());
        assert!(edit.revert(&mut ctx).is_err());
    }
}
