//! The file picker seam (ADR 0036): native dialogs on desktop, upload
//! and download on wasm.
//!
//! Only the shell calls a [`FilePicker`], and only to answer
//! [`CommandError::NeedsInput`](ferrogram_commands::CommandError::NeedsInput) from an interactive source. A
//! programmatic call never opens a dialog (AGENTS.md section 6).
//!
//! Owned by task P4-03. Alpha status (task P4-01): the interface is
//! defined as stubs.

use std::path::PathBuf;

use crate::UiError;

/// What a file picker returns (ADR 0036).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileChoice {
    /// Desktop: a real path.
    Path(PathBuf),
    /// Wasm (or any picker that returns bytes): a name and the bytes.
    Bytes {
        /// The file's display name (no directory part).
        name: String,
        /// The file's content.
        data: Vec<u8>,
    },
}

impl FileChoice {
    /// The display name of the choice.
    pub fn name(&self) -> String {
        match self {
            FileChoice::Path(path) => path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            FileChoice::Bytes { name, .. } => name.clone(),
        }
    }
}

/// A request to open one existing file (ADR 0036).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenRequest {
    /// Dialog title.
    pub title: String,
    /// Accepted extensions without the dot (`vsdx`, `vssx`, `vstx`).
    pub extensions: Vec<String>,
    /// The suggested starting path, when the command supplied one.
    pub suggested: Option<PathBuf>,
}

/// A request to save one file (ADR 0036).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveRequest {
    /// Dialog title.
    pub title: String,
    /// The default file name (e.g. `drawing.vsdx`).
    pub default_name: String,
}

/// The dialog seam (ADR 0036). Implementations: [`NativeFilePicker`]
/// (rfd) on desktop, an rfd-async bridge on wasm.
pub trait FilePicker {
    /// Asks for a file to open; `Ok(None)` when the user cancels.
    fn open(&self, request: &OpenRequest) -> Result<Option<FileChoice>, UiError>;

    /// Asks where to save `data`; `Ok(None)` when the user cancels.
    fn save(&self, request: &SaveRequest, data: &[u8]) -> Result<Option<FileChoice>, UiError>;
}

/// The native file picker (rfd, ADR 0030).
pub struct NativeFilePicker;

impl FilePicker for NativeFilePicker {
    fn open(&self, request: &OpenRequest) -> Result<Option<FileChoice>, UiError> {
        let _ = request;
        Err(UiError::NotImplemented {
            what: "NativeFilePicker::open",
        })
    }

    fn save(&self, request: &SaveRequest, data: &[u8]) -> Result<Option<FileChoice>, UiError> {
        let _ = (request, data);
        Err(UiError::NotImplemented {
            what: "NativeFilePicker::save",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_choice_names() {
        let path = FileChoice::Path(PathBuf::from("/tmp/a.vsdx"));
        assert_eq!(path.name(), "a.vsdx");
        let bytes = FileChoice::Bytes {
            name: "b.vsdx".into(),
            data: vec![1, 2],
        };
        assert_eq!(bytes.name(), "b.vsdx");
    }

    #[test]
    fn the_native_picker_is_a_stub() {
        let picker = NativeFilePicker;
        let open = OpenRequest {
            title: "Open".into(),
            extensions: vec!["vsdx".into()],
            suggested: None,
        };
        let save = SaveRequest {
            title: "Save".into(),
            default_name: "drawing.vsdx".into(),
        };
        assert!(picker.open(&open).is_err());
        assert!(picker.save(&save, b"x").is_err());
    }
}
