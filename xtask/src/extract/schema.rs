//! JSON Schema validator shared by every extractor (P1-03).
//!
//! Loads `ui-spec/schema.json` once and validates a file or a string
//! against it. The `jsonschema` crate is the validator (MIT).
//!
//! See ADR 0007 for the schema and ADR 0009 for how extractors call
//! [`Schema::validate_file`].

use std::path::{Path, PathBuf};
use std::sync::Arc;

use jsonschema::{Draft, JSONSchema};
use serde_json::Value;

use super::{ExtractError, ExtractResult};

/// Compiled schema handle. Cheap to clone; holds the original
/// `serde_json::Value` of the file. We re-compile sub-schemas
/// (`$defs/{layout,commands,shortcuts}`) on demand because
/// `jsonschema` 0.18 doesn't expose `$defs` lookup on the compiled
/// tree; the cost is one extra compile per validate, but the
/// schema is small.
#[derive(Clone)]
pub struct Schema {
    /// The original JSON of the schema file. Used to compile
    /// sub-schemas.
    raw: Arc<Value>,
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
    /// Loads `ui-spec/schema.json` from the workspace root. The
    /// schema is compiled with `Draft::Draft7` (matches the file's
    /// `$schema`).
    pub fn load_default() -> ExtractResult<Self> {
        let root = crate::util::root();
        let path = root.join("ui-spec").join("schema.json");
        Self::load(&path)
    }

    /// Loads a schema from a JSON file. Sub-schemas are compiled on
    /// demand in [`Schema::validate_json`].
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
            raw: Arc::new(value),
            source: path.to_path_buf(),
        })
    }

    /// Validates a YAML file already on disk. P1-03 implementation
    /// parses the YAML, then validates the chosen
    /// `$defs/{layout,commands,shortcuts}` definition.
    pub fn validate_file(&self, path: &Path) -> ExtractResult<()> {
        let text = std::fs::read_to_string(path).map_err(|e| ExtractError::Io {
            path: path.to_path_buf(),
            source: e.to_string(),
        })?;
        let kind = SchemaKind::for_path(path).ok_or_else(|| ExtractError::Parse {
            path: path.to_path_buf(),
            message: "cannot infer schema kind from file name".into(),
        })?;
        self.validate_str(&text, kind)
    }

    /// Validates an in-memory YAML string.
    pub fn validate_str(&self, text: &str, kind: SchemaKind) -> ExtractResult<()> {
        let value = parse_yaml_to_json(text).map_err(|e| ExtractError::Parse {
            path: self.source.clone(),
            message: e,
        })?;
        self.validate_json(&value, kind)
    }

    /// Validates an already-parsed JSON value.
    pub fn validate_json(&self, value: &Value, kind: SchemaKind) -> ExtractResult<()> {
        // We validate by compiling a synthetic root schema that
        // re-exports the `definitions` block plus a top-level
        // `$ref` to the target definition. The `oneOf`/nested
        // `$ref`s in the target resolve against the re-exported
        // definitions.
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
        let compiled = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&Value::Object(synthetic))
            .map_err(|e| ExtractError::Parse {
                path: self.source.clone(),
                message: format!("sub-schema compilation failed: {e}"),
            })?;
        if let Err(errors) = compiled.validate(value) {
            // Deduplicate messages to a small set so the error
            // stays readable (some failure modes repeat per-item).
            let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
            let mut msgs: Vec<String> = Vec::new();
            for e in errors {
                let s = e.to_string();
                if seen.insert(s.clone()) {
                    msgs.push(s);
                    if msgs.len() >= 10 {
                        msgs.push("... (truncated)".to_string());
                        break;
                    }
                }
            }
            return Err(ExtractError::Schema {
                path: self.source.clone(),
                message: msgs.join("; "),
            });
        }
        Ok(())
    }

    /// Borrow the original raw schema value. Used by the dispatcher
    /// when re-validating without recompiling.
    #[allow(dead_code)]
    pub fn raw(&self) -> &Value {
        &self.raw
    }
}

/// Which top-level definition to validate against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaKind {
    /// `ui-spec/visio-2016.yaml` or `ui-spec/visio-365.yaml`.
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
            "visio-2016.yaml" | "visio-365.yaml" => Some(Self::Layout),
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
        let p = Path::new("ui-spec/visio-365.yaml");
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
        s.validate_str(yaml, SchemaKind::Layout)
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
            .validate_str(yaml, SchemaKind::Layout)
            .expect_err("leading-digit id should fail");
        let msg = format!("{err}");
        assert!(msg.contains("schema"), "got: {msg}");
    }
}
