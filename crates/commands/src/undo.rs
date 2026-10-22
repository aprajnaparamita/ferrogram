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
use ferrogram_shapesheet::CellAddress;

/// Runs `step` with the undo-recording gate closed (ADR 0040 rule 2).
///
/// Every [`Edit`]'s `apply`/`revert` runs inside this: while it is
/// open, [`AppCtx::push_edit`] drops any edit a nested command or edit
/// tries to record and warns once. The gate is a saved-and-restored
/// flag, so nested runs keep it closed until the outermost step ends.
pub(crate) fn run_suspended<T>(ctx: &mut AppCtx, step: impl FnOnce(&mut AppCtx) -> T) -> T {
    let previous = ctx.set_undo_suspended(true);
    let out = step(ctx);
    ctx.set_undo_suspended(previous);
    out
}

/// One undoable step (ADR 0028).
///
/// Implementations hold whatever they need to replay the step; state
/// lives in [`AppCtx`]. `apply` and `revert` must not panic on any
/// state the stack can produce (AGENTS.md section 6).
///
/// Two rules every implementation keeps (ADR 0040, task P5-04):
///
/// - `apply`/`revert` never record an edit:
///   [`AppCtx::push_edit`] drops it and warns while the recording gate
///   is closed (which is whenever this trait's methods run).
/// - Both bump the change counter of the document they changed, with
///   [`AppCtx::touch_session`], after a successful mutation.
pub trait Edit: Send {
    /// Short label for the undo menu ("Move shape").
    fn label(&self) -> &str;

    /// Re-applies the step (redo).
    fn apply(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError>;

    /// Reverses the step (undo).
    fn revert(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError>;

    /// The cells this step writes, so the recalc pass knows what is
    /// dirty (ADR 0056).
    ///
    /// [`AppCtx::push_edit`](crate::AppCtx::push_edit) runs the pass over
    /// these cells after `apply` and again on every re-apply (the redo
    /// that follows a revert); `revert` restores what the pass changed by
    /// snapshot. The default is an empty dirty set: a step that changes
    /// no cell needs nothing recomputed.
    fn dirty_cells(&self) -> &[CellAddress] {
        &[]
    }
}

/// The undo stack of one document (ADR 0028, ADR 0029).
///
/// The cursor splits the stack: entries before it are undoable, entries
/// after it are redoable. Pushing truncates the redo tail.
///
/// [`UndoStack::default`] builds a stack with
/// [`UndoStack::DEFAULT_CAPACITY`]: a defaulted stack keeps its edits
/// (P4-15 review minor 2). A deliberately silent stack is
/// `UndoStack::new(0)`, which is what `builtins::with_undo` uses as
/// its short-lived placeholder.
pub struct UndoStack {
    entries: Vec<Box<dyn Edit>>,
    cursor: usize,
    capacity: usize,
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new(Self::DEFAULT_CAPACITY)
    }
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
    /// dropping the oldest entries.
    pub fn push(&mut self, edit: Box<dyn Edit>) {
        self.entries.truncate(self.cursor);
        self.entries.push(edit);
        if self.entries.len() > self.capacity {
            let drop = self.entries.len() - self.capacity;
            self.entries.drain(..drop);
        }
        self.cursor = self.entries.len();
    }

    /// Reverses the newest undoable edit and returns its label.
    ///
    /// `Ok(None)` when nothing is undoable. A failing `revert` is
    /// returned as [`CommandError`] and leaves the cursor where it was.
    /// The call runs with the recording gate closed (ADR 0040).
    pub fn undo(&mut self, ctx: &mut AppCtx) -> Result<Option<String>, CommandError> {
        let Some(index) = self.cursor.checked_sub(1) else {
            return Ok(None);
        };
        let Some(edit) = self.entries.get_mut(index) else {
            return Ok(None);
        };
        run_suspended(ctx, |ctx| edit.revert(ctx))?;
        self.cursor = index;
        Ok(Some(edit.label().to_owned()))
    }

    /// Re-applies the newest redoable edit and returns its label.
    ///
    /// `Ok(None)` when nothing is redoable. A failing `apply` is
    /// returned as [`CommandError`] and leaves the cursor where it was.
    /// The call runs with the recording gate closed (ADR 0040).
    pub fn redo(&mut self, ctx: &mut AppCtx) -> Result<Option<String>, CommandError> {
        let Some(edit) = self.entries.get_mut(self.cursor) else {
            return Ok(None);
        };
        run_suspended(ctx, |ctx| edit.apply(ctx))?;
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
        stack.push(Box::new(NamedEdit("one")));
        stack.push(Box::new(NamedEdit("two")));
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
        stack.push(Box::new(NamedEdit("one")));
        stack.push(Box::new(NamedEdit("two")));
        stack.undo(&mut ctx).expect("undo");
        assert!(stack.can_redo());
        stack.push(Box::new(NamedEdit("three")));
        assert!(!stack.can_redo());
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.undo_labels(), vec!["one", "three"]);
    }

    #[test]
    fn push_clamps_to_the_capacity() {
        let mut stack = UndoStack::new(2);
        for label in ["one", "two", "three"] {
            stack.push(Box::new(NamedEdit(label)));
        }
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.undo_labels(), vec!["two", "three"]);
        stack.push(Box::new(NamedEdit("four")));
        assert_eq!(stack.undo_labels(), vec!["three", "four"]);

        // A zero-capacity stack keeps nothing undoable.
        let mut stack = UndoStack::new(0);
        stack.push(Box::new(NamedEdit("one")));
        assert!(stack.is_empty());
        assert!(!stack.can_undo());
    }

    #[test]
    fn a_failing_edit_leaves_the_cursor_where_it_was() {
        let mut stack = UndoStack::new(UndoStack::DEFAULT_CAPACITY);
        let mut ctx = AppCtx::new(Settings::default());
        stack.push(Box::new(BadEdit));
        let error = stack.undo(&mut ctx).expect_err("revert fails");
        assert_eq!(error.to_string(), "revert failed");
        assert!(stack.can_undo(), "the cursor did not move");

        let mut stack = UndoStack::new(UndoStack::DEFAULT_CAPACITY);
        stack.push(Box::new(BadEdit));
        stack.undo(&mut ctx).expect_err("revert fails");
        stack.push(Box::new(NamedEdit("good")));
        assert_eq!(stack.undo_labels(), vec!["bad", "good"]);
    }

    /// The failing `apply` of `BadEdit` and the "nothing to redo"
    /// answer of a stack whose cursor sits at the end.
    #[test]
    fn a_failing_apply_and_an_empty_redo_answer_cleanly() {
        let mut edit = BadEdit;
        let mut ctx = AppCtx::new(Settings::default());
        let error = edit.apply(&mut ctx).expect_err("apply fails");
        assert_eq!(error.to_string(), "apply failed");

        let mut stack = UndoStack::new(UndoStack::DEFAULT_CAPACITY);
        assert!(stack.redo(&mut ctx).expect("redo").is_none());
        assert!(stack.undo(&mut ctx).expect("undo").is_none());
    }

    #[test]
    fn clear_drops_everything() {
        let mut stack = UndoStack::default();
        let mut ctx = AppCtx::new(Settings::default());
        stack.push(Box::new(NoopEdit));
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

    /// P4-15 review minor 2: a defaulted stack keeps its edits (the
    /// old capacity-0 default silently dropped every push). The silent
    /// placeholder is the explicit `UndoStack::new(0)`.
    #[test]
    fn a_default_stack_keeps_its_entries() {
        let mut stack = UndoStack::default();
        let mut ctx = AppCtx::new(Settings::default());
        stack.push(Box::new(NamedEdit("one")));
        stack.push(Box::new(NamedEdit("two")));
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.undo(&mut ctx).expect("undo").as_deref(), Some("two"));

        let mut silent = UndoStack::new(0);
        silent.push(Box::new(NamedEdit("dropped")));
        assert!(silent.is_empty());
        assert!(!silent.can_undo());
    }

    /// P4-12 coverage minor 5: the `Debug` impl reports the shape.
    #[test]
    fn debug_shows_len_cursor_and_capacity() {
        let mut stack = UndoStack::new(2);
        stack.push(Box::new(NamedEdit("one")));
        let text = format!("{stack:?}");
        assert!(text.contains("len: 1"), "{text}");
        assert!(text.contains("cursor: 1"), "{text}");
        assert!(text.contains("capacity: 2"), "{text}");
    }

    /// An edit that runs while `revert`/`apply` records a nested edit
    /// and notes whether the gate was closed (ADR 0040 rule 2).
    struct NestedEdit {
        session: crate::SessionId,
        gate_was_closed: std::sync::Arc<std::sync::Mutex<Vec<bool>>>,
    }

    impl Edit for NestedEdit {
        fn label(&self) -> &str {
            "nested"
        }

        fn apply(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError> {
            self.push(ctx)
        }

        fn revert(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError> {
            self.push(ctx)
        }
    }

    impl NestedEdit {
        fn push(&self, ctx: &mut AppCtx) -> Result<(), CommandError> {
            if let Ok(mut seen) = self.gate_was_closed.lock() {
                seen.push(ctx.undo_recording_suspended());
            }
            ctx.push_edit(self.session, Box::new(NamedEdit("dropped")))
        }
    }

    /// ADR 0040 rule 2: undo and redo run the edit with the recording
    /// gate closed, so a nested push is dropped with one warning.
    #[test]
    fn undo_and_redo_run_with_the_recording_gate_closed() {
        let mut ctx = AppCtx::new(Settings::default());
        let index = ctx.open_document(crate::DocumentSession::new(
            ferrogram_model::Document::new(),
            None,
        ));
        let session = ctx.documents.get(index).expect("session").id;
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut stack = UndoStack::new(UndoStack::DEFAULT_CAPACITY);
        stack.push(Box::new(NestedEdit {
            session,
            gate_was_closed: seen.clone(),
        }));

        stack.undo(&mut ctx).expect("undo");
        assert_eq!(&seen.lock().expect("seen")[..], [true], "gate closed");
        assert_eq!(ctx.warnings.len(), 1, "one dropped-edit warning");
        stack.redo(&mut ctx).expect("redo");
        assert_eq!(&seen.lock().expect("seen")[..], [true, true]);
        assert_eq!(ctx.warnings.len(), 2);
        let entry = ctx.documents.get(index).expect("session");
        assert!(
            entry.undo.is_empty(),
            "the nested edit never landed on the session's stack"
        );
    }
}
