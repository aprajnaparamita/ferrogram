//! The drawing canvas (ADR 0033): page raster, pan, zoom, rulers, grid,
//! page tabs and the status bar.
//!
//! Owned by task P4-05. Alpha status (task P4-01): the interface is
//! defined as stubs.

use egui::Ui;
use ferrogram_commands::{AppCtx, DocumentSession};

use crate::{Intent, UiError};

/// The raster texture cache of the canvas (ADR 0033).
///
/// Holds the last uploaded [`ferrogram_render::RasterImage`] and the key
/// it was rasterized for (document generation, page, raster zoom).
#[derive(Debug, Clone, PartialEq)]
pub struct CanvasCache {
    generation: u64,
    raster_zoom: f64,
    populated: bool,
}

impl CanvasCache {
    /// An empty cache.
    pub fn new() -> Self {
        Self {
            generation: 0,
            raster_zoom: 0.0,
            populated: false,
        }
    }

    /// Drops the cached texture (document changed or closed).
    pub fn clear(&mut self) {
        self.generation = 0;
        self.raster_zoom = 0.0;
        self.populated = false;
    }

    /// Whether a texture is cached.
    pub fn is_empty(&self) -> bool {
        !self.populated
    }

    /// The document generation the cached texture belongs to.
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

impl Default for CanvasCache {
    fn default() -> Self {
        Self::new()
    }
}

/// What one canvas frame produced (ADR 0033).
#[derive(Debug, Default, PartialEq)]
pub struct CanvasOutput {
    /// Dispatch requests (zoom commands, page switches).
    pub intents: Vec<Intent>,
    /// Status bar text from this frame (page name, zoom, warnings).
    pub status: String,
}

/// Draws the canvas area: grid, page raster, rulers (ADR 0033).
///
/// Not implemented yet (P4-05).
pub fn show(
    ui: &mut Ui,
    session: &mut DocumentSession,
    cache: &mut CanvasCache,
) -> Result<CanvasOutput, UiError> {
    let _ = (ui, session, cache);
    Err(UiError::NotImplemented {
        what: "canvas::show",
    })
}

/// Draws the rulers along the canvas edges (ADR 0033).
///
/// Not implemented yet (P4-05).
pub fn show_rulers(ui: &mut Ui, session: &DocumentSession) -> Result<(), UiError> {
    let _ = (ui, session);
    Err(UiError::NotImplemented {
        what: "canvas::show_rulers",
    })
}

/// Draws the page tab strip under the canvas (ADR 0033).
///
/// Not implemented yet (P4-05).
pub fn show_page_tabs(ui: &mut Ui, session: &mut DocumentSession) -> Result<Vec<Intent>, UiError> {
    let _ = (ui, session);
    Err(UiError::NotImplemented {
        what: "canvas::show_page_tabs",
    })
}

/// Draws the status bar (page, units, zoom, last warning; ADR 0033).
///
/// Not implemented yet (P4-05).
pub fn show_status_bar(ui: &mut Ui, app: &AppCtx) -> Result<(), UiError> {
    let _ = (ui, app);
    Err(UiError::NotImplemented {
        what: "canvas::show_status_bar",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::with_ui;
    use ferrogram_model::Document;

    #[test]
    fn the_cache_starts_empty_and_clears() {
        let mut cache = CanvasCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.generation(), 0);
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache, CanvasCache::default());
    }

    #[test]
    fn canvas_widgets_are_stubs() {
        let mut session = DocumentSession::new(Document::new(), None);
        let mut cache = CanvasCache::new();
        let app = AppCtx::new(ferrogram_commands::Settings::default());
        with_ui(|ui| {
            assert!(show(ui, &mut session, &mut cache).is_err());
            assert!(show_rulers(ui, &session).is_err());
            assert!(show_page_tabs(ui, &mut session).is_err());
            assert!(show_status_bar(ui, &app).is_err());
        });
    }

    #[test]
    fn canvas_output_defaults_to_empty() {
        let output = CanvasOutput::default();
        assert!(output.intents.is_empty());
        assert!(output.status.is_empty());
    }
}
