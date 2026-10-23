//! Fuzz target for the rendering pipeline (task P3-10, ADR 0027
//! section 6).
//!
//! Raw bytes go into `read_document` (the entry point for untrusted
//! files); whatever reads builds a scene for every page and renders it
//! through both backends. A panic fails the fuzz run; an `Err` is a
//! valid outcome at every step.

#![no_main]

use larrygram_model::PageId;
use larrygram_render::{BuildOptions, PngOptions, SvgOptions, build_scene, render_png, render_svg};
use larrygram_vsdx::{Limits, read_document};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(document) = read_document(data, &Limits::DEFAULT) {
        for index in 0..document.pages.len() {
            let page = PageId {
                index: u32::try_from(index).unwrap_or(u32::MAX),
            };
            if let Ok(scene) = build_scene(&document, page, &BuildOptions::new()) {
                // Both backends: any result is fine, no panic allowed.
                let _ = render_png(&scene, &PngOptions::new());
                let _ = render_svg(&scene, &SvgOptions::new());
            }
        }
    }
});
