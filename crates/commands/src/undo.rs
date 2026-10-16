//! Undo and redo (ADR 0028).
//!
//! An [`Edit`] is one undoable step: `apply` redoes it, `revert` undoes
//! it, both over the application state. Commands return their edit in
//! [`Outcome::edit`](crate::Outcome::edit); the dispatcher pushes it onto
//! the per-document [`UndoStack`] (ADR 0029). Phase 4 commands produce
//! no edits; Phase 5 fills the stack and property-tests that `revert` is
//! the inverse of `apply`.
//!
//! Alpha status (task P4-01): the interface is defined; `push`, `undo`
//! and `redo` are "not implemented" stubs until P4-02.

use crate::app::AppCtx;
use crate::error::CommandError;

/// One undoable step (ADR 0028).
///
/// Implementations hold whatever they need to replay the step; state
/// lives in [`AppCtx`]. `apply` and `revert` must not panic on any
/// state the stack can produce (AGENTS.md section 6).
pub trait Edit: Send {
    /// Short label for the undo menu ("Move shape").
    fn label(&self) -> &str;

    /// Re-applies the step (redo).
    fn apply(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError>;

    /// Reverses the step (undo).
    fn revert(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError>;
}

/// The undo stack of one document (ADR 0028, ADR 0029).
///
/// The cursor splits the stack: entries before it are undoable, entries
/// after it are redoable. Pushing truncates the redo tail.
#[derive(Default)]
pub struct UndoStack {
    entries: Vec<Box<dyn Edit>>,
    cursor: usize,
    capacity: usize,
}

impl std::fmt::Debug for UndoStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UndoStack")
            .field("len", &self.entries.len())
            .field("cursor", &self.cursor)
            .field("capacity", &self.capacity)
            .finish()
    }
}

impl UndoStack {
    /// The default stack capacity (ADR 0028).
    pub const DEFAULT_CAPACITY: usize = 100;

    /// An empty stack with a maximum number of entries.
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Vec::new(),
            cursor: 0,
            capacity,
        }
    }

    /// Whether an undo step is available.
    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    /// Whether a redo step is available.
    pub fn can_redo(&self) -> bool {
        self.cursor < self.entries.len()
    }

    /// Number of entries on the stack (undoable and redoable).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when the stack holds no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Drops every entry (document closed or saved).
    pub fn clear(&mut self) {
        self.entries.clear();
        self.cursor = 0;
    }

    /// The labels of the undoable edits, oldest first (the undo menu).
    pub fn undo_labels(&self) -> Vec<&str> {
        self.entries
            .iter()
            .take(self.cursor)
            .map(|edit| edit.label())
            .collect()
    }

    /// The labels of the redoable edits, oldest first (the redo menu).
    pub fn redo_labels(&self) -> Vec<&str> {
        self.entries
            .iter()
            .skip(self.cursor)
            .map(|edit| edit.label())
            .collect()
    }

    /// Pushes a completed edit (the command already applied it).
    ///
    /// Not implemented yet (P4-02).
    pub fn push(&mut self, edit: Box<dyn Edit>) -> Result<(), CommandError> {
        let _ = edit;
        Err(CommandError::NotImplemented {
            what: "UndoStack::push",
        })
    }

    /// Reverses the newest undoable edit and returns its label.
    ///
    /// Not implemented yet (P4-02).
    pub fn undo(&mut self, ctx: &mut AppCtx) -> Result<Option<String>, CommandError> {
        let _ = ctx;
        Err(CommandError::NotImplemented {
            what: "UndoStack::undo",
        })
    }

    /// Re-applies the newest redoable edit and returns its label.
    ///
    /// Not implemented yet (P4-02).
    pub fn redo(&mut self, ctx: &mut AppCtx) -> Result<Option<String>, CommandError> {
        let _ = ctx;
        Err(CommandError::NotImplemented {
            what: "UndoStack::redo",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Settings;

    struct NoopEdit;

    impl Edit for NoopEdit {
        fn label(&self) -> &str {
            "noop"
        }

        fn apply(&mut self, _ctx: &mut AppCtx) -> Result<(), CommandError> {
            Ok(())
        }

        fn revert(&mut self, _ctx: &mut AppCtx) -> Result<(), CommandError> {
            Ok(())
        }
    }

    #[test]
    fn noop_edit_applies_and_reverts() {
        let mut edit = NoopEdit;
        let mut ctx = AppCtx::new(Settings::default());
        assert_eq!(edit.label(), "noop");
        assert!(edit.apply(&mut ctx).is_ok());
        assert!(edit.revert(&mut ctx).is_ok());
    }

    #[test]
    fn a_new_stack_holds_nothing() {
        let stack = UndoStack::new(UndoStack::DEFAULT_CAPACITY);
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());
        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);
        assert_eq!(UndoStack::DEFAULT_CAPACITY, 100);
    }

    #[test]
    fn stack_operations_are_stubs_and_clear_is_trivial() {
        let mut stack = UndoStack::default();
        let mut ctx = AppCtx::new(Settings::default());
        assert!(stack.push(Box::new(NoopEdit)).is_err());
        assert!(stack.undo(&mut ctx).is_err());
        assert!(stack.redo(&mut ctx).is_err());
        stack.clear();
        assert!(stack.is_empty());
    }

    #[test]
    fn the_label_views_are_empty_until_push_works() {
        let stack = UndoStack::new(UndoStack::DEFAULT_CAPACITY);
        assert!(stack.undo_labels().is_empty());
        assert!(stack.redo_labels().is_empty());
    }
}
