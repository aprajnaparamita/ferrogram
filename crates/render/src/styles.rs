//! The visual styles we define ourselves (ADR 0023).
//!
//! Visio's line patterns, line ends, fill patterns and shadow styles are
//! not in [MS-VSDX]; we draw our own and commit them as data in
//! `crates/render/data/visual-styles.yaml`. Indices and names come from
//! the Visio VBA enumerations (derived facts); the drawings are our own
//! work. This module resolves an index to a template; the scene builder
//! scales templates by the shape's cells (ADR 0020).

use ferrogram_geom::{Path, Size};

use crate::error::Error;

/// One line pattern: a dash rhythm as multiples of the line width
/// (ADR 0023).
#[derive(Debug, Clone, PartialEq)]
pub struct LinePatternTemplate {
    /// The dash lengths as multiples of the line width; empty is solid.
    pub dashes: Vec<f64>,
    /// Whether the pattern hides the line entirely.
    pub invisible: bool,
}

/// One line end (arrowhead) template (ADR 0023).
#[derive(Debug, Clone, PartialEq)]
pub struct LineEndTemplate {
    /// The arrowhead shape in unit coordinates.
    pub shape: Path,
    /// The width as a multiple of the size class.
    pub width: f64,
    /// The length as a multiple of the size class.
    pub length: f64,
    /// Whether the shape is filled.
    pub filled: bool,
}

/// One fill pattern template (ADR 0023).
#[derive(Debug, Clone, PartialEq)]
pub struct PatternTemplate {
    /// The tile shape in unit coordinates.
    pub tile: Path,
    /// The tile size in unit coordinates.
    pub tile_size: Size,
    /// Whether the tile lines are filled or stroked.
    pub filled: bool,
}

/// One shadow template (ADR 0023).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowTemplate {
    /// The x offset as a multiple of the line width.
    pub dx: f64,
    /// The y offset as a multiple of the line width.
    pub dy: f64,
    /// The blur radius as a multiple of the line width.
    pub blur: f64,
    /// The shadow alpha, 0 to 1.
    pub alpha: f64,
}

/// The visual style definitions (ADR 0023).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VisualStyles {
    loaded: bool,
}

impl VisualStyles {
    /// Loads the committed data file `crates/render/data/visual-styles.yaml`.
    ///
    /// Returns [`Error::NotImplemented`] until the style work lands
    /// (P3-08).
    pub fn load() -> Result<VisualStyles, Error> {
        Err(Error::NotImplemented)
    }

    /// The dash pattern of a Visio line pattern index.
    ///
    /// Returns `None` until the style work lands (P3-08).
    pub fn line_pattern(&self, index: u32) -> Option<LinePatternTemplate> {
        let _ = (self, index);
        None
    }

    /// The arrowhead template of a Visio line end index.
    ///
    /// Returns `None` until the style work lands (P3-08).
    pub fn line_end(&self, index: u32) -> Option<LineEndTemplate> {
        let _ = (self, index);
        None
    }

    /// The tile template of a Visio fill pattern index.
    ///
    /// Returns `None` until the style work lands (P3-08).
    pub fn fill_pattern(&self, index: u32) -> Option<PatternTemplate> {
        let _ = (self, index);
        None
    }

    /// The shadow template of a Visio shadow pattern index.
    ///
    /// Returns `None` until the style work lands (P3-08).
    pub fn shadow(&self, index: u32) -> Option<ShadowTemplate> {
        let _ = (self, index);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_is_not_implemented_yet() {
        assert_eq!(VisualStyles::load(), Err(Error::NotImplemented));
    }

    #[test]
    fn lookups_return_none_until_the_data_lands() {
        let styles = VisualStyles::default();
        assert_eq!(styles.line_pattern(1), None);
        assert_eq!(styles.line_end(1), None);
        assert_eq!(styles.fill_pattern(1), None);
        assert_eq!(styles.shadow(1), None);
    }
}
