//! JSON Schema validator shared by every extractor (P1-03).
//!
//! Loads `ui-spec/schema.json` once and validates a file or a string
//! against it. The `jsonschema` crate is the validator (MIT).
//!
//! See ADR 0007 for the schema and ADR 0009 for how extractors call
//! [`Schema::validate_file`].
//!
//! Status: stub. The P1-02 skeleton provides the function signatures
//! and the error mapping. P1-03 wires the actual `jsonschema` crate
//! and adds unit tests with a minimal fixture.

#![allow(dead_code)]

use std::path::Path;

use super::{ExtractError, ExtractResult};

/// Compiled schema handle. Cheap to clone; holds an `Arc<JSONSchema>`
/// in the P1-03 implementation.
#[derive(Clone, Default)]
pub struct Schema {
    /// Marker field; the real schema is loaded lazily in P1-03.
    _private: (),
}

impl Schema {
    /// Loads `ui-spec/schema.json` from the workspace root.
    pub fn load_default() -> ExtractResult<Self> {
        Ok(Self { _private: () })
    }

    /// Validates a YAML or JSON file already on disk. P1-03
    /// implementation parses the file, picks the right
    /// `$defs/{layout,commands,shortcuts}` definition, and returns the
    /// validator's error list joined by `; `.
    pub fn validate_file(&self, _path: &Path) -> ExtractResult<()> {
        Ok(())
    }

    /// Validates an in-memory YAML or JSON value. P1-03 implementation.
    pub fn validate_str(&self, _text: &str, _kind: SchemaKind) -> ExtractResult<()> {
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

fn _unused(_: &Path) -> ExtractResult<()> {
    Err(ExtractError::Unimplemented { task: "P1-03" })
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
