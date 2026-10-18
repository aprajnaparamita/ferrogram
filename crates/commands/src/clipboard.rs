//! The internal clipboard (ADR 0047): copy, cut and paste of model
//! fragments between documents and pages.
//!
//! The clipboard lives in [`AppCtx`](crate::AppCtx) so headless callers (the CLI, the
//! control channel in Phase 8) copy and paste without a GUI. There are no
//! system clipboard formats in Phase 5 (ADR 0047): a copy stores a deep
//! clone of the copied shapes and the `Connects` records between them,
//! and a paste re-inserts them with fresh ids. Task P5-09 implements the
//! commands over this data; the data and its trivial constructors are
//! fixed here by task P5-01.

use ferrogram_model::{Connect, PageId, Shape};

/// What a copy holds (ADR 0047).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardKind {
    /// Shapes (with their sub-shapes and internal connects).
    Shapes,
}

/// One clipboard payload (ADR 0047).
#[derive(Debug, Clone, PartialEq)]
pub struct ClipboardContent {
    /// What kind of fragment this is.
    pub kind: ClipboardKind,
    /// The copied shapes, root first, then their descendants (arena
    /// order).
    pub shapes: Vec<Shape>,
    /// The `Connects` records between the copied shapes.
    pub connects: Vec<Connect>,
    /// The page the fragment was copied from (paste offsets and glue
    /// targets, ADR 0047).
    pub source_page: PageId,
}

/// The application clipboard: one payload slot (ADR 0047).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Clipboard {
    /// The copied fragment, or `None` when the clipboard is empty.
    pub content: Option<ClipboardContent>,
}

impl Clipboard {
    /// An empty clipboard.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the clipboard holds nothing.
    pub fn is_empty(&self) -> bool {
        self.content.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboards_start_empty() {
        let clipboard = Clipboard::new();
        assert!(clipboard.is_empty());
        assert_eq!(clipboard.content, None);
        let content = ClipboardContent {
            kind: ClipboardKind::Shapes,
            shapes: Vec::new(),
            connects: Vec::new(),
            source_page: PageId { index: 0 },
        };
        let clipboard = Clipboard {
            content: Some(content),
        };
        assert!(!clipboard.is_empty());
    }
}
