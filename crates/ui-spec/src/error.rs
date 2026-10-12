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
    #[error("cross-reference ({kind}): {id} (referenced by {referenced_by}) is not known")]
    CrossReference {
        /// What was violated:
        ///
        /// - `"fluent_id"`: an entry in `commands.yaml` lists a Fluent
        ///   id the layout does not use.
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
