//! The Shape Data and Size & Position panels (ADR 0035).
//!
//! Read-only in Phase 4: the rows show page data with no selection.
//! Phase 5 fills the `edit` slot of each [`PanelRow`] and the row's
//! command dispatches through the registry.
//!
//! Owned by task P4-07. Alpha status (task P4-01): the interface is
//! defined as stubs.

use egui::Ui;
use ferrogram_commands::{AppCtx, Args, CommandId};

use crate::UiError;

/// One label/value row of a panel (ADR 0035).
#[derive(Debug, Clone, PartialEq)]
pub struct PanelRow {
    /// The row label ("Width", "Name", ...).
    pub label: String,
    /// The formatted value; "—" for missing or non-finite values.
    pub value: String,
    /// The edit command and arguments for Phase 5; `None` while the
    /// panel is read-only.
    pub edit: Option<(CommandId, Args)>,
}

impl PanelRow {
    /// A read-only row.
    pub fn read_only(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            edit: None,
        }
    }
}

/// Builds the Shape Data rows (ADR 0035).
///
/// With no selection: document properties and the active page's name
/// and size. Not implemented yet (P4-07).
pub fn shape_data_rows(app: &AppCtx) -> Result<Vec<PanelRow>, UiError> {
    let _ = app;
    Err(UiError::NotImplemented {
        what: "panels::shape_data_rows",
    })
}

/// Builds the Size & Position rows (ADR 0035).
///
/// With no selection: the active page's width and height in
/// `Settings::units`. Not implemented yet (P4-07).
pub fn size_position_rows(app: &AppCtx) -> Result<Vec<PanelRow>, UiError> {
    let _ = app;
    Err(UiError::NotImplemented {
        what: "panels::size_position_rows",
    })
}

/// Draws the Shape Data panel. Not implemented yet (P4-07).
pub fn show_shape_data(ui: &mut Ui, app: &AppCtx) -> Result<(), UiError> {
    let _ = (ui, app);
    Err(UiError::NotImplemented {
        what: "panels::show_shape_data",
    })
}

/// Draws the Size & Position panel. Not implemented yet (P4-07).
pub fn show_size_position(ui: &mut Ui, app: &AppCtx) -> Result<(), UiError> {
    let _ = (ui, app);
    Err(UiError::NotImplemented {
        what: "panels::show_size_position",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::with_ui;

    #[test]
    fn a_read_only_row_has_no_edit() {
        let row = PanelRow::read_only("Width", "8.5 in");
        assert_eq!(row.label, "Width");
        assert_eq!(row.value, "8.5 in");
        assert!(row.edit.is_none());
    }

    #[test]
    fn panel_builders_are_stubs() {
        let app = AppCtx::new(ferrogram_commands::Settings::default());
        assert!(shape_data_rows(&app).is_err());
        assert!(size_position_rows(&app).is_err());
    }

    #[test]
    fn panel_widgets_are_stubs() {
        let app = AppCtx::new(ferrogram_commands::Settings::default());
        with_ui(|ui| {
            assert!(show_shape_data(ui, &app).is_err());
            assert!(show_size_position(ui, &app).is_err());
        });
    }
}
