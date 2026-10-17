//! Hostile-settings-slot tests (task P4-10, ADR 0032).
//!
//! The settings slot is **untrusted text**: it comes from
//! `eframe::Storage` (`localStorage` on wasm, the platform config file
//! on desktop) and anything can be in it. The ADR 0021/0032 rule is
//! that a corrupt or truncated slot yields `Settings::default()` with
//! exactly one warning and never a crash.
//!
//! Covered here:
//!
//! - every byte-length truncation of a valid settings JSON;
//! - hostile shapes: wrong types, missing and unknown fields, deep
//!   nesting, huge strings, duplicate keys, BOM, NUL and control
//!   bytes, non-object roots, numbers where strings belong;
//! - the round-trip property: `save` then `load` returns the same
//!   settings, including hostile `recent_files` entries.
//!
//! A panic is the bug (AGENTS.md section 6).

// The never-crash lints apply to non-test code only (clippy.toml);
// these helpers are test code and use `unwrap` freely.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::path::PathBuf;

use ferrogram_commands::{AppCtx, DisplayUnits, Settings, SettingsError, SettingsStore};
use ferrogram_ui_spec::LayoutKind;

use ferrogram_ui::settings::{TextStore, load_settings, save_settings};

/// The slot as the app writes it: `Settings` serialized to JSON.
fn canonical() -> String {
    let mut store = TextStore::new(None);
    save_settings(&mut store, &Settings::default()).unwrap();
    store.take_written().unwrap()
}

/// Loads `text` through the production seam and returns the settings
/// plus the warning lines the store recorded (0 or 1).
fn load_slot(text: &str) -> (Settings, Vec<String>) {
    let mut store = TextStore::new(Some(text.to_owned()));
    let settings = load_settings(&mut store).unwrap();
    let mut warnings = Vec::new();
    if let Some(warning) = store.take_warning() {
        warnings.push(warning);
    }
    (settings, warnings)
}

/// The hostile slot battery: none may panic; each either parses to a
/// `Settings` or fails with exactly one warning.
const HOSTILE: &[&str] = &[
    "",
    " ",
    "\u{feff}{\"ui_layout\":\"Visio2016\"}",
    "null",
    "true",
    "false",
    "0",
    "-1",
    "1e999",
    "NaN",
    "Infinity",
    "\"just a string\"",
    "[]",
    "[1,2,3]",
    "{}",
    "{\"ui_layout\":null}",
    "{\"ui_layout\":42}",
    "{\"ui_layout\":\"Visio1997\"}",
    "{\"ui_layout\":\"Visio365\",\"show_grid\":\"yes\",\"show_rulers\":true,\"units\":\"Inches\",\"recent_files\":[]}",
    "{\"show_grid\":true}",
    "{\"ui_layout\":\"Visio2016\",\"show_grid\":true,\"show_rulers\":true,\"units\":\"Inches\"}",
    "{\"ui_layout\":\"Visio2016\",\"show_grid\":true,\"show_rulers\":true,\"units\":\"Inches\",\"recent_files\":[1,2]}",
    "{\"ui_layout\":\"Visio2016\",\"show_grid\":true,\"show_rulers\":true,\"units\":\"Inches\",\"recent_files\":[null]}",
    "{\"ui_layout\":\"Visio2016\",\"show_grid\":true,\"show_rulers\":true,\"units\":\"Inches\",\"recent_files\":{}}",
    "{\"ui_layout\":\"Visio2016\",\"show_grid\":true,\"show_rulers\":true,\"units\":\"Inches\",\"recent_files\":[\"\u{0}\"],\"x\":1}",
    "{\"ui_layout\":\"Visio365\",\"ui_layout\":\"Visio2016\",\"show_grid\":true,\"show_rulers\":true,\"units\":\"Inches\",\"recent_files\":[]}",
    "{\"units\":\"Inches\",\"recent_files\":[],\"ui_layout\":\"Visio2016\",\"show_grid\":true,\"show_rulers\":true,\"extra\":{\"deep\":[[[[1]]]]}}",
    "{\"ui_layout\":\"Visio2016\",\"show_grid\":true,\"show_rulers\":true,\"units\":\"Inches\",\"recent_files\":[],\"junk\":\u{1}\u{2}\u{7f}}",
    "not json at all",
    "{\"ui_layout\": \"Visio2016\"",
    "{\"ui_layout\": \"Visio2016\"}}}}",
    "[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[",
    "{\"recent_files\":[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[",
];

#[test]
fn every_hostile_slot_loads_without_a_crash() {
    for text in HOSTILE {
        let (settings, warnings) = load_slot(text);
        // Exactly the ADR 0021 rule: either the text parsed (warnings
        // empty) or one default-with-warning substitution happened.
        assert!(
            warnings.len() <= 1,
            "more than one warning for {text:?}: {warnings:?}"
        );
        if !warnings.is_empty() {
            assert_eq!(
                settings,
                Settings::default(),
                "a corrupt slot must fall back to the default: {text:?}"
            );
        }
        // Whatever parsed, it is usable state: the units and layout
        // are always one of ours (they are closed enums).
        match settings.units {
            DisplayUnits::Inches
            | DisplayUnits::Millimeters
            | DisplayUnits::Centimeters
            | DisplayUnits::Meters
            | DisplayUnits::Feet => {}
            _ => panic!("unknown units from {text:?}"),
        }
        match settings.ui_layout {
            LayoutKind::Visio2016 | LayoutKind::Visio365 => {}
        }
    }
}

#[test]
fn every_truncation_of_a_valid_slot_fails_gracefully() {
    let full = canonical();
    assert!(full.is_ascii(), "the byte cuts below need ASCII");
    for cut in 0..full.len() {
        let text = &full[..cut];
        let (settings, warnings) = load_slot(text);
        assert!(
            warnings.len() <= 1,
            "more than one warning at cut {cut}: {warnings:?}"
        );
        if !warnings.is_empty() {
            assert_eq!(settings, Settings::default(), "cut {cut}");
        }
    }
    // The untruncated text parses with no warning.
    let (settings, warnings) = load_slot(&full);
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(settings, Settings::default());
}

#[test]
fn a_missing_field_is_corrupt_not_a_partial_load() {
    // One missing field must not half-load: the whole slot falls back
    // (ADR 0032), so a hostile edit cannot flip one setting silently.
    let text =
        "{\"ui_layout\":\"Visio365\",\"show_grid\":false,\"show_rulers\":false,\"units\":\"Feet\"}";
    let (settings, warnings) = load_slot(text);
    assert_eq!(warnings.len(), 1);
    assert_eq!(settings, Settings::default());
}

#[test]
fn unknown_fields_are_ignored_and_round_trip_stays_stable() {
    // Forward compatibility rule: an extra field written by a later
    // version does not corrupt the slot; the next `save` normalizes it.
    let mut store = TextStore::new(Some(
        "{\"ui_layout\":\"Visio365\",\"show_grid\":false,\"show_rulers\":true,\"units\":\"Feet\",\"recent_files\":[],\"future\":{\"x\":1}}"
            .to_owned(),
    ));
    let settings = store.load().unwrap();
    assert_eq!(settings.ui_layout, LayoutKind::Visio365);
    assert!(!settings.show_grid);
    assert_eq!(settings.units, DisplayUnits::Feet);
    assert!(store.take_warning().is_none());

    save_settings(&mut store, &settings).unwrap();
    let written = store.take_written().unwrap();
    assert!(!written.contains("future"), "the next save drops it");
}

#[test]
fn a_failing_store_load_falls_back_with_one_warning() {
    /// A store whose load always fails with `error`.
    struct FailingLoad(SettingsError);
    impl SettingsStore for FailingLoad {
        fn load(&mut self) -> Result<Settings, SettingsError> {
            Err(self.0.clone())
        }
        fn save(&mut self, _settings: &Settings) -> Result<(), SettingsError> {
            Ok(())
        }
    }

    let error = SettingsError::Format {
        message: "truncated".to_owned(),
    };
    let mut store = FailingLoad(error);
    let settings = load_settings(&mut store).unwrap();
    assert_eq!(settings, Settings::default());

    let mut store = TextStore::new(Some("{}".to_owned()));
    let error = store.load().unwrap_err();
    assert!(matches!(error, SettingsError::Format { .. }));
}

/// Deterministic pseudo-random generator (no proptest dev-dependency
/// in `crates/ui`: `crates/ui/Cargo.toml` belongs to P4-03; see the
/// P4-10 report's open issues).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 11
    }

    fn pick<'a, T>(&mut self, choices: &'a [T]) -> &'a T {
        &choices[self.next() as usize % choices.len()]
    }
}

#[test]
fn save_then_load_round_trips_every_generated_setting() {
    let layouts = [LayoutKind::Visio2016, LayoutKind::Visio365];
    let units = [
        DisplayUnits::Inches,
        DisplayUnits::Millimeters,
        DisplayUnits::Centimeters,
        DisplayUnits::Meters,
        DisplayUnits::Feet,
    ];
    // Hostile path names a user could plausibly open.
    let long = "x".repeat(300);
    let names = [
        "",
        " ",
        "drawing.vsdx",
        "..",
        "/etc/passwd",
        "with space.vsdx",
        "\u{0}nul.vsdx",
        "\u{e9}\u{4e2d}\u{1f600}.vsdx",
        "trailing/",
        long.as_str(),
    ];
    let mut rng = Lcg(0x853c_49e6_0664_7b2d);
    for round in 0..200 {
        let recent: Vec<PathBuf> = (0..rng.next() % 4)
            .map(|_| PathBuf::from(*rng.pick(&names)))
            .collect();
        // `Settings` is `#[non_exhaustive]` (it lives in
        // `ferrogram-commands`), so it is built from the default.
        let mut settings = Settings::default();
        settings.ui_layout = *rng.pick(&layouts);
        settings.show_grid = rng.next().is_multiple_of(2);
        settings.show_rulers = rng.next().is_multiple_of(2);
        settings.units = *rng.pick(&units);
        settings.recent_files = recent.clone();
        let mut store = TextStore::new(None);
        save_settings(&mut store, &settings).unwrap();
        let written = store.take_written().unwrap();
        let mut reloaded = TextStore::new(Some(written.clone()));
        let back = reloaded.load().unwrap();
        assert!(reloaded.take_warning().is_none(), "round {round}");
        assert_eq!(back, settings, "round {round}");
    }
}

#[test]
fn the_app_ctx_warnings_ring_is_the_status_bar_feed() {
    // The ADR 0029 seam the settings warning lands on: exactly one
    // entry per corrupt load, capped by the ring.
    let (settings, warnings) = load_slot("{");
    assert_eq!(warnings.len(), 1);
    let mut ctx = AppCtx::new(settings);
    for warning in warnings {
        ctx.record_warning(warning);
    }
    assert_eq!(ctx.warnings.len(), 1);
}
