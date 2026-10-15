//! No-panic corpus rendering (task P3-10, ADR 0027 section 5): the
//! Phase 3 exit criterion's first half.
//!
//! For every `corpus/vsdx/*.vsdx` file (15 in `corpus/manifest.toml`),
//! every page builds a scene and renders through both backends
//! (`render_png` and `render_svg`). Warnings are allowed (ADR 0021:
//! bad values warn and substitute); an `Err` or a panic fails. The
//! corpus is fetched, never committed (ADR 0005): the test skips with a
//! message when `corpus/vsdx/` is absent and fails instead when
//! `FERROGRAM_REQUIRE_CORPUS=1` is set.

mod common;

use ferrogram_render::{PngOptions, SvgOptions, render_png, render_svg};
use ferrogram_vsdx::{Limits, read_document};

#[test]
fn every_corpus_file_renders_every_page_in_both_backends() {
    let files = common::corpus_files();
    if common::require_corpus_or(&files) {
        return;
    }
    let mut pages = 0usize;
    for file in &files {
        let name = file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let bytes = std::fs::read(file).unwrap_or_else(|e| panic!("{name}: {e}"));
        let doc = read_document(&bytes, &Limits::DEFAULT)
            .unwrap_or_else(|e| panic!("{name}: does not read: {e}"));
        for scene in common::page_scenes(&doc, true) {
            let png = render_png(&scene, &PngOptions::new())
                .unwrap_or_else(|e| panic!("{name} page {pages}: render_png: {e}"));
            assert!(!png.is_empty(), "{name} page {pages}: empty PNG");
            let svg = render_svg(&scene, &SvgOptions::new())
                .unwrap_or_else(|e| panic!("{name} page {pages}: render_svg: {e}"));
            assert!(
                svg.starts_with("<?xml") && svg.ends_with("</svg>\n"),
                "{name} page {pages}: not a standalone SVG document"
            );
            assert!(
                scene.width.is_finite() && scene.height.is_finite(),
                "{name} page {pages}: non-finite page size"
            );
            pages += 1;
        }
    }
    assert!(pages > 0, "no pages rendered");
}
