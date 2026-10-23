//! Selection contexts (ADR 0064, `docs/ui/contexts.md`).
//!
//! [`current`] computes the one context per frame that drives the
//! contextual tabs, the context menus and the enablement cross-check.
//! Task P7-01 defines the enum and the entry point as an interface
//! stub; task P7-08 implements it. Nothing below L6 learns about the
//! UI, so the enum lives here and not in `crates/ui-spec`
//! ([`ContextId`] there is the name used in the data files).

use larrygram_commands::AppCtx;
use larrygram_ui_spec::v2::ContextId;

use crate::UiError;

/// What the user is doing right now (ADR 0064).
///
/// One variant per context in `ui-spec/contexts.yaml`, plus
/// [`PageSelected`](Self::PageSelected) (the page has shapes but
/// nothing is selected, which counts as
/// [`EmptyPage`](Self::EmptyPage) for availability) and
/// [`StencilEdit`](Self::StencilEdit) (a stencil is open for editing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelectionContext {
    /// A page with no shapes, nothing selected.
    EmptyPage,
    /// The page has shapes but nothing is selected.
    PageSelected,
    /// One ordinary 2-D shape selected.
    ShapeSelected,
    /// Two or more shapes selected.
    ShapesSelected,
    /// A shape's text is open for editing.
    TextEditing,
    /// One connector selected.
    ConnectorSelected,
    /// One group selected.
    GroupSelected,
    /// Another, empty page is active.
    SecondPage,
    /// One picture selected.
    PictureSelected,
    /// One container selected.
    ContainerSelected,
    /// A ShapeSheet window is the active window.
    ShapesheetWindow,
    /// A stencil is open for editing.
    StencilEdit,
}

impl SelectionContext {
    /// The context name the data files use.
    ///
    /// `PageSelected` counts as [`ContextId::EmptyPage`] for
    /// availability (ADR 0064).
    pub fn id(self) -> ContextId {
        match self {
            Self::EmptyPage | Self::PageSelected => ContextId::EmptyPage,
            Self::ShapeSelected => ContextId::ShapeSelected,
            Self::ShapesSelected => ContextId::ShapesSelected,
            Self::TextEditing => ContextId::TextEditing,
            Self::ConnectorSelected => ContextId::ConnectorSelected,
            Self::GroupSelected => ContextId::GroupSelected,
            Self::SecondPage => ContextId::SecondPage,
            Self::PictureSelected => ContextId::PictureSelected,
            Self::ContainerSelected => ContextId::ContainerSelected,
            Self::ShapesheetWindow => ContextId::ShapesheetWindow,
            Self::StencilEdit => ContextId::StencilEdit,
        }
    }
}

/// Computes the current context from the application state (stub,
/// task P7-01).
///
/// Task P7-08 follows the rules in ADR 0064: selection, shape kinds,
/// the text editor state and the active window. It may drop the
/// `Result` if the computation cannot fail.
pub fn current(_app: &AppCtx) -> Result<SelectionContext, UiError> {
    Err(UiError::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_match_the_data_names() {
        let expected = [
            (SelectionContext::EmptyPage, ContextId::EmptyPage),
            (SelectionContext::PageSelected, ContextId::EmptyPage),
            (SelectionContext::ShapeSelected, ContextId::ShapeSelected),
            (SelectionContext::ShapesSelected, ContextId::ShapesSelected),
            (SelectionContext::TextEditing, ContextId::TextEditing),
            (
                SelectionContext::ConnectorSelected,
                ContextId::ConnectorSelected,
            ),
            (SelectionContext::GroupSelected, ContextId::GroupSelected),
            (SelectionContext::SecondPage, ContextId::SecondPage),
            (
                SelectionContext::PictureSelected,
                ContextId::PictureSelected,
            ),
            (
                SelectionContext::ContainerSelected,
                ContextId::ContainerSelected,
            ),
            (
                SelectionContext::ShapesheetWindow,
                ContextId::ShapesheetWindow,
            ),
            (SelectionContext::StencilEdit, ContextId::StencilEdit),
        ];
        for (context, id) in expected {
            assert_eq!(context.id(), id);
        }
    }

    #[test]
    fn current_is_not_implemented() {
        let app = AppCtx::new(larrygram_commands::Settings::default());
        assert!(matches!(current(&app), Err(UiError::NotImplemented)));
    }
}
