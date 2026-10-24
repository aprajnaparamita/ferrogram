//! The Panel Modern widgets: tab strip, groups, controls, Quick Access
//! Toolbar and File view (ADR 0061, `docs/ui/panel-modern.md`).
//!
//! Task P7-01 defines the entry point as an interface stub; task P7-07
//! implements it. The widgets read a [`LayoutV2`] and return
//! [`Intent`]s; they never execute commands themselves (ADR 0028).

use egui::Ui;
use larrygram_ui_spec::v2::LayoutV2;

use crate::contexts::SelectionContext;
use crate::{Intent, UiError};

/// Draws the Panel Modern chrome for `layout` and returns one
/// [`Intent`] per activation (stub, task P7-01).
///
/// `context` selects the contextual tabs (ADR 0064): task P7-08
/// computes it once per frame and the shell passes it in. Task P7-07
/// implements the tab strip, the `stack` and `rows` group layouts, the
/// control kinds, the collapsible command bar and the File view, as
/// described in `docs/ui/panel-modern.md`.
pub fn show(
    _ui: &mut Ui,
    _layout: &LayoutV2,
    _context: SelectionContext,
) -> Result<Vec<Intent>, UiError> {
    Err(UiError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::with_ui;

    #[test]
    fn show_is_not_implemented() {
        let layout: LayoutV2 = serde_json::from_value(serde_json::json!({
            "schema_version": 2, "id": "panel-modern", "name": "Panel Modern",
            "kind": "panel-modern", "theme": "panel-modern",
            "window": {"regions": []}
        }))
        .expect("a minimal layout");
        let outcome = with_ui(|ui| show(ui, &layout, SelectionContext::EmptyPage));
        assert!(matches!(outcome, Err(UiError::NotImplemented)));
    }
}
