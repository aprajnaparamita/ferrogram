//! Settings persistence and the View/Options dialog (ADR 0029,
//! ADR 0032).
//!
//! [`load_settings`] and [`save_settings`] are the thin seam over the
//! app's [`SettingsStore`]; the
//! store is the only thing that touches `eframe::Storage` (ADR 0032).
//! A corrupt or unreadable store falls back to [`Settings::default`]
//! with one `warning:` line (ADR 0021: bad data substitutes a default
//! and warns, never crashes); the apps record the same warning on
//! [`AppCtx::warnings`](ferrogram_commands::AppCtx) so the status bar
//! shows it.
//!
//! [`show_options`] draws the View/Options dialog body: the ribbon
//! layout radio choice (ADR 0032), the grid and rulers toggles and the
//! display units. It edits the [`Settings`] it is given and reports
//! whether anything changed; the caller marks the store dirty.

use egui::Ui;
use ferrogram_commands::{DisplayUnits, Settings, SettingsError, SettingsStore};
use ferrogram_ui_spec::LayoutKind;

use crate::UiError;

/// The settings JSON slot of the eframe storage (ADR 0032).
///
/// `eframe::Storage` is not `Send`, while
/// [`SettingsStore`] is; the apps
/// therefore read the stored slot into this plain-data store, load and
/// save through the [`SettingsStore`] seam, and write
/// the serialized text back to the slot. `crates/ui` never touches
/// eframe itself (ADR 0002).
pub struct TextStore {
    /// The stored settings text, when the slot has one.
    stored: Option<String>,
    /// The serialized settings of the last [`TextStore::save`].
    written: Option<String>,
    /// The one-line warning of a corrupt slot (ADR 0021).
    warning: Option<String>,
}

impl TextStore {
    /// A store over the text of one storage slot (`None` when empty).
    pub fn new(stored: Option<String>) -> Self {
        Self {
            stored,
            written: None,
            warning: None,
        }
    }

    /// The serialized settings to write back to the slot.
    pub fn take_written(&mut self) -> Option<String> {
        self.written.take()
    }

    /// The warning of a corrupt slot, once.
    pub fn take_warning(&mut self) -> Option<String> {
        self.warning.take()
    }
}

impl SettingsStore for TextStore {
    fn load(&mut self) -> Result<Settings, SettingsError> {
        let Some(text) = self.stored.clone() else {
            return Ok(Settings::default());
        };
        match serde_json::from_str::<Settings>(&text) {
            Ok(settings) => Ok(settings),
            Err(error) => {
                self.warning = Some(format!("settings: {error}"));
                Err(SettingsError::Format {
                    message: error.to_string(),
                })
            }
        }
    }

    fn save(&mut self, settings: &Settings) -> Result<(), SettingsError> {
        let text = serde_json::to_string(settings).map_err(|error| SettingsError::Format {
            message: error.to_string(),
        })?;
        self.written = Some(text);
        Ok(())
    }
}

/// The display units offered by the options dialog, in menu order.
///
/// `DisplayUnits` is `#[non_exhaustive]`, so the dialog lists its
/// choices explicitly instead of matching over the enum.
const UNIT_CHOICES: &[(DisplayUnits, &str)] = &[
    (DisplayUnits::Inches, "Inches"),
    (DisplayUnits::Millimeters, "Millimeters"),
    (DisplayUnits::Centimeters, "Centimeters"),
    (DisplayUnits::Meters, "Meters"),
    (DisplayUnits::Feet, "Feet"),
];

/// Loads settings through a [`SettingsStore`], falling back to
/// [`Settings::default`] on any [`SettingsError`] (ADR 0032: a corrupt
/// or missing store warns, never crashes).
///
/// The fallback prints exactly one `warning: <error>` line so the
/// failure is never silent; the caller that owns an
/// [`AppCtx`](ferrogram_commands::AppCtx) records the same text for
/// the status bar.
pub fn load_settings(store: &mut dyn SettingsStore) -> Result<Settings, UiError> {
    match store.load() {
        Ok(settings) => Ok(settings),
        Err(error) => {
            eprintln!("warning: {error}");
            Ok(Settings::default())
        }
    }
}

/// Saves settings through a [`SettingsStore`].
pub fn save_settings(store: &mut dyn SettingsStore, settings: &Settings) -> Result<(), UiError> {
    store.save(settings).map_err(settings_error)
}

/// Draws the View/Options dialog body (ADR 0032): the ribbon layout
/// radio choice, grid and rulers toggles, display units.
///
/// Returns `true` when the user changed a setting. The dialog only
/// edits `settings`; persisting is the caller's job
/// ([`save_settings`]).
pub fn show_options(ui: &mut Ui, settings: &mut Settings) -> Result<bool, UiError> {
    let before = settings.clone();

    ui.heading("Options");
    ui.separator();
    ui.label("Ribbon layout");
    ui.radio_value(
        &mut settings.ui_layout,
        LayoutKind::Visio2016,
        "Visio 2016 ribbon",
    );
    ui.radio_value(
        &mut settings.ui_layout,
        LayoutKind::Visio365,
        "Microsoft 365 ribbon",
    );
    ui.separator();
    ui.checkbox(&mut settings.show_grid, "Show grid");
    ui.checkbox(&mut settings.show_rulers, "Show rulers");
    ui.horizontal(|ui| {
        ui.label("Units");
        let choice = egui::ComboBox::from_id_salt("settings-units")
            .selected_text(unit_label(settings.units))
            .show_ui(ui, |ui| {
                for (unit, label) in UNIT_CHOICES {
                    ui.selectable_value(&mut settings.units, *unit, *label);
                }
            });
        // A combo box with no label atom has no accessible name; egui
        // documents `on_hover_text` as the fix (ADR 0031 rule).
        choice.response.on_hover_text("Display units");
    });

    Ok(settings != &before)
}

/// The display label of one unit choice, or "Inches" for a unit this
/// build does not know.
fn unit_label(units: DisplayUnits) -> &'static str {
    UNIT_CHOICES
        .iter()
        .find(|(unit, _)| *unit == units)
        .map(|(_, label)| *label)
        .unwrap_or("Inches")
}

/// Adapts any [`SettingsError`] into a [`UiError`].
pub fn settings_error(error: SettingsError) -> UiError {
    UiError::Io {
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

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

    /// A store that always fails its save with `error`.
    struct FailingSave(SettingsError);

    impl SettingsStore for FailingSave {
        fn load(&mut self) -> Result<Settings, SettingsError> {
            Ok(Settings::default())
        }

        fn save(&mut self, _settings: &Settings) -> Result<(), SettingsError> {
            Err(self.0.clone())
        }
    }

    #[test]
    fn load_returns_the_stored_settings() {
        let mut stored = Settings::default();
        stored.units = DisplayUnits::Feet;
        stored.show_grid = false;
        let mut store = MemoryStore(Ok(stored.clone()));
        let settings = load_settings(&mut store).expect("load");
        assert_eq!(settings, stored);
    }

    #[test]
    fn a_corrupt_store_falls_back_to_the_defaults() {
        let mut store = MemoryStore(Err(SettingsError::Format {
            message: "truncated".into(),
        }));
        let settings = load_settings(&mut store).expect("fallback");
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn an_unreadable_store_falls_back_to_the_defaults() {
        let mut store = MemoryStore(Err(SettingsError::Io {
            message: "denied".into(),
        }));
        let settings = load_settings(&mut store).expect("fallback");
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn save_writes_through_the_store() {
        let mut store = MemoryStore(Ok(Settings::default()));
        save_settings(&mut store, &Settings::default()).expect("save");
    }

    #[test]
    fn a_failed_save_becomes_a_ui_error() {
        let mut store = FailingSave(SettingsError::Io {
            message: "disk full".into(),
        });
        let error = save_settings(&mut store, &Settings::default()).expect_err("save fails");
        assert_eq!(error.to_string(), "io error: settings store: disk full");
    }

    #[test]
    fn a_quiet_options_dialog_changes_nothing() {
        let mut settings = Settings::default();
        with_ui(|ui| {
            let changed = show_options(ui, &mut settings).expect("the dialog draws");
            assert!(!changed);
        });
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn options_dialog_edits_the_settings() {
        use egui::accesskit::Role;
        use egui_kittest::{Harness, kittest::Queryable};

        let settings = Rc::new(RefCell::new(Settings::default()));
        let changed = Rc::new(RefCell::new(false));
        let mut harness = Harness::builder().build_ui({
            let settings = Rc::clone(&settings);
            let changed = Rc::clone(&changed);
            move |ui| {
                let mut settings = settings.borrow_mut();
                let moved = show_options(ui, &mut settings).expect("the dialog draws");
                *changed.borrow_mut() = *changed.borrow() || moved;
            }
        });
        harness.run();
        {
            let root = harness.root();
            let node = root
                .query_all_by_role_and_label(Role::RadioButton, "Microsoft 365 ribbon")
                .next()
                .expect("the 365 radio");
            node.click();
        }
        harness.run();
        assert!(*changed.borrow(), "the radio click is a change");
        assert_eq!(settings.borrow().ui_layout, LayoutKind::Visio365);
    }

    #[test]
    fn every_unit_has_a_label() {
        for (unit, label) in UNIT_CHOICES {
            assert_eq!(unit_label(*unit), *label);
        }
    }

    #[test]
    fn the_text_store_round_trips_the_settings() {
        let mut store = TextStore::new(None);
        let mut settings = Settings::default();
        settings.units = DisplayUnits::Meters;
        settings.recent_files = vec![std::path::PathBuf::from("/tmp/a.vsdx")];
        save_settings(&mut store, &settings).expect("save");
        let text = store.take_written().expect("serialized");
        let mut reloaded = TextStore::new(Some(text));
        assert_eq!(load_settings(&mut reloaded).expect("load"), settings);
    }

    #[test]
    fn an_empty_slot_loads_the_defaults() {
        let mut store = TextStore::new(None);
        assert_eq!(store.load().expect("load"), Settings::default());
        assert!(store.take_warning().is_none());
    }

    #[test]
    fn a_corrupt_slot_warns_and_falls_back() {
        let mut store = TextStore::new(Some("{not json".to_owned()));
        let error = store.load().expect_err("corrupt");
        assert!(matches!(error, SettingsError::Format { .. }));
        let warning = store.take_warning().expect("one warning");
        assert!(warning.starts_with("settings:"), "{warning}");
        assert!(store.take_warning().is_none(), "exactly one warning");
        assert_eq!(
            load_settings(&mut store).expect("fallback"),
            Settings::default()
        );
    }

    #[test]
    fn settings_errors_wrap_into_ui_errors() {
        let error = settings_error(SettingsError::Format {
            message: "truncated".into(),
        });
        assert_eq!(error.to_string(), "io error: settings format: truncated");
    }
}
