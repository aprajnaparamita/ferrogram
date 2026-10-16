//! The application shell (ADR 0029, ADR 0031): hosts the ribbon, the
//! canvas, the panels and the status bar, and dispatches every [`Intent`]
//! through the command registry.
//!
//! Owned by task P4-03 (window layout, menus, file open/save). Alpha
//! status (task P4-01): the interface is defined as stubs.

use egui::Context;
use ferrogram_commands::{AppCtx, CommandError, Invocation, Outcome, Registry};
use ferrogram_ui_spec::{LayoutKind, LayoutSet, LayoutSets};

use crate::{Intent, UiError};

/// The application shell (one per window, single window in Phase 4).
pub struct Shell {
    /// The command registry every intent dispatches through (ADR 0028).
    pub registry: Registry,
    /// Both ribbon layouts, loaded from `ui-spec` (ADR 0031).
    pub layouts: LayoutSets,
    /// The application state (ADR 0029).
    pub app: AppCtx,
}

impl Shell {
    /// Builds a shell from its three owned parts.
    pub fn new(registry: Registry, layouts: LayoutSets, app: AppCtx) -> Self {
        Self {
            registry,
            layouts,
            app,
        }
    }

    /// The active ribbon layout (ADR 0032).
    pub fn layout_kind(&self) -> LayoutKind {
        self.app.settings.ui_layout
    }

    /// Switches the ribbon layout without a restart (ADR 0032).
    pub fn set_layout_kind(&mut self, kind: LayoutKind) {
        self.app.settings.ui_layout = kind;
    }

    /// The layout set for the active kind (ADR 0031).
    pub fn layout_set(&self) -> &LayoutSet {
        self.layouts.get(self.layout_kind())
    }

    /// Draws one frame and returns the intents the widgets produced.
    ///
    /// Not implemented yet (P4-03).
    pub fn show(&mut self, egui_ctx: &Context) -> Result<Vec<Intent>, UiError> {
        let _ = egui_ctx;
        Err(UiError::NotImplemented {
            what: "Shell::show",
        })
    }

    /// Dispatches intents through the registry (ADR 0028), in order.
    ///
    /// Not implemented yet (P4-03).
    pub fn dispatch(
        &mut self,
        intents: &[Intent],
    ) -> Result<Vec<Result<Outcome, CommandError>>, UiError> {
        let _ = intents;
        Err(UiError::NotImplemented {
            what: "Shell::dispatch",
        })
    }
}

/// Dispatches one already-built invocation; the seam the CLI and the
/// Phase 8 control channel reuse (ADR 0028).
///
/// Not implemented yet (P4-03).
pub fn dispatch_one(
    registry: &Registry,
    app: &mut AppCtx,
    command: &ferrogram_commands::CommandId,
    invocation: Invocation,
) -> Result<Outcome, UiError> {
    let _ = (registry, app, command, invocation);
    Err(UiError::NotImplemented {
        what: "shell::dispatch_one",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell() -> Shell {
        Shell::new(
            Registry::new(),
            LayoutSets::default(),
            AppCtx::new(ferrogram_commands::Settings::default()),
        )
    }

    #[test]
    fn shell_tracks_the_layout_kind() {
        let mut shell = shell();
        assert_eq!(shell.layout_kind(), LayoutKind::Visio2016);
        shell.set_layout_kind(LayoutKind::Visio365);
        assert_eq!(shell.layout_kind(), LayoutKind::Visio365);
        assert_eq!(shell.layout_set(), &shell.layouts.visio_365);
    }

    #[test]
    fn shell_stubs_report_not_implemented() {
        let mut shell = shell();
        let ctx = Context::default();
        assert!(shell.show(&ctx).is_err());
        assert!(shell.dispatch(&[]).is_err());
    }

    #[test]
    fn dispatch_one_is_a_stub() {
        let id = ferrogram_commands::CommandId::parse("file.save").expect("id");
        let mut app = AppCtx::new(ferrogram_commands::Settings::default());
        let outcome = dispatch_one(
            &Registry::new(),
            &mut app,
            &id,
            Invocation::empty(ferrogram_commands::Source::Test),
        );
        assert!(outcome.is_err());
    }
}
