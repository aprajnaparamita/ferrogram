//! The egui user interface (L6, ADR 0002): ribbon, canvas, panels,
//! shapes window, shortcuts and the shell that hosts them.
//!
//! The widgets read the ui-spec layouts (ADR 0031) and the application
//! state (ADR 0029) and report [`Intent`]s; the shell dispatches them
//! through the command registry (ADR 0028). Nothing here mutates
//! `AppCtx` directly, and nothing below L6 depends on egui.
//!
//! Alpha status (task P4-01): the module interfaces are defined as
//! stubs; each module is implemented by its Phase 4 task (see
//! `docs/tasks/P4-03.md` and later).

#![forbid(unsafe_code)]

pub mod canvas;
pub mod dialogs;
pub mod handles;
pub mod icons;
pub mod panels;
pub mod ribbon;
pub mod settings;
pub mod shapes_window;
pub mod shell;
pub mod shortcuts;
pub mod snap;
pub mod text_edit;
pub mod tools;

use std::fmt;

use ferrogram_commands::{CommandError, CommandId, Invocation};
use ferrogram_ui_spec::LoadError;

/// A dispatch request produced by a widget (ADR 0028).
///
/// Widgets never execute commands themselves; they return intents and
/// the shell runs them through `Registry::execute`.
#[derive(Debug, Clone, PartialEq)]
pub struct Intent {
    /// The command to dispatch.
    pub command: CommandId,
    /// The arguments and the source (UI, shortcut, ...).
    pub invocation: Invocation,
}

impl Intent {
    /// An intent for one command and invocation.
    pub fn new(command: CommandId, invocation: Invocation) -> Self {
        Self {
            command,
            invocation,
        }
    }
}

/// A UI failure (ADR 0031 and the Phase 4 task stubs).
///
/// No UI error path panics (AGENTS.md section 6).
#[derive(Debug)]
#[non_exhaustive]
pub enum UiError {
    /// A dispatched command failed.
    Command(CommandError),
    /// The ui-spec layout data failed to load (ADR 0007).
    Layout(LoadError),
    /// A file or dialog operation failed.
    Io {
        /// The formatted cause.
        message: String,
    },
    /// A scene build or raster operation failed (the canvas, task
    /// P4-05's request to P4-03).
    Render {
        /// The formatted cause.
        message: String,
    },
    /// Invalid geometry or snap math (task P5-19 review F8): nothing
    /// did IO, the numbers could not be computed.
    Geometry {
        /// The formatted cause.
        message: String,
    },
    /// The operation is not implemented yet (a Phase 5 interface stub,
    /// task P5-01).
    NotImplemented,
}

impl fmt::Display for UiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UiError::Command(error) => write!(f, "command error: {error}"),
            UiError::Layout(error) => write!(f, "ui-spec error: {error}"),
            UiError::Io { message } => write!(f, "io error: {message}"),
            UiError::Render { message } => write!(f, "render error: {message}"),
            UiError::Geometry { message } => write!(f, "geometry error: {message}"),
            UiError::NotImplemented => write!(f, "not implemented yet"),
        }
    }
}

impl std::error::Error for UiError {}

impl From<CommandError> for UiError {
    fn from(error: CommandError) -> Self {
        UiError::Command(error)
    }
}

#[cfg(test)]
pub(crate) mod test_util {
    //! Shared helpers for the stub tests (task P4-01).

    /// Runs `f` with a scratch [`egui::Ui`] and returns its result.
    pub(crate) fn with_ui<R>(f: impl FnOnce(&mut egui::Ui) -> R) -> R {
        let ctx = egui::Context::default();
        let mut out = None;
        let mut f = Some(f);
        let mut output = ctx.run_ui(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                if let Some(func) = f.take() {
                    out = Some(func(ui));
                }
            });
        });
        // The scratch harness draws nothing; drop the texture deltas on
        // purpose (epaint would otherwise panic on drop).
        output.textures_delta.clear();
        match out {
            Some(value) => value,
            None => panic!("the scratch ui did not run"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_carries_command_and_invocation() {
        let id = CommandId::parse("file.save").expect("id");
        let intent = Intent::new(id, Invocation::empty(ferrogram_commands::Source::Ui));
        assert_eq!(intent.command, id);
        assert_eq!(intent.invocation.source, ferrogram_commands::Source::Ui);
    }

    #[test]
    fn ui_error_displays_every_variant() {
        let command = UiError::Command(CommandError::failed("nope"));
        assert_eq!(command.to_string(), "command error: nope");
        let io = UiError::Io {
            message: "denied".into(),
        };
        assert_eq!(io.to_string(), "io error: denied");
        let render = UiError::Render {
            message: "too large".into(),
        };
        assert_eq!(render.to_string(), "render error: too large");
        assert_eq!(UiError::NotImplemented.to_string(), "not implemented yet");
    }

    #[test]
    fn command_errors_convert_into_ui_errors() {
        let error: UiError = CommandError::failed("nope").into();
        assert!(matches!(error, UiError::Command(_)));
    }
}
