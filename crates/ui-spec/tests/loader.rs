//! Integration tests for the [`Loader`], driven by the hand-written
//! fixture files in `tests/fixtures/`.
//!
//! The fixtures are small workspace roots: `good/` holds a complete
//! consistent data set (both layouts plus `commands.yaml` and
//! `shortcuts.yaml`), `xref-*/` each break one cross-reference, and
//! `bad.yaml` violates the schema.

use std::path::{Path, PathBuf};

use ferrogram_ui_spec::{CommandId, FluentId, LayoutKind, LayoutSet, LoadError, Loader};

/// The fixture directory.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// A fresh empty workspace root in the temp directory, with the
/// `ui-spec/` subdirectory created.
#[allow(clippy::unwrap_used)] // test helper; tests may panic
fn temp_workspace(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ferrogram-ui-spec-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("ui-spec")).unwrap();
    dir
}

/// Copies `commands.yaml` and `shortcuts.yaml` from the good fixture
/// into a temp workspace.
#[allow(clippy::unwrap_used)] // test helper; tests may panic
fn copy_good_tables(dir: &Path) {
    let source = fixtures().join("good").join("ui-spec");
    for name in ["commands.yaml", "shortcuts.yaml"] {
        std::fs::copy(source.join(name), dir.join("ui-spec").join(name)).unwrap();
    }
}

#[test]
fn loader_reads_both_layouts_from_the_good_fixture() {
    let loader = Loader::new(fixtures().join("good")).unwrap();
    let set_2016 = loader.load(LayoutKind::Visio2016).unwrap();
    let set_365 = loader.load(LayoutKind::Visio365).unwrap();

    // A LayoutSet has layout, commands and shortcuts all populated,
    // and layout ids are Fluent ids, not command ids.
    assert_eq!(set_2016.layout.tabs[0].id, FluentId::new("TabHome"));
    assert_eq!(set_2016.layout.tabs[1].id, FluentId::new("TabInsert"));
    assert_eq!(set_2016.commands.entries.len(), 7);
    assert_eq!(set_2016.shortcuts.entries.len(), 3);

    // Both layouts load from the same commands/shortcuts tables.
    assert_eq!(set_365.layout.tabs[0].id, FluentId::new("TabHome"));
    assert_eq!(set_365.commands.entries, set_2016.commands.entries);
    assert_eq!(set_365.shortcuts.entries, set_2016.shortcuts.entries);
}

#[test]
fn command_lookup_indexes_labels_commands_and_reverse() {
    let loader = Loader::new(fixtures().join("good")).unwrap();
    let set_2016 = loader.load(LayoutKind::Visio2016).unwrap();
    let lookup = set_2016.command_lookup();

    assert_eq!(
        lookup.labels.get(&FluentId::new("FileSave")),
        Some(&"Save".to_owned())
    );
    assert_eq!(
        lookup.commands.get(&FluentId::new("FileSave")),
        Some(&CommandId::new("file.save"))
    );
    // `file.save` is surfaced by two controls in the 2016 layout.
    assert_eq!(
        lookup.fluent_by_command.get(&CommandId::new("file.save")),
        Some(&vec![
            FluentId::new("FileSave"),
            FluentId::new("FileSaveQuickAccess")
        ])
    );
    // An alias resolves to the control's label but is not a dispatch
    // key: the primary path wins.
    assert_eq!(
        lookup.labels.get(&FluentId::new("PasteMenu")),
        Some(&"Paste".to_owned())
    );
    assert!(!lookup.commands.contains_key(&FluentId::new("PasteMenu")));
    // Tabs and groups are labelled too.
    assert_eq!(
        lookup.labels.get(&FluentId::new("GroupClipboard")),
        Some(&"Clipboard".to_owned())
    );
}

#[test]
fn select_switches_between_the_two_layouts() {
    let loader = Loader::new(fixtures().join("good")).unwrap();
    let set_2016 = loader.load(LayoutKind::Visio2016).unwrap();
    let set_365 = loader.load(LayoutKind::Visio365).unwrap();
    assert_eq!(
        LayoutSet::select(LayoutKind::Visio2016, &set_2016, &set_365),
        &set_2016
    );
    assert_eq!(
        LayoutSet::select(LayoutKind::Visio365, &set_2016, &set_365),
        &set_365
    );
}

#[test]
fn bad_fixture_is_rejected_with_a_schema_error() {
    let dir = temp_workspace("bad");
    std::fs::copy(
        fixtures().join("bad.yaml"),
        dir.join("ui-spec").join("visio-2016.yaml"),
    )
    .unwrap();
    copy_good_tables(&dir);

    let loader = Loader::new(&dir).unwrap();
    match loader.load(LayoutKind::Visio2016) {
        Err(LoadError::Schema { path, message }) => {
            assert!(path.ends_with("ui-spec/visio-2016.yaml"));
            assert!(
                !message.is_empty(),
                "the validator message must not be empty"
            );
        }
        other => panic!("expected LoadError::Schema, got {other:?}"),
    }
}

#[test]
fn unknown_fluent_id_in_commands_is_rejected() {
    let loader = Loader::new(fixtures().join("xref-fluent")).unwrap();
    match loader.load(LayoutKind::Visio2016) {
        Err(LoadError::CrossReference {
            kind,
            id,
            referenced_by,
        }) => {
            assert_eq!(kind, "fluent_id");
            assert_eq!(id, "GhostControl");
            assert_eq!(referenced_by, "file.save");
        }
        other => panic!("expected a fluent_id cross-reference error, got {other:?}"),
    }
}

#[test]
fn unknown_control_command_is_rejected() {
    let loader = Loader::new(fixtures().join("xref-control")).unwrap();
    match loader.load(LayoutKind::Visio2016) {
        Err(LoadError::CrossReference {
            kind,
            id,
            referenced_by,
        }) => {
            assert_eq!(kind, "control_command");
            assert_eq!(id, "edit.paste");
            assert_eq!(referenced_by, "PasteGallery");
        }
        other => panic!("expected a control_command cross-reference error, got {other:?}"),
    }
}

#[test]
fn unknown_shortcut_command_is_rejected() {
    let loader = Loader::new(fixtures().join("xref-shortcut")).unwrap();
    match loader.load(LayoutKind::Visio2016) {
        Err(LoadError::CrossReference {
            kind,
            id,
            referenced_by,
        }) => {
            assert_eq!(kind, "command_id");
            assert_eq!(id, "app.quit");
            assert_eq!(referenced_by, "Ctrl+Q");
        }
        other => panic!("expected a command_id cross-reference error, got {other:?}"),
    }
}

#[test]
fn json_data_files_load_too() {
    let dir = temp_workspace("json");
    let data = dir.join("ui-spec");
    let layout = r#"{
        "schema_version": 1,
        "product": "visio",
        "source": "JSON fixture",
        "source_url": "https://example.com/fixture/json",
        "source_revision": "fixture",
        "tabs": [],
        "backstage": {"kind": "backstage", "tabs": []},
        "context_menus": [],
        "quick_access_toolbar": {"id": "qat", "controls": []}
    }"#;
    std::fs::write(data.join("visio-2016.json"), layout).unwrap();
    // The schema's root `oneOf` rejects empty tables, so both get one
    // entry (kept consistent: the shortcut dispatches `edit.undo`).
    std::fs::write(
        data.join("commands.json"),
        r#"[{"id": "edit.undo", "label": "Undo", "visuicmd": null, "fluent_ids": [], "mapping_confidence": "unknown"}]"#,
    )
    .unwrap();
    std::fs::write(
        data.join("shortcuts.json"),
        r#"[{"sequence": [["Ctrl", "Z"]], "command": "edit.undo", "scope": "global"}]"#,
    )
    .unwrap();

    let loader = Loader::new(&dir).unwrap();
    let set = loader.load(LayoutKind::Visio2016).unwrap();
    assert!(set.layout.tabs.is_empty());
    assert_eq!(set.commands.entries.len(), 1);
    assert_eq!(set.shortcuts.entries.len(), 1);
}

#[test]
fn missing_data_files_report_an_io_error() {
    let dir = temp_workspace("missing");
    let loader = Loader::new(&dir).unwrap();
    match loader.load(LayoutKind::Visio2016) {
        Err(LoadError::Io { path, source }) => {
            assert!(path.ends_with("ui-spec/visio-2016.yaml"));
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }
        other => panic!("expected LoadError::Io, got {other:?}"),
    }
}

#[test]
fn wrong_document_kind_is_rejected() {
    // commands.yaml holding a shortcuts table: schema-valid (the
    // schema's oneOf allows any of the three documents at any path)
    // but not what this file must contain.
    let dir = temp_workspace("wrong-kind");
    let data = dir.join("ui-spec");
    std::fs::copy(
        fixtures().join("good/ui-spec/visio-2016.yaml"),
        data.join("visio-2016.yaml"),
    )
    .unwrap();
    std::fs::copy(
        fixtures().join("good/ui-spec/shortcuts.yaml"),
        data.join("commands.yaml"),
    )
    .unwrap();
    std::fs::copy(
        fixtures().join("good/ui-spec/shortcuts.yaml"),
        data.join("shortcuts.yaml"),
    )
    .unwrap();

    let loader = Loader::new(&dir).unwrap();
    match loader.load(LayoutKind::Visio2016) {
        Err(LoadError::Unknown { message }) => {
            assert!(message.contains("commands.yaml"), "message: {message}");
        }
        other => panic!("expected LoadError::Unknown, got {other:?}"),
    }
}

#[test]
fn controls_per_layout_are_stable() {
    // Counts reported in the task's Result section: 7 controls in the
    // 2016 fixture layout, 8 in the 365 one (RedoButton is 365-only).
    let loader = Loader::new(fixtures().join("good")).unwrap();
    let count = |set: &LayoutSet| {
        let mut n = set.layout.quick_access_toolbar.controls.len();
        for tab in &set.layout.tabs {
            for group in &tab.groups {
                n += group.controls.len();
            }
        }
        for tab in &set.layout.backstage.tabs {
            n += tab.controls.len();
        }
        for menu in &set.layout.context_menus {
            n += menu.controls.len();
        }
        n
    };
    assert_eq!(count(&loader.load(LayoutKind::Visio2016).unwrap()), 7);
    assert_eq!(count(&loader.load(LayoutKind::Visio365).unwrap()), 8);
}
