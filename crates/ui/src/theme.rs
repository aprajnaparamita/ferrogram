//! Theme tokens applied to egui (ADR 0063, `docs/ui/themes.md`).
//!
//! Task P7-01 defines the entry point as an interface stub; task P7-05
//! implements it. A widget asks for a token by name; a missing token
//! falls back to an egui default and warns once, so a theme file can
//! never crash the app.

use egui::Context;
use ferrogram_ui_spec::v2::Theme;

use crate::UiError;

/// Applies `theme` to egui's `Style` and `Visuals` (stub, task P7-01).
///
/// Task P7-05 maps the tokens listed in `docs/ui/themes.md` and
/// registers Selawik as the proportional UI font. Switching layout
/// re-applies its theme on the next frame.
pub fn apply(_ctx: &Context, _theme: &Theme) -> Result<(), UiError> {
    Err(UiError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    /// A minimal theme for the contract tests.
    fn theme() -> Theme {
        Theme {
            schema_version: 1,
            id: "panel-modern".to_owned(),
            name: "Panel Modern (white)".to_owned(),
            fonts: BTreeMap::new(),
            colors: BTreeMap::new(),
            metrics: BTreeMap::new(),
            contextual_tab_sets: BTreeMap::new(),
        }
    }

    #[test]
    fn apply_is_not_implemented() {
        let outcome = apply(&Context::default(), &theme());
        assert!(matches!(outcome, Err(UiError::NotImplemented)));
    }
}
