//! The file picker seam (ADR 0036): native dialogs on desktop, upload
//! and download on wasm.
//!
//! Only the shell calls a [`FilePicker`], and only to answer
//! [`CommandError::NeedsInput`](ferrogram_commands::CommandError::NeedsInput) from an interactive source. A
//! programmatic call never opens a dialog (AGENTS.md section 6).
//!
//! [`NativeFilePicker`] is rfd (ADR 0030): a synchronous dialog on
//! desktop and, on wasm, rfd's async upload dialog bridged back into
//! the synchronous [`FilePicker`] seam (spawn the pick, report
//! [`PickState::Pending`], hand the bytes over when they arrive).
//! Saving on wasm is a browser download of the bytes (rfd has no wasm
//! save), written here from `web-sys` object URLs.
//!
//! # Saving
//!
//! `save` asks where to put `data` and writes it when `data` is not
//! empty (a wasm download, or a desktop write after the dialog). An
//! empty `data` means "ask only": the caller re-dispatches the command
//! with the returned path and the command writes the file (the ADR
//! 0036 `NeedsInput` flow).
//!
//! Owned by task P4-03.

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

/// The state of an asynchronous pick (the wasm bridge, ADR 0036).
///
/// A desktop dialog is synchronous and never leaves [`PickState::Idle`]
/// or hands back a choice from [`FilePicker::open`]; the wasm dialog is
/// asynchronous, so `open` starts it and `poll` reports its progress.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum PickState {
    /// No pick is in flight.
    #[default]
    Idle,
    /// An asynchronous pick is in flight; ask again next frame.
    Pending,
    /// The pick finished: here is the file (consumed by `poll`).
    Picked(FileChoice),
    /// The pick finished with no file (consumed by `poll`).
    Cancelled,
}

/// The dialog seam (ADR 0036). Implementations: [`NativeFilePicker`]
/// (rfd) on desktop and on wasm (upload, download).
pub trait FilePicker {
    /// Asks for a file to open; `Ok(None)` when the user cancels (or,
    /// on wasm, when the asynchronous pick is still in flight; check
    /// [`FilePicker::poll`]).
    fn open(&self, request: &OpenRequest) -> Result<Option<FileChoice>, UiError>;

    /// Asks where to save `data`, and writes `data` there when it is
    /// not empty (see the module docs); `Ok(None)` when the user
    /// cancels.
    fn save(&self, request: &SaveRequest, data: &[u8]) -> Result<Option<FileChoice>, UiError>;

    /// Reports (and consumes) the result of an asynchronous pick.
    ///
    /// The default is [`PickState::Idle`]: a synchronous picker has
    /// nothing pending. The wasm picker reports [`PickState::Pending`]
    /// while its dialog is open and the [`PickState::Picked`] file (or
    /// [`PickState::Cancelled`]) once, when it is done.
    fn poll(&self) -> PickState {
        PickState::Idle
    }
}

/// The native file picker (rfd, ADR 0030).
pub struct NativeFilePicker;

#[cfg(not(target_arch = "wasm32"))]
impl FilePicker for NativeFilePicker {
    fn open(&self, request: &OpenRequest) -> Result<Option<FileChoice>, UiError> {
        let dialog = open_dialog(request);
        Ok(dialog.pick_file().map(FileChoice::Path))
    }

    fn save(&self, request: &SaveRequest, data: &[u8]) -> Result<Option<FileChoice>, UiError> {
        let dialog = rfd::FileDialog::new()
            .set_title(request.title.as_str())
            .set_file_name(request.default_name.as_str());
        let Some(path) = dialog.save_file() else {
            return Ok(None);
        };
        if !data.is_empty() {
            std::fs::write(&path, data).map_err(|error| UiError::Io {
                message: format!("cannot write {}: {error}", path.display()),
            })?;
        }
        Ok(Some(FileChoice::Path(path)))
    }
}

/// The rfd dialog configured from one [`OpenRequest`].
#[cfg(not(target_arch = "wasm32"))]
fn open_dialog(request: &OpenRequest) -> rfd::FileDialog {
    let mut dialog = rfd::FileDialog::new().set_title(request.title.as_str());
    if !request.extensions.is_empty() {
        dialog = dialog.add_filter("Visio files", &request.extensions);
    }
    if let Some(suggested) = &request.suggested {
        if let Some(parent) = suggested.parent() {
            dialog = dialog.set_directory(parent);
        }
        if let Some(name) = suggested.file_name() {
            dialog = dialog.set_file_name(name.to_string_lossy().as_ref());
        }
    }
    dialog
}

/// The wasm picker: rfd's async upload dialog bridged to the sync
/// seam, and a browser download for saving (ADR 0036).
#[cfg(target_arch = "wasm32")]
impl FilePicker for NativeFilePicker {
    fn open(&self, request: &OpenRequest) -> Result<Option<FileChoice>, UiError> {
        wasm_bridge::begin_open(request.clone());
        Ok(None)
    }

    fn save(&self, request: &SaveRequest, data: &[u8]) -> Result<Option<FileChoice>, UiError> {
        if data.is_empty() {
            // Nothing to download and no save dialog on wasm.
            return Ok(None);
        }
        download(&request.default_name, data)?;
        Ok(Some(FileChoice::Bytes {
            name: request.default_name.clone(),
            data: data.to_vec(),
        }))
    }

    fn poll(&self) -> PickState {
        wasm_bridge::poll()
    }
}

/// The asynchronous pick slot: one dialog at a time, polled by the
/// shell each frame (ADR 0036).
#[cfg(target_arch = "wasm32")]
mod wasm_bridge {
    use std::cell::RefCell;

    use super::{FileChoice, OpenRequest, PickState};

    thread_local! {
        /// The pick in flight and its result, when one has finished.
        static PICK: RefCell<Option<PickState>> = const { RefCell::new(None) };
    }

    /// Starts an asynchronous pick and records it as pending.
    pub(super) fn begin_open(request: OpenRequest) {
        PICK.with(|slot| {
            *slot.borrow_mut() = Some(PickState::Pending);
        });
        wasm_bindgen_futures::spawn_local(async move {
            let mut dialog = rfd::AsyncFileDialog::new().set_title(request.title.as_str());
            if !request.extensions.is_empty() {
                dialog = dialog.add_filter("Visio files", &request.extensions);
            }
            let finished = match dialog.pick_file().await {
                Some(handle) => {
                    let name = handle.file_name();
                    let data = handle.read().await;
                    PickState::Picked(FileChoice::Bytes { name, data })
                }
                None => PickState::Cancelled,
            };
            PICK.with(|slot| {
                *slot.borrow_mut() = Some(finished);
            });
        });
    }

    /// Reports the pick's state, consuming a finished one.
    pub(super) fn poll() -> PickState {
        PICK.with(|slot| {
            let mut slot = slot.borrow_mut();
            match slot.take() {
                None => PickState::Idle,
                Some(state @ (PickState::Picked(_) | PickState::Cancelled)) => state,
                Some(state) => {
                    *slot = Some(state.clone());
                    state
                }
            }
        })
    }
}

/// Downloads `data` as `name` (a browser save, ADR 0036).
#[cfg(target_arch = "wasm32")]
fn download(name: &str, data: &[u8]) -> Result<(), UiError> {
    use wasm_bindgen::JsCast;

    let window = web_sys::window().ok_or_else(|| io_error("no browser window"))?;
    let document = window
        .document()
        .ok_or_else(|| io_error("no browser document"))?;
    let parts = js_sys::Array::new();
    parts.push(&js_sys::Uint8Array::from(data));
    let blob = web_sys::Blob::new_with_u8_array_sequence(&parts).map_err(js_error)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(js_error)?;
    let anchor = document.create_element("a").map_err(js_error)?;
    anchor.set_attribute("href", &url).map_err(js_error)?;
    anchor.set_attribute("download", name).map_err(js_error)?;
    let element: web_sys::HtmlElement = anchor.dyn_into().map_err(|error| UiError::Io {
        message: format!("the download anchor: {error:?}"),
    })?;
    element.click();
    web_sys::Url::revoke_object_url(&url).map_err(js_error)?;
    Ok(())
}

/// A [`UiError::Io`] with one formatted cause.
#[cfg(target_arch = "wasm32")]
fn io_error(message: impl Into<String>) -> UiError {
    UiError::Io {
        message: message.into(),
    }
}

/// A JavaScript failure as a [`UiError::Io`] (one line).
#[cfg(target_arch = "wasm32")]
fn js_error(error: wasm_bindgen::JsValue) -> UiError {
    io_error(format!("{error:?}"))
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
    fn a_path_without_a_file_name_has_no_name() {
        let path = FileChoice::Path(PathBuf::from("/"));
        assert_eq!(path.name(), "");
    }

    #[test]
    fn pick_state_defaults_to_idle() {
        assert_eq!(PickState::default(), PickState::Idle);
        assert_eq!(PickState::Idle, PickState::Idle);
        assert_ne!(PickState::Pending, PickState::Cancelled);
        assert_ne!(
            PickState::Picked(FileChoice::Bytes {
                name: "a".into(),
                data: Vec::new()
            }),
            PickState::Cancelled
        );
    }

    #[test]
    fn the_default_poll_is_idle() {
        struct Sync;
        impl FilePicker for Sync {
            fn open(&self, _request: &OpenRequest) -> Result<Option<FileChoice>, UiError> {
                Ok(None)
            }
            fn save(
                &self,
                _request: &SaveRequest,
                _data: &[u8],
            ) -> Result<Option<FileChoice>, UiError> {
                Ok(None)
            }
        }
        assert_eq!(Sync.poll(), PickState::Idle);
    }

    #[test]
    fn requests_are_plain_data() {
        let open = OpenRequest {
            title: "Open".into(),
            extensions: vec!["vsdx".into()],
            suggested: Some(PathBuf::from("/tmp/a.vsdx")),
        };
        let save = SaveRequest {
            title: "Save".into(),
            default_name: "drawing.vsdx".into(),
        };
        assert_eq!(open, open.clone());
        assert_eq!(save, save.clone());
    }
}
