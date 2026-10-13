//! JSON Schema validator shared by every extractor (P1-03).
//!
//! Loads `ui-spec/schema.json` once and validates a file or a string
//! against it. The `jsonschema` crate is the validator (MIT), pinned
//! to the same major version as `crates/ui-spec` (P1-03 review M1).
//!
//! See ADR 0007 for the schema and ADR 0009 for how extractors call
//! [`Schema::validate_file`].

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{ExtractError, ExtractResult};

/// Compiled schema handle. Cheap to clone; holds the original
/// `serde_json::Value` of the file plus a `Validator` for the
/// full document (re-compiled sub-schemas via `jsonschema::draft7::new`).
#[derive(Clone)]
pub struct Schema {
    /// The original JSON of the schema file. Used to compile
    /// sub-schemas.
    raw: Value,
    /// Path the schema was loaded from. Used for error messages.
    source: PathBuf,
}

impl std::fmt::Debug for Schema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Schema")
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}

impl Schema {
    /// Loads `ui-spec/schema.json` from the workspace root.
    pub fn load_default() -> ExtractResult<Self> {
        let root = crate::util::root();
        let path = root.join("ui-spec").join("schema.json");
        Self::load(&path)
    }

    /// Loads a schema from a JSON file.
    pub fn load(path: &Path) -> ExtractResult<Self> {
        let text = std::fs::read_to_string(path).map_err(|e| ExtractError::Io {
            path: path.to_path_buf(),
            source: e.to_string(),
        })?;
        let value: Value = serde_json::from_str(&text).map_err(|e| ExtractError::Parse {
            path: path.to_path_buf(),
            message: format!("not valid JSON: {e}"),
        })?;
        Ok(Self {
            raw: value,
            source: path.to_path_buf(),
        })
    }

    /// Validates a YAML file already on disk. P1-03 implementation
    /// parses the YAML, then validates the chosen
    /// `$defs/{layout,commands,shortcuts}` definition. Every error
    /// names `path` (the data file), never the schema file.
    pub fn validate_file(&self, path: &Path) -> ExtractResult<()> {
        let text = std::fs::read_to_string(path).map_err(|e| ExtractError::Io {
            path: path.to_path_buf(),
            source: e.to_string(),
        })?;
        let kind = SchemaKind::for_path(path).ok_or_else(|| ExtractError::Parse {
            path: path.to_path_buf(),
            message: "cannot infer schema kind from file name".into(),
        })?;
        self.validate_str(&text, kind, path)
    }

    /// Validates an in-memory YAML string. `path` is the data file the
    /// string came from; it is used in every error message.
    pub fn validate_str(&self, text: &str, kind: SchemaKind, path: &Path) -> ExtractResult<()> {
        let value = parse_yaml_to_json(text).map_err(|e| ExtractError::Parse {
            path: path.to_path_buf(),
            message: e,
        })?;
        self.validate_json(&value, kind, path)
    }

    /// Validates an already-parsed JSON value that came from `path`.
    /// Only the two schema-file failures (a missing definition, a
    /// sub-schema that does not compile) name `self.source`.
    pub fn validate_json(&self, value: &Value, kind: SchemaKind, path: &Path) -> ExtractResult<()> {
        let def_name = kind.def_name();
        if self
            .raw
            .pointer(&format!("/definitions/{def_name}"))
            .is_none()
            && self.raw.pointer(&format!("/$defs/{def_name}")).is_none()
        {
            return Err(ExtractError::Parse {
                path: self.source.clone(),
                message: format!("schema missing definitions/{def_name}"),
            });
        }
        // Build a synthetic root that re-anchors the sub-schema via
        // `$ref` and re-supplies the document's `definitions` block.
        // jsonschema 0.58 doesn't expose a way to scope a
        // sub-schema, so we wrap it in a `{ "$ref":
        // "#/definitions/<name>", "definitions": { ... } }` document
        // and compile that.
        let mut synthetic = serde_json::Map::new();
        synthetic.insert(
            "$ref".into(),
            Value::String(format!("#/definitions/{def_name}")),
        );
        if let Some(defs) = self.raw.get("definitions") {
            synthetic.insert("definitions".into(), defs.clone());
        }
        if let Some(defs) = self.raw.get("$defs") {
            synthetic.insert("$defs".into(), defs.clone());
        }
        if let Some(s) = self.raw.get("$schema") {
            synthetic.insert("$schema".into(), s.clone());
        }
        let compiled = jsonschema::draft7::new(&Value::Object(synthetic)).map_err(|e| {
            ExtractError::Parse {
                path: self.source.clone(),
                message: format!("sub-schema compilation failed: {e}"),
            }
        })?;
        // Deduplicate messages to a small set so the error stays
        // readable (some failure modes repeat per-item).
        let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut msgs: Vec<String> = Vec::new();
        for e in compiled.iter_errors(value) {
            let s = e.to_string();
            if seen.insert(s.clone()) {
                msgs.push(s);
                if msgs.len() >= 10 {
                    msgs.push("... (truncated)".to_string());
                    break;
                }
            }
        }
        if !msgs.is_empty() {
            return Err(ExtractError::Schema {
                path: path.to_path_buf(),
                message: msgs.join("; "),
            });
        }
        Ok(())
    }
}

/// Which top-level definition to validate against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaKind {
    /// `ui-spec/panel-2016.yaml` or `ui-spec/panel-365.yaml`.
    Layout,
    /// `ui-spec/commands.yaml`.
    Commands,
    /// `ui-spec/shortcuts.yaml`.
    Shortcuts,
}

impl SchemaKind {
    /// The name of the `$defs` entry.
    pub fn def_name(self) -> &'static str {
        match self {
            Self::Layout => "layout",
            Self::Commands => "commands",
            Self::Shortcuts => "shortcuts",
        }
    }

    /// Infers the kind from the file name.
    pub fn for_path(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_str()?;
        match name {
            "panel-2016.yaml" | "panel-365.yaml" => Some(Self::Layout),
            "commands.yaml" => Some(Self::Commands),
            "shortcuts.yaml" => Some(Self::Shortcuts),
            _ => None,
        }
    }
}

/// Parses a YAML string into a `serde_json::Value` using
/// `serde-saphyr`. Returns a flat error message on failure.
fn parse_yaml_to_json(text: &str) -> Result<Value, String> {
    serde_saphyr::from_str::<Value>(text).map_err(|e| format!("YAML parse error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn workspace_root() -> PathBuf {
        crate::util::root()
    }

    #[test]
    fn schema_kind_for_path() {
        let p = Path::new("ui-spec/panel-365.yaml");
        assert_eq!(SchemaKind::for_path(p), Some(SchemaKind::Layout));
        let p = Path::new("ui-spec/commands.yaml");
        assert_eq!(SchemaKind::for_path(p), Some(SchemaKind::Commands));
        let p = Path::new("ui-spec/shortcuts.yaml");
        assert_eq!(SchemaKind::for_path(p), Some(SchemaKind::Shortcuts));
        assert_eq!(SchemaKind::for_path(Path::new("foo/bar")), None);
    }

    #[test]
    fn schema_kind_def_name() {
        assert_eq!(SchemaKind::Layout.def_name(), "layout");
        assert_eq!(SchemaKind::Commands.def_name(), "commands");
        assert_eq!(SchemaKind::Shortcuts.def_name(), "shortcuts");
    }

    #[test]
    fn load_real_schema() {
        let path = workspace_root().join("ui-spec").join("schema.json");
        if !path.is_file() {
            eprintln!("schema.json not present; skipping");
            return;
        }
        let s = Schema::load(&path).expect("schema loads");
        let _ = s.source;
    }

    #[test]
    fn validate_minimal_layout_fixture() {
        let path = workspace_root().join("ui-spec").join("schema.json");
        if !path.is_file() {
            eprintln!("schema.json not present; skipping");
            return;
        }
        let s = Schema::load(&path).expect("schema loads");

        // A hand-written minimal layout fixture (B1: control id is
        // the Fluent name `FileSave`).
        let yaml = r#"
schema_version: 1
product: visio
source: "test"
source_url: "https://example.invalid/"
source_revision: "test"
tabs:
  - id: TabHome
    label: Home
    order: 0
    bounds: null
    icon_ref: null
    keytip: null
    size_variant: null
    visual_notes: null
    groups:
      - id: GroupClipboard
        label: Clipboard
        order: 0
        bounds: null
        icon_ref: null
        keytip: null
        size_variant: null
        visual_notes: null
        controls:
          - id: FileSave
            type: button
            label: Save
            order: 0
            command: null
            bounds: null
            icon_ref: null
            keytip: null
            size_variant: null
            visual_notes: null
backstage:
  kind: backstage
  tabs: []
context_menus: []
quick_access_toolbar:
  id: qat
  controls: []
"#;
        s.validate_str(yaml, SchemaKind::Layout, Path::new("/data/panel-365.yaml"))
            .expect("minimal layout is valid");
    }

    #[test]
    fn validate_rejects_bad_control_id() {
        let path = workspace_root().join("ui-spec").join("schema.json");
        if !path.is_file() {
            eprintln!("schema.json not present; skipping");
            return;
        }
        let s = Schema::load(&path).expect("schema loads");

        // Control id "3DSave" (leading digit) violates the
        // fluentId pattern `[A-Za-z_][A-Za-z0-9_]*`.
        let yaml = r#"
schema_version: 1
product: visio
source: "test"
source_url: "https://example.invalid/"
source_revision: "test"
tabs:
  - id: TabHome
    label: Home
    order: 0
    bounds: null
    icon_ref: null
    keytip: null
    size_variant: null
    visual_notes: null
    groups:
      - id: GroupClipboard
        label: Clipboard
        order: 0
        bounds: null
        icon_ref: null
        keytip: null
        size_variant: null
        visual_notes: null
        controls:
          - id: 3DSave
            type: button
            label: Save
            order: 0
            command: null
            bounds: null
            icon_ref: null
            keytip: null
            size_variant: null
            visual_notes: null
backstage:
  kind: backstage
  tabs: []
context_menus: []
quick_access_toolbar:
  id: qat
  controls: []
"#;
        let err = s
            .validate_str(yaml, SchemaKind::Layout, Path::new("/data/panel-365.yaml"))
            .expect_err("leading-digit id should fail");
        let msg = format!("{err}");
        assert!(msg.contains("schema"), "got: {msg}");
    }

    /// R1: schema validation and parse errors name the data file, not
    /// `ui-spec/schema.json`.
    #[test]
    fn errors_name_the_data_file_not_the_schema_file() {
        let schema_path = workspace_root().join("ui-spec").join("schema.json");
        if !schema_path.is_file() {
            eprintln!("schema.json not present; skipping");
            return;
        }
        let s = Schema::load(&schema_path).expect("schema loads");
        let data = Path::new("/data/commands.yaml");

        // A YAML syntax error names the data file.
        let err = s
            .validate_str(":: not yaml ::", SchemaKind::Commands, data)
            .expect_err("bad YAML fails");
        assert_eq!(error_path(&err), Some(data), "{err}");
        assert!(!format!("{err}").contains("schema.json"), "{err}");

        // A schema violation names the data file.
        let err = s
            .validate_str("- bogus: 1\n", SchemaKind::Commands, data)
            .expect_err("bad commands fail");
        assert_eq!(error_path(&err), Some(data), "{err}");
        assert!(!format!("{err}").contains("schema.json"), "{err}");
    }

    /// The path carried by an error variant (R1 regression helper).
    fn error_path(err: &ExtractError) -> Option<&Path> {
        match err {
            ExtractError::Parse { path, .. }
            | ExtractError::Io { path, .. }
            | ExtractError::Schema { path, .. }
            | ExtractError::Missing { path } => Some(path),
            ExtractError::Unimplemented { .. } => None,
        }
    }
}
