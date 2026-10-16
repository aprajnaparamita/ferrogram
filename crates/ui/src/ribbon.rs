//! The ribbon, the quick access toolbar and the backstage (ADR 0031),
//! generated from the ui-spec layouts.
//!
//! Owned by task P4-04. Alpha status (task P4-01): the interface is
//! defined as stubs.

use egui::Ui;
use ferrogram_commands::{AppCtx, Registry};
use ferrogram_ui_spec::{LayoutKind, LayoutSet};

use crate::{Intent, UiError};

/// The ribbon widget state (ADR 0031).
pub struct Ribbon {
    kind: LayoutKind,
    active_tab: usize,
    backstage_open: bool,
}

impl Ribbon {
    /// A ribbon for one layout kind, first tab active, backstage closed.
    pub fn new(kind: LayoutKind) -> Self {
        Self {
            kind,
            active_tab: 0,
            backstage_open: false,
        }
    }

    /// The layout kind this ribbon renders.
    pub fn kind(&self) -> LayoutKind {
        self.kind
    }

    /// The index of the active tab in the layout's `tabs` list.
    pub fn active_tab(&self) -> usize {
        self.active_tab
    }

    /// Whether the backstage (File) view is open.
    pub fn backstage_open(&self) -> bool {
        self.backstage_open
    }

    /// Switches the layout kind, resetting to the first tab (ADR 0032).
    pub fn set_kind(&mut self, kind: LayoutKind) {
        self.kind = kind;
        self.active_tab = 0;
    }

    /// Draws the tab strip and the active tab's groups.
    ///
    /// Controls without an implemented command render disabled with a
    /// "Not yet implemented" tooltip (ADR 0031). Not implemented yet
    /// (P4-04).
    pub fn show(
        &mut self,
        ui: &mut Ui,
        set: &LayoutSet,
        registry: &Registry,
        app: &AppCtx,
    ) -> Result<Vec<Intent>, UiError> {
        let _ = (ui, set, registry, app);
        Err(UiError::NotImplemented {
            what: "Ribbon::show",
        })
    }

    /// Draws the quick access toolbar (ADR 0031).
    ///
    /// Not implemented yet (P4-04).
    pub fn show_quick_access(
        &mut self,
        ui: &mut Ui,
        set: &LayoutSet,
        registry: &Registry,
        app: &AppCtx,
    ) -> Result<Vec<Intent>, UiError> {
        let _ = (ui, set, registry, app);
        Err(UiError::NotImplemented {
            what: "Ribbon::show_quick_access",
        })
    }

    /// Draws the backstage tabs (File view) as a full-window stack.
    ///
    /// Not implemented yet (P4-04).
    pub fn show_backstage(
        &mut self,
        ui: &mut Ui,
        set: &LayoutSet,
        registry: &Registry,
        app: &AppCtx,
    ) -> Result<Vec<Intent>, UiError> {
        let _ = (ui, set, registry, app);
        Err(UiError::NotImplemented {
            what: "Ribbon::show_backstage",
        })
    }

    /// Draws one context menu from the layout (right-click, ADR 0031).
    ///
    /// Not implemented yet (P4-04).
    pub fn show_context_menu(
        &mut self,
        ui: &mut Ui,
        set: &LayoutSet,
        registry: &Registry,
        app: &AppCtx,
    ) -> Result<Vec<Intent>, UiError> {
        let _ = (ui, set, registry, app);
        Err(UiError::NotImplemented {
            what: "Ribbon::show_context_menu",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::with_ui;

    #[test]
    fn ribbon_state_transitions() {
        let mut ribbon = Ribbon::new(LayoutKind::Visio2016);
        assert_eq!(ribbon.kind(), LayoutKind::Visio2016);
        assert_eq!(ribbon.active_tab(), 0);
        assert!(!ribbon.backstage_open());
        ribbon.set_kind(LayoutKind::Visio365);
        assert_eq!(ribbon.kind(), LayoutKind::Visio365);
        assert_eq!(ribbon.active_tab(), 0);
    }

    #[test]
    fn ribbon_widgets_are_stubs() {
        let mut ribbon = Ribbon::new(LayoutKind::Visio365);
        let set = LayoutSet::default();
        let registry = Registry::new();
        let app = AppCtx::new(ferrogram_commands::Settings::default());
        with_ui(|ui| {
            assert!(ribbon.show(ui, &set, &registry, &app).is_err());
            assert!(ribbon.show_quick_access(ui, &set, &registry, &app).is_err());
            assert!(ribbon.show_backstage(ui, &set, &registry, &app).is_err());
            assert!(ribbon.show_context_menu(ui, &set, &registry, &app).is_err());
        });
    }
}
