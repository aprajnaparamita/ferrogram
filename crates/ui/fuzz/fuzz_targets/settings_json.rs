//! Fuzz target for the settings-slot parser (task P4-10, ADR 0032).
//!
//! The settings slot is **untrusted text**: it comes from
//! `eframe::Storage` (`localStorage` on wasm, the platform config file
//! on desktop) and anything can be in it. `TextStore::load` and
//! `load_settings` must always answer (`Ok`, or `Err` and the
//! `Settings::default()` fallback with one warning) — libFuzzer reports
//! any panic as a crash.
//!
//! The `ui-spec` embedded loader needs no target here: it parses
//! `include_str!` build-time text, which is not untrusted input; the
//! hostile-YAML case is `ferrogram-ui-spec`'s `loader_fuzz`.

#![no_main]

use ferrogram_commands::{Settings, SettingsStore};
use ferrogram_ui::settings::{TextStore, load_settings, save_settings};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // The slot is a `String` in production; invalid UTF-8 is lossy.
    let text = String::from_utf8_lossy(data).into_owned();

    // The store seam itself: the parse must answer, never panic.
    let mut store = TextStore::new(Some(text.clone()));
    let _ = store.load();
    let _ = store.take_warning();

    // The production fallback path (default + one warning).
    let mut store = TextStore::new(Some(text));
    let settings = match load_settings(&mut store) {
        Ok(settings) => settings,
        Err(_) => Settings::default(),
    };
    let _ = store.take_warning();

    // Whatever loaded must serialize back out (the slot write path).
    let mut writer = TextStore::new(None);
    let _ = save_settings(&mut writer, &settings);
    let _ = writer.take_written();
});
