//! In-place text editing on shapes (ADR 0044).
//!
//! The editor is an egui text field over the shape's text block (the
//! canvas places it from the scene's `TextNode` bounds). On commit the
//! buffer is written as one text run ([`TextContent`], ADR 0019): run and
//! paragraph markers collapse to the first row's `IX`, which is the
//! documented Phase 5 limit until a run-aware editor lands (ADR 0044).
//! While editing, the shortcut scope is `Text` (ADR 0036). Task P5-07
//! implements the bodies.

use ferrogram_commands::AppCtx;
use ferrogram_model::{ShapeId, TextContent};

use crate::UiError;

/// The in-place text editor state (ADR 0044).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextEditor {
    /// The shape being edited, or `None` while idle.
    pub shape: Option<ShapeId>,
    /// The editable buffer (plain text, ADR 0044).
    pub buffer: String,
}

impl TextEditor {
    /// An idle editor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether an edit is in progress (the `Text` shortcut scope, ADR 0036).
    pub fn is_active(&self) -> bool {
        self.shape.is_some()
    }

    /// Opens the editor on `shape` with its current text (ADR 0044).
    ///
    /// Fails when the shape has no text sheet or the id is stale. Task
    /// P5-07 implements the body.
    pub fn begin(&mut self, shape: ShapeId, text: &TextContent) -> Result<(), UiError> {
        let _ = (shape, text);
        Err(UiError::NotImplemented)
    }

    /// Finishes the edit and returns the text to write (one run, ADR
    /// 0044), or `None` when the buffer is unchanged. Task P5-07
    /// implements the body.
    pub fn finish(&mut self) -> Result<Option<TextContent>, UiError> {
        Err(UiError::NotImplemented)
    }

    /// Abandons the edit without writing anything.
    pub fn cancel(&mut self) {
        *self = Self::new();
    }

    /// The shape's text as the editor started with it (unchanged-buffer
    /// detection). Task P5-07 implements the body.
    pub fn original_text(&self, ctx: &AppCtx) -> Result<Option<TextContent>, UiError> {
        let _ = ctx;
        Err(UiError::NotImplemented)
    }
}
