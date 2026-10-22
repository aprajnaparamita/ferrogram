//! The About window (ADR 0066, `docs/phase-7/README.md`).
//!
//! It opens from **Why Larrygram?** in the welcome wizard and from
//! **Help > About Larrygram** in every layout. Task P7-01 defines the
//! entry point as an interface stub; task P7-16 implements it.

use egui::Ui;

use crate::UiError;

/// The video link of the About window.
///
/// The `?si=...` share-tracking id is deliberately absent. The
/// button opens this URL through egui's open-URL output, which works
/// on desktop and on web.
pub const VIDEO_URL: &str = "https://youtu.be/wpgiRvcGY_U";

/// What the caller must do after a [`show`] frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The window is open and nothing was chosen this frame.
    Open,
    /// Close was pressed: hide the window.
    Close,
    /// The video button was pressed: open [`VIDEO_URL`] through egui's
    /// open-URL output.
    WatchVideo,
}

/// Draws one frame of the About window (stub, task P7-01).
///
/// Task P7-16 implements the two-column window of
/// `docs/phase-7/README.md` section "About window": `larry.png` on
/// the left, the dedication text and the video button on the right,
/// and Close at the bottom.
pub fn show(_ui: &mut Ui) -> Result<Outcome, UiError> {
    Err(UiError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::with_ui;

    #[test]
    fn video_url_has_no_tracking_id() {
        assert_eq!(VIDEO_URL, "https://youtu.be/wpgiRvcGY_U");
        assert!(!VIDEO_URL.contains("si="));
    }

    #[test]
    fn show_is_not_implemented() {
        let outcome = with_ui(show);
        assert!(matches!(outcome, Err(UiError::NotImplemented)));
    }
}
