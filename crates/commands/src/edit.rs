//! Composite edits: one drag is one undo step (ADR 0040).
//!
//! A command that builds its undo step out of several smaller edits (a
//! multi-shape move, a group, a paste) wraps them in [`CompositeEdit`]:
//! undo reverts them in reverse order and redo applies them in order. The
//! individual edit types (`CellEdit`, `StructuralEdit`, `GeometryEdit`)
//! are task P5-04's implementation detail and stay in
//! `crates/commands::builtins::edit`.
//!
//! Both directions run with the undo-recording gate closed (ADR 0040
//! rule 2), the same as [`UndoStack`](crate::UndoStack) runs them.

use crate::app::AppCtx;
use crate::error::CommandError;
use crate::undo::{Edit, run_suspended};

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

    /// Adds one step (applied after the steps already added).
    pub fn push(&mut self, edit: Box<dyn Edit>) {
        self.edits.push(edit);
    }

    /// Number of steps.
    pub fn len(&self) -> usize {
        self.edits.len()
    }

    /// True when there are no steps.
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }
}

impl Edit for CompositeEdit {
    fn label(&self) -> &str {
        &self.label
    }

    /// Re-applies every step in order (redo).
    ///
    /// A failing step stops the run and its error is returned; the
    /// steps before it stay applied (the undo stack keeps its cursor, so
    /// the state and the stack stay in the order they were left in).
    fn apply(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError> {
        run_suspended(ctx, |ctx| {
            for edit in &mut self.edits {
                edit.apply(ctx)?;
            }
            Ok(())
        })
    }

    /// Reverts every step in reverse order (undo).
    ///
    /// A failing step stops the run and its error is returned (the same
    /// partial-state rule as [`CompositeEdit::apply`]).
    fn revert(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError> {
        run_suspended(ctx, |ctx| {
            for edit in self.edits.iter_mut().rev() {
                edit.revert(ctx)?;
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppCtx, Settings};

    /// One step that appends `mark` to `log` (apply) or removes it
    /// again (revert), so order and direction are observable.
    struct MarkEdit {
        mark: char,
        log: std::sync::Arc<std::sync::Mutex<String>>,
        fail: bool,
    }

    impl Edit for MarkEdit {
        fn label(&self) -> &str {
            "mark"
        }

        fn apply(&mut self, _ctx: &mut AppCtx) -> Result<(), CommandError> {
            if self.fail {
                return Err(CommandError::failed("apply failed"));
            }
            self.log.lock().expect("log").push(self.mark);
            Ok(())
        }

        fn revert(&mut self, _ctx: &mut AppCtx) -> Result<(), CommandError> {
            if self.fail {
                return Err(CommandError::failed("revert failed"));
            }
            self.log.lock().expect("log").retain(|c| c != self.mark);
            Ok(())
        }
    }

    fn marks(
        log: &std::sync::Arc<std::sync::Mutex<String>>,
        marks: &str,
        fail_at: Option<char>,
    ) -> CompositeEdit {
        let mut edit = CompositeEdit::new("Move 3 shapes");
        for mark in marks.chars() {
            edit.push(Box::new(MarkEdit {
                mark,
                log: log.clone(),
                fail: fail_at == Some(mark),
            }));
        }
        edit
    }

    #[test]
    fn composites_hold_their_label_and_steps() {
        let log = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let edit = marks(&log, "abc", None);
        assert_eq!(edit.label(), "Move 3 shapes");
        assert_eq!(edit.edits[0].label(), "mark");
        assert_eq!(edit.len(), 3);
        assert!(!edit.is_empty());

        let mut empty = CompositeEdit::new("Nothing");
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
        let mut ctx = AppCtx::new(Settings::default());
        assert!(empty.apply(&mut ctx).is_ok());
        assert!(empty.revert(&mut ctx).is_ok());
    }

    #[test]
    fn apply_runs_forwards_and_revert_backwards() {
        let log = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let mut edit = marks(&log, "abc", None);
        let mut ctx = AppCtx::new(Settings::default());
        edit.apply(&mut ctx).expect("apply");
        assert_eq!(log.lock().expect("log").as_str(), "abc");
        edit.revert(&mut ctx).expect("revert");
        assert_eq!(log.lock().expect("log").as_str(), "");
    }

    #[test]
    fn a_failing_step_stops_the_run_and_keeps_the_earlier_steps() {
        let log = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let mut edit = marks(&log, "abc", Some('b'));
        let mut ctx = AppCtx::new(Settings::default());
        let error = edit.apply(&mut ctx).expect_err("apply fails on b");
        assert_eq!(error.to_string(), "apply failed");
        assert_eq!(log.lock().expect("log").as_str(), "a", "a stayed applied");

        let log = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let mut edit = marks(&log, "abc", Some('b'));
        edit.apply(&mut ctx).expect_err("apply fails on b");
        let error = edit.revert(&mut ctx).expect_err("revert fails on b");
        assert_eq!(error.to_string(), "revert failed");
    }
}
