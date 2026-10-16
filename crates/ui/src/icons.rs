//! Placeholder icons for ribbon and menu controls (ADR 0030).
//!
//! Phase 4 draws simple placeholders of our own (no icon set, no Office
//! icons). The screenshot pass may replace them with an open-licensed
//! set whose licence goes in `ATTRIBUTION.md` in the same commit
//! (AGENTS.md section 4).
//!
//! Owned by task P4-04. Alpha status (task P4-01): the interface is
//! defined as a stub.

use egui::{Response, Ui};

use crate::UiError;

/// Draws the placeholder icon for a control id at `size` points.
///
/// Not implemented yet (P4-04).
pub fn placeholder(ui: &mut Ui, id: &str, size: f32) -> Result<Response, UiError> {
    let _ = (ui, id, size);
    Err(UiError::NotImplemented {
        what: "icons::placeholder",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::with_ui;

    #[test]
    fn placeholders_are_stubs() {
        with_ui(|ui| {
            assert!(placeholder(ui, "FileSave", 16.0).is_err());
        });
    }
}
