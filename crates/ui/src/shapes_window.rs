//! The Shapes window: the stencil catalog of original masters
//! (ADR 0034).
//!
//! Owned by task P4-06. Alpha status (task P4-01): the interface is
//! defined as stubs. Dragging a master onto the page is Phase 5.

use egui::Ui;
use ferrogram_commands::AppCtx;

use crate::UiError;

/// A picked stencil master (ADR 0034).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StencilPick {
    /// The stencil's display name (e.g. "Basic Shapes").
    pub stencil: &'static str,
    /// The master's unique name (`NameU`, e.g. "Rectangle").
    pub master: &'static str,
}

/// The Shapes window widget state (ADR 0034).
#[derive(Debug, Default, PartialEq)]
pub struct ShapesWindow {
    selected: Option<StencilPick>,
}

impl ShapesWindow {
    /// An empty window with nothing selected.
    pub fn new() -> Self {
        Self::default()
    }

    /// The currently selected master, if any.
    pub fn selected(&self) -> Option<&StencilPick> {
        self.selected.as_ref()
    }

    /// Draws the stencil sections and master list with previews
    /// (ADR 0034).
    ///
    /// Returns the picks the user made this frame. Not implemented yet
    /// (P4-06).
    pub fn show(&mut self, ui: &mut Ui, app: &AppCtx) -> Result<Vec<StencilPick>, UiError> {
        let _ = (ui, app);
        Err(UiError::NotImplemented {
            what: "ShapesWindow::show",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::with_ui;

    #[test]
    fn the_window_starts_with_nothing_selected() {
        let window = ShapesWindow::new();
        assert!(window.selected().is_none());
    }

    #[test]
    fn the_window_is_a_stub() {
        let mut window = ShapesWindow::new();
        let app = AppCtx::new(ferrogram_commands::Settings::default());
        with_ui(|ui| {
            assert!(window.show(ui, &app).is_err());
        });
    }

    #[test]
    fn a_pick_names_stencil_and_master() {
        let pick = StencilPick {
            stencil: "Basic Shapes",
            master: "Rectangle",
        };
        assert_eq!(pick.stencil, "Basic Shapes");
        assert_eq!(pick.master, "Rectangle");
    }
}
