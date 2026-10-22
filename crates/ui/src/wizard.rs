//! The welcome wizard (ADR 0066, `docs/phase-7/README.md`).
//!
//! The wizard opens on the first launch, before any document, and
//! again from **Help > Welcome**. It has two pages: Welcome, and
//! Choose your layout. Task P7-01 defines the entry point as an
//! interface stub; task P7-15 implements it.

use egui::Ui;
use ferrogram_ui_spec::v2::LayoutKind;

use crate::UiError;

/// The wizard page shown in [`show`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    /// Page 1: the picture, the title and the short description.
    Welcome,
    /// Page 2: the two layout buttons.
    ChooseLayout,
}

/// The transient state of one wizard run.
///
/// The persisted parts live in
/// [`WelcomeSettings`](crate::settings::WelcomeSettings): `welcome_seen`
/// and the default layout. [`show`] reads neither; the caller passes
/// the state in and applies the [`Outcome`] (task P7-15 wires the
/// settings, task P7-09 persists them).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    page: Page,
    default_layout: LayoutKind,
}

impl State {
    /// A fresh wizard on the Welcome page with Panel Modern selected.
    pub fn new() -> Self {
        Self {
            page: Page::Welcome,
            default_layout: LayoutKind::PanelModern,
        }
    }

    /// The page shown now.
    pub fn page(&self) -> Page {
        self.page
    }

    /// The layout the wizard would start with.
    pub fn default_layout(&self) -> LayoutKind {
        self.default_layout
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

/// What the caller must do after a [`show`] frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Cancel on either page: close the wizard and keep the default
    /// layout. The wizard is marked as seen.
    Cancel,
    /// Start on page 2: close the wizard, make `default_layout` the
    /// default layout and apply it at once. The wizard is marked as
    /// seen.
    Start {
        /// The layout the user chose.
        default_layout: LayoutKind,
    },
}

/// Draws one frame of the welcome wizard (stub, task P7-01).
///
/// Returns `Ok(None)` while the wizard is open, and an [`Outcome`]
/// when the user closes it. Next and Back move between the pages
/// inside `state`. Task P7-15 implements the two pages of
/// `docs/phase-7/README.md` section "Welcome wizard": the
/// `about_screen.svg` picture, the two layout buttons with their
/// pictures, and the **Why Larrygram?** button that calls
/// [`about::show`](crate::about::show).
pub fn show(_ui: &mut Ui, _state: &mut State) -> Result<Option<Outcome>, UiError> {
    Err(UiError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::with_ui;

    #[test]
    fn state_starts_on_the_welcome_page_with_panel_modern() {
        let state = State::new();
        assert_eq!(state.page(), Page::Welcome);
        assert_eq!(state.default_layout(), LayoutKind::PanelModern);
        assert_eq!(State::default(), state);
    }

    #[test]
    fn show_is_not_implemented() {
        let mut state = State::new();
        let outcome = with_ui(|ui| show(ui, &mut state));
        assert!(matches!(outcome, Err(UiError::NotImplemented)));
    }
}
