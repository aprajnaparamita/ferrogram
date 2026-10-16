//! Settings persistence and the View/Options dialog (ADR 0029,
//! ADR 0032).
//!
//! Owned by task P4-03. Alpha status (task P4-01): the interface is
//! defined as stubs.

use egui::Ui;
use ferrogram_commands::{Settings, SettingsError, SettingsStore};

use crate::UiError;

/// Loads settings through a [`SettingsStore`], falling back to
/// [`Settings::default`] on a [`SettingsError::Format`] (ADR 0032: a
/// corrupt store warns, never crashes).
///
/// Not implemented yet (P4-03).
pub fn load_settings(store: &mut dyn SettingsStore) -> Result<Settings, UiError> {
    let _ = store;
    Err(UiError::NotImplemented {
        what: "settings::load_settings",
    })
}

/// Saves settings through a [`SettingsStore`].
///
/// Not implemented yet (P4-03).
pub fn save_settings(store: &mut dyn SettingsStore, settings: &Settings) -> Result<(), UiError> {
    let _ = (store, settings);
    Err(UiError::NotImplemented {
        what: "settings::save_settings",
    })
}

/// Draws the View/Options dialog body (ADR 0032): the ribbon layout
/// radio choice, grid and rulers toggles, display units.
///
/// Returns `true` when the user changed a setting. Not implemented yet
/// (P4-03).
pub fn show_options(ui: &mut Ui, settings: &mut Settings) -> Result<bool, UiError> {
    let _ = (ui, settings);
    Err(UiError::NotImplemented {
        what: "settings::show_options",
    })
}

/// Adapts any [`SettingsError`] into a [`UiError`].
pub fn settings_error(error: SettingsError) -> UiError {
    UiError::Io {
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::with_ui;

    struct MemoryStore(Result<Settings, SettingsError>);

    impl SettingsStore for MemoryStore {
        fn load(&mut self) -> Result<Settings, SettingsError> {
            self.0.clone()
        }

        fn save(&mut self, _settings: &Settings) -> Result<(), SettingsError> {
            Ok(())
        }
    }

    #[test]
    fn settings_seams_are_stubs() {
        let mut store = MemoryStore(Ok(Settings::default()));
        assert!(load_settings(&mut store).is_err());
        assert!(save_settings(&mut store, &Settings::default()).is_err());
    }

    #[test]
    fn options_dialog_is_a_stub() {
        let mut settings = Settings::default();
        with_ui(|ui| {
            assert!(show_options(ui, &mut settings).is_err());
        });
    }

    #[test]
    fn settings_errors_wrap_into_ui_errors() {
        let error = settings_error(SettingsError::Format {
            message: "truncated".into(),
        });
        assert_eq!(error.to_string(), "io error: settings format: truncated");
    }
}
