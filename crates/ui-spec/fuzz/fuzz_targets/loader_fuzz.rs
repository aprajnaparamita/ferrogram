//! Fuzz target for the `ferrogram-ui-spec` loader (P1-08 goal 6).
//!
//! Feeds the input bytes to the loader's parse-and-validate path: the
//! first byte picks which of the four data files is fuzzed, the rest
//! of the input is that file's content. The other three files hold the
//! `tests/fixtures/good` documents, so cross-references can exercise
//! the whole pipeline. `Loader::load_all` must always answer (Ok or
//! Err) — libFuzzer reports any panic as a crash.

#![no_main]

use std::path::PathBuf;
use std::sync::OnceLock;

use ferrogram_ui_spec::Loader;
use libfuzzer_sys::fuzz_target;

/// The four data files, in loader read order.
const NAMES: [&str; 4] = [
    "panel-2016.yaml",
    "panel-365.yaml",
    "commands.yaml",
    "shortcuts.yaml",
];

/// The baseline documents for the files not being fuzzed.
const BASELINE: [&str; 4] = [
    include_str!("../../tests/fixtures/good/ui-spec/panel-2016.yaml"),
    include_str!("../../tests/fixtures/good/ui-spec/panel-365.yaml"),
    include_str!("../../tests/fixtures/good/ui-spec/commands.yaml"),
    include_str!("../../tests/fixtures/good/ui-spec/shortcuts.yaml"),
];

/// The process-unique fuzz workspace.
fn workspace() -> PathBuf {
    std::env::temp_dir().join(format!("ferrogram-ui-spec-fuzz-{}", std::process::id()))
}

/// The shared `Loader` (compiles the schema once per process).
fn loader() -> &'static Loader {
    static LOADER: OnceLock<Loader> = OnceLock::new();
    LOADER.get_or_init(|| {
        let root = workspace();
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("ui-spec")).expect("fuzz workspace");
        Loader::new(&root).expect("the embedded schema always compiles")
    })
}

fuzz_target!(|data: &[u8]| {
    let Some((first, body)) = data.split_first() else {
        return;
    };
    // Creating the loader creates the fuzz workspace directory.
    let loader = loader();
    let fuzzed = usize::from(*first) % NAMES.len();
    for (i, name) in NAMES.iter().enumerate() {
        let bytes = if i == fuzzed { body } else { BASELINE[i].as_bytes() };
        std::fs::write(workspace().join("ui-spec").join(name), bytes).expect("fuzz input written");
    }
    let _ = loader.load_all();
});
