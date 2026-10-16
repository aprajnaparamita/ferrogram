//! Raw RGBA raster output (ADR 0033): the canvas texture backend.
//!
//! [`render_raster`] shares the PNG backend's rasterization path: the
//! same pixels, without the PNG encoding step. The egui canvas (L6)
//! uploads the result as a texture; `render_png` encodes the same raster
//! for files and goldens. The bytes are straight (non-premultiplied)
//! RGBA, exactly the bytes the PNG encoder writes, so decoding a rendered
//! PNG yields the same buffer (the parity test in `tests/raster.rs`
//! pins this).

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
/// The output is exactly the raster [`crate::render_png`] encodes, as
/// straight (non-premultiplied) RGBA bytes: `render_png` and
/// `render_raster` share one rasterizer (ADR 0033), so the PNG goldens
/// cover these pixels.
pub fn render_raster(scene: &Scene, opts: &RasterOptions) -> Result<RasterImage, Error> {
    let pixmap = crate::png::rasterize(scene, opts.scale, opts.antialias)?;
    Ok(RasterImage {
        width: pixmap.width(),
        height: pixmap.height(),
        rgba: pixmap.take_demultiplied(),
    })
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
    fn render_raster_draws_the_background_at_the_requested_scale() {
        let mut scene = Scene::new(2.0, 1.0);
        scene.background = Some(crate::scene::Rgba::opaque(255, 255, 255));
        let image = render_raster(&scene, &RasterOptions::new()).expect("raster");
        assert_eq!((image.width, image.height), (192, 96));
        assert_eq!(image.rgba.len(), 192 * 96 * 4);
        assert_eq!(&image.rgba[0..4], &[255, 255, 255, 255]);
        assert_eq!(image.pixel_count(), 192 * 96);
    }

    #[test]
    fn render_raster_keeps_a_transparent_background_transparent() {
        let scene = Scene::new(1.0, 1.0);
        let image = render_raster(&scene, &RasterOptions::new()).expect("raster");
        assert_eq!(image.rgba.len(), 96 * 96 * 4);
        assert_eq!(&image.rgba[0..4], &[0, 0, 0, 0]);
    }

    #[test]
    fn render_raster_enforces_the_adr_0025_caps() {
        let scene = Scene::new(1.0, 1.0);
        for scale in [0.0, -1.0, f64::NAN, f64::INFINITY, 16_385.0] {
            let error = render_raster(
                &scene,
                &RasterOptions {
                    scale,
                    antialias: true,
                },
            )
            .expect_err("bad scale");
            assert_eq!(error, Error::InvalidOptions, "scale {scale}");
        }
        let big = Scene::new(20_000.0, 1.0);
        let error = render_raster(&big, &RasterOptions::new()).expect_err("page too large");
        assert_eq!(error, Error::TooLarge);
    }
}
