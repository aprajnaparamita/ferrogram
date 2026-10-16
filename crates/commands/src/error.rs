//! The error types of the command registry (ADR 0028).
//!
//! No error path panics (AGENTS.md section 6). Display strings are one
//! line each so the CLI and the status bar can show them verbatim.

use std::fmt;

/// A `CommandId` failed the ADR 0006 grammar, or the id pool is full.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandIdError {
    /// The rejected value.
    pub value: String,
    /// Why it was rejected.
    pub reason: &'static str,
}

impl fmt::Display for CommandIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid command id {:?}: {}", self.value, self.reason)
    }
}

impl std::error::Error for CommandIdError {}

/// A [`Registry`](crate::Registry) construction or lookup failed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RegistryError {
    /// The same id was registered twice.
    Duplicate {
        /// The duplicated id.
        id: String,
    },
    /// A YAML mapping carried a bad id.
    BadId {
        /// The bad id.
        id: String,
        /// Why it was rejected.
        reason: &'static str,
    },
    /// The registry machinery is a task P4-01 stub (ADR 0028).
    NotImplemented {
        /// The stubbed operation, e.g. `Registry::register`.
        what: &'static str,
    },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::Duplicate { id } => write!(f, "command id {id} is already registered"),
            RegistryError::BadId { id, reason } => write!(f, "bad command id {id:?}: {reason}"),
            RegistryError::NotImplemented { what } => {
                write!(f, "{what} is not implemented yet")
            }
        }
    }
}

impl std::error::Error for RegistryError {}

/// What a command needs from the user or the caller (ADR 0028).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum NeededInput {
    /// A file system path.
    Path {
        /// What the path is for.
        purpose: PathPurpose,
        /// A suggested starting path, when the command has one.
        suggested: Option<std::path::PathBuf>,
    },
    /// A line of text.
    Text {
        /// The prompt shown to interactive callers.
        prompt: String,
        /// The default value, when the command has one.
        default: Option<String>,
    },
    /// A number.
    Number {
        /// The prompt shown to interactive callers.
        prompt: String,
        /// The default value, when the command has one.
        default: Option<f64>,
    },
}

/// Why a command wants a path (ADR 0036).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PathPurpose {
    /// Opening an existing file.
    OpenFile,
    /// Writing a new file.
    SaveFile,
    /// Choosing an output directory.
    ChooseFolder,
}

impl fmt::Display for PathPurpose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathPurpose::OpenFile => f.write_str("open file"),
            PathPurpose::SaveFile => f.write_str("save file"),
            PathPurpose::ChooseFolder => f.write_str("choose folder"),
        }
    }
}

/// A dispatch or execution failure (ADR 0028).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum CommandError {
    /// The id is not in the registry.
    UnknownCommand {
        /// The unknown id.
        id: String,
    },
    /// The command is registered but its enabled-state says no.
    NotEnabled {
        /// The command id.
        id: String,
        /// The disabled reason, when the command gave one.
        reason: Option<String>,
    },
    /// A required argument is missing; supply it and dispatch again.
    NeedsInput(NeededInput),
    /// An argument has the wrong name, type or value.
    InvalidArgs {
        /// What is wrong, in one line.
        message: String,
    },
    /// The command is registered as a stub (ADR 0006).
    Unsupported {
        /// The stub command's id.
        id: String,
    },
    /// The operation is a task P4-01 stub (ADR 0028).
    NotImplemented {
        /// The stubbed operation, e.g. `UndoStack::push`.
        what: &'static str,
    },
    /// A domain failure (read, render, io), already formatted as one
    /// line by the domain crate's error.
    Failed {
        /// The formatted cause.
        message: String,
    },
}

impl CommandError {
    /// Wraps a domain error message as [`CommandError::Failed`].
    pub fn failed(message: impl Into<String>) -> Self {
        CommandError::Failed {
            message: message.into(),
        }
    }

    /// Wraps an argument problem as [`CommandError::InvalidArgs`].
    pub fn invalid_args(message: impl Into<String>) -> Self {
        CommandError::InvalidArgs {
            message: message.into(),
        }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::UnknownCommand { id } => write!(f, "unknown command {id}"),
            CommandError::NotEnabled { id, reason } => match reason {
                Some(reason) => write!(f, "command {id} is disabled: {reason}"),
                None => write!(f, "command {id} is disabled"),
            },
            CommandError::NeedsInput(needed) => match needed {
                NeededInput::Path { purpose, suggested } => match suggested {
                    Some(suggested) => {
                        write!(
                            f,
                            "command needs a path to {purpose} (suggested: {})",
                            suggested.display()
                        )
                    }
                    None => write!(f, "command needs a path to {purpose}"),
                },
                NeededInput::Text { prompt, default } => match default {
                    Some(default) => {
                        write!(f, "command needs text for {prompt} (default: {default})")
                    }
                    None => write!(f, "command needs text for {prompt}"),
                },
                NeededInput::Number { prompt, default } => match default {
                    Some(default) => write!(
                        f,
                        "command needs a number for {prompt} (default: {default})"
                    ),
                    None => write!(f, "command needs a number for {prompt}"),
                },
            },
            CommandError::InvalidArgs { message } => write!(f, "bad arguments: {message}"),
            CommandError::Unsupported { id } => write!(f, "command {id} is not implemented yet"),
            CommandError::NotImplemented { what } => write!(f, "{what} is not implemented yet"),
            CommandError::Failed { message } => f.write_str(message),
        }
    }
}

impl std::error::Error for CommandError {}

/// Loading or saving [`Settings`](crate::Settings) failed (ADR 0029).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SettingsError {
    /// The store could not be read or written.
    Io {
        /// The formatted cause.
        message: String,
    },
    /// The stored bytes are not valid settings.
    Format {
        /// The formatted cause.
        message: String,
    },
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingsError::Io { message } => write!(f, "settings store: {message}"),
            SettingsError::Format { message } => write!(f, "settings format: {message}"),
        }
    }
}

impl std::error::Error for SettingsError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn command_id_error_names_the_value() {
        let e = CommandIdError {
            value: "nope".into(),
            reason: "bad character",
        };
        assert_eq!(e.to_string(), "invalid command id \"nope\": bad character");
    }

    #[test]
    fn registry_error_variants() {
        let dup = RegistryError::Duplicate {
            id: "file.save".into(),
        };
        assert_eq!(
            dup.to_string(),
            "command id file.save is already registered"
        );
        let bad = RegistryError::BadId {
            id: "x".into(),
            reason: "empty part",
        };
        assert_eq!(bad.to_string(), "bad command id \"x\": empty part");
        let stub = RegistryError::NotImplemented {
            what: "Registry::register",
        };
        assert_eq!(
            stub.to_string(),
            "Registry::register is not implemented yet"
        );
    }

    #[test]
    fn path_purpose_displays() {
        assert_eq!(PathPurpose::OpenFile.to_string(), "open file");
        assert_eq!(PathPurpose::SaveFile.to_string(), "save file");
        assert_eq!(PathPurpose::ChooseFolder.to_string(), "choose folder");
    }

    #[test]
    fn command_error_displays_every_variant() {
        let cases: Vec<(CommandError, &str)> = vec![
            (
                CommandError::UnknownCommand {
                    id: "file.nope".into(),
                },
                "unknown command file.nope",
            ),
            (
                CommandError::NotEnabled {
                    id: "file.save".into(),
                    reason: None,
                },
                "command file.save is disabled",
            ),
            (
                CommandError::NotEnabled {
                    id: "file.save".into(),
                    reason: Some("no document".into()),
                },
                "command file.save is disabled: no document",
            ),
            (
                CommandError::NeedsInput(NeededInput::Path {
                    purpose: PathPurpose::OpenFile,
                    suggested: None,
                }),
                "command needs a path to open file",
            ),
            (
                CommandError::NeedsInput(NeededInput::Path {
                    purpose: PathPurpose::SaveFile,
                    suggested: Some(PathBuf::from("drawing.vsdx")),
                }),
                "command needs a path to save file (suggested: drawing.vsdx)",
            ),
            (
                CommandError::NeedsInput(NeededInput::Text {
                    prompt: "a name".into(),
                    default: None,
                }),
                "command needs text for a name",
            ),
            (
                CommandError::NeedsInput(NeededInput::Text {
                    prompt: "a name".into(),
                    default: Some("Page-1".into()),
                }),
                "command needs text for a name (default: Page-1)",
            ),
            (
                CommandError::NeedsInput(NeededInput::Number {
                    prompt: "a scale".into(),
                    default: None,
                }),
                "command needs a number for a scale",
            ),
            (
                CommandError::NeedsInput(NeededInput::Number {
                    prompt: "a scale".into(),
                    default: Some(96.0),
                }),
                "command needs a number for a scale (default: 96)",
            ),
            (
                CommandError::invalid_args("page must be positive"),
                "bad arguments: page must be positive",
            ),
            (
                CommandError::Unsupported {
                    id: "file.new".into(),
                },
                "command file.new is not implemented yet",
            ),
            (
                CommandError::NotImplemented {
                    what: "UndoStack::push",
                },
                "UndoStack::push is not implemented yet",
            ),
            (CommandError::failed("read failed"), "read failed"),
        ];
        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
    }

    #[test]
    fn settings_error_variants() {
        let io = SettingsError::Io {
            message: "denied".into(),
        };
        assert_eq!(io.to_string(), "settings store: denied");
        let fmt = SettingsError::Format {
            message: "truncated".into(),
        };
        assert_eq!(fmt.to_string(), "settings format: truncated");
    }
}
