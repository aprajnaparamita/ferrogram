//! The [`Loader`]: read, validate, decode and cross-check `ui-spec/`
//! data.
//!
//! Loading is deliberately staged (ADR 0007 "Validation"):
//!
//! 1. read the file ([`LoadError::Io`]),
//! 2. parse YAML (or JSON) into a raw document ([`LoadError::Parse`]),
//! 3. validate against `ui-spec/schema.json`
//!    ([`LoadError::Schema`]),
//! 4. check the document is the kind of document this file should
//!    hold ([`LoadError::Unknown`]),
//! 5. decode into the typed model ([`LoadError::Parse`]),
//! 6. reject a `commands.yaml` id used twice ([`LoadError::Duplicate`]),
//! 7. cross-check references between the layouts, `commands.yaml` and
//!    `shortcuts.yaml` ([`LoadError::CrossReference`]).
//!
//! `commands.yaml` and `shortcuts.yaml` are shared by both layouts,
//! so every load reads *both* layout files: the `fluent_id` check
//! runs against the union of the Fluent ids of the two layouts (ADR
//! 0007 "Cross-references" 1). [`Loader::load`] returns one
//! [`LayoutSet`], [`Loader::load_all`] returns both together as
//! [`LayoutSets`].

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

use crate::error::LoadError;
use crate::lookup::all_controls;
use crate::{
    CommandId, CommandMapping, Commands, FluentId, Layout, LayoutKind, LayoutSet, LayoutSets,
    Shortcut, Shortcuts,
};

/// The JSON Schema, compiled into the binary so the loader does not
/// depend on a runtime path lookup. Kept in sync with
/// `ui-spec/schema.json` by the gate (the file is committed next to
/// the data files; ADR 0007).
const SCHEMA_JSON: &str = include_str!("../../../ui-spec/schema.json");

/// Display path used in errors for the embedded schema.
const SCHEMA_PATH: &str = "ui-spec/schema.json";

/// Subdirectory of the workspace root holding the data files.
const DATA_DIR: &str = "ui-spec";

/// The command mappings, shared by both layouts.
const COMMANDS_FILE: &str = "commands.yaml";

/// The keyboard shortcuts, shared by both layouts.
const SHORTCUTS_FILE: &str = "shortcuts.yaml";

/// The kind of document a data file holds. The schema's `oneOf`
/// allows any of the three at any path; the loader knows what each
/// file is supposed to contain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DocKind {
    /// `visio-2016.yaml` / `visio-365.yaml`: a ribbon layout.
    Layout,
    /// `commands.yaml`: the command mapping table.
    Commands,
    /// `shortcuts.yaml`: the keyboard shortcut table.
    Shortcuts,
}

impl DocKind {
    /// Short name used in error messages.
    fn noun(self) -> &'static str {
        match self {
            Self::Layout => "layout",
            Self::Commands => "commands",
            Self::Shortcuts => "shortcuts",
        }
    }
}

/// Loads the `ui-spec/` data files from a workspace root.
///
/// The schema is compiled once in [`Loader::new`] and shared by every
/// load call, so one `Loader` can serve both layouts:
///
/// ```no_run
/// use ferrogram_ui_spec::{FluentId, CommandId, LayoutKind, Loader};
///
/// let loader = Loader::new(".")?;
/// let both = loader.load_all()?;
/// let set_2016 = both.get(LayoutKind::Visio2016);
/// let lookup = set_2016.command_lookup();
/// assert_eq!(
///     lookup.commands.get(&FluentId::new("FileSave")),
///     Some(&CommandId::new("file.save")),
/// );
/// # Ok::<(), ferrogram_ui_spec::LoadError>(())
/// ```
#[derive(Debug, Clone)]
pub struct Loader {
    /// Directory containing the `ui-spec/` data files.
    workspace_root: PathBuf,
    /// The compiled schema validator (`Send + Sync`, cheap to clone).
    validator: jsonschema::Validator,
}

impl Loader {
    /// Creates a loader for the workspace rooted at `workspace_root`.
    ///
    /// Compiles `ui-spec/schema.json` (embedded in the binary) once.
    /// The only failure is [`LoadError::Schema`] when the embedded
    /// schema itself is broken.
    pub fn new(workspace_root: impl AsRef<Path>) -> Result<Self, LoadError> {
        Ok(Self {
            workspace_root: workspace_root.as_ref().to_path_buf(),
            validator: compile_schema(SCHEMA_JSON)?,
        })
    }

    /// Loads the full data set for one layout.
    ///
    /// Reads `ui-spec/<layout>.yaml` (see [`LayoutKind::file_name`])
    /// for **both** layouts, plus `ui-spec/commands.yaml` and
    /// `ui-spec/shortcuts.yaml` from the workspace root, validates
    /// each against the schema, decodes them, and cross-checks the
    /// three cross-references of ADR 0007:
    ///
    /// 1. every Fluent id listed in `commands.yaml` appears in at
    ///    least one of the two layouts (`kind: "fluent_id"`; the
    ///    shared table may legitimately name a 365-only or a
    ///    2016-only control),
    /// 2. every `command` on a control of the *requested* layout is
    ///    defined in `commands.yaml` (`kind: "control_command"`),
    /// 3. every `command` in `shortcuts.yaml` is defined in
    ///    `commands.yaml` (`kind: "command_id"`).
    ///
    /// Only the requested layout is returned; use [`Self::load_all`]
    /// to get both (its `control_command` check then covers both
    /// layouts). The data files may be YAML or JSON: a missing
    /// `.yaml` file is looked up as `.json` with the same stem.
    pub fn load(&self, kind: LayoutKind) -> Result<LayoutSet, LoadError> {
        let docs = self.read_documents()?;
        check_references(
            &[docs.layout(kind)],
            &docs.fluent_union(),
            &docs.commands,
            &docs.shortcuts,
        )?;
        Ok(docs.into_set(kind))
    }

    /// Loads the data set for **both** layouts in one pass.
    ///
    /// Reads every `ui-spec/` file exactly once and runs all three
    /// ADR 0007 cross-references over the pair: the `fluent_id` check
    /// against the union of both layouts' Fluent ids, the
    /// `control_command` check against each layout's controls, and
    /// the `command_id` check against the shared shortcut table.
    ///
    /// ```no_run
    /// # use ferrogram_ui_spec::{LayoutKind, Loader};
    /// let loader = Loader::new(".")?;
    /// let sets = loader.load_all()?;
    /// let set_365 = sets.get(LayoutKind::Visio365);
    /// # Ok::<(), ferrogram_ui_spec::LoadError>(())
    /// ```
    pub fn load_all(&self) -> Result<LayoutSets, LoadError> {
        let docs = self.read_documents()?;
        check_references(
            &[&docs.visio_2016, &docs.visio_365],
            &docs.fluent_union(),
            &docs.commands,
            &docs.shortcuts,
        )?;
        Ok(docs.into_sets())
    }

    /// Reads and validates all four data documents, in file order:
    /// both layouts, then the shared command and shortcut tables.
    fn read_documents(&self) -> Result<Documents, LoadError> {
        let visio_2016: Layout = self.load_document(
            &self.data_file(LayoutKind::Visio2016.file_name())?,
            DocKind::Layout,
        )?;
        let visio_365: Layout = self.load_document(
            &self.data_file(LayoutKind::Visio365.file_name())?,
            DocKind::Layout,
        )?;
        let commands_path = self.data_file(COMMANDS_FILE)?;
        let entries: Vec<CommandMapping> = self.load_document(&commands_path, DocKind::Commands)?;
        check_duplicate_command_ids(&entries, &commands_path)?;
        let commands = Commands { entries };
        let entries: Vec<Shortcut> =
            self.load_document(&self.data_file(SHORTCUTS_FILE)?, DocKind::Shortcuts)?;
        let shortcuts = Shortcuts { entries };
        Ok(Documents {
            visio_2016,
            visio_365,
            commands,
            shortcuts,
        })
    }

    /// Resolves one data file under `<workspace_root>/ui-spec/`,
    /// falling back from `<name>.yaml` to `<name>.json`.
    fn data_file(&self, name: &str) -> Result<PathBuf, LoadError> {
        let dir = self.workspace_root.join(DATA_DIR);
        let primary = dir.join(name);
        if primary.is_file() {
            return Ok(primary);
        }
        if let Some(stem) = name.strip_suffix(".yaml") {
            let alternative = dir.join(format!("{stem}.json"));
            if alternative.is_file() {
                return Ok(alternative);
            }
        }
        Err(LoadError::Io {
            path: primary,
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "ui-spec data file not found (expected YAML or JSON)",
            ),
        })
    }

    /// Reads and processes one document.
    fn load_document<T: DeserializeOwned>(
        &self,
        path: &Path,
        kind: DocKind,
    ) -> Result<T, LoadError> {
        let text = std::fs::read_to_string(path).map_err(|source| LoadError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        self.load_text(&text, path, kind)
    }

    /// Runs the staged load on document text already in memory.
    fn load_text<T: DeserializeOwned>(
        &self,
        text: &str,
        path: &Path,
        kind: DocKind,
    ) -> Result<T, LoadError> {
        let value = parse_document(text, path)?;
        self.validate(&value, path)?;
        if !document_matches(&value, kind) {
            return Err(LoadError::Unknown {
                path: path.to_path_buf(),
                message: format!("expected a {} document", kind.noun()),
            });
        }
        serde_json::from_value(value).map_err(|e| LoadError::Parse {
            path: path.to_path_buf(),
            message: e.to_string(),
        })
    }

    /// Validates one raw document against the schema.
    fn validate(&self, value: &serde_json::Value, path: &Path) -> Result<(), LoadError> {
        let errors: Vec<String> = self
            .validator
            .iter_errors(value)
            .map(|error| error.to_string())
            .collect();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(LoadError::Schema {
                path: path.to_path_buf(),
                message: errors.join("; "),
            })
        }
    }

    /// Loads both layout sets from the embedded YAML (ADR 0031).
    ///
    /// Same validation path as [`Self::load_all`], minus the file
    /// reads: the schema and the four data documents are compiled into
    /// the binary (the schema by `include_str!`, the documents by
    /// [`crate::embedded`]), so this works without a filesystem (the
    /// wasm build). Every document still goes through the full staged
    /// load of `load_text` (parse, schema validation, document
    /// kind, decode) and the three ADR 0007 cross-references run over
    /// the pair of layouts, exactly as on disk.
    pub fn from_embedded() -> Result<LayoutSets, LoadError> {
        use crate::embedded::{COMMANDS_YAML, SHORTCUTS_YAML, VISIO_365_YAML, VISIO_2016_YAML};
        let loader = Self {
            workspace_root: PathBuf::from(DATA_DIR),
            validator: compile_schema(SCHEMA_JSON)?,
        };
        let path = |name: &str| PathBuf::from(DATA_DIR).join(name);
        let name_2016 = LayoutKind::Visio2016.file_name();
        let name_365 = LayoutKind::Visio365.file_name();
        let visio_2016 = loader.load_text(VISIO_2016_YAML, &path(name_2016), DocKind::Layout)?;
        let visio_365 = loader.load_text(VISIO_365_YAML, &path(name_365), DocKind::Layout)?;
        let commands_path = path(COMMANDS_FILE);
        let entries: Vec<CommandMapping> =
            loader.load_text(COMMANDS_YAML, &commands_path, DocKind::Commands)?;
        check_duplicate_command_ids(&entries, &commands_path)?;
        let commands = Commands { entries };
        let entries: Vec<Shortcut> =
            loader.load_text(SHORTCUTS_YAML, &path(SHORTCUTS_FILE), DocKind::Shortcuts)?;
        let shortcuts = Shortcuts { entries };
        let docs = Documents {
            visio_2016,
            visio_365,
            commands,
            shortcuts,
        };
        let union = docs.fluent_union();
        let layouts = [&docs.visio_2016, &docs.visio_365];
        check_references(&layouts, &union, &docs.commands, &docs.shortcuts)?;
        Ok(docs.into_sets())
    }
}

/// The four data documents of one workspace, read and validated
/// together so the cross-references can see the whole data set.
struct Documents {
    /// `ui-spec/visio-2016.yaml`.
    visio_2016: Layout,
    /// `ui-spec/visio-365.yaml`.
    visio_365: Layout,
    /// `ui-spec/commands.yaml`, shared by both layouts.
    commands: Commands,
    /// `ui-spec/shortcuts.yaml`, shared by both layouts.
    shortcuts: Shortcuts,
}

impl Documents {
    /// The layout for `kind`.
    fn layout(&self, kind: LayoutKind) -> &Layout {
        match kind {
            LayoutKind::Visio2016 => &self.visio_2016,
            LayoutKind::Visio365 => &self.visio_365,
        }
    }

    /// The union of both layouts' Fluent ids (ADR 0007
    /// cross-reference 1: "must appear in at least one layout").
    fn fluent_union(&self) -> BTreeSet<FluentId> {
        collect_fluent_ids(&self.visio_2016)
            .union(&collect_fluent_ids(&self.visio_365))
            .cloned()
            .collect()
    }

    /// Turns the documents into the set for one layout.
    fn into_set(self, kind: LayoutKind) -> LayoutSet {
        let Self {
            visio_2016,
            visio_365,
            commands,
            shortcuts,
        } = self;
        let layout = match kind {
            LayoutKind::Visio2016 => visio_2016,
            LayoutKind::Visio365 => visio_365,
        };
        LayoutSet {
            layout,
            commands,
            shortcuts,
        }
    }

    /// Turns the documents into both sets.
    fn into_sets(self) -> LayoutSets {
        LayoutSets {
            visio_2016: LayoutSet {
                layout: self.visio_2016,
                commands: self.commands.clone(),
                shortcuts: self.shortcuts.clone(),
            },
            visio_365: LayoutSet {
                layout: self.visio_365,
                commands: self.commands,
                shortcuts: self.shortcuts,
            },
        }
    }
}

/// Compiles the schema document. Split out of [`Loader::new`] so the
/// failure paths are testable.
fn compile_schema(text: &str) -> Result<jsonschema::Validator, LoadError> {
    let schema: serde_json::Value = serde_json::from_str(text).map_err(|e| LoadError::Schema {
        path: PathBuf::from(SCHEMA_PATH),
        message: format!("the embedded schema is not valid JSON: {e}"),
    })?;
    jsonschema::validator_for(&schema).map_err(|e| LoadError::Schema {
        path: PathBuf::from(SCHEMA_PATH),
        message: format!("the embedded schema is not a valid JSON Schema: {e}"),
    })
}

/// Parses document text as JSON (for `.json` paths) or YAML
/// (everything else; YAML 1.2 is a superset of JSON).
fn parse_document(text: &str, path: &Path) -> Result<serde_json::Value, LoadError> {
    let is_json = path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
    let parsed: Result<serde_json::Value, String> = if is_json {
        serde_json::from_str(text).map_err(|e| e.to_string())
    } else {
        serde_saphyr::from_str(text).map_err(|e| e.to_string())
    };
    parsed.map_err(|message| LoadError::Parse {
        path: path.to_path_buf(),
        message,
    })
}

/// Checks that a schema-valid document is the kind of document the
/// file is supposed to hold. An empty list matches both tables (it
/// carries no markers).
fn document_matches(value: &serde_json::Value, kind: DocKind) -> bool {
    match kind {
        DocKind::Layout => value.get("schema_version").is_some(),
        DocKind::Commands => value.as_array().is_some_and(|entries| {
            entries
                .iter()
                .all(|entry| entry.get("fluent_ids").is_some())
        }),
        DocKind::Shortcuts => value
            .as_array()
            .is_some_and(|entries| entries.iter().all(|entry| entry.get("sequence").is_some())),
    }
}

/// Every Fluent id used by the layout: tab, group, backstage tab and
/// context-menu ids, every control id, and every control alias.
fn collect_fluent_ids(layout: &Layout) -> BTreeSet<FluentId> {
    let mut ids = BTreeSet::new();
    for tab in &layout.tabs {
        ids.insert(tab.id.clone());
        for group in &tab.groups {
            ids.insert(group.id.clone());
        }
    }
    for tab in &layout.backstage.tabs {
        ids.insert(tab.id.clone());
    }
    for menu in &layout.context_menus {
        ids.insert(menu.id.clone());
    }
    for control in all_controls(layout) {
        ids.insert(control.id.clone());
        for alias in &control.aliases {
            ids.insert(alias.clone());
        }
    }
    ids
}

/// Rejects a `commands.yaml` id used twice.
///
/// The table is one entry per Ferrogram command id (ADR 0006), so a
/// duplicate would silently merge two entries in the cross-reference
/// checks and in `CommandLookup::build`. The layout id space is
/// *not* checked here: the committed data reuses Fluent ids across
/// surfaces (see `docs/tasks/P1-14.md`, Open issues), so the master
/// decides that policy.
fn check_duplicate_command_ids(entries: &[CommandMapping], path: &Path) -> Result<(), LoadError> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for entry in entries {
        if !seen.insert(entry.id.as_str()) {
            return Err(LoadError::Duplicate {
                path: path.to_path_buf(),
                kind: "command_id",
                id: entry.id.to_string(),
            });
        }
    }
    Ok(())
}

/// Renders a shortcut's chord sequence for error messages
/// (`[[Ctrl, Z]]` becomes `Ctrl+Z`).
fn render_sequence(sequence: &[Vec<String>]) -> String {
    sequence
        .iter()
        .map(|chord| chord.join("+"))
        .collect::<Vec<String>>()
        .join(", ")
}

/// Runs the three cross-references of ADR 0007. Returns the first
/// violation in document order.
///
/// - `layouts` are the layouts being returned by the load; each one's
///   controls must dispatch a defined command (`control_command`).
///   [`Loader::load`] passes only the requested layout,
///   [`Loader::load_all`] passes both.
/// - `fluent_ids` is the union over **both** layout files: a command
///   may name a 365-only or a 2016-only control
///   (`fluent_id` fails only when the id is in no layout at all).
fn check_references(
    layouts: &[&Layout],
    fluent_ids: &BTreeSet<FluentId>,
    commands: &Commands,
    shortcuts: &Shortcuts,
) -> Result<(), LoadError> {
    // 1. Every Fluent id in commands.yaml is in at least one layout.
    for entry in &commands.entries {
        for fluent_id in &entry.fluent_ids {
            if !fluent_ids.contains(fluent_id) {
                return Err(LoadError::CrossReference {
                    kind: "fluent_id",
                    id: fluent_id.to_string(),
                    referenced_by: entry.id.to_string(),
                });
            }
        }
    }

    // 2. Every checked layout control's command is in commands.yaml.
    let command_ids: BTreeSet<&CommandId> =
        commands.entries.iter().map(|entry| &entry.id).collect();
    for layout in layouts {
        for control in all_controls(layout) {
            if let Some(command) = &control.command
                && !command_ids.contains(command)
            {
                return Err(LoadError::CrossReference {
                    kind: "control_command",
                    id: command.to_string(),
                    referenced_by: control.id.to_string(),
                });
            }
        }
    }

    // 3. Every shortcut's command is in commands.yaml.
    for shortcut in &shortcuts.entries {
        if !command_ids.contains(&shortcut.command) {
            return Err(LoadError::CrossReference {
                kind: "command_id",
                id: shortcut.command.to_string(),
                referenced_by: render_sequence(&shortcut.sequence),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Backstage, BackstageTab, ContextMenu, Control, ControlType, Group, MappingConfidence,
        QuickAccessToolbar, ShortcutScope, Tab,
    };

    fn empty_layout() -> Layout {
        Layout::default()
    }

    fn control(id: &str, command: Option<&str>) -> Control {
        Control {
            id: FluentId::new(id),
            r#type: ControlType::Button,
            label: "Label".to_owned(),
            order: 0,
            parent: None,
            command: command.map(CommandId::new),
            aliases: Vec::new(),
            bounds: None,
            icon_ref: None,
            keytip: None,
            size_variant: None,
            visual_notes: None,
        }
    }

    fn command(id: &str, fluent_ids: &[&str]) -> CommandMapping {
        CommandMapping {
            id: CommandId::new(id),
            label: "Label".to_owned(),
            visuicmd: None,
            fluent_ids: fluent_ids.iter().map(|s| FluentId::new(*s)).collect(),
            mapping_confidence: MappingConfidence::Unknown,
        }
    }

    fn shortcut(sequence: &[&str], command: &str) -> Shortcut {
        Shortcut {
            sequence: sequence
                .iter()
                .map(|chord| chord.split('+').map(str::to_owned).collect())
                .collect(),
            command: CommandId::new(command),
            scope: ShortcutScope::Global,
            section: None,
        }
    }

    fn loader() -> Loader {
        Loader::new(".").expect("the embedded schema always compiles")
    }

    /// The good fixture workspace (used by the `load_all` unit test).
    fn good_fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/good")
    }

    #[test]
    fn compile_schema_accepts_the_embedded_schema() {
        assert!(compile_schema(SCHEMA_JSON).is_ok());
    }

    #[test]
    fn compile_schema_rejects_broken_json() {
        let err = compile_schema("{not json").expect_err("must fail");
        match err {
            LoadError::Schema { path, message } => {
                assert_eq!(path, PathBuf::from(SCHEMA_PATH));
                assert!(message.contains("not valid JSON"));
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn compile_schema_rejects_a_non_schema() {
        // Valid JSON, but `type` is not a string: not a valid schema.
        let err = compile_schema(r#"{"type": 42}"#).expect_err("must fail");
        assert!(matches!(err, LoadError::Schema { .. }));
    }

    #[test]
    fn parse_document_reads_yaml() {
        let value = parse_document("a: 1\n", Path::new("x.yaml")).expect("valid yaml");
        assert_eq!(value["a"], 1);
    }

    #[test]
    fn parse_document_reads_json_paths_as_json() {
        let value = parse_document("{\"a\": [1, 2]}", Path::new("x.json")).expect("valid json");
        assert_eq!(value["a"][1], 2);
    }

    #[test]
    fn parse_document_rejects_broken_yaml() {
        let err = parse_document("a: [1\n", Path::new("x.yaml")).expect_err("must fail");
        assert!(matches!(err, LoadError::Parse { .. }));
    }

    #[test]
    fn parse_document_rejects_broken_json() {
        let err = parse_document("{oops}", Path::new("x.json")).expect_err("must fail");
        assert!(matches!(err, LoadError::Parse { .. }));
    }

    #[test]
    fn document_matches_kinds() {
        let layout = serde_json::json!({"schema_version": 1});
        let commands = serde_json::json!([{"fluent_ids": []}]);
        let shortcuts = serde_json::json!([{"sequence": [["Ctrl", "Z"]]}]);
        let empty = serde_json::json!([]);
        let scalar = serde_json::json!("nope");

        assert!(document_matches(&layout, DocKind::Layout));
        assert!(!document_matches(&layout, DocKind::Commands));
        assert!(!document_matches(&commands, DocKind::Layout));
        assert!(document_matches(&commands, DocKind::Commands));
        assert!(!document_matches(&commands, DocKind::Shortcuts));
        assert!(document_matches(&shortcuts, DocKind::Shortcuts));
        assert!(!document_matches(&shortcuts, DocKind::Commands));
        // An empty list is a valid (empty) commands or shortcuts table.
        assert!(document_matches(&empty, DocKind::Commands));
        assert!(document_matches(&empty, DocKind::Shortcuts));
        assert!(!document_matches(&empty, DocKind::Layout));
        assert!(!document_matches(&scalar, DocKind::Layout));
        assert!(!document_matches(&scalar, DocKind::Commands));
        assert!(!document_matches(&scalar, DocKind::Shortcuts));
    }

    #[test]
    fn render_sequence_formats_chords() {
        assert_eq!(
            render_sequence(&[vec!["Ctrl".into(), "Z".into()]]),
            "Ctrl+Z"
        );
        assert_eq!(
            render_sequence(&[vec!["Alt".into(), "H".into()], vec!["F".into(), "O".into()]]),
            "Alt+H, F+O"
        );
        assert_eq!(render_sequence(&[]), "");
    }

    #[test]
    fn collect_fluent_ids_covers_every_surface() {
        let layout = Layout {
            tabs: vec![Tab {
                id: FluentId::new("TabHome"),
                label: "Home".to_owned(),
                order: 0,
                tab_set: None,
                bounds: None,
                icon_ref: None,
                keytip: None,
                size_variant: None,
                visual_notes: None,
                groups: vec![Group {
                    id: FluentId::new("GroupClipboard"),
                    label: "Clipboard".to_owned(),
                    order: 0,
                    bounds: None,
                    icon_ref: None,
                    keytip: None,
                    size_variant: None,
                    visual_notes: None,
                    controls: vec![{
                        let mut paste = control("PasteGallery", Some("edit.paste"));
                        paste.aliases = vec![FluentId::new("PasteMenu")];
                        paste
                    }],
                }],
            }],
            backstage: Backstage {
                tabs: vec![BackstageTab {
                    id: FluentId::new("FileNew"),
                    label: "New".to_owned(),
                    order: 0,
                    bounds: None,
                    icon_ref: None,
                    keytip: None,
                    size_variant: None,
                    visual_notes: None,
                    controls: vec![control("FileSave", Some("file.save"))],
                }],
            },
            context_menus: vec![ContextMenu {
                id: FluentId::new("ShapeShortcutMenu"),
                label: "Menu".to_owned(),
                order: 0,
                bounds: None,
                icon_ref: None,
                keytip: None,
                size_variant: None,
                visual_notes: None,
                controls: vec![control("Cut", Some("edit.cut"))],
            }],
            quick_access_toolbar: QuickAccessToolbar {
                controls: vec![control("UndoQuickAccess", Some("edit.undo"))],
            },
            ..Layout::default()
        };
        let ids: Vec<String> = collect_fluent_ids(&layout)
            .iter()
            .map(|id| id.to_string())
            .collect();
        assert_eq!(
            ids,
            vec![
                "Cut",
                "FileNew",
                "FileSave",
                "GroupClipboard",
                "PasteGallery",
                "PasteMenu",
                "ShapeShortcutMenu",
                "TabHome",
                "UndoQuickAccess",
            ]
        );
    }

    #[test]
    fn check_references_accepts_consistent_data() {
        let layout = Layout {
            quick_access_toolbar: QuickAccessToolbar {
                controls: vec![control("FileSave", Some("file.save"))],
            },
            ..empty_layout()
        };
        let commands = Commands {
            entries: vec![command("file.save", &["FileSave"])],
        };
        let shortcuts = Shortcuts {
            entries: vec![shortcut(&["Ctrl+S"], "file.save")],
        };
        let fluent_ids = collect_fluent_ids(&layout);
        assert!(check_references(&[&layout], &fluent_ids, &commands, &shortcuts).is_ok());
    }

    #[test]
    fn check_references_accepts_an_id_only_the_other_layout_has() {
        // `commands.yaml` is shared: a command may name a control of
        // the layout being *returned* only (union reading of ADR
        // 0007 cross-reference 1).
        let checked = Layout {
            quick_access_toolbar: QuickAccessToolbar {
                controls: vec![control("FileSave", Some("file.save"))],
            },
            ..empty_layout()
        };
        let other = Layout {
            quick_access_toolbar: QuickAccessToolbar {
                controls: vec![control("RedoButton", Some("edit.redo"))],
            },
            ..empty_layout()
        };
        let fluent_ids: BTreeSet<FluentId> = collect_fluent_ids(&checked)
            .union(&collect_fluent_ids(&other))
            .cloned()
            .collect();
        let commands = Commands {
            entries: vec![
                command("file.save", &["FileSave"]),
                command("edit.redo", &["RedoButton"]),
            ],
        };
        assert!(
            check_references(&[&checked], &fluent_ids, &commands, &Shortcuts::default()).is_ok()
        );
    }

    #[test]
    fn check_references_rejects_unknown_fluent_id() {
        let layout = Layout {
            quick_access_toolbar: QuickAccessToolbar {
                controls: vec![control("FileSave", Some("file.save"))],
            },
            ..empty_layout()
        };
        let commands = Commands {
            entries: vec![command("file.save", &["FileSave", "Ghost"])],
        };
        let fluent_ids = collect_fluent_ids(&layout);
        let err = check_references(&[&layout], &fluent_ids, &commands, &Shortcuts::default())
            .expect_err("must fail");
        // The union reading: the id is in *no* layout.
        let message = err.to_string();
        assert!(message.contains("is not in any layout"), "got {message}");
        match err {
            LoadError::CrossReference {
                kind,
                id,
                referenced_by,
            } => {
                assert_eq!(kind, "fluent_id");
                assert_eq!(id, "Ghost");
                assert_eq!(referenced_by, "file.save");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn check_references_rejects_unknown_control_command() {
        let layout = Layout {
            quick_access_toolbar: QuickAccessToolbar {
                controls: vec![control("FileSave", Some("file.save"))],
            },
            ..empty_layout()
        };
        let commands = Commands {
            entries: vec![command("edit.paste", &[])],
        };
        let fluent_ids = collect_fluent_ids(&layout);
        let err = check_references(&[&layout], &fluent_ids, &commands, &Shortcuts::default())
            .expect_err("must fail");
        match err {
            LoadError::CrossReference {
                kind,
                id,
                referenced_by,
            } => {
                assert_eq!(kind, "control_command");
                assert_eq!(id, "file.save");
                assert_eq!(referenced_by, "FileSave");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn check_references_rejects_unknown_shortcut_command() {
        let commands = Commands {
            entries: vec![command("file.save", &[])],
        };
        let shortcuts = Shortcuts {
            entries: vec![shortcut(&["Ctrl+Z"], "edit.undo")],
        };
        let layout = empty_layout();
        let fluent_ids = collect_fluent_ids(&layout);
        let err = check_references(&[&layout], &fluent_ids, &commands, &shortcuts)
            .expect_err("must fail");
        match err {
            LoadError::CrossReference {
                kind,
                id,
                referenced_by,
            } => {
                assert_eq!(kind, "command_id");
                assert_eq!(id, "edit.undo");
                assert_eq!(referenced_by, "Ctrl+Z");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn load_all_returns_both_layouts_and_get_picks_one() {
        let loader = Loader::new(good_fixture()).expect("the fixture workspace loads");
        let sets = loader.load_all().expect("the good fixture is consistent");
        // Same data as loading each kind on its own.
        assert_eq!(
            sets.get(LayoutKind::Visio2016),
            &loader.load(LayoutKind::Visio2016).expect("2016 loads")
        );
        assert_eq!(
            sets.get(LayoutKind::Visio365),
            &loader.load(LayoutKind::Visio365).expect("365 loads")
        );
        // Distinct layouts, shared tables.
        assert_ne!(sets.visio_2016.layout, sets.visio_365.layout);
        assert_eq!(sets.visio_2016.commands, sets.visio_365.commands);
        assert_eq!(sets.visio_2016.shortcuts, sets.visio_365.shortcuts);
    }

    #[test]
    fn load_all_reports_the_same_fluent_id_failure_as_load() {
        let loader =
            Loader::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/xref-fluent"))
                .expect("the fixture workspace loads");
        match loader.load_all() {
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
    fn load_text_rejects_schema_violations() {
        let err = loader()
            .load_text::<Vec<Shortcut>>("{}", Path::new("shortcuts.yaml"), DocKind::Shortcuts)
            .expect_err("must fail");
        match err {
            LoadError::Schema { path, message } => {
                assert_eq!(path, PathBuf::from("shortcuts.yaml"));
                assert!(!message.is_empty());
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn load_text_rejects_the_wrong_document_kind() {
        // A shortcuts table in commands.yaml: schema-valid (both are
        // root alternatives) but not what this file should hold.
        let text = "- sequence: [[Ctrl, Z]]\n  command: edit.undo\n  scope: global\n";
        let err = loader()
            .load_text::<Vec<CommandMapping>>(text, Path::new("commands.yaml"), DocKind::Commands)
            .expect_err("must fail");
        match err {
            LoadError::Unknown { path, message } => {
                assert_eq!(path, PathBuf::from("commands.yaml"));
                assert!(message.contains("commands"), "got {message}");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn load_text_names_layout_when_a_layout_file_holds_a_table() {
        // A commands/shortcuts table in visio-2016.yaml: the error
        // says "layout" (DocKind::noun).
        let text = "[]\n";
        let err = loader()
            .load_text::<Layout>(text, Path::new("visio-2016.yaml"), DocKind::Layout)
            .expect_err("must fail");
        match err {
            LoadError::Unknown { path, message } => {
                assert_eq!(path, PathBuf::from("visio-2016.yaml"));
                assert_eq!(message, "expected a layout document");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn load_text_names_shortcuts_when_a_shortcut_file_holds_commands() {
        // A commands table in shortcuts.yaml: the error says
        // "shortcuts" (DocKind::noun).
        let text = "- id: file.save\n  label: Save\n  visuicmd: null\n  fluent_ids: [FileSave]\n  mapping_confidence: exact\n";
        let err = loader()
            .load_text::<Vec<Shortcut>>(text, Path::new("shortcuts.yaml"), DocKind::Shortcuts)
            .expect_err("must fail");
        match err {
            LoadError::Unknown { path, message } => {
                assert_eq!(path, PathBuf::from("shortcuts.yaml"));
                assert_eq!(message, "expected a shortcuts document");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn data_file_falls_through_a_missing_non_yaml_name() {
        // A name that does not end in `.yaml` has no `.json`
        // alternative to try; the miss reports the primary path.
        let err = loader().data_file("commands.toml").expect_err("must fail");
        match err {
            LoadError::Io { path, source } => {
                assert!(path.ends_with("ui-spec/commands.toml"), "got {path:?}");
                assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn data_file_prefers_yaml_and_falls_back_to_json() {
        let dir = std::env::temp_dir().join(format!(
            "ferrogram-ui-spec-data-file-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("ui-spec")).expect("temp workspace");
        let loader = Loader::new(&dir).expect("the embedded schema always compiles");
        // Missing both: the error names the .yaml path.
        assert!(loader.data_file("commands.yaml").is_err());
        // Only the .json alternative exists: the fallback finds it.
        std::fs::write(dir.join("ui-spec/commands.json"), "[]").expect("json file");
        assert!(
            loader
                .data_file("commands.yaml")
                .expect("the .json fallback")
                .ends_with("ui-spec/commands.json")
        );
        // With both present the primary .yaml path wins.
        std::fs::write(dir.join("ui-spec/commands.yaml"), "[]").expect("yaml file");
        assert!(
            loader
                .data_file("commands.yaml")
                .expect("the .yaml primary")
                .ends_with("ui-spec/commands.yaml")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn duplicate_command_ids_are_a_load_error() {
        let path = Path::new("ui-spec/commands.yaml");
        let entries = vec![command("file.save", &[]), command("edit.undo", &[])];
        assert!(check_duplicate_command_ids(&entries, path).is_ok());
        let dup = vec![
            command("file.save", &[]),
            command("edit.undo", &[]),
            command("file.save", &["FileSave"]),
        ];
        let err = check_duplicate_command_ids(&dup, path).expect_err("must fail");
        match err {
            LoadError::Duplicate { path: p, kind, id } => {
                assert_eq!(p, PathBuf::from("ui-spec/commands.yaml"));
                assert_eq!(kind, "command_id");
                assert_eq!(id, "file.save");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn load_text_rejects_values_the_model_cannot_represent() {
        // Schema-valid (`order` is any integer >= 0) but wider than
        // the typed model's `u32`: decoding must fail, not wrap.
        let layout = r#"
schema_version: 1
product: visio
source: s
source_url: https://example.com/x
source_revision: r
tabs: []
backstage:
  kind: backstage
  tabs: []
context_menus: []
quick_access_toolbar:
  id: qat
  controls:
    - id: FileSave
      type: button
      label: Save
      order: 4294967296
      command: file.save
      bounds: null
      icon_ref: null
      keytip: null
      size_variant: null
      visual_notes: null
"#;
        let err = loader()
            .load_text::<Layout>(layout, Path::new("visio-2016.yaml"), DocKind::Layout)
            .expect_err("must fail");
        assert!(matches!(err, LoadError::Parse { .. }), "got {err:?}");
    }
}
