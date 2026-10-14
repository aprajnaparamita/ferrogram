//! The SVG backend: our own deterministic writer (ADR 0025).
//!
//! Text is emitted as glyph outlines, never as `<text>` elements, so the
//! output is self-contained and byte-stable.

use crate::error::Error;
use crate::scene::Scene;

/// Options for SVG output (ADR 0025).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SvgOptions {
    /// Pixels per inch (default 96).
    pub scale: f64,
}

impl SvgOptions {
    /// The default options: 96 pixels per inch.
    pub fn new() -> Self {
        Self { scale: 96.0 }
    }
}

impl Default for SvgOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Writes a scene as a standalone SVG document (ADR 0025).
///
/// Returns [`Error::NotImplemented`] until the backend work lands (P3-07).
pub fn render_svg(scene: &Scene, options: &SvgOptions) -> Result<String, Error> {
    let _ = (scene, options);
    Err(Error::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_default_to_96_dpi() {
        assert_eq!(SvgOptions::new().scale, 96.0);
        assert_eq!(SvgOptions::default(), SvgOptions::new());
    }

    #[test]
    fn render_svg_is_not_implemented_yet() {
        let scene = Scene::new(1.0, 1.0);
        assert_eq!(
            render_svg(&scene, &SvgOptions::new()),
            Err(Error::NotImplemented)
        );
    }
}
