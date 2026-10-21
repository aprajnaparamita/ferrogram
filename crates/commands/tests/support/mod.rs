//! Shared test helpers (task P5-19).
//!
//! `dispatch` is a thin wrapper over [`ferrogram_commands::dispatch`],
//! the one dispatch-and-push seam (review F3): the test suites must not
//! re-implement the undo-step recording of [`Outcome::edit`].

#![allow(dead_code, clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use ferrogram_commands::{
    AppCtx, Args, CommandError, CommandId, Invocation, Outcome, Registry, Source,
};

/// One command the way the shell dispatches it: the run and its
/// undo-step recording live in [`ferrogram_commands::dispatch`].
pub fn dispatch(
    ctx: &mut AppCtx,
    registry: &Registry,
    id: &str,
    args: Args,
) -> Result<Outcome, CommandError> {
    ferrogram_commands::dispatch(
        registry,
        ctx,
        &CommandId::intern(id).expect("id"),
        Invocation::new(args, Source::Test),
    )
}
