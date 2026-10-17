//! Property tests for the undo stack (task P4-10, ADR 0028).
//!
//! The stack is model-checked against a plain reference (entries plus a
//! cursor) over random push/undo/redo/clear sequences, with one rule
//! per property:
//!
//! - `revert` is the inverse of `apply` (the ADR 0028 rule): after any
//!   sequence, the shared counter equals the sum of the deltas that
//!   are applied and not reverted;
//! - `push` truncates the redo tail and clamps to the capacity, so the
//!   stack length never exceeds the capacity and never disagrees
//!   with the model;
//! - `undo`/`redo` move the cursor by one and answer the edit label,
//!   `Ok(None)` at the ends, and a failing edit leaves the cursor
//!   where it was (its error is returned, never a panic).
//!
//! A pushed edit is *already applied* (ADR 0028: "the command already
//! applied it"), so the harness adds its delta to the counter at push
//! time. An entry dropped by the capacity clamp can never be reverted:
//! its delta stays applied forever, and the model accounts for it in
//! `stranded`.
//!
//! A panic is the bug (AGENTS.md section 6).

// The never-crash lints apply to non-test code only (clippy.toml);
// these helpers are test code and use `unwrap` freely.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::sync::{Arc, Mutex};

use ferrogram_commands::app::{AppCtx, Settings};
use ferrogram_commands::{CommandError, Edit, UndoStack};
use proptest::prelude::*;

/// Shared state the counter edits apply to.
#[derive(Clone, Default)]
struct Counter(Arc<Mutex<i64>>);

impl Counter {
    fn get(&self) -> i64 {
        *self.0.lock().unwrap()
    }

    fn add(&self, delta: i64) {
        // Wrapping: the model arithmetic below wraps the same way, so
        // extreme `i64` deltas compare equal everywhere.
        let mut value = self.0.lock().unwrap();
        *value = value.wrapping_add(delta);
    }
}

/// One undoable step: `apply` adds `delta`, `revert` subtracts it, so
/// `revert(apply(x)) == x` holds by construction and the property
/// checks that the stack calls them the right number of times. The
/// `fail_*` flags make `apply` or `revert` answer an error instead.
struct DeltaEdit {
    delta: i64,
    state: Counter,
    label: String,
    fail_apply: bool,
    fail_revert: bool,
}

impl Edit for DeltaEdit {
    fn label(&self) -> &str {
        &self.label
    }

    fn apply(&mut self, _ctx: &mut AppCtx) -> Result<(), CommandError> {
        if self.fail_apply {
            return Err(CommandError::failed("apply failed"));
        }
        self.state.add(self.delta);
        Ok(())
    }

    fn revert(&mut self, _ctx: &mut AppCtx) -> Result<(), CommandError> {
        if self.fail_revert {
            return Err(CommandError::failed("revert failed"));
        }
        self.state.add(self.delta.wrapping_neg());
        Ok(())
    }
}

/// One model-checked operation.
#[derive(Debug, Clone)]
enum Op {
    /// The command already applied this delta; push it.
    Push(i64),
    /// Push an edit whose `revert` fails (a stuck undo).
    PushStuck,
    /// Push an edit whose `apply` fails (a stuck redo).
    PushNoRedo,
    Undo,
    Redo,
    Clear,
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        any::<i64>().prop_map(Op::Push),
        Just(Op::PushStuck),
        Just(Op::PushNoRedo),
        Just(Op::Undo),
        Just(Op::Redo),
        Just(Op::Clear),
    ]
}

/// One entry of the reference model.
#[derive(Clone, Copy)]
struct ModelEntry {
    delta: i64,
    fail_apply: bool,
    fail_revert: bool,
}

/// The reference model: entries plus a cursor (the undoable prefix).
#[derive(Default)]
struct Model {
    entries: Vec<ModelEntry>,
    cursor: usize,
    /// Deltas that can never be reverted (clamped away or cleared);
    /// they stay applied forever.
    stranded: i64,
}

impl Model {
    /// One step, mirroring `UndoStack` exactly. Returns what the stack
    /// must answer: `Ok(Some(label))`, `Ok(None)` or `Err`.
    fn step(&mut self, op: &Op, capacity: usize, label: &str) -> Result<Option<String>, ()> {
        match op {
            Op::Push(delta) => {
                self.push(ModelEntry {
                    delta: *delta,
                    fail_apply: false,
                    fail_revert: false,
                });
                self.clamp(capacity);
                Ok(Some(label.to_owned()))
            }
            Op::PushStuck => {
                self.push(ModelEntry {
                    delta: 1,
                    fail_apply: false,
                    fail_revert: true,
                });
                self.clamp(capacity);
                Ok(Some(label.to_owned()))
            }
            Op::PushNoRedo => {
                self.push(ModelEntry {
                    delta: -1,
                    fail_apply: true,
                    fail_revert: false,
                });
                self.clamp(capacity);
                Ok(Some(label.to_owned()))
            }
            Op::Undo => {
                if self.cursor == 0 {
                    return Ok(None);
                }
                let entry = self.entries[self.cursor - 1];
                if entry.fail_revert {
                    return Err(());
                }
                self.cursor -= 1;
                Ok(Some(label.to_owned()))
            }
            Op::Redo => {
                if self.cursor >= self.entries.len() {
                    return Ok(None);
                }
                let entry = self.entries[self.cursor];
                if entry.fail_apply {
                    return Err(());
                }
                self.cursor += 1;
                Ok(Some(label.to_owned()))
            }
            Op::Clear => {
                self.stranded = self.applied();
                self.entries.clear();
                self.cursor = 0;
                Ok(Some(label.to_owned()))
            }
        }
    }

    fn push(&mut self, entry: ModelEntry) {
        // The new edit supersedes the redo tail: those entries were
        // reverted, so their deltas are not applied.
        self.entries.truncate(self.cursor);
        self.entries.push(entry);
        self.cursor = self.entries.len();
    }

    fn clamp(&mut self, capacity: usize) {
        if self.entries.len() > capacity {
            let drop = self.entries.len() - capacity;
            // Dropped entries sit in the undoable prefix: their deltas
            // stay applied forever.
            for entry in &self.entries[..drop] {
                self.stranded = self.stranded.wrapping_add(entry.delta);
            }
            self.entries.drain(..drop);
            self.cursor = self.entries.len();
        }
    }

    /// The delta sum that must be applied right now.
    fn applied(&self) -> i64 {
        self.entries[..self.cursor]
            .iter()
            .fold(self.stranded, |acc, e| acc.wrapping_add(e.delta))
    }
}

proptest! {
    /// The stack agrees with the model after every operation: length,
    /// cursor, labels and the applied-state all match, and the length
    /// never exceeds the capacity.
    #[test]
    fn the_stack_matches_its_model(
        capacity in 0usize..8,
        ops in proptest::collection::vec(op_strategy(), 1..60),
    ) {
        let counter = Counter::default();
        let mut stack = UndoStack::new(capacity);
        let mut model = Model::default();
        let mut ctx = AppCtx::new(Settings::default());

        for (index, op) in ops.iter().enumerate() {
            let label = format!("edit {index}");
            match op {
                Op::Push(delta) => {
                    counter.add(*delta);
                    stack.push(Box::new(DeltaEdit {
                        delta: *delta,
                        state: counter.clone(),
                        label: label.clone(),
                        fail_apply: false,
                        fail_revert: false,
                    })).unwrap();
                }
                Op::PushStuck => {
                    counter.add(1);
                    stack.push(Box::new(DeltaEdit {
                        delta: 1,
                        state: counter.clone(),
                        label: label.clone(),
                        fail_apply: false,
                        fail_revert: true,
                    })).unwrap();
                }
                Op::PushNoRedo => {
                    counter.add(-1);
                    stack.push(Box::new(DeltaEdit {
                        delta: -1,
                        state: counter.clone(),
                        label: label.clone(),
                        fail_apply: true,
                        fail_revert: false,
                    })).unwrap();
                }
                Op::Undo | Op::Redo | Op::Clear => {}
            }

            let expected = model.step(op, capacity, &label);
            let answered = match op {
                Op::Undo => stack.undo(&mut ctx).map_err(|_| ()).map(|o| o.unwrap_or_default()),
                Op::Redo => stack.redo(&mut ctx).map_err(|_| ()).map(|o| o.unwrap_or_default()),
                Op::Clear => {
                    stack.clear();
                    Ok(String::new())
                }
                _ => Ok(String::new()),
            };
            match (expected, answered) {
                (Ok(_), Ok(_)) => {}
                (Err(()), Err(())) => {}
                other => panic!("op {op:?}: model/stack disagree: {other:?}"),
            }

            prop_assert_eq!(stack.len(), model.entries.len());
            prop_assert!(stack.len() <= capacity);
            prop_assert_eq!(stack.can_undo(), model.cursor > 0);
            prop_assert_eq!(stack.can_redo(), model.cursor < model.entries.len());
            prop_assert_eq!(stack.undo_labels().len(), model.cursor);
            prop_assert_eq!(counter.get(), model.applied());
        }
    }
}

#[test]
fn a_default_stack_keeps_no_entries() {
    // `Default` is `UndoStack::new(0)`: the capacity-0 stack is a
    // black hole. `DocumentSession` uses `new(DEFAULT_CAPACITY)`; this
    // pins the `Default` semantics so a caller that relies on it
    // notices (documented rule, not a silent surprise).
    let mut stack = UndoStack::default();
    let counter = Counter::default();
    let mut ctx = AppCtx::new(Settings::default());
    counter.add(5);
    stack
        .push(Box::new(DeltaEdit {
            delta: 5,
            state: counter.clone(),
            label: "edit".to_owned(),
            fail_apply: false,
            fail_revert: false,
        }))
        .unwrap();
    assert!(stack.is_empty());
    assert_eq!(stack.undo(&mut ctx).unwrap(), None);
    assert_eq!(counter.get(), 5, "the edit ran once and stays applied");
}

#[test]
fn an_undo_and_redo_pair_restores_the_state_exactly() {
    // The ADR 0028 "revert is the inverse of apply" rule, spelled out
    // on one edit.
    let counter = Counter::default();
    let mut ctx = AppCtx::new(Settings::default());
    let mut edit = DeltaEdit {
        delta: -7,
        state: counter.clone(),
        label: "move".to_owned(),
        fail_apply: false,
        fail_revert: false,
    };
    edit.apply(&mut ctx).unwrap();
    assert_eq!(counter.get(), -7);
    edit.revert(&mut ctx).unwrap();
    assert_eq!(counter.get(), 0);
    edit.apply(&mut ctx).unwrap();
    assert_eq!(counter.get(), -7);
}

#[test]
fn a_failing_redo_leaves_the_cursor_where_it_was() {
    // The failing-`apply` twin of the failing-`revert` unit test in
    // `undo.rs`: the cursor must not move past an edit that cannot
    // re-apply.
    let counter = Counter::default();
    let mut ctx = AppCtx::new(Settings::default());
    let mut stack = UndoStack::new(4);
    counter.add(1); // the command already applied the edit (ADR 0028)
    stack
        .push(Box::new(DeltaEdit {
            delta: 1,
            state: counter.clone(),
            label: "bad".to_owned(),
            fail_apply: true,
            fail_revert: false,
        }))
        .unwrap();
    stack.undo(&mut ctx).unwrap();
    let error = stack.redo(&mut ctx).unwrap_err();
    assert_eq!(error.to_string(), "apply failed");
    assert!(stack.can_redo(), "the cursor did not move");
    assert_eq!(counter.get(), 0);
}
