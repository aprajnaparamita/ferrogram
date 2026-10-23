//! The Panel Classic widgets: menu bar, menus, toolbars, context menus
//! (ADR 0061, `docs/ui/panel-classic.md`).
//!
//! Task P7-01 defines the entry point as an interface stub; task P7-06
//! implements it. The widgets read a [`LayoutV2`] and return
//! [`Intent`]s; they never execute commands themselves (ADR 0028).

use egui::Ui;
use larrygram_ui_spec::v2::LayoutV2;

use crate::{Intent, UiError};

/// Draws the Panel Classic chrome (menu bar, docked toolbars) and its
/// context menus, and returns one [`Intent`] per activation (stub,
/// task P7-01).
///
/// Task P7-06 implements the menu bar (access keys, hover switching),
/// the menus (icon gutter, shortcuts, submenus, run-time lists), the
/// toolbars (split buttons, palettes, combos) and the context menus,
/// as described in `docs/ui/panel-classic.md`.
pub fn show(_ui: &mut Ui, _layout: &LayoutV2) -> Result<Vec<Intent>, UiError> {
    Err(UiError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::with_ui;

    #[test]
    fn show_is_not_implemented() {
        let layout: LayoutV2 = serde_json::from_value(serde_json::json!({
            "schema_version": 2, "id": "panel-classic", "name": "Panel Classic",
            "kind": "panel-classic", "theme": "panel-classic",
            "window": {"regions": []}
        }))
        .expect("a minimal layout");
        let outcome = with_ui(|ui| show(ui, &layout));
        assert!(matches!(outcome, Err(UiError::NotImplemented)));
    }
}
