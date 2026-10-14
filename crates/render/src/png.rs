//! The PNG backend: rasterize a scene with `tiny-skia` (ADR 0025).

use crate::error::Error;
use crate::scene::Scene;

/// Options for PNG output (ADR 0025).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PngOptions {
    /// Pixels per inch (default 96).
    pub scale: f64,
    /// Whether to anti-alias (default true).
    pub antialias: bool,
}

impl PngOptions {
    /// The default options: 96 pixels per inch, anti-aliasing on.
    pub fn new() -> Self {
        Self {
            scale: 96.0,
            antialias: true,
        }
    }
}

impl Default for PngOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Rasterizes a scene to PNG bytes at `scale` pixels per inch (ADR 0025).
///
/// Returns [`Error::NotImplemented`] until the backend work lands (P3-06).
pub fn render_png(scene: &Scene, options: &PngOptions) -> Result<Vec<u8>, Error> {
    let _ = (scene, options);
    Err(Error::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_default_to_96_dpi_with_antialiasing() {
        let o = PngOptions::new();
        assert_eq!(o.scale, 96.0);
        assert!(o.antialias);
        assert_eq!(PngOptions::default(), o);
    }

    #[test]
    fn render_png_is_not_implemented_yet() {
        let scene = Scene::new(1.0, 1.0);
        assert_eq!(
            render_png(&scene, &PngOptions::new()),
            Err(Error::NotImplemented)
        );
    }
}
