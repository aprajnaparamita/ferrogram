//! The Phase 4 exit criterion (task P4-09, ADR 0037): opening corpus
//! files shows them on the canvas.
//!
//! For every `corpus/vsdx/*.vsdx` (15 files, `corpus/PROVENANCE.md`):
//! `file.open` runs through `ferrogram_commands::builtins::registry()`
//! in a real [`Shell`], one shell frame renders, and the canvas
//! raster matches `render_raster` of the same page within the ADR
//! 0027 metric (the canvas and the PNG backend share one rasterizer,
//! ADR 0033) and is non-blank.
//!
//! Per-file `skip_reason`s (ADR 0037) cover the pages the renderer
//! cannot draw yet (see [`SKIP_RASTER`] and [`KNOWN_BLANK`], and the
//! P4-09 Result's Open issues); every other assertion still runs for
//! those files.
//!
//! The corpus is fetched, never committed (ADR 0005): the test
//! self-skips with a message when `corpus/vsdx/` is absent and fails
//! instead when `FERROGRAM_REQUIRE_CORPUS=1` is set (the Phase 3
//! convention of `crates/render/tests/common`). Run it as
//! `FERROGRAM_REQUIRE_CORPUS=1 cargo test -p ferrogram --test
//! open_corpus`.
//!
//! Known gap (see the P4-09 Result): `Shell` keeps its `CanvasCache`
//! private, so the raster assertions read a `canvas::show` frame of
//! the same document with the test's own cache, while the shell frame
//! proves the app draws it.

// The never-crash lints apply to non-test code only (clippy.toml);
// these helpers are test code and use `expect`/`panic` freely.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eframe::egui;
use ferrogram_commands::app::{AppCtx, Settings};
use ferrogram_commands::{ArgValue, Args, CommandId, Invocation, Source};
use ferrogram_render::{
    BuildOptions, MAX_DEVICE_EXTENT, RasterImage, RasterOptions, build_scene, render_raster,
};
use ferrogram_ui::Intent;
use ferrogram_ui::canvas::{self, CanvasCache, POINTS_PER_INCH};
use ferrogram_ui::dialogs::{FileChoice, FilePicker, OpenRequest, PickState, SaveRequest};
use ferrogram_ui::shell::Shell;
use ferrogram_ui_spec::Loader;

// ---------------------------------------------------------------------------
// Corpus discovery (the Phase 3 convention)
// ---------------------------------------------------------------------------

/// The corpus directory with the real `.vsdx` packages (fetched).
fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("vsdx")
}

/// The `.vsdx` files of the corpus, sorted for a stable order.
fn corpus_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = match std::fs::read_dir(corpus_dir()) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "vsdx"))
            .collect(),
        Err(_) => Vec::new(),
    };
    files.sort();
    files
}

/// Fails the calling test when `FERROGRAM_REQUIRE_CORPUS=1` is set
/// and the corpus is absent; returns `true` (the test returns early)
/// when the corpus is absent and the test may skip.
fn require_corpus_or(files: &[PathBuf]) -> bool {
    if files.is_empty() {
        assert!(
            std::env::var_os("FERROGRAM_REQUIRE_CORPUS").is_none(),
            "corpus/vsdx is missing and FERROGRAM_REQUIRE_CORPUS=1 is set"
        );
        eprintln!("skipping: corpus/vsdx is absent; run `cargo xtask corpus`");
        true
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// Documented per-file skip reasons (ADR 0037)
// ---------------------------------------------------------------------------

/// Files with no page raster to check (the whole raster section
/// skips).
const SKIP_RASTER: &[(&str, &str)] = &[(
    "recursion-cycle.vsdx",
    "the package has no page part (visio/pages is empty), so there is no page raster",
)];

/// Files whose page rasterizes to nothing with the current renderer.
/// Only the non-blank check skips; the open, frame and raster-match
/// assertions still run. The causes are **not** missing cached values
/// (task P6-08 proved the fill-in pass finds no cell to fill in any
/// corpus file: no corpus `Cell` has a formula without a cached
/// value):
///
/// - `dwg`, `fdo86664`, `testfile5`: the shapes are master instances
///   with no `FillStyle`/`LineStyle`/`TextStyle` of their own, and
///   `Document::resolve` never consults the **master's** style chain,
///   so `FillPattern`/`LinePattern` fall through to the hard default 0
///   and the shapes draw nothing. Copying the master's style links onto
///   the instances makes all three rasterize non-blank (P6-08 Result,
///   open issue 1; a `model` change, not a value-pipeline one).
/// - `tab-short-prefix`: the shape has no size, geometry or text cells
///   and no formula anywhere (its only section is `Tabs`), so there is
///   nothing to draw and nothing a fill-in pass could evaluate.
const KNOWN_BLANK: &[(&str, &str)] = &[
    (
        "dwg.vsdx",
        "the instance does not resolve FillPattern/LinePattern through its master's style chain (P6-08 open issue 1)",
    ),
    (
        "fdo86664.vsdx",
        "the instance does not resolve FillPattern/LinePattern through its master's style chain (P6-08 open issue 1)",
    ),
    (
        "tab-short-prefix.vsdx",
        "the shape has no size, geometry or text cells and no formula; nothing draws (P6-08 open issue 2)",
    ),
    (
        "testfile5.vsdx",
        "the instance does not resolve FillPattern/LinePattern through its master's style chain (P6-08 open issue 1)",
    ),
];

/// The documented skip reason for `name` in `table`, if any.
fn skip_reason(table: &'static [(&'static str, &'static str)], name: &str) -> Option<&'static str> {
    table
        .iter()
        .copied()
        .find(|(file, _)| *file == name)
        .map(|(_, reason)| reason)
}

// ---------------------------------------------------------------------------
// The ADR 0027 comparison metric (ours, small: same dimensions, each
// channel within 2 of 255, at most 0.5% of pixels may exceed that)
// ---------------------------------------------------------------------------

/// The largest per-channel difference that still counts as equal.
const MAX_CHANNEL_DELTA: u8 = 2;

/// The largest fraction of pixels allowed to exceed
/// [`MAX_CHANNEL_DELTA`].
const MAX_FRACTION: f64 = 0.005;

/// Compares two rasters with the ADR 0027 metric.
fn compare_rasters(expected: &RasterImage, actual: &RasterImage) -> Result<(), String> {
    if (expected.width, expected.height) != (actual.width, actual.height) {
        return Err(format!(
            "dimensions differ: expected {}x{}, got {}x{}",
            expected.width, expected.height, actual.width, actual.height
        ));
    }
    let len = expected.width as usize * expected.height as usize * 4;
    if expected.rgba.len() != len || actual.rgba.len() != len {
        return Err("a raster has an inconsistent byte length".to_owned());
    }
    let pixels = (expected.width as usize) * (expected.height as usize);
    let mut over = 0usize;
    for (a, b) in expected
        .rgba
        .chunks_exact(4)
        .zip(actual.rgba.chunks_exact(4))
    {
        let exceeds = (0..4).any(|channel| a[channel].abs_diff(b[channel]) > MAX_CHANNEL_DELTA);
        if exceeds {
            over += 1;
        }
    }
    let fraction = over as f64 / pixels.max(1) as f64;
    if fraction > MAX_FRACTION {
        return Err(format!(
            "{over} of {pixels} pixels ({:.3}%) exceed the ADR 0027 metric",
            fraction * 100.0
        ));
    }
    Ok(())
}

/// The dominant pixel colour: the background of a raster whose scene
/// has no background fill (ADR 0020: `Scene::background` is always
/// `None`, so anything else is drawing).
fn dominant_pixel(raster: &RasterImage) -> [u8; 4] {
    let mut counts: HashMap<[u8; 4], usize> = HashMap::new();
    for pixel in raster.rgba.chunks_exact(4) {
        let key = [pixel[0], pixel[1], pixel[2], pixel[3]];
        *counts.entry(key).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(colour, _)| colour)
        .unwrap_or([0, 0, 0, 0])
}

/// Whether every pixel equals the raster's dominant colour.
fn is_blank(raster: &RasterImage) -> bool {
    let background = dominant_pixel(raster);
    raster
        .rgba
        .chunks_exact(4)
        .all(|pixel| [pixel[0], pixel[1], pixel[2], pixel[3]] == background)
}

/// The scale `canvas::show` rasterizes at (a mirror of
/// `canvas::clamp_scale`, P4-05: ADR 0025 caps plus the backend
/// texture limit). The two private constants are the ones
/// `crates/ui/src/canvas.rs` documents for the shared rasterizer's
/// byte cap and smallest scale.
fn raster_scale(texture_side: usize, width_in: f64, height_in: f64, zoom: f64) -> f64 {
    const MAX_RASTER_BYTES: u64 = 256 * 1024 * 1024;
    const MIN_RASTER_SCALE: f64 = 0.01;
    let longest = width_in.max(height_in).max(0.01);
    let side_cap = f64::from(MAX_DEVICE_EXTENT) / longest;
    let area_cap = ((MAX_RASTER_BYTES / 4) as f64 / (width_in * height_in).max(0.01)).sqrt();
    let texture_cap = texture_side.max(1) as f64 / longest;
    let cap = side_cap.min(area_cap).min(texture_cap);
    let wanted = POINTS_PER_INCH * zoom;
    let floor = MIN_RASTER_SCALE.min(cap);
    wanted.clamp(floor, cap)
}

// ---------------------------------------------------------------------------
// The exit criterion
// ---------------------------------------------------------------------------

/// A picker that must never be called: the test dispatches
/// `file.open` with a `path`, so no dialog can appear (AGENTS.md
/// section 6).
struct NoDialogPicker;

impl FilePicker for NoDialogPicker {
    fn open(&self, _request: &OpenRequest) -> Result<Option<FileChoice>, ferrogram_ui::UiError> {
        panic!("the exit-criterion test must never open a dialog")
    }

    fn save(
        &self,
        _request: &SaveRequest,
        _data: &[u8],
    ) -> Result<Option<FileChoice>, ferrogram_ui::UiError> {
        panic!("the exit-criterion test must never open a dialog")
    }

    fn poll(&self) -> PickState {
        PickState::Idle
    }
}

#[test]
fn every_corpus_file_opens_and_shows_on_the_canvas() {
    let files = corpus_files();
    if require_corpus_or(&files) {
        return;
    }
    assert!(!files.is_empty(), "the corpus has .vsdx files");
    for file in &files {
        let name = file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        check_one_file(file, &name);
    }
}

/// One corpus file: open through the registry in a `Shell`, render a
/// frame, then match the canvas raster against `render_raster`.
fn check_one_file(file: &Path, name: &str) {
    // The shell: the builtin registry and the committed layouts, the
    // way the app builds them.
    let registry = ferrogram_commands::builtins::registry().expect("the registry builds");
    let layouts = Loader::from_embedded().expect("the embedded ui-spec loads");
    let mut shell = Shell::new(registry, layouts, AppCtx::new(Settings::default()));
    shell.set_picker(Box::new(NoDialogPicker));

    // `file.open` through the registry (ADR 0028).
    let command = CommandId::parse("file.open").expect("a valid command id");
    let args = Args::new().with("path", ArgValue::Path(file.to_path_buf()));
    let intent = Intent::new(command, Invocation::new(args, Source::Ui));
    let mut results = shell.dispatch(&[intent]).expect("the intent dispatches");
    let outcome = results
        .pop()
        .expect("one result per intent")
        .unwrap_or_else(|error| panic!("{name}: file.open failed: {error}"));
    assert!(outcome.message.is_some(), "{name}: no outcome message");

    // One shell frame: the opened document draws on the canvas.
    let ctx = egui::Context::default();
    let mut output = ctx.run_ui(Default::default(), |ui| {
        let intents = shell
            .show_in(ui)
            .unwrap_or_else(|error| panic!("{name}: the shell frame failed: {error}"));
        let results = shell.dispatch(&intents).expect("the intents dispatch");
        for result in results {
            result.unwrap_or_else(|error| panic!("{name}: a frame intent failed: {error}"));
        }
    });
    output.textures_delta.clear();

    // The raster section: the canvas raster of the same page (see
    // the module docs), or a documented per-file skip (ADR 0037).
    if let Some(reason) = skip_reason(SKIP_RASTER, name) {
        eprintln!("skip {name}: {reason}");
        return;
    }
    let mut cache = CanvasCache::new();
    let mut texture_side = 0usize;
    let mut canvas_error = None;
    let mut output = ctx.run_ui(Default::default(), |ui| {
        texture_side = ui.ctx().input(|input| input.max_texture_side);
        if let Some(session) = shell.app.active_document_mut()
            && let Err(error) = canvas::show(ui, session, &mut cache)
        {
            canvas_error = Some(error.to_string());
        }
    });
    output.textures_delta.clear();
    if let Some(error) = canvas_error {
        panic!("{name}: the canvas failed: {error}");
    }

    let raster = cache
        .raster()
        .unwrap_or_else(|| panic!("{name}: the canvas rasterized no page"));
    if let Some(reason) = skip_reason(KNOWN_BLANK, name) {
        eprintln!("skip non-blank check for {name}: {reason}");
    } else {
        assert!(
            !is_blank(raster),
            "{name}: the canvas raster is blank (nothing drawn)"
        );
    }

    // The same page through `render_raster`, at the scale the canvas
    // rasterized at (ADR 0033: one rasterizer for both).
    let session = shell
        .app
        .active_document()
        .unwrap_or_else(|| panic!("{name}: no open document"));
    let page = session
        .active_page
        .unwrap_or_else(|| panic!("{name}: no active page"));
    let scene = build_scene(&session.document, page, &BuildOptions::new())
        .unwrap_or_else(|error| panic!("{name}: build_scene failed: {error}"));
    let (width_in, height_in) = cache.page_size();
    let scale = raster_scale(texture_side, width_in, height_in, cache.raster_zoom());
    let expected = render_raster(
        &scene,
        &RasterOptions {
            scale,
            antialias: true,
        },
    )
    .unwrap_or_else(|error| panic!("{name}: render_raster failed: {error}"));
    compare_rasters(&expected, raster)
        .unwrap_or_else(|error| panic!("{name}: the canvas raster differs: {error}"));
}

// ---------------------------------------------------------------------------
// The metric's own tests (synthetic images; run without the corpus)
// ---------------------------------------------------------------------------

fn flat(width: u32, height: u32, rgba: [u8; 4]) -> RasterImage {
    let mut bytes = Vec::with_capacity(width as usize * height as usize * 4);
    for _ in 0..width * height {
        bytes.extend_from_slice(&rgba);
    }
    RasterImage {
        width,
        height,
        rgba: bytes,
    }
}

#[test]
fn identical_images_match() {
    let a = flat(4, 4, [10, 20, 30, 255]);
    assert!(compare_rasters(&a, &a.clone()).is_ok());
}

#[test]
fn a_channel_delta_within_the_metric_matches() {
    // 100 x 100 pixels: one pixel may exceed the per-channel delta
    // (0.01% is under the 0.5% allowance).
    let a = flat(100, 100, [10, 20, 30, 255]);
    let mut b = a.clone();
    b.rgba[0] = 12; // one channel, delta 2
    b.rgba[5000 * 4 + 1] = 17; // one channel, delta 3 on one pixel
    assert!(compare_rasters(&a, &b).is_ok());
}

#[test]
fn a_large_delta_on_too_many_pixels_differs() {
    let a = flat(4, 4, [10, 20, 30, 255]);
    let mut b = a.clone();
    for pixel in 0..4 {
        b.rgba[pixel * 4] = 200;
    }
    let error = compare_rasters(&a, &b).expect_err("4 of 16 pixels exceed the metric");
    assert!(error.contains("pixels"), "{error}");
}

#[test]
fn different_dimensions_never_match() {
    let a = flat(4, 4, [0, 0, 0, 255]);
    let b = flat(2, 8, [0, 0, 0, 255]);
    let error = compare_rasters(&a, &b).expect_err("dimensions differ");
    assert!(error.contains("dimensions"), "{error}");
}

#[test]
fn a_blank_raster_has_no_drawing() {
    assert!(is_blank(&flat(3, 3, [255, 255, 255, 255])));
    let mut drawn = flat(3, 3, [255, 255, 255, 255]);
    drawn.rgba[4] = 0; // one darker pixel: a drawing
    assert!(!is_blank(&drawn));
}

#[test]
fn the_scale_mirror_keeps_the_raster_within_the_caps() {
    // A small page at zoom 1 rasterizes at 96 PPI (ADR 0025).
    assert_eq!(raster_scale(2048, 8.5, 11.0, 1.0), POINTS_PER_INCH);
    // A huge page clamps to the texture cap instead of overflowing.
    let scale = raster_scale(2048, 400.0, 400.0, 1.0);
    assert!(scale > 0.0 && scale < POINTS_PER_INCH);
    // Any finite input is a finite, positive scale.
    for texture_side in [0usize, 1, 2048] {
        let scale = raster_scale(texture_side, 0.0, 0.0, 1.0);
        assert!(scale.is_finite() && scale > 0.0);
    }
}
