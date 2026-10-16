//! Ferrogram desktop app (eframe/egui) (ADR 0030).
//!
//! The thin shell over `crates/ui` (L7, ADR 0002): it parses
//! `--ui-layout` (ADR 0032), builds the
//! [`Registry`](ferrogram_commands::Registry) (ADR 0028), loads both
//! ribbon layouts from the embedded `ui-spec` data (ADR 0031) and runs
//! the eframe event loop around
//! [`ferrogram_ui::shell::Shell`].
//!
//! Settings persist in eframe's storage slot as `settings.json` (ADR
//! 0032); a corrupt slot falls back to the defaults with one warning
//! (ADR 0021). The app never panics on input (AGENTS.md section 6).

#![forbid(unsafe_code)]

use std::fmt;

use eframe::egui;
use ferrogram_commands::{AppCtx, Settings};
use ferrogram_ui::settings::TextStore;
use ferrogram_ui::shell::Shell;
use ferrogram_ui_spec::{LayoutKind, Loader};

/// A startup failure (one line; the app never panics, AGENTS.md
/// section 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppError {
    /// The formatted cause.
    pub message: String,
}

impl AppError {
    /// An error from one formatted line.
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for AppError {}

/// The settings slot in the eframe storage (ADR 0032).
const SETTINGS_KEY: &str = "settings.json";

/// The app id and window title (eframe's `run_native` name).
const APP_NAME: &str = "ferrogram";

/// What the command line asked for (ADR 0032: `--ui-layout` wins over
/// the persisted setting).
#[derive(Debug, Default, PartialEq, Eq)]
struct Options {
    /// The ribbon layout from `--ui-layout`, when given.
    ui_layout: Option<LayoutKind>,
    /// `--help`: print the usage line and exit.
    help: bool,
}

/// Parses the command line (`--ui-layout 2016|365`, `--help`).
fn parse_args(args: &[String]) -> Result<Options, AppError> {
    let mut options = Options::default();
    let mut index = 0;
    while index < args.len() {
        let arg = args.get(index).map(String::as_str).unwrap_or_default();
        match arg {
            "--help" | "-h" => options.help = true,
            "--ui-layout" => {
                index += 1;
                let value = args.get(index).map(String::as_str).unwrap_or_default();
                options.ui_layout = Some(parse_layout(value)?);
            }
            other if other.starts_with("--ui-layout=") => {
                let value = other.get("--ui-layout=".len()..).unwrap_or_default();
                options.ui_layout = Some(parse_layout(value)?);
            }
            other => {
                return Err(AppError::new(format!(
                    "unknown option {other:?}; try --help"
                )));
            }
        }
        index += 1;
    }
    Ok(options)
}

/// The `--ui-layout` value: `2016` or `365` (ADR 0032).
fn parse_layout(value: &str) -> Result<LayoutKind, AppError> {
    match value {
        "2016" => Ok(LayoutKind::Visio2016),
        "365" => Ok(LayoutKind::Visio365),
        other => Err(AppError::new(format!(
            "unknown layout {other:?}; expected 2016 or 365"
        ))),
    }
}

/// The one-line usage, printed by `--help`.
fn help_text() -> &'static str {
    "usage: ferrogram [--ui-layout 2016|365] [--help]"
}

/// Builds the registry, loads both layouts and runs the eframe loop
/// (ADR 0030, ADR 0031).
fn run_with(args: &[String]) -> Result<(), AppError> {
    let options = parse_args(args)?;
    if options.help {
        println!("{}", help_text());
        return Ok(());
    }
    let registry = ferrogram_commands::builtins::registry()
        .map_err(|error| AppError::new(error.to_string()))?;
    let layouts = Loader::from_embedded().map_err(|error| AppError::new(error.to_string()))?;
    let cli_layout = options.ui_layout;
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        APP_NAME,
        native_options,
        Box::new(move |cc| {
            let shell = build_shell(cc.storage, registry, layouts, cli_layout)?;
            Ok(Box::new(FerrogramApp::new(shell)))
        }),
    )
    .map_err(|error| AppError::new(error.to_string()))
}

/// The shell for the app: settings from the storage slot (ADR 0032)
/// with the `--ui-layout` flag winning over the persisted value.
fn build_shell(
    storage: Option<&dyn eframe::Storage>,
    registry: ferrogram_commands::Registry,
    layouts: ferrogram_ui_spec::LayoutSets,
    cli_layout: Option<LayoutKind>,
) -> Result<Shell, AppError> {
    let mut slot = TextStore::new(storage.and_then(|storage| storage.get_string(SETTINGS_KEY)));
    let mut settings = ferrogram_ui::settings::load_settings(&mut slot)
        .map_err(|error| AppError::new(error.to_string()))?;
    if let Some(kind) = cli_layout {
        settings.ui_layout = kind;
    }
    let mut app = AppCtx::new(settings);
    if let Some(warning) = slot.take_warning() {
        app.record_warning(warning);
    }
    Ok(Shell::new(registry, layouts, app))
}

/// The eframe app: one [`Shell`] plus the last persisted settings.
struct FerrogramApp {
    /// The UI shell (ADR 0029).
    shell: Shell,
    /// The settings last written to the store (the dirty check).
    saved_settings: Settings,
}

impl FerrogramApp {
    /// Wraps a shell and remembers its settings.
    fn new(shell: Shell) -> Self {
        let saved_settings = shell.app.settings.clone();
        Self {
            shell,
            saved_settings,
        }
    }

    /// One UI pass: draw the frame and dispatch its intents.
    fn update_ui(&mut self, ui: &mut egui::Ui) {
        let intents = match self.shell.show_in(ui) {
            Ok(intents) => intents,
            Err(error) => {
                self.shell.note_error(error.to_string());
                Vec::new()
            }
        };
        if let Err(error) = self.shell.dispatch(&intents) {
            self.shell.note_error(error.to_string());
        }
    }

    /// Writes the settings into the storage slot (ADR 0032).
    fn persist(&mut self, storage: &mut dyn eframe::Storage) {
        let mut slot = TextStore::new(storage.get_string(SETTINGS_KEY));
        if let Err(error) =
            ferrogram_ui::settings::save_settings(&mut slot, &self.shell.app.settings)
        {
            self.shell.note_error(error.to_string());
            return;
        }
        if let Some(text) = slot.take_written() {
            storage.set_string(SETTINGS_KEY, text);
        }
    }
}

impl eframe::App for FerrogramApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.update_ui(ui);
        if self.shell.app.settings != self.saved_settings {
            if let Some(storage) = frame.storage_mut() {
                self.persist(storage);
            }
            self.saved_settings = self.shell.app.settings.clone();
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.persist(storage);
        self.saved_settings = self.shell.app.settings.clone();
    }
}

/// Entry point: prints `error: <message>` and exits non-zero on
/// failure (ADR 0026 style).
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(error) = run_with(&args) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use eframe::Storage as _;

    /// A storage slot in memory (the eframe seam).
    #[derive(Default)]
    struct MemoryStorage {
        entries: BTreeMap<String, String>,
    }

    impl eframe::Storage for MemoryStorage {
        fn get_string(&self, key: &str) -> Option<String> {
            self.entries.get(key).cloned()
        }
        fn set_string(&mut self, key: &str, value: String) {
            self.entries.insert(key.to_owned(), value);
        }
        fn remove_string(&mut self, key: &str) {
            self.entries.remove(key);
        }
        fn flush(&mut self) {}
    }

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| (*arg).to_owned()).collect()
    }

    #[test]
    fn the_command_line_parses_the_layout_flag() {
        let options = parse_args(&strings(&["--ui-layout", "365"])).expect("args");
        assert_eq!(options.ui_layout, Some(LayoutKind::Visio365));
        assert!(!options.help);

        let options = parse_args(&strings(&["--ui-layout=2016"])).expect("args");
        assert_eq!(options.ui_layout, Some(LayoutKind::Visio2016));

        let options = parse_args(&[]).expect("args");
        assert_eq!(options, Options::default());
    }

    #[test]
    fn help_is_asked_for_and_printed() {
        let options = parse_args(&strings(&["--help"])).expect("args");
        assert!(options.help);
        assert!(help_text().contains("--ui-layout"));
    }

    #[test]
    fn a_bad_command_line_names_the_problem() {
        let error = parse_args(&strings(&["--nope"])).expect_err("unknown flag");
        assert!(error.to_string().contains("--nope"), "{error}");
        let error = parse_args(&strings(&["--ui-layout"])).expect_err("missing value");
        assert!(error.to_string().contains("2016 or 365"), "{error}");
        let error = parse_args(&strings(&["--ui-layout", "95"])).expect_err("bad layout");
        assert!(error.to_string().contains("95"), "{error}");
        let error = parse_args(&strings(&["--ui-layout=nope"])).expect_err("bad layout");
        assert!(error.to_string().contains("2016 or 365"), "{error}");
    }

    #[test]
    fn help_exits_before_the_event_loop() {
        run_with(&strings(&["--help"])).expect("--help runs no window");
    }

    #[test]
    fn a_bad_command_line_never_starts_the_event_loop() {
        let error = run_with(&strings(&["--oops"])).expect_err("unknown flag");
        assert!(error.to_string().contains("--oops"));
    }

    #[test]
    fn settings_round_trip_through_the_storage_slot() {
        let mut storage = MemoryStorage::default();
        let mut app = FerrogramApp::new(
            build_shell(
                Some(&storage),
                ferrogram_commands::builtins::registry().expect("registry"),
                Loader::from_embedded().expect("layouts"),
                None,
            )
            .expect("shell"),
        );
        assert!(storage.entries.is_empty(), "nothing persisted yet");
        app.persist(&mut storage);
        let text = storage.get_string(SETTINGS_KEY).expect("stored");
        assert!(text.contains("ui_layout"), "{text}");

        let restored = build_shell(
            Some(&storage),
            ferrogram_commands::builtins::registry().expect("registry"),
            Loader::from_embedded().expect("layouts"),
            Some(LayoutKind::Visio365),
        )
        .expect("shell");
        assert_eq!(
            restored.app.settings.ui_layout,
            LayoutKind::Visio365,
            "--ui-layout wins over the persisted setting"
        );
    }

    #[test]
    fn a_corrupt_store_falls_back_with_one_warning() {
        let mut storage = MemoryStorage::default();
        storage.set_string(SETTINGS_KEY, "{not json".to_owned());
        let shell = build_shell(
            Some(&storage),
            ferrogram_commands::builtins::registry().expect("registry"),
            Loader::from_embedded().expect("layouts"),
            None,
        )
        .expect("shell");
        assert_eq!(shell.app.settings, Settings::default());
        assert_eq!(shell.app.warnings.len(), 1, "one warning");
    }

    #[test]
    fn a_missing_store_is_the_defaults_without_a_warning() {
        let shell = build_shell(
            None,
            ferrogram_commands::builtins::registry().expect("registry"),
            Loader::from_embedded().expect("layouts"),
            None,
        )
        .expect("shell");
        assert_eq!(shell.app.settings, Settings::default());
        assert!(shell.app.warnings.is_empty());
    }

    #[test]
    fn one_frame_draws_the_shell_without_crashing() {
        let shell = build_shell(
            None,
            ferrogram_commands::builtins::registry().expect("registry"),
            Loader::from_embedded().expect("layouts"),
            None,
        )
        .expect("shell");
        let mut app = FerrogramApp::new(shell);
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut output = ctx.run_ui(Default::default(), |ui| {
                app.update_ui(ui);
            });
            output.textures_delta.clear();
        }
        assert_eq!(app.saved_settings, app.shell.app.settings);
    }

    #[test]
    fn saving_persists_the_settings() {
        let mut app = FerrogramApp::new(
            build_shell(
                None,
                ferrogram_commands::builtins::registry().expect("registry"),
                Loader::from_embedded().expect("layouts"),
                None,
            )
            .expect("shell"),
        );
        let mut storage = MemoryStorage::default();
        app.shell.app.settings.show_grid = false;
        app.persist(&mut storage);
        assert!(storage.get_string(SETTINGS_KEY).is_some());
    }
}
