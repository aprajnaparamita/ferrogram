//! Undo and redo (ADR 0028).
//!
//! An [`Edit`] is one undoable step: `apply` redoes it, `revert` undoes
//! it, both over the application state. Commands return their edit in
//! [`Outcome::edit`](crate::Outcome::edit); the dispatcher pushes it onto
//! the per-document [`UndoStack`] (ADR 0029). Phase 4 commands produce
//! no edits; Phase 5 fills the stack and property-tests that `revert` is
//! the inverse of `apply`.

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
    /// The redo tail after the cursor is dropped first (the new edit
    /// supersedes it) and the stack is clamped to its capacity by
    /// dropping the oldest entries. Never fails.
    pub fn push(&mut self, edit: Box<dyn Edit>) -> Result<(), CommandError> {
        self.entries.truncate(self.cursor);
        self.entries.push(edit);
        if self.entries.len() > self.capacity {
            let drop = self.entries.len() - self.capacity;
            self.entries.drain(..drop);
        }
        self.cursor = self.entries.len();
        Ok(())
    }

    /// Reverses the newest undoable edit and returns its label.
    ///
    /// `Ok(None)` when nothing is undoable. A failing `revert` is
    /// returned as [`CommandError`] and leaves the cursor where it was.
    pub fn undo(&mut self, ctx: &mut AppCtx) -> Result<Option<String>, CommandError> {
        let Some(index) = self.cursor.checked_sub(1) else {
            return Ok(None);
        };
        let Some(edit) = self.entries.get_mut(index) else {
            return Ok(None);
        };
        edit.revert(ctx)?;
        self.cursor = index;
        Ok(Some(edit.label().to_owned()))
    }

    /// Re-applies the newest redoable edit and returns its label.
    ///
    /// `Ok(None)` when nothing is redoable. A failing `apply` is
    /// returned as [`CommandError`] and leaves the cursor where it was.
    pub fn redo(&mut self, ctx: &mut AppCtx) -> Result<Option<String>, CommandError> {
        let Some(edit) = self.entries.get_mut(self.cursor) else {
            return Ok(None);
        };
        edit.apply(ctx)?;
        self.cursor = self.cursor.saturating_add(1);
        Ok(Some(edit.label().to_owned()))
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

    /// An edit with a label of its own.
    struct NamedEdit(&'static str);

    impl Edit for NamedEdit {
        fn label(&self) -> &str {
            self.0
        }

        fn apply(&mut self, _ctx: &mut AppCtx) -> Result<(), CommandError> {
            Ok(())
        }

        fn revert(&mut self, _ctx: &mut AppCtx) -> Result<(), CommandError> {
            Ok(())
        }
    }

    /// An edit whose steps always fail.
    struct BadEdit;

    impl Edit for BadEdit {
        fn label(&self) -> &str {
            "bad"
        }

        fn apply(&mut self, _ctx: &mut AppCtx) -> Result<(), CommandError> {
            Err(CommandError::failed("apply failed"))
        }

        fn revert(&mut self, _ctx: &mut AppCtx) -> Result<(), CommandError> {
            Err(CommandError::failed("revert failed"))
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
    fn push_undo_and_redo_round_trip() {
        let mut stack = UndoStack::new(UndoStack::DEFAULT_CAPACITY);
        let mut ctx = AppCtx::new(Settings::default());
        stack.push(Box::new(NamedEdit("one"))).expect("push");
        stack.push(Box::new(NamedEdit("two"))).expect("push");
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.undo_labels(), vec!["one", "two"]);
        assert!(stack.redo_labels().is_empty());

        assert_eq!(stack.undo(&mut ctx).expect("undo").as_deref(), Some("two"));
        assert_eq!(stack.undo(&mut ctx).expect("undo").as_deref(), Some("one"));
        assert!(!stack.can_undo());
        assert_eq!(stack.undo_labels(), Vec::<&str>::new());
        assert_eq!(stack.redo_labels(), vec!["one", "two"]);

        assert_eq!(stack.redo(&mut ctx).expect("redo").as_deref(), Some("one"));
        assert_eq!(stack.redo(&mut ctx).expect("redo").as_deref(), Some("two"));
        assert!(!stack.can_redo());

        // Nothing left in either direction.
        assert_eq!(stack.undo(&mut ctx).unwrap().as_deref(), Some("two"));
        assert_eq!(stack.undo(&mut ctx).unwrap().as_deref(), Some("one"));
        assert_eq!(stack.undo(&mut ctx).unwrap(), None);
        assert_eq!(stack.redo(&mut ctx).unwrap().as_deref(), Some("one"));
        assert_eq!(stack.redo(&mut ctx).unwrap().as_deref(), Some("two"));
        assert_eq!(stack.redo(&mut ctx).unwrap(), None);
    }

    #[test]
    fn push_truncates_the_redo_tail() {
        let mut stack = UndoStack::new(UndoStack::DEFAULT_CAPACITY);
        let mut ctx = AppCtx::new(Settings::default());
        stack.push(Box::new(NamedEdit("one"))).expect("push");
        stack.push(Box::new(NamedEdit("two"))).expect("push");
        stack.undo(&mut ctx).expect("undo");
        assert!(stack.can_redo());
        stack.push(Box::new(NamedEdit("three"))).expect("push");
        assert!(!stack.can_redo());
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.undo_labels(), vec!["one", "three"]);
    }

    #[test]
    fn push_clamps_to_the_capacity() {
        let mut stack = UndoStack::new(2);
        for label in ["one", "two", "three"] {
            stack.push(Box::new(NamedEdit(label))).expect("push");
        }
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.undo_labels(), vec!["two", "three"]);
        stack.push(Box::new(NamedEdit("four"))).expect("push");
        assert_eq!(stack.undo_labels(), vec!["three", "four"]);

        // A zero-capacity stack keeps nothing undoable.
        let mut stack = UndoStack::new(0);
        stack.push(Box::new(NamedEdit("one"))).expect("push");
        assert!(stack.is_empty());
        assert!(!stack.can_undo());
    }

    #[test]
    fn a_failing_edit_leaves_the_cursor_where_it_was() {
        let mut stack = UndoStack::new(UndoStack::DEFAULT_CAPACITY);
        let mut ctx = AppCtx::new(Settings::default());
        stack.push(Box::new(BadEdit)).expect("push");
        let error = stack.undo(&mut ctx).expect_err("revert fails");
        assert_eq!(error.to_string(), "revert failed");
        assert!(stack.can_undo(), "the cursor did not move");

        let mut stack = UndoStack::new(UndoStack::DEFAULT_CAPACITY);
        stack.push(Box::new(BadEdit)).expect("push");
        stack.undo(&mut ctx).expect_err("revert fails");
        stack.push(Box::new(NamedEdit("good"))).expect("push");
        assert_eq!(stack.undo_labels(), vec!["bad", "good"]);
    }

    #[test]
    fn clear_drops_everything() {
        let mut stack = UndoStack::default();
        let mut ctx = AppCtx::new(Settings::default());
        stack.push(Box::new(NoopEdit)).expect("push");
        stack.undo(&mut ctx).expect("undo");
        stack.clear();
        assert!(stack.is_empty());
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    fn the_label_views_are_empty_on_a_new_stack() {
        let stack = UndoStack::new(UndoStack::DEFAULT_CAPACITY);
        assert!(stack.undo_labels().is_empty());
        assert!(stack.redo_labels().is_empty());
    }
}
