//! The undo runner contracts of task P5-04 (ADR 0040):
//!
//! - `AppCtx::push_edit` is the only push path and lands on the session
//!   it names, whatever the active document is;
//! - recording is suspended while an `Edit`'s `apply`/`revert` runs: a
//!   nested edit is dropped with exactly one warning;
//! - one command = one undo step (`CompositeEdit`), undone in one
//!   `edit.undo`, and redo replays it.
//!
//! The per-command pair tests (apply/revert round-trips the model) live
//! in `tests/editing.rs`; the stack mechanics are property-tested in
//! `tests/undo_properties.rs` (P4-10).

// The never-crash lints apply to non-test code only (clippy.toml).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::sync::{Arc, Mutex};

use ferrogram_commands::app::{AppCtx, DocumentSession, Settings};
use ferrogram_commands::builtins::registry;
use ferrogram_commands::{
    CommandError, CommandId, CompositeEdit, Edit, Invocation, Registry, SessionId, Source,
};
use ferrogram_model::Document;

/// An edit that records what it saw of the recording gate and pushes a
/// nested edit while it runs (the P4-10/P4-15 hazard of ADR 0040).
struct NestedPush {
    session: SessionId,
    /// Was the gate closed when `apply`/`revert` ran?
    gate: Arc<Mutex<Vec<bool>>>,
    /// Did the nested push succeed in landing?
    landed: Arc<Mutex<bool>>,
}

impl NestedPush {
    fn run(&self, ctx: &mut AppCtx) -> Result<(), CommandError> {
        if let Ok(mut gate) = self.gate.lock() {
            gate.push(ctx.undo_recording_suspended());
        }
        ctx.push_edit(self.session, Box::new(Named("nested")))?;
        if let Ok(mut landed) = self.landed.lock() {
            *landed = true;
        }
        Ok(())
    }
}

impl Edit for NestedPush {
    fn label(&self) -> &str {
        "outer"
    }

    fn apply(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError> {
        self.run(ctx)
    }

    fn revert(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError> {
        self.run(ctx)
    }
}

/// An edit with a label only.
struct Named(&'static str);

impl Edit for Named {
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

/// An edit that appends one mark to a log: apply pushes, revert pops.
struct Mark {
    mark: char,
    log: Arc<Mutex<String>>,
}

impl Edit for Mark {
    fn label(&self) -> &str {
        "mark"
    }

    fn apply(&mut self, _ctx: &mut AppCtx) -> Result<(), CommandError> {
        if let Ok(mut log) = self.log.lock() {
            log.push(self.mark);
        }
        Ok(())
    }

    fn revert(&mut self, _ctx: &mut AppCtx) -> Result<(), CommandError> {
        if let Ok(mut log) = self.log.lock() {
            log.retain(|c| c != self.mark);
        }
        Ok(())
    }
}

fn state() -> (AppCtx, Registry, SessionId) {
    let mut ctx = AppCtx::new(Settings::default());
    let index = ctx.open_document(DocumentSession::new(Document::new(), None));
    let session = ctx.documents.get(index).expect("session").id;
    (ctx, registry().expect("registry"), session)
}

fn undo(ctx: &mut AppCtx, registry: &Registry) {
    registry
        .execute(
            &CommandId::intern("edit.undo").expect("id"),
            ctx,
            Invocation::empty(Source::Test),
        )
        .expect("edit.undo");
}

fn redo(ctx: &mut AppCtx, registry: &Registry) {
    registry
        .execute(
            &CommandId::intern("edit.redo").expect("id"),
            ctx,
            Invocation::empty(Source::Test),
        )
        .expect("edit.redo");
}

/// ADR 0040 rule 2: an edit that records during `apply`/`revert` is
/// dropped with exactly one warning and the stack keeps its cursor.
#[test]
fn a_nested_edit_during_undo_and_redo_is_dropped_with_one_warning() {
    let (mut ctx, registry, session) = state();
    let gate = Arc::new(Mutex::new(Vec::new()));
    let landed = Arc::new(Mutex::new(false));
    ctx.push_edit(
        session,
        Box::new(NestedPush {
            session,
            gate: gate.clone(),
            landed: landed.clone(),
        }),
    )
    .expect("push");
    assert_eq!(ctx.warnings.len(), 0, "a direct push warns not");

    undo(&mut ctx, &registry);
    assert_eq!(&gate.lock().expect("gate")[..], [true], "gate closed");
    assert_eq!(ctx.warnings.len(), 1);
    assert_eq!(
        ctx.warnings.first().map(String::as_str),
        Some("an edit tried to record during undo; step dropped")
    );
    let session_entry = ctx
        .documents
        .iter()
        .find(|entry| entry.id == session)
        .expect("session");
    assert!(
        session_entry.undo.len() == 1,
        "the nested edit landed not: {:?}",
        session_entry.undo.undo_labels()
    );

    redo(&mut ctx, &registry);
    assert_eq!(&gate.lock().expect("gate")[..], [true, true]);
    assert_eq!(ctx.warnings.len(), 2, "one warning per dropped edit");
    assert!(
        *landed.lock().expect("landed"),
        "the push_edit call itself ran"
    );
}

/// The recording gate is closed for direct `CompositeEdit` runs too
/// (the command's first application), and nested composites stay closed
/// until the whole step is done.
#[test]
fn the_gate_closes_for_composite_runs() {
    let (mut ctx, _registry, session) = state();
    let gate = Arc::new(Mutex::new(Vec::new()));
    let landed = Arc::new(Mutex::new(false));
    let mut inner = CompositeEdit::new("inner");
    inner.push(Box::new(NestedPush {
        session,
        gate: gate.clone(),
        landed: landed.clone(),
    }));
    let mut outer = CompositeEdit::new("outer");
    outer.push(Box::new(Named("first")));
    outer.push(Box::new(inner));

    outer.apply(&mut ctx).expect("apply");
    assert_eq!(&gate.lock().expect("gate")[..], [true]);
    assert_eq!(ctx.warnings.len(), 1, "the nested push was dropped");
    outer.revert(&mut ctx).expect("revert");
    assert_eq!(&gate.lock().expect("gate")[..], [true, true]);
    assert_eq!(ctx.warnings.len(), 2);
    assert!(!ctx.undo_recording_suspended(), "the gate re-opened");
}

/// One command = one undo step: a `CompositeEdit` is undone in one
/// `edit.undo` and redone in one `edit.redo`, forwards and backwards.
#[test]
fn one_composite_is_one_undo_step() {
    let (mut ctx, registry, session) = state();
    let log = Arc::new(Mutex::new(String::new()));
    let mut edit = CompositeEdit::new("Move 3 shapes");
    for mark in "abc".chars() {
        edit.push(Box::new(Mark {
            mark,
            log: log.clone(),
        }));
    }
    // The command's own application runs the steps forwards.
    edit.apply(&mut ctx).expect("apply");
    assert_eq!(log.lock().expect("log").as_str(), "abc");
    ctx.push_edit(session, Box::new(edit)).expect("push");

    let entry = ctx
        .documents
        .iter()
        .find(|entry| entry.id == session)
        .expect("session");
    assert_eq!(entry.undo.len(), 1);
    assert_eq!(entry.undo.undo_labels(), ["Move 3 shapes"]);

    undo(&mut ctx, &registry);
    assert_eq!(
        log.lock().expect("log").as_str(),
        "",
        "one undo reverted all"
    );
    redo(&mut ctx, &registry);
    assert_eq!(
        log.lock().expect("log").as_str(),
        "abc",
        "one redo redid all"
    );
}

/// `push_edit` names its session: the edit lands on that document's
/// stack even when another document is active (the P4-15 `with_undo`
/// rule).
#[test]
fn push_edit_lands_on_the_named_session_not_the_active_one() {
    let (mut ctx, _registry, session) = state();
    let other = ctx.open_document(DocumentSession::new(Document::new(), None));
    let other = ctx.documents.get(other).expect("session").id;
    assert_ne!(session, other);

    ctx.push_edit(session, Box::new(Named("background")))
        .expect("push");
    let named = ctx
        .documents
        .iter()
        .find(|entry| entry.id == session)
        .expect("session");
    assert_eq!(named.undo.undo_labels(), ["background"]);
    let active = ctx
        .documents
        .iter()
        .find(|entry| entry.id == other)
        .expect("session");
    assert!(active.undo.is_empty());

    // A session nothing names is an error, never a silent drop.
    let ghost = DocumentSession::new(Document::new(), None).id;
    let error = ctx
        .push_edit(ghost, Box::new(Named("lost")))
        .expect_err("no such session");
    assert!(
        matches!(error, CommandError::Failed { ref message } if message.contains("no document session")),
        "{error:?}"
    );
}
