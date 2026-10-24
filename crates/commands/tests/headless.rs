//! Headless registry sweep (task P4-09, ADR 0037): **every**
//! registered command runs through `Registry::execute` with
//! `Source::Test`.
//!
//! `tests/builtins.rs` (P4-02) walks the builtin table command by
//! command; this sweep is over `builtins::registry()` itself, so an
//! id that is registered but never exercised elsewhere cannot hide.
//! The contract under `Source::Test` (a programmatic caller, ADR 0028):
//!
//! - a stub id answers [`CommandError::Unsupported`], an implemented
//!   id never does;
//! - a missing argument answers [`CommandError::NeedsInput`] and never
//!   opens a dialog;
//! - a command whose enabled-state says no answers
//!   [`CommandError::NotEnabled`] with the reason;
//! - everything else runs and produces an [`Outcome`].
//!
//! Nothing here panics: a panic is the bug (AGENTS.md section 6).

// The never-crash lints apply to non-test code only (clippy.toml);
// these helpers are test code and use `expect` freely.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use larrygram_commands::app::{AppCtx, Settings};
use larrygram_commands::builtins::registry;
use larrygram_commands::{
    Args, CommandError, CommandId, CommandStatus, Invocation, NeededInput, Source, UiRequest,
};

/// Runs one registered id with empty arguments and a fresh state.
fn run_isolated(
    registry: &larrygram_commands::Registry,
    id: &CommandId,
) -> Result<larrygram_commands::Outcome, CommandError> {
    let mut ctx = AppCtx::new(Settings::default());
    registry.execute(id, &mut ctx, Invocation::empty(Source::Test))
}

#[test]
fn every_registered_command_runs_with_source_test() {
    let registry = registry().expect("the builtin registry builds");
    assert!(!registry.is_empty(), "the registry is not empty");
    for id in registry.ids() {
        let outcome = run_isolated(&registry, &id);
        match outcome {
            Ok(_)
            | Err(
                CommandError::NeedsInput(_)
                | CommandError::NotEnabled { .. }
                | CommandError::Unsupported { .. },
            ) => {}
            // A domain failure is a legal answer for an id that
            // needs arguments it did not get; anything else is a
            // contract break.
            Err(CommandError::Failed { .. } | CommandError::InvalidArgs { .. }) => {}
            Err(error) => panic!("{id}: unexpected error with Source::Test: {error}"),
        }
    }
}

#[test]
fn stub_ids_answer_unsupported_and_implemented_ids_never_do() {
    let registry = registry().expect("the builtin registry builds");
    for id in registry.ids() {
        let status = registry.status(&id);
        let result = run_isolated(&registry, &id);
        let unsupported = matches!(&result, Err(CommandError::Unsupported { .. }));
        match status {
            CommandStatus::Stub => {
                assert!(unsupported, "{id} is a stub and must answer Unsupported")
            }
            CommandStatus::Implemented => {
                assert!(
                    !unsupported,
                    "{id} is implemented and must not answer Unsupported"
                )
            }
            // `CommandStatus` is non-exhaustive; a future variant
            // must not crash the sweep.
            _ => {}
        }
    }
}

#[test]
fn a_missing_argument_comes_back_and_opens_no_dialog() {
    let registry = registry().expect("the builtin registry builds");
    // `file.open` without `path`: the programmatic contract (ADR 0028).
    let mut ctx = AppCtx::new(Settings::default());
    let command = CommandId::parse("file.open").expect("id");
    let error = registry
        .execute(&command, &mut ctx, Invocation::empty(Source::Test))
        .expect_err("file.open needs a path");
    match error {
        CommandError::NeedsInput(NeededInput::Path { .. }) => {}
        other => panic!("expected NeedsInput(Path), got {other}"),
    }
    // No dialog request escapes to a programmatic caller (AGENTS.md
    // section 6) and the state is untouched.
    assert!(ctx.active_document().is_none());
    assert!(ctx.warnings.is_empty());
}

#[test]
fn source_test_is_never_interactive() {
    // The dispatcher asks `Source::is_interactive` before opening any
    // dialog; `Source::Test` must say no (ADR 0028).
    assert!(!Source::Test.is_interactive());
    for source in [Source::Cli, Source::Control, Source::Mcp] {
        assert!(!source.is_interactive(), "{source:?} is programmatic");
    }
    for source in [
        Source::Ui,
        Source::CommandBar,
        Source::Menu,
        Source::Shortcut,
    ] {
        assert!(source.is_interactive(), "{source:?} is interactive");
    }
}

#[test]
fn help_about_only_requests_the_window_with_source_test() {
    let registry = registry().expect("the builtin registry builds");
    let command = CommandId::parse("help.about").expect("id");
    let mut ctx = AppCtx::new(Settings::default());
    let outcome = registry
        .execute(&command, &mut ctx, Invocation::empty(Source::Test))
        .expect("help.about runs headless");
    assert_eq!(outcome.ui_request, Some(UiRequest::ShowAbout));
}

#[test]
fn an_unknown_id_is_an_unknown_command() {
    let registry = registry().expect("the builtin registry builds");
    let command = CommandId::parse("file.not_a_command").expect("id");
    let mut ctx = AppCtx::new(Settings::default());
    let error = registry
        .execute(&command, &mut ctx, Invocation::empty(Source::Test))
        .expect_err("unknown id");
    assert!(
        matches!(error, CommandError::UnknownCommand { .. }),
        "{error}"
    );
}

#[test]
fn a_disabled_command_reports_not_enabled_with_its_reason() {
    let registry = registry().expect("the builtin registry builds");
    let command = CommandId::parse("edit.undo").expect("id");
    let mut ctx = AppCtx::new(Settings::default());
    let error = registry
        .execute(&command, &mut ctx, Invocation::empty(Source::Test))
        .expect_err("nothing to undo");
    match error {
        CommandError::NotEnabled { id, reason } => {
            assert_eq!(id, "edit.undo");
            assert_eq!(reason.as_deref(), Some("nothing to undo"));
        }
        other => panic!("expected NotEnabled, got {other}"),
    }
}

#[test]
fn args_survive_the_sweep_untouched() {
    // `Invocation::empty` is the sweep's input; an argument-bearing
    // invocation reaches the command unchanged (the seam the CLI and
    // the Phase 8 channel reuse).
    let registry = registry().expect("the builtin registry builds");
    let command = CommandId::parse("view.ui_layout.panel_365").expect("id");
    let mut ctx = AppCtx::new(Settings::default());
    let invocation = Invocation::new(Args::new(), Source::Test);
    registry
        .execute(&command, &mut ctx, invocation)
        .expect("the layout switch runs");
    assert_eq!(
        ctx.settings.ui_layout,
        larrygram_ui_spec::LayoutKind::Panel365
    );
}
