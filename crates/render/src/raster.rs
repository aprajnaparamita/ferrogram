//! Raw RGBA raster output (ADR 0033): the canvas texture backend.
//!
//! [`render_raster`] shares the PNG backend's rasterization path: the
//! same pixels, without the PNG encoding step. The egui canvas (L6)
//! uploads the result as a texture; `render_png` encodes the same raster
//! for files and goldens.
//!
//! Alpha status (task P4-01): the interface is fixed here;
//! `render_raster` returns [`Error::NotImplemented`] until P4-05.

use crate::build::DEFAULT_SCALE;
use crate::error::Error;
use crate::scene::Scene;

/// RGBA raster of a scene (ADR 0033): row-major, 8 bits per channel,
/// no padding between rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RasterImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes, RGBA order.
    pub rgba: Vec<u8>,
}

impl RasterImage {
    /// Number of pixels.
    pub fn pixel_count(&self) -> usize {
        self.width as usize * self.height as usize
    }
}

/// Options for [`render_raster`] (ADR 0033, ADR 0025 caps apply).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RasterOptions {
    /// Pixels per inch (ADR 0025). [`DEFAULT_SCALE`] when 96.
    pub scale: f64,
    /// Anti-aliasing, on by default (ADR 0025).
    pub antialias: bool,
}

impl RasterOptions {
    /// Options at [`DEFAULT_SCALE`], anti-aliased.
    pub fn new() -> Self {
        Self {
            scale: DEFAULT_SCALE,
            antialias: true,
        }
    }
}

impl Default for RasterOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Rasterizes a scene to raw RGBA at `scale` pixels per inch (ADR 0033).
///
/// The ADR 0025 caps apply unchanged: [`Error::InvalidOptions`] for a
/// `scale` that is not finite, not positive or above the device extent
/// cap; [`Error::TooLarge`] when the page at that scale does not fit.
///
/// Not implemented yet (P4-05).
pub fn render_raster(scene: &Scene, opts: &RasterOptions) -> Result<RasterImage, Error> {
    let _ = (scene, opts);
    Err(Error::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raster_options_default_to_96_ppi_anti_aliased() {
        let opts = RasterOptions::new();
        assert_eq!(opts.scale, 96.0);
        assert!(opts.antialias);
        assert_eq!(opts, RasterOptions::default());
    }

    #[test]
    fn raster_image_counts_pixels() {
        let image = RasterImage {
            width: 3,
            height: 2,
            rgba: vec![0; 24],
        };
        assert_eq!(image.pixel_count(), 6);
    }

    #[test]
    fn render_raster_is_a_stub() {
        let scene = Scene {
            width: 1.0,
            height: 1.0,
            background: None,
            nodes: Vec::new(),
            warnings: Vec::new(),
        };
        let error = render_raster(&scene, &RasterOptions::new()).expect_err("stub");
        assert_eq!(error, Error::NotImplemented);
    }
}
