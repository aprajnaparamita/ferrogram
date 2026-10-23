//! The `file.*` commands (ADR 0026, ADR 0036): new, open, save, close,
//! quit, inspect, render and export.
//!
//! The `inspect`, `render` and `export` bodies are the ADR 0026 CLI
//! seam: the CLI's `Command` enum dispatches here, so the CLI, the
//! control channel (Phase 8) and MCP share one implementation. Failures
//! carry the ADR 0026 one-line report (`cannot read input: ...`,
//! `cannot write output: ...`, `render failed: ...`); the CLI maps
//! those prefixes to its exit codes.

use std::path::{Path, PathBuf};

use ferrogram_model::{
    Cell, CellValue, Document, DocumentKind, Page, PageId, RowTarget, SectionName, Sheet, SheetRef,
};
use ferrogram_render::{
    BuildOptions, PngOptions, Rgba, Scene, SvgOptions, build_scene, render_png, render_svg,
};
use ferrogram_shapesheet::{CellAddress, RecalcOptions, recalc};
use ferrogram_vsdx::{Limits, Package, read_package, to_model, write_document};

use crate::app::{AppCtx, DocumentSession, ViewState};
use crate::error::{CommandError, NeededInput, PathPurpose};
use crate::{Invocation, Outcome, UiRequest};

use super::{no_document, outcome, record_warnings};

/// The default device scale: 96 pixels per inch (ADR 0025).
pub(crate) const DEFAULT_SCALE: f64 = ferrogram_render::build::DEFAULT_SCALE;

/// The output formats of `render` and `export` (ADR 0026).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    /// PNG raster (ADR 0025).
    Png,
    /// SVG document (ADR 0025).
    Svg,
}

impl Format {
    /// The file extension written for this format.
    fn extension(self) -> &'static str {
        match self {
            Format::Png => "png",
            Format::Svg => "svg",
        }
    }
}

/// What the `background` argument asks for (ADR 0026 `--background`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Background {
    /// No argument: the page's own background.
    Keep,
    /// `none`: a transparent page.
    Transparent,
    /// `#RRGGBB`: an opaque page colour.
    Color(Rgba),
}

// ---------------------------------------------------------------------------
// Arguments (the ADR 0028 names)
// ---------------------------------------------------------------------------

/// The `path` argument, or [`CommandError::NeedsInput`] (ADR 0028).
pub(crate) fn need_path(
    args: &crate::Args,
    name: &str,
    purpose: PathPurpose,
) -> Result<PathBuf, CommandError> {
    match args.path(name) {
        Some(path) => Ok(path.to_owned()),
        None => Err(CommandError::NeedsInput(NeededInput::Path {
            purpose,
            suggested: None,
        })),
    }
}

/// The `page` argument (1-based), or `None` when the caller left it out.
///
/// A number outside `1..=u32::MAX` is an argument error, and every
/// error carries the caller's number (P4-15 review minor 6: `page: 0`
/// is rejected instead of silently selecting the first page, and no
/// saturated value is ever reported back).
fn optional_page(args: &crate::Args) -> Result<Option<u32>, CommandError> {
    match args.integer("page") {
        Some(number) if number < 1 => Err(CommandError::invalid_args(format!(
            "page must be counted from 1, got {number}"
        ))),
        Some(number) => u32::try_from(number).map(Some).map_err(|_| {
            CommandError::invalid_args(format!(
                "page must be counted from 1 and at most {}, got {number}",
                u32::MAX
            ))
        }),
        None => Ok(None),
    }
}

/// The `page` argument (1-based): the first page when left out.
fn page_arg(args: &crate::Args) -> Result<u32, CommandError> {
    Ok(optional_page(args)?.unwrap_or(1))
}

/// The `format` argument, or the format named by `out`'s extension
/// (ADR 0026: "the output extension, overridable with --format").
fn format_arg(args: &crate::Args, out: &Path) -> Result<Format, CommandError> {
    match args.text("format") {
        Some("png") => Ok(Format::Png),
        Some("svg") => Ok(Format::Svg),
        Some(other) => Err(CommandError::invalid_args(format!(
            "unknown format: {other}; expected png or svg"
        ))),
        None => match out.extension().and_then(|e| e.to_str()) {
            Some("svg") | Some("SVG") => Ok(Format::Svg),
            _ => Ok(Format::Png),
        },
    }
}

/// The `scale` argument (pixels per inch), 96 when left out (ADR 0025).
fn scale_arg(args: &crate::Args) -> Result<f64, CommandError> {
    match args.number("scale") {
        Some(scale) if scale.is_finite() && scale > 0.0 => Ok(scale),
        Some(_) => Err(CommandError::invalid_args(
            "scale must be finite and positive",
        )),
        None => Ok(DEFAULT_SCALE),
    }
}

/// The `background` argument (ADR 0026 `--background #RRGGBB|none`).
fn background_arg(args: &crate::Args) -> Result<Background, CommandError> {
    let Some(text) = args.text("background") else {
        return Ok(Background::Keep);
    };
    if text == "none" {
        return Ok(Background::Transparent);
    }
    if let Some(hex) = text.strip_prefix('#')
        && hex.len() == 6
        && let Ok(rgb) = u32::from_str_radix(hex, 16)
    {
        return Ok(Background::Color(Rgba::opaque(
            ((rgb >> 16) & 0xFF) as u8,
            ((rgb >> 8) & 0xFF) as u8,
            (rgb & 0xFF) as u8,
        )));
    }
    Err(CommandError::invalid_args(format!(
        "background must be #RRGGBB or none, got {text:?}"
    )))
}

/// The `json` flag of `inspect`, false when left out.
fn json_arg(args: &crate::Args) -> bool {
    args.flag("json").unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Domain failures (the ADR 0026 report lines)
// ---------------------------------------------------------------------------

/// A failure that names a file (`cannot read input: <label>: <cause>`).
fn read_error(label: &Path, cause: impl std::fmt::Display) -> CommandError {
    read_error_text(format!("{}: {cause}", label.display()))
}

/// A read failure without a file name (a page number, an empty export).
fn read_error_text(message: impl Into<String>) -> CommandError {
    CommandError::failed(format!("cannot read input: {}", message.into()))
}

/// A write failure without a file name (`cannot write output: <cause>`,
/// the ADR 0026 prefix the CLI maps to exit code 3).
fn write_error_text(cause: impl std::fmt::Display) -> CommandError {
    CommandError::failed(format!("cannot write output: {cause}"))
}

/// A write failure (`cannot write output: <label>: <cause>`).
fn io_error(label: &Path, cause: impl std::fmt::Display) -> CommandError {
    write_error_text(format!("{}: {cause}", label.display()))
}

/// A render failure (`render failed: <cause>`).
fn render_error(cause: impl std::fmt::Display) -> CommandError {
    CommandError::failed(format!("render failed: {cause}"))
}

// ---------------------------------------------------------------------------
// Reading and writing
// ---------------------------------------------------------------------------

/// Reads a document and keeps its package for the lossless writer
/// (ADR 0018).
///
/// The fill-in pass (ADR 0057) runs on the model before it is returned:
/// a cell with a formula and no cached value gains one, so every
/// consumer of a read document (the session, `render`, `export`,
/// `inspect`) sees the same values.
fn read_package_document(bytes: &[u8], label: &Path) -> Result<(Document, Package), CommandError> {
    let package =
        read_package(bytes, &Limits::DEFAULT).map_err(|error| read_error(label, error))?;
    let mut document = to_model(&package).map_err(|error| read_error(label, error))?;
    fill_in_values(&mut document);
    Ok((document, package))
}

/// Fills the cached values a read document is missing (ADR 0057).
///
/// One pass evaluates every cell that has a formula and no cached value
/// and writes what it computes. A cell that has a value keeps it: the
/// file's cache is Visio's own last computation and the cross-check
/// measures against it (ADR 0059). The pass runs at load, before the
/// session exists, so it takes no undo entry and does not bump the
/// change counter.
///
/// The pass never fails the read: a formula the engine cannot evaluate
/// keeps its cell as it was, and the pass outcome is dropped (its
/// warnings belong to editing, ADR 0056, not to opening a file).
///
/// `commands::recalc::fill_missing` (task P6-07) is the future home of
/// this entry point; until it lands, the pass drives
/// [`recalc`] with [`RecalcOptions::fill_missing_only`] directly.
fn fill_in_values(document: &mut Document) {
    let dirty = missing_value_cells(document);
    if dirty.is_empty() {
        return;
    }
    let options = RecalcOptions {
        fill_missing_only: true,
        ..RecalcOptions::new()
    };
    let _ = recalc(document, &dirty, &options);
}

/// The addresses of every cell that has a formula and no cached value
/// (the ADR 0057 fill-in contract), sorted for a deterministic pass.
fn missing_value_cells(document: &Document) -> Vec<CellAddress> {
    let mut out = Vec::new();
    for (sheet, data) in sheet_refs(document) {
        collect_missing(sheet, &data.cells, None, None, &mut out);
        for section in &data.sections {
            let name = Some(section.name.clone());
            collect_missing(sheet, &section.cells, name.clone(), None, &mut out);
            for (position, row) in section.rows.iter().enumerate() {
                let target = RowTarget {
                    position: Some(position),
                    ix: row.ix,
                };
                collect_missing(sheet, &row.cells, name.clone(), Some(target), &mut out);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// The sheets of a document: page sheets and shape sheets, in the
/// deterministic order the dependency graph walks them (ADR 0055).
fn sheet_refs(document: &Document) -> Vec<(SheetRef, &Sheet)> {
    let mut out = Vec::new();
    for page in &document.pages {
        out.push((SheetRef::Page(page.id), &page.sheet));
    }
    for shape in &document.shapes {
        out.push((SheetRef::Shape(shape.id), &shape.sheet));
    }
    out
}

/// Collects the addresses of the `cells` that have a formula and no
/// cached value, under `section` and `row` (when they apply).
fn collect_missing(
    sheet: SheetRef,
    cells: &[Cell],
    section: Option<SectionName>,
    row: Option<RowTarget>,
    out: &mut Vec<CellAddress>,
) {
    for cell in cells {
        if cell.formula.is_some() && cell.value.is_none() {
            let mut address = CellAddress::new(sheet, cell.name.clone());
            address.section = section.clone();
            address.row = row;
            out.push(address);
        }
    }
}

/// Reads and parses the input file (ADR 0026 exit code 2).
fn load(path: &Path) -> Result<(Document, Option<Package>), CommandError> {
    let bytes = std::fs::read(path).map_err(|error| read_error(path, error))?;
    let (document, package) = read_package_document(&bytes, path)?;
    Ok((document, Some(package)))
}

/// Writes one output file (ADR 0026 exit code 3).
fn write_file(path: &Path, bytes: &[u8]) -> Result<(), CommandError> {
    std::fs::write(path, bytes).map_err(|error| io_error(path, error))
}

/// The selected page (1-based), or the first one.
fn select_page(doc: &Document, page: Option<u32>) -> Result<&Page, CommandError> {
    let number = page.unwrap_or(1);
    doc.pages
        .get(number.saturating_sub(1) as usize)
        .ok_or_else(|| {
            read_error_text(format!(
                "page {number} does not exist; the document has {} pages",
                doc.pages.len()
            ))
        })
}

/// Builds the scene of one page (ADR 0020/0021).
fn build(doc: &Document, page: PageId, scale: f64) -> Result<Scene, CommandError> {
    let mut options = BuildOptions::new();
    options.scale = scale;
    build_scene(doc, page, &options).map_err(render_error)
}

/// Encodes a scene in the requested format (ADR 0025).
fn encode(scene: &Scene, format: Format, scale: f64) -> Result<Vec<u8>, CommandError> {
    match format {
        Format::Png => {
            let options = PngOptions {
                scale,
                antialias: true,
            };
            render_png(scene, &options).map_err(render_error)
        }
        Format::Svg => {
            let options = SvgOptions { scale };
            let svg = render_svg(scene, &options).map_err(render_error)?;
            Ok(svg.into_bytes())
        }
    }
}

// ---------------------------------------------------------------------------
// The commands
// ---------------------------------------------------------------------------

/// `file.new`: an empty document session, which becomes active.
///
/// The session's view toggles are seeded from the settings defaults
/// (ADR 0032 amendment, P4-15 review major 1).
pub(super) fn run_new(ctx: &mut AppCtx, _inv: &Invocation) -> Result<Outcome, CommandError> {
    let mut session = DocumentSession::new(Document::new(), None);
    session.view = ViewState::from_settings(&ctx.settings);
    ctx.open_document(session);
    Ok(outcome("New drawing", Vec::new()))
}

/// `file.open`: reads `path` (or `name` + `data` on wasm, ADR 0036)
/// into a new session, which becomes active. The session's view
/// toggles are seeded from the settings defaults (ADR 0032 amendment).
pub(super) fn run_open(ctx: &mut AppCtx, inv: &Invocation) -> Result<Outcome, CommandError> {
    let (document, package, path, label) = match inv.args.path("path") {
        Some(path) => {
            let bytes = std::fs::read(path).map_err(|error| read_error(path, error))?;
            let (document, package) = read_package_document(&bytes, path)?;
            (
                document,
                package,
                Some(path.to_owned()),
                path.display().to_string(),
            )
        }
        None => match (inv.args.text("name"), inv.args.bytes("data")) {
            (Some(name), Some(data)) => {
                let (document, package) = read_package_document(data, Path::new(name))?;
                (document, package, None, name.to_owned())
            }
            _ => {
                return Err(CommandError::NeedsInput(NeededInput::Path {
                    purpose: PathPurpose::OpenFile,
                    suggested: None,
                }));
            }
        },
    };
    let recent = path.clone();
    let mut session = DocumentSession::new(document, path).with_preserve(package);
    session.view = ViewState::from_settings(&ctx.settings);
    ctx.open_document(session);
    if let Some(path) = recent {
        remember_recent(&mut ctx.settings, path);
    }
    Ok(outcome(format!("Opened {label}"), Vec::new()))
}

/// Records a recently opened file: most recent first, de-duplicated and
/// capped at [`RECENT_FILES_MAX`] entries (P4-02, task P5-09).
fn remember_recent(settings: &mut crate::Settings, path: PathBuf) {
    settings.recent_files.retain(|entry| entry != &path);
    settings.recent_files.insert(0, path);
    settings.recent_files.truncate(RECENT_FILES_MAX);
}

/// How many recently opened files the settings keep.
const RECENT_FILES_MAX: usize = 10;

/// `file.save`: writes the active document to its `path`.
///
/// A never-saved document has no target: interactive callers get
/// [`CommandError::NeedsInput`] (the desktop save dialog, ADR 0036) and
/// the wasm build returns the package in [`Outcome::payload`] for the
/// browser download (ADR 0036).
pub(super) fn run_save(ctx: &mut AppCtx, inv: &Invocation) -> Result<Outcome, CommandError> {
    save_document(ctx, inv, false)
}

/// `file.save_as`: like `file.save`, and the `path` argument becomes
/// the session's `path`.
pub(super) fn run_save_as(ctx: &mut AppCtx, inv: &Invocation) -> Result<Outcome, CommandError> {
    save_document(ctx, inv, true)
}

/// The shared body of `file.save` and `file.save_as`.
fn save_document(
    ctx: &mut AppCtx,
    inv: &Invocation,
    set_path: bool,
) -> Result<Outcome, CommandError> {
    let arg_path = inv.args.path("path").map(PathBuf::from);
    // On wasm there is no file system: a `path` argument still hands
    // the package back for the browser download (P4-02, task P5-09)
    // instead of failing inside `std::fs::write`.
    #[cfg(target_arch = "wasm32")]
    if arg_path.is_some() {
        return payload_save(ctx);
    }
    let target = arg_path
        .clone()
        .or_else(|| ctx.active_document().and_then(|s| s.path.clone()));
    let Some(path) = target else {
        return save_without_path(ctx);
    };
    let session = ctx.active_document_mut().ok_or_else(no_document)?;
    let empty = Package::new();
    let preserve = session.preserve.as_ref().unwrap_or(&empty);
    let bytes =
        write_document(&session.document, preserve).map_err(|error| io_error(&path, error))?;
    write_file(&path, &bytes)?;
    if set_path {
        session.path = Some(path.clone());
    }
    session.dirty = false;
    Ok(outcome(format!("Saved {}", path.display()), Vec::new()))
}

/// The no-target save of a never-saved document on the desktop: the
/// caller gets [`CommandError::NeedsInput`] and picks a path (ADR 0036).
#[cfg(not(target_arch = "wasm32"))]
fn save_without_path(_ctx: &mut AppCtx) -> Result<Outcome, CommandError> {
    Err(CommandError::NeedsInput(NeededInput::Path {
        purpose: PathPurpose::SaveFile,
        suggested: None,
    }))
}

/// The no-target save on wasm: there is no disk to write to, so the
/// serialized package goes back in [`Outcome::payload`] and the web
/// app downloads it (ADR 0036).
#[cfg(target_arch = "wasm32")]
fn save_without_path(ctx: &mut AppCtx) -> Result<Outcome, CommandError> {
    payload_save(ctx)
}

/// The body of the no-target save, kept on every target so the host
/// tests cover the wasm download path (P4-15 review minor 7): a
/// `write_document` failure is a write failure with the ADR 0026
/// `cannot write output:` prefix, not a `render failed:` one.
#[cfg(any(test, target_arch = "wasm32"))]
fn payload_save(ctx: &mut AppCtx) -> Result<Outcome, CommandError> {
    let session = ctx.active_document_mut().ok_or_else(no_document)?;
    let empty = Package::new();
    let preserve = session.preserve.as_ref().unwrap_or(&empty);
    let bytes = write_document(&session.document, preserve).map_err(write_error_text)?;
    let name = download_name(&session.display_name());
    session.dirty = false;
    Ok(Outcome {
        message: Some(format!("Saved {name}")),
        payload: Some(crate::Payload::Bytes { name, data: bytes }),
        ..Outcome::default()
    })
}

/// The suggested download name of a never-saved document.
#[cfg(any(test, target_arch = "wasm32"))]
fn download_name(display_name: &str) -> String {
    if display_name.contains('.') {
        display_name.to_owned()
    } else {
        format!("{display_name}.vsdx")
    }
}

/// `file.close`: closes the active session.
///
/// Unsaved work on an interactive source returns
/// [`UiRequest::ConfirmClose`] instead of closing (AGENTS.md section 6:
/// the command never opens the dialog itself); the caller answers by
/// dispatching `file.close` again with `confirm: true`. Programmatic
/// sources close without asking.
pub(super) fn run_close(ctx: &mut AppCtx, inv: &Invocation) -> Result<Outcome, CommandError> {
    let Some(index) = ctx.active else {
        return Err(no_document());
    };
    let confirm = inv.args.flag("confirm").unwrap_or(false);
    let (name, dirty) = match ctx.active_document() {
        Some(session) => (session.display_name(), session.dirty),
        None => return Err(no_document()),
    };
    if dirty && inv.source.is_interactive() && !confirm {
        return Ok(Outcome {
            message: Some(format!("Unsaved changes in {name}")),
            ui_request: Some(UiRequest::ConfirmClose { name }),
            ..Outcome::default()
        });
    }
    ctx.close_document(index);
    Ok(outcome(format!("Closed {name}"), Vec::new()))
}

/// `app.quit` and `file.exit`: asks the dispatcher to quit (P4-15; one
/// handler for both ids, so "one quit id" is answered by binding both).
///
/// The command closes nothing itself and never exits a process: an
/// interactive dispatcher turns [`UiRequest::Quit`] into a viewport
/// close, a programmatic caller records it and keeps running
/// (AGENTS.md section 6).
pub(super) fn run_quit(_ctx: &mut AppCtx, _inv: &Invocation) -> Result<Outcome, CommandError> {
    Ok(Outcome {
        message: Some("Quit".to_owned()),
        ui_request: Some(UiRequest::Quit),
        ..Outcome::default()
    })
}

/// `file.inspect`: the ADR 0026 document summary in
/// [`Outcome::message`], as text or (with `json`) as one JSON object.
///
/// `path` reads a file; without it the active document is summarised,
/// and a caller without either gets [`CommandError::NeedsInput`].
pub(super) fn run_inspect(ctx: &mut AppCtx, inv: &Invocation) -> Result<Outcome, CommandError> {
    let json = json_arg(&inv.args);
    let page = optional_page(&inv.args)?;
    let (text, warnings) = match inv.args.path("path") {
        Some(path) => {
            let (document, _package) = load(path)?;
            inspect_document(&document, page, json)?
        }
        None => match ctx.active_document() {
            Some(session) => inspect_document(&session.document, page, json)?,
            None => {
                return Err(CommandError::NeedsInput(NeededInput::Path {
                    purpose: PathPurpose::OpenFile,
                    suggested: None,
                }));
            }
        },
    };
    record_warnings(ctx, &warnings);
    Ok(outcome(text, warnings))
}

/// The summary body of `file.inspect`.
fn inspect_document(
    doc: &Document,
    page: Option<u32>,
    json: bool,
) -> Result<(String, Vec<String>), CommandError> {
    let pages: Vec<(usize, &Page)> = match page {
        Some(number) => vec![(number as usize, select_page(doc, Some(number))?)],
        None => doc
            .pages
            .iter()
            .enumerate()
            .map(|(index, page)| (index.saturating_add(1), page))
            .collect(),
    };
    let mut summaries = Vec::new();
    let mut warnings = Vec::new();
    for (number, page) in pages {
        let scene = build(doc, page.id, DEFAULT_SCALE)?;
        warnings.extend(scene.warnings.iter().map(|w| warning_line(doc, w)));
        summaries.push(PageSummary {
            index: number,
            name: page.name.clone(),
            width: scene.width,
            height: scene.height,
            shapes: shape_count(doc, page),
            warnings: scene.warnings.len(),
        });
    }
    let summary = Summary {
        kind: kind_text(doc.kind),
        pages: summaries,
        masters: doc.masters.len(),
        styles: doc.stylesheet.styles.len(),
        themes: doc.themes.len(),
    };
    let text = if json { summary.json() } else { summary.text() };
    Ok((text, warnings))
}

/// `file.render`: renders one page to `out` (ADR 0026).
pub(super) fn run_render(ctx: &mut AppCtx, inv: &Invocation) -> Result<Outcome, CommandError> {
    let path = need_path(&inv.args, "path", PathPurpose::OpenFile)?;
    let out = need_path(&inv.args, "out", PathPurpose::SaveFile)?;
    let format = format_arg(&inv.args, &out)?;
    let scale = scale_arg(&inv.args)?;
    let background = background_arg(&inv.args)?;
    let page = page_arg(&inv.args)?;
    let (document, _package) = load(&path)?;
    let selected = select_page(&document, Some(page))?;
    let mut scene = build(&document, selected.id, scale)?;
    match background {
        Background::Keep => {}
        Background::Transparent => scene.background = None,
        Background::Color(color) => scene.background = Some(color),
    }
    let warnings: Vec<String> = scene
        .warnings
        .iter()
        .map(|warning| warning_line(&document, warning))
        .collect();
    let bytes = encode(&scene, format, scale)?;
    write_file(&out, &bytes)?;
    record_warnings(ctx, &warnings);
    Ok(outcome(format!("Rendered {}", out.display()), warnings))
}

/// `file.export`: renders every page into `out` as `page-<N>.<ext>`
/// (ADR 0026).
pub(super) fn run_export(ctx: &mut AppCtx, inv: &Invocation) -> Result<Outcome, CommandError> {
    let path = need_path(&inv.args, "path", PathPurpose::OpenFile)?;
    let out = need_path(&inv.args, "out", PathPurpose::SaveFile)?;
    let format = format_arg(&inv.args, &out)?;
    let scale = scale_arg(&inv.args)?;
    let (document, _package) = load(&path)?;
    if document.pages.is_empty() {
        return Err(read_error_text("the document has no pages to export"));
    }
    std::fs::create_dir_all(&out).map_err(|error| io_error(&out, error))?;
    let mut warnings = Vec::new();
    for (index, page) in document.pages.iter().enumerate() {
        let number = index.saturating_add(1);
        let scene = build(&document, page.id, scale)?;
        let page_warnings: Vec<String> = scene
            .warnings
            .iter()
            .map(|warning| warning_line(&document, warning))
            .collect();
        record_warnings(ctx, &page_warnings);
        warnings.extend(page_warnings);
        let bytes = encode(&scene, format, scale)?;
        write_file(
            &out.join(format!("page-{number}.{}", format.extension())),
            &bytes,
        )?;
    }
    Ok(outcome(
        format!(
            "Exported {} pages to {}",
            document.pages.len(),
            out.display()
        ),
        warnings,
    ))
}

// ---------------------------------------------------------------------------
// The inspect summary (ADR 0026) and the warning report (ADR 0021)
// ---------------------------------------------------------------------------

/// One page row of the inspect summary (ADR 0026).
pub struct PageSummary {
    /// The page number, counted from 1 in document order.
    pub index: usize,
    /// The page name, as written in the file.
    pub name: String,
    /// The page width in inches.
    pub width: f64,
    /// The page height in inches.
    pub height: f64,
    /// The number of shapes on the page, groups included.
    pub shapes: usize,
    /// The number of rendering warnings of the page (ADR 0021).
    pub warnings: usize,
}

/// The inspect summary of a document (ADR 0026).
pub struct Summary {
    /// The document kind: `drawing`, `stencil` or `template`.
    pub kind: &'static str,
    /// The pages, in document order.
    pub pages: Vec<PageSummary>,
    /// The number of masters.
    pub masters: usize,
    /// The number of stylesheet rows.
    pub styles: usize,
    /// The number of theme parts.
    pub themes: usize,
}

impl Summary {
    /// The total rendering warning count of the summarised pages.
    pub fn warnings(&self) -> usize {
        self.pages.iter().map(|page| page.warnings).sum()
    }

    /// The compact text form (ADR 0026).
    pub fn text(&self) -> String {
        let mut text = String::new();
        text.push_str(&format!("kind: {}\n", self.kind));
        text.push_str(&format!("pages: {}\n", self.pages.len()));
        for page in &self.pages {
            let name = if page.name.is_empty() {
                "(unnamed)"
            } else {
                page.name.as_str()
            };
            text.push_str(&format!(
                "  {}: {} {} x {} in, {} shapes, {} warnings\n",
                page.index, name, page.width, page.height, page.shapes, page.warnings
            ));
        }
        text.push_str(&format!("masters: {}\n", self.masters));
        text.push_str(&format!("styles: {}\n", self.styles));
        text.push_str(&format!("themes: {}\n", self.themes));
        text.push_str(&format!("warnings: {}\n", self.warnings()));
        text
    }

    /// The JSON form: one object, stable key order (ADR 0026).
    pub fn json(&self) -> String {
        let value = serde_json::json!({
            "kind": self.kind,
            "pages": self.pages.iter().map(|page| serde_json::json!({
                "index": page.index,
                "name": page.name,
                "width": finite(page.width),
                "height": finite(page.height),
                "shapes": page.shapes,
                "warnings": page.warnings,
            })).collect::<Vec<_>>(),
            "masters": self.masters,
            "styles": self.styles,
            "themes": self.themes,
            "warnings": self.warnings(),
        });
        serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())
    }
}

/// A finite JSON number, or 0 (serde_json rejects NaN and infinity).
fn finite(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

/// The document kind as a word (ADR 0026).
pub fn kind_text(kind: DocumentKind) -> &'static str {
    match kind {
        DocumentKind::Drawing => "drawing",
        DocumentKind::Stencil => "stencil",
        DocumentKind::Template => "template",
        _ => "unknown",
    }
}

/// The number of shapes on a page, group sub-shapes included.
///
/// A broken or cyclic `children` list is walked once, never looped on.
pub fn shape_count(doc: &Document, page: &Page) -> usize {
    let mut seen = vec![false; doc.shapes.len()];
    let mut count = 0usize;
    let mut stack: Vec<ferrogram_model::ShapeId> = page.shapes.to_vec();
    while let Some(id) = stack.pop() {
        let Some(slot) = seen.get_mut(id.index as usize) else {
            continue;
        };
        if *slot {
            continue;
        }
        *slot = true;
        count = count.saturating_add(1);
        if let Some(shape) = doc.shape(id) {
            stack.extend(shape.children.iter().copied());
        }
    }
    count
}

/// One warning line, e.g. `shape 12 LineWeight: no cached value, using 0`
/// (ADR 0021).
pub fn warning_line(doc: &Document, warning: &ferrogram_model::CellWarning) -> String {
    let used = value_text(&warning.substituted);
    let reason = match &warning.kind {
        ferrogram_model::WarningKind::Missing => format!("no cached value, using {used}"),
        ferrogram_model::WarningKind::ErrorValue(error) => {
            format!("cached error {error:?}, using {used}")
        }
        ferrogram_model::WarningKind::KindMismatch => format!("wrong kind, using {used}"),
        ferrogram_model::WarningKind::UnresolvedTheme => {
            format!("the theme cannot supply it, using {used}")
        }
        _ => format!("value unavailable, using {used}"),
    };
    format!(
        "{} {}: {reason}",
        sheet_text(doc, warning.sheet),
        warning.cell.as_str()
    )
}

/// The sheet of a warning as `shape 12` or `page 2`.
pub fn sheet_text(doc: &Document, sheet: SheetRef) -> String {
    match sheet {
        SheetRef::Shape(id) => match doc.shape(id) {
            Some(shape) => format!("shape {}", shape.visio_id),
            None => format!("shape #{}", id.index),
        },
        SheetRef::Page(id) => match doc.page(id) {
            Some(page) => format!("page {}", page.visio_id),
            None => format!("page #{}", id.index),
        },
    }
}

/// The substituted value of a warning, for the report line.
pub fn value_text(value: &CellValue) -> String {
    match value {
        CellValue::Number(number) => format!("{number}"),
        CellValue::Str(text) => text.clone(),
        CellValue::Bool(flag) => flag.to_string(),
        CellValue::Color(color) => format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b),
        CellValue::List(text) => text.clone(),
        CellValue::Error(text) => text.clone(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrogram_model::{CellName, Row, Section, SectionName, Shape, ShapeId};
    use ferrogram_vsdx::{ContentType, Part, PartData, PartName, from_model};

    use crate::{ArgValue, Args, Source};

    /// A `Probe` cell with `formula` and `value`.
    fn probe_cell(name: &str, formula: Option<&str>, value: Option<CellValue>) -> Cell {
        let mut cell = Cell::new(CellName::new(name));
        cell.formula = formula.map(str::to_owned);
        cell.value = value;
        cell
    }

    /// The bytes of a one-shape package whose cells carry the given
    /// formulas and cached values.
    #[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test helper
    fn probe_bytes(cells: Vec<Cell>, sections: Vec<Section>) -> Vec<u8> {
        let mut shape = Shape::new(ShapeId { index: 0 }, 1);
        shape.sheet.cells = cells;
        shape.sheet.sections = sections;
        let mut doc = Document::new();
        let page = doc.add_page("Page-1").expect("page");
        doc.insert_shape(page, shape, None).expect("shape");
        let package = from_model(&doc).expect("from_model");
        write_document(&doc, &package).expect("write")
    }

    /// The value of the first shape's `name` cell.
    #[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)] // test helper
    fn cell_value(document: &Document, name: &str) -> Option<CellValue> {
        let mut found = None;
        for shape in &document.shapes {
            for cell in &shape.sheet.cells {
                if cell.name.as_str() == name {
                    found = cell.value.clone();
                }
            }
            for section in &shape.sheet.sections {
                for cell in &section.cells {
                    if cell.name.as_str() == name {
                        found = cell.value.clone();
                    }
                }
                for row in &section.rows {
                    for cell in &row.cells {
                        if cell.name.as_str() == name {
                            found = cell.value.clone();
                        }
                    }
                }
            }
        }
        found
    }

    /// P6-08: a document with a formula and no cached value gains one
    /// after the fill-in pass (ADR 0057), wherever the cell sits: a
    /// sheet cell, a section cell or a row cell.
    #[test]
    fn a_formula_without_a_cached_value_gains_one_on_read() {
        let mut section = Section::new(SectionName::from_static("Geometry"));
        section
            .cells
            .push(probe_cell("SectionCell", Some("2+2"), None));
        let mut row = Row::new();
        row.row_type = Some("MoveTo".to_owned());
        row.ix = Some(0);
        row.cells.push(probe_cell("X", Some("3+3"), None));
        section.rows.push(row);
        let bytes = probe_bytes(
            vec![probe_cell("SheetCell", Some("1+1"), None)],
            vec![section],
        );
        let (document, _) = read_package_document(&bytes, Path::new("probe.vsdx")).expect("read");
        assert_eq!(
            cell_value(&document, "SheetCell"),
            Some(CellValue::Number(2.0))
        );
        assert_eq!(cell_value(&document, "X"), Some(CellValue::Number(6.0)));
        // The section cell too (P6-11): a section-level cell resolves
        // against its section (the `graph::effective` dispatch), so the
        // fill-in pass evaluates it like any other cell.
        assert_eq!(
            cell_value(&document, "SectionCell"),
            Some(CellValue::Number(4.0))
        );
        for shape in &document.shapes {
            for section in &shape.sheet.sections {
                for cell in &section.cells {
                    assert_eq!(cell.name.as_str(), "SectionCell");
                    assert_eq!(cell.formula.as_deref(), Some("2+2"));
                }
            }
        }
    }

    /// P6-08: a document whose cells carry cached values is unchanged
    /// by the fill-in pass (ADR 0057: the cache is trusted).
    #[test]
    fn a_cached_value_is_unchanged_by_the_fill_in_pass() {
        let bytes = probe_bytes(
            vec![probe_cell(
                "Probe",
                Some("1+1"),
                Some(CellValue::Number(7.0)),
            )],
            Vec::new(),
        );
        let (document, _) = read_package_document(&bytes, Path::new("probe.vsdx")).expect("read");
        assert_eq!(cell_value(&document, "Probe"), Some(CellValue::Number(7.0)));
    }

    /// P6-08: the fill-in pass runs at load, so `file.open` takes no
    /// undo entry and does not mark the session dirty (ADR 0057).
    #[test]
    fn the_fill_in_pass_takes_no_undo_entry() {
        let bytes = probe_bytes(vec![probe_cell("Probe", Some("1+1"), None)], Vec::new());
        let mut ctx = AppCtx::new(crate::Settings::default());
        let invocation = Invocation::new(
            Args::new()
                .with("name", ArgValue::Text("probe.vsdx".to_owned()))
                .with("data", ArgValue::Bytes(bytes)),
            Source::Ui,
        );
        let outcome = run_open(&mut ctx, &invocation).expect("open");
        assert!(outcome.message.is_some());
        let session = ctx.active_document().expect("session");
        assert!(
            session.undo.is_empty(),
            "the fill-in pass must take no undo entry"
        );
        assert!(!session.dirty, "the fill-in pass must not mark the session");
        assert_eq!(
            cell_value(&session.document, "Probe"),
            Some(CellValue::Number(2.0))
        );
    }

    /// P4-15 review minor 7: the no-target save (the wasm download
    /// path, kept compiled on every target) returns a payload and
    /// reports a write failure with the ADR 0026 `cannot write
    /// output:` prefix, never a `render failed:` one.
    #[test]
    fn payload_save_returns_a_download_payload() {
        let mut ctx = AppCtx::new(crate::Settings::default());
        ctx.open_document(DocumentSession::new(Document::new(), None));
        let outcome = payload_save(&mut ctx).expect("payload");
        assert_eq!(outcome.message.as_deref(), Some("Saved Untitled.vsdx"));
        match outcome.payload {
            Some(crate::Payload::Bytes { name, data }) => {
                assert_eq!(name, "Untitled.vsdx");
                assert!(!data.is_empty(), "a package was written");
            }
            other => panic!("expected a payload, got {other:?}"),
        }
        let session = ctx.active_document().expect("session");
        assert!(!session.dirty);
    }

    #[test]
    fn payload_save_without_a_document_reports_no_open_document() {
        let mut ctx = AppCtx::new(crate::Settings::default());
        let error = payload_save(&mut ctx).expect_err("no document");
        assert_eq!(error.to_string(), "no open document");
    }

    #[test]
    fn payload_save_reports_a_write_failure_as_a_write_failure() {
        let mut preserve = Package::new();
        preserve.parts.push(Part {
            name: PartName::new("/not-a-visio-main.bin"),
            content_type: ContentType::new("application/octet-stream"),
            data: PartData::Binary(vec![1, 2, 3]),
        });
        let mut ctx = AppCtx::new(crate::Settings::default());
        let session = DocumentSession::new(Document::new(), None).with_preserve(preserve);
        ctx.open_document(session);
        let error = payload_save(&mut ctx).expect_err("the write fails");
        let message = error.to_string();
        assert!(message.starts_with("cannot write output:"), "{message}");
        assert!(!message.starts_with("render failed:"), "{message}");
    }

    #[test]
    fn recent_files_are_most_recent_first_deduplicated_and_capped() {
        let mut settings = crate::Settings::default();
        remember_recent(&mut settings, PathBuf::from("/a.vsdx"));
        remember_recent(&mut settings, PathBuf::from("/b.vsdx"));
        remember_recent(&mut settings, PathBuf::from("/a.vsdx"));
        assert_eq!(
            settings.recent_files,
            vec![PathBuf::from("/a.vsdx"), PathBuf::from("/b.vsdx")]
        );
        for index in 0..12 {
            remember_recent(&mut settings, PathBuf::from(format!("/{index}.vsdx")));
        }
        assert_eq!(settings.recent_files.len(), RECENT_FILES_MAX);
        assert_eq!(
            settings.recent_files.first(),
            Some(&PathBuf::from("/11.vsdx"))
        );
        let mut unique = settings.recent_files.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), settings.recent_files.len());
    }

    #[test]
    fn download_name_keeps_real_names_and_extends_bare_ones() {
        assert_eq!(download_name("drawing.vsdx"), "drawing.vsdx");
        assert_eq!(download_name("Untitled"), "Untitled.vsdx");
    }
}
