//! Errors produced while loading `ui-spec/` data.
//!
//! Every failure of [`Loader::new`](crate::Loader::new) and
//! [`Loader::load`](crate::Loader::load) is one of the [`LoadError`]
//! variants. There is no catch-all panic path: malformed input is an
//! error, never a crash (AGENTS.md section 6).

use std::path::PathBuf;

/// Everything that can go wrong while loading a [`LayoutSet`](crate::LayoutSet).
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    /// A data file could not be read (missing file, permission denied,
    /// ...).
    #[error("cannot read {}: {source}", path.display())]
    Io {
        /// The file that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// The embedded `ui-spec/schema.json` is broken, or a document
    /// failed validation against it.
    #[error("schema error for {}: {message}", path.display())]
    Schema {
        /// `ui-spec/schema.json` when the schema itself failed to
        /// compile, otherwise the data file that failed validation.
        path: PathBuf,
        /// The validator's error list, joined by `; `. Never empty for
        /// a validation failure.
        message: String,
    },

    /// A document could not be parsed into the typed model: broken
    /// YAML/JSON, or data that does not fit the public types.
    #[error("cannot parse {}: {message}", path.display())]
    Parse {
        /// The file that failed to parse.
        path: PathBuf,
        /// The parser's error message.
        message: String,
    },

    /// A cross-reference between a layout, `commands.yaml` and
    /// `shortcuts.yaml` is broken (ADR 0007 "Cross-references").
    #[error("{}", cross_reference_message(kind, id, referenced_by))]
    CrossReference {
        /// What was violated:
        ///
        /// - `"fluent_id"`: an entry in `commands.yaml` lists a Fluent
        ///   id that appears in no layout (the check runs against the
        ///   union of both layouts, per ADR 0007).
        /// - `"control_command"`: a layout control dispatches a
        ///   `CommandId` that `commands.yaml` does not define.
        /// - `"command_id"`: a shortcut in `shortcuts.yaml` dispatches
        ///   a `CommandId` that `commands.yaml` does not define.
        kind: &'static str,
        /// The unknown id (Fluent id or command id).
        id: String,
        /// The id of the referring entry: the command id for
        /// `"fluent_id"`, the control's Fluent id for
        /// `"control_command"`, and the rendered key sequence for
        /// `"command_id"`.
        referenced_by: String,
    },

    /// The document is well-formed and schema-valid, but it is not the
    /// kind of document expected at this path (e.g. `commands.yaml`
    /// holding a shortcuts list).
    #[error("{message}")]
    Unknown {
        /// What was expected and what was found.
        message: String,
    },
}

/// The `Display` text of a [`LoadError::CrossReference`].
///
/// The `fluent_id` case names the union check of ADR 0007: the id
/// appears in *no* layout, not merely in the layout being loaded.
/// Every other violation is an id no document defines.
fn cross_reference_message(kind: &str, id: &str, referenced_by: &str) -> String {
    let problem = match kind {
        "fluent_id" => "is not in any layout",
        _ => "is not known",
    };
    format!("cross-reference ({kind}): {id} (referenced by {referenced_by}) {problem}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fluent_id_violation_says_not_in_any_layout() {
        let err = LoadError::CrossReference {
            kind: "fluent_id",
            id: "GhostControl".to_owned(),
            referenced_by: "file.save".to_owned(),
        };
        assert_eq!(
            err.to_string(),
            "cross-reference (fluent_id): GhostControl \
             (referenced by file.save) is not in any layout"
        );
    }

    #[test]
    fn other_violations_say_not_known() {
        for kind in ["control_command", "command_id"] {
            let err = LoadError::CrossReference {
                kind,
                id: "edit.undo".to_owned(),
                referenced_by: "Ctrl+Z".to_owned(),
            };
            assert_eq!(
                err.to_string(),
                format!("cross-reference ({kind}): edit.undo (referenced by Ctrl+Z) is not known")
            );
        }
    }
}
