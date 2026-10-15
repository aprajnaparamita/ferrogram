//! `insta` snapshots of `render_svg` output (task P3-10, ADR 0027
//! "golden SVG text snapshots too").
//!
//! The SVG writer is deterministic (ADR 0025), so a few fixtures pin
//! their full output as reviewable text: a changed snapshot is a diff a
//! reviewer reads, unlike a PNG. The PNG goldens in `tests/golden.rs`
//! remain the visual check. Regenerate with `cargo insta review` (or
//! `INSTA_UPDATE=always`).

mod common;

use ferrogram_render::{SvgOptions, render_svg};

/// The fixtures snapshotted as SVG text.
const SNAPSHOTS: &[&str] = &["render-geometry-absolute", "render-fills", "render-lines"];

#[test]
fn svg_output_is_stable_and_reviewable() {
    for name in SNAPSHOTS {
        let doc = common::fixture_document(&format!("{name}.vsdx"));
        let scenes = common::page_scenes(&doc, true);
        let scene = scenes.first().expect("the fixture has a page");
        let svg = render_svg(scene, &SvgOptions::new()).expect("renders");
        insta::assert_snapshot!(*name, svg);
    }
}
