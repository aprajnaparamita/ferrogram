//! Command execution (ADR 0026).
//!
//! `run_to` executes one [`Command`] through library calls
//! (`ferrogram_vsdx::read_document`, `ferrogram_render::build_scene`,
//! `render_png`/`render_svg`) and maps every outcome to an exit code.
//! Nothing writes to stdout except `inspect` output; errors and warnings
//! are one line each on the error stream (ADR 0021/0026).

use std::io::Write;
use std::path::Path;

use ferrogram_model::{CellValue, Document, Page, PageId, SheetRef};
use ferrogram_render::{
    BuildOptions, PngOptions, Scene, SvgOptions, build_scene, render_png, render_svg,
};
use ferrogram_vsdx::{Limits, read_document};

use super::inspect::{PageSummary, Summary, kind_text, shape_count};
use crate::cli::{Background, Command, Error, Export, Inspect, OutputFormat, Render, exit};

/// The default device scale: 96 pixels per inch (ADR 0025).
pub(crate) const DEFAULT_SCALE: f64 = ferrogram_render::build::DEFAULT_SCALE;

/// Runs a command against the given output and error streams.
///
/// A failure is reported to `err` as `error: <message>` and mapped to its
/// exit code by [`Error::exit_code`]. `Err` is returned only when that
/// report cannot be written.
pub(crate) fn run_to(
    command: &Command,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<i32, Error> {
    match execute(command, out, err) {
        Ok(code) => Ok(code),
        Err(error) => match report(err, &format!("error: {error}")) {
            Ok(()) => Ok(error.exit_code()),
            Err(_) => Err(error),
        },
    }
}

/// Writes one report line to `to`, adding the newline, and flushes.
pub(crate) fn report(to: &mut dyn Write, line: &str) -> std::io::Result<()> {
    to.write_all(line.as_bytes())?;
    if !line.ends_with('\n') {
        to.write_all(b"\n")?;
    }
    to.flush()
}

/// Executes a command and returns the exit code of a successful or
/// warning-laden run (ADR 0026).
fn execute(command: &Command, out: &mut dyn Write, err: &mut dyn Write) -> Result<i32, Error> {
    match command {
        Command::Inspect(args) => inspect(args, out, err),
        Command::Render(args) => render(args, err),
        Command::Export(args) => export(args, err),
    }
}

/// `inspect`: the document summary to `out`, the warnings to `err`.
fn inspect(args: &Inspect, out: &mut dyn Write, err: &mut dyn Write) -> Result<i32, Error> {
    let doc = load(&args.file)?;
    let pages: Vec<(usize, &Page)> = match args.page {
        Some(number) => vec![(number as usize, select_page(&doc, Some(number))?)],
        None => doc
            .pages
            .iter()
            .enumerate()
            .map(|(index, page)| (index.saturating_add(1), page))
            .collect(),
    };
    let mut summaries = Vec::new();
    for (number, page) in pages {
        let scene = build(&doc, page.id, DEFAULT_SCALE)?;
        print_warnings(&doc, &scene.warnings, err)?;
        summaries.push(PageSummary {
            index: number,
            name: page.name.clone(),
            width: scene.width,
            height: scene.height,
            shapes: shape_count(&doc, page),
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
    let text = if args.json {
        summary.json()
    } else {
        summary.text()
    };
    let mut text = text.into_bytes();
    if text.last() != Some(&b'\n') {
        text.push(b'\n');
    }
    out.write_all(&text)
        .and_then(|()| out.flush())
        .map_err(|error| Error::Io(format!("standard output: {error}")))?;
    Ok(exit::OK)
}

/// `render`: one page to one file.
fn render(args: &Render, err: &mut dyn Write) -> Result<i32, Error> {
    let doc = load(&args.file)?;
    let page = select_page(&doc, args.page)?;
    let mut scene = build(&doc, page.id, args.scale)?;
    match args.background {
        Some(Background::Transparent) => scene.background = None,
        Some(Background::Color(color)) => scene.background = Some(color),
        None => {}
    }
    let warnings = std::mem::take(&mut scene.warnings);
    print_warnings(&doc, &warnings, err)?;
    let bytes = encode(&scene, args.format, args.scale)?;
    write_file(&args.out, &bytes)?;
    Ok(strict_exit(args.strict, warnings.len()))
}

/// `export`: every page into `--out` as `page-<N>.<ext>` (ADR 0026).
fn export(args: &Export, err: &mut dyn Write) -> Result<i32, Error> {
    let doc = load(&args.file)?;
    if doc.pages.is_empty() {
        return Err(Error::Read(
            "the document has no pages to export".to_owned(),
        ));
    }
    std::fs::create_dir_all(&args.out)
        .map_err(|error| Error::Io(format!("{}: {error}", args.out.display())))?;
    let extension = match args.format {
        OutputFormat::Png => "png",
        OutputFormat::Svg => "svg",
    };
    let mut warnings_total = 0usize;
    for (index, page) in doc.pages.iter().enumerate() {
        let number = index.saturating_add(1);
        let scene = build(&doc, page.id, args.scale)?;
        print_warnings(&doc, &scene.warnings, err)?;
        warnings_total = warnings_total.saturating_add(scene.warnings.len());
        let bytes = encode(&scene, args.format, args.scale)?;
        write_file(&args.out.join(format!("page-{number}.{extension}")), &bytes)?;
    }
    Ok(strict_exit(args.strict, warnings_total))
}

/// Reads and parses the input file (ADR 0026 exit code 2).
fn load(path: &Path) -> Result<Document, Error> {
    let bytes =
        std::fs::read(path).map_err(|error| Error::Read(format!("{}: {error}", path.display())))?;
    read_document(&bytes, &Limits::DEFAULT)
        .map_err(|error| Error::Read(format!("{}: {error}", path.display())))
}

/// The selected page (1-based), or the first one.
fn select_page(doc: &Document, page: Option<u32>) -> Result<&Page, Error> {
    let number = page.unwrap_or(1);
    doc.pages
        .get(number.saturating_sub(1) as usize)
        .ok_or_else(|| {
            Error::Read(format!(
                "page {number} does not exist; the document has {} pages",
                doc.pages.len()
            ))
        })
}

/// Builds the scene of one page (ADR 0020/0021).
fn build(doc: &Document, page: PageId, scale: f64) -> Result<Scene, Error> {
    let mut options = BuildOptions::new();
    options.scale = scale;
    build_scene(doc, page, &options).map_err(|error| Error::Render(error.to_string()))
}

/// Encodes a scene in the requested format (ADR 0025).
fn encode(scene: &Scene, format: OutputFormat, scale: f64) -> Result<Vec<u8>, Error> {
    match format {
        OutputFormat::Png => {
            let options = PngOptions {
                scale,
                antialias: true,
            };
            render_png(scene, &options).map_err(|error| Error::Render(error.to_string()))
        }
        OutputFormat::Svg => {
            let options = SvgOptions { scale };
            let svg =
                render_svg(scene, &options).map_err(|error| Error::Render(error.to_string()))?;
            Ok(svg.into_bytes())
        }
    }
}

/// Writes one output file (ADR 0026 exit code 3).
fn write_file(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    std::fs::write(path, bytes).map_err(|error| Error::Io(format!("{}: {error}", path.display())))
}

/// The exit code of a run that produced warnings (ADR 0021).
fn strict_exit(strict: bool, warnings: usize) -> i32 {
    if strict && warnings > 0 {
        exit::WARNINGS
    } else {
        exit::OK
    }
}

/// Prints one line per warning to `err` (ADR 0021).
fn print_warnings(
    doc: &Document,
    warnings: &[ferrogram_model::CellWarning],
    err: &mut dyn Write,
) -> Result<(), Error> {
    for warning in warnings {
        let line = format!("warning: {}", warning_line(doc, warning));
        report(err, &line).map_err(|error| Error::Io(format!("standard error: {error}")))?;
    }
    Ok(())
}

/// One warning line, e.g. `shape 12 LineWeight: no cached value, using 0`
/// (ADR 0021).
fn warning_line(doc: &Document, warning: &ferrogram_model::CellWarning) -> String {
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
fn sheet_text(doc: &Document, sheet: SheetRef) -> String {
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
fn value_text(value: &CellValue) -> String {
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
    use std::ffi::OsString;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    use ferrogram_model::{Cell, CellName, CellValue, Shape, ShapeId, Stylesheet, Theme};
    use ferrogram_vsdx::{from_model, write_package};

    use super::*;
    use crate::cli::parse::USAGE_TEXT;

    /// A private temp directory for one test.
    fn temp_dir() -> PathBuf {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let name = format!(
            "ferrogram-cli-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let dir = std::env::temp_dir().join(name);
        std::fs::create_dir_all(&dir).expect("temp directory");
        dir
    }

    /// A cell with a cached value.
    fn cell(name: &str, value: CellValue) -> Cell {
        let mut cell = Cell::new(CellName::new(name));
        cell.value = Some(value);
        cell
    }

    /// A cell with a formula but no cached value: the reader warns.
    fn formula_cell(name: &str, formula: &str) -> Cell {
        let mut cell = Cell::new(CellName::new(name));
        cell.formula = Some(formula.to_owned());
        cell
    }

    /// A two-page document; the first page has one shape whose `PinX` is a
    /// string (a kind mismatch warning), the second is empty.
    fn document() -> Document {
        let mut doc = Document::new();
        let mut shape = Shape::new(ShapeId { index: 0 }, 1);
        shape.name = "Rect".to_owned();
        shape
            .sheet
            .cells
            .push(cell("PinX", CellValue::Str("oops".to_owned())));
        shape.sheet.cells.push(formula_cell("PinY", "1.5"));
        let id = doc.add_shape(shape);

        let mut page = Page::new(PageId { index: 0 }, 1);
        page.name = "Page-1".to_owned();
        page.sheet
            .cells
            .push(cell("PageWidth", CellValue::Number(8.5)));
        page.sheet
            .cells
            .push(cell("PageHeight", CellValue::Number(11.0)));
        page.shapes.push(id);
        doc.pages.push(page);

        let mut page = Page::new(PageId { index: 1 }, 2);
        page.name = "Page-2".to_owned();
        page.sheet
            .cells
            .push(cell("PageWidth", CellValue::Number(11.0)));
        page.sheet
            .cells
            .push(cell("PageHeight", CellValue::Number(8.5)));
        doc.pages.push(page);
        doc
    }

    /// The bytes of a document as a `.vsdx` package.
    fn package_bytes(doc: &Document) -> Vec<u8> {
        let package = from_model(doc).expect("package");
        write_package(&package).expect("bytes")
    }

    /// Writes the sample document to a temp file and returns its path.
    fn sample_file() -> (PathBuf, PathBuf) {
        let dir = temp_dir();
        let file = dir.join("sample.vsdx");
        std::fs::write(&file, package_bytes(&document())).expect("sample file");
        (dir, file)
    }

    /// Writes raw bytes to a temp file and returns its path.
    fn raw_file(bytes: &[u8]) -> (PathBuf, PathBuf) {
        let dir = temp_dir();
        let file = dir.join("raw.vsdx");
        std::fs::write(&file, bytes).expect("raw file");
        (dir, file)
    }

    fn render_command(file: &Path, out: &Path) -> Render {
        Render {
            file: file.to_owned(),
            out: out.to_owned(),
            page: None,
            format: OutputFormat::Png,
            scale: DEFAULT_SCALE,
            background: None,
            strict: false,
        }
    }

    fn run_buffered(command: &Command) -> (Result<i32, Error>, String, String) {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let result = run_to(command, &mut out, &mut err);
        (
            result,
            String::from_utf8_lossy(&out).into_owned(),
            String::from_utf8_lossy(&err).into_owned(),
        )
    }

    #[test]
    fn render_writes_a_png_and_exits_zero() {
        let (dir, file) = sample_file();
        let out = dir.join("page.png");
        let (result, stdout, stderr) = run_buffered(&Command::Render(render_command(&file, &out)));
        assert_eq!(result, Ok(exit::OK));
        assert!(stdout.is_empty(), "render must not write to stdout");
        let bytes = std::fs::read(&out).expect("png");
        assert_eq!(&bytes[..4], &[0x89, b'P', b'N', b'G']);
        assert!(
            stderr.contains("warning: "),
            "warnings go to stderr: {stderr}"
        );
    }

    #[test]
    fn render_writes_svg_and_applies_the_background() {
        let (dir, file) = sample_file();
        let out = dir.join("page.svg");
        let mut command = render_command(&file, &out);
        command.format = OutputFormat::Svg;
        command.background = Some(Background::Color(ferrogram_render::Rgba::opaque(1, 2, 3)));
        let (result, _, _) = run_buffered(&Command::Render(command));
        assert_eq!(result, Ok(exit::OK));
        let svg = std::fs::read_to_string(&out).expect("svg");
        assert!(svg.contains("<svg"), "not an SVG document: {svg}");
    }

    #[test]
    fn render_background_none_is_transparent() {
        let (dir, file) = sample_file();
        let out = dir.join("page.svg");
        let mut command = render_command(&file, &out);
        command.format = OutputFormat::Svg;
        command.background = Some(Background::Transparent);
        let (result, _, _) = run_buffered(&Command::Render(command));
        assert_eq!(result, Ok(exit::OK));
    }

    #[test]
    fn render_selects_a_page() {
        let (dir, file) = sample_file();
        let out = dir.join("page-2.png");
        let mut command = render_command(&file, &out);
        command.page = Some(2);
        let (result, _, _) = run_buffered(&Command::Render(command));
        assert_eq!(result, Ok(exit::OK));
        assert!(out.exists());
    }

    #[test]
    fn export_writes_every_page() {
        let (dir, file) = sample_file();
        let out = dir.join("pages");
        let command = Command::Export(Export {
            file,
            out: out.clone(),
            format: OutputFormat::Png,
            scale: DEFAULT_SCALE,
            strict: false,
        });
        let (result, stdout, _) = run_buffered(&command);
        assert_eq!(result, Ok(exit::OK));
        assert!(stdout.is_empty(), "export must not write to stdout");
        assert!(out.join("page-1.png").exists());
        assert!(out.join("page-2.png").exists());
    }

    #[test]
    fn export_writes_svg_names() {
        let (dir, file) = sample_file();
        let out = dir.join("pages-svg");
        let command = Command::Export(Export {
            file,
            out: out.clone(),
            format: OutputFormat::Svg,
            scale: DEFAULT_SCALE,
            strict: false,
        });
        let (result, _, _) = run_buffered(&command);
        assert_eq!(result, Ok(exit::OK));
        assert!(out.join("page-1.svg").exists());
    }

    #[test]
    fn inspect_prints_the_text_summary() {
        let (_dir, file) = sample_file();
        let command = Command::Inspect(Inspect {
            file,
            json: false,
            page: None,
        });
        let (result, stdout, _) = run_buffered(&command);
        assert_eq!(result, Ok(exit::OK));
        assert!(stdout.contains("kind: drawing"), "{stdout}");
        assert!(stdout.contains("pages: 2"), "{stdout}");
    }

    #[test]
    fn inspect_json_lists_the_pages() {
        let (_dir, file) = sample_file();
        let command = Command::Inspect(Inspect {
            file,
            json: true,
            page: Some(1),
        });
        let (result, stdout, _) = run_buffered(&command);
        assert_eq!(result, Ok(exit::OK));
        let value: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
        assert_eq!(value["pages"].as_array().map(Vec::len), Some(1));
        assert_eq!(value["kind"], "drawing");
        assert!(stdout.ends_with('\n'), "output ends with a newline");
    }

    #[test]
    fn a_missing_file_is_exit_two() {
        let (dir, _file) = sample_file();
        let out = dir.join("page.png");
        let mut command = render_command(&dir.join("missing.vsdx"), &out);
        command.page = None;
        let (result, stdout, stderr) = run_buffered(&Command::Render(command));
        assert_eq!(result, Ok(exit::READ));
        assert!(stdout.is_empty());
        assert!(stderr.starts_with("error: cannot read input:"), "{stderr}");
    }

    #[test]
    fn an_unparsable_package_is_exit_two() {
        let (dir, file) = raw_file(b"this is not a zip archive");
        let out = dir.join("page.png");
        let (result, _, stderr) = run_buffered(&Command::Render(render_command(&file, &out)));
        assert_eq!(result, Ok(exit::READ));
        assert!(stderr.starts_with("error: cannot read input:"), "{stderr}");
    }

    #[test]
    fn a_page_beyond_the_document_is_exit_two() {
        let (dir, file) = sample_file();
        let out = dir.join("page.png");
        let mut command = render_command(&file, &out);
        command.page = Some(9);
        let (result, _, stderr) = run_buffered(&Command::Render(command));
        assert_eq!(result, Ok(exit::READ));
        assert!(stderr.contains("page 9 does not exist"), "{stderr}");
    }

    #[test]
    fn an_unwritable_output_is_exit_three() {
        let (_dir, file) = sample_file();
        let out = PathBuf::from("/this/directory/does/not/exist/page.png");
        let (result, _, stderr) = run_buffered(&Command::Render(render_command(&file, &out)));
        assert_eq!(result, Ok(exit::RENDER));
        assert!(stderr.contains("error: cannot write output:"), "{stderr}");
    }

    #[test]
    fn a_huge_scale_is_exit_three_not_a_panic() {
        let (dir, file) = sample_file();
        let out = dir.join("page.png");
        let mut command = render_command(&file, &out);
        command.scale = 1.0e300;
        let (result, _, stderr) = run_buffered(&Command::Render(command));
        assert_eq!(result, Ok(exit::RENDER));
        assert!(stderr.contains("larger than the allowed caps"), "{stderr}");
        assert!(!out.exists(), "no output for a capped raster");
    }

    #[test]
    fn a_huge_page_is_exit_three_not_a_panic() {
        let dir = temp_dir();
        let mut doc = Document::new();
        let mut shape = Shape::new(ShapeId { index: 0 }, 1);
        shape.sheet.cells.push(cell("PinX", CellValue::Number(1.0)));
        let id = doc.add_shape(shape);
        let mut page = Page::new(PageId { index: 0 }, 1);
        page.sheet
            .cells
            .push(cell("PageWidth", CellValue::Number(1.0e6)));
        page.sheet
            .cells
            .push(cell("PageHeight", CellValue::Number(1.0e6)));
        page.shapes.push(id);
        doc.pages.push(page);
        let file = dir.join("huge.vsdx");
        std::fs::write(&file, package_bytes(&doc)).expect("huge file");
        let out = dir.join("page.png");
        let (result, _, stderr) = run_buffered(&Command::Render(render_command(&file, &out)));
        assert_eq!(result, Ok(exit::RENDER));
        assert!(stderr.contains("larger than the allowed caps"), "{stderr}");
    }

    #[test]
    fn warnings_exit_zero_and_strict_exits_four() {
        let (dir, file) = sample_file();
        let out = dir.join("page.png");
        let (result, _, stderr) = run_buffered(&Command::Render(render_command(&file, &out)));
        assert_eq!(result, Ok(exit::OK));
        assert!(stderr.contains("warning: "), "{stderr}");

        let mut command = render_command(&file, &out);
        command.strict = true;
        let (result, _, stderr) = run_buffered(&Command::Render(command));
        assert_eq!(result, Ok(exit::WARNINGS));
        assert!(out.exists(), "the output is written even under --strict");
        assert!(stderr.contains("warning: "), "{stderr}");
    }

    #[test]
    fn export_strict_exits_four_on_warnings() {
        let (dir, file) = sample_file();
        let out = dir.join("pages");
        let command = Command::Export(Export {
            file,
            out: out.clone(),
            format: OutputFormat::Png,
            scale: DEFAULT_SCALE,
            strict: true,
        });
        let (result, _, _) = run_buffered(&command);
        assert_eq!(result, Ok(exit::WARNINGS));
        assert!(out.join("page-1.png").exists());
    }

    #[test]
    fn a_document_without_pages_cannot_export() {
        let dir = temp_dir();
        let mut doc = Document::new();
        doc.stylesheet = Stylesheet::new();
        doc.themes.push(Theme::new("none"));
        let file = dir.join("empty.vsdx");
        std::fs::write(&file, package_bytes(&doc)).expect("empty file");
        let out = dir.join("pages");
        let command = Command::Export(Export {
            file,
            out,
            format: OutputFormat::Png,
            scale: DEFAULT_SCALE,
            strict: false,
        });
        let (result, _, stderr) = run_buffered(&command);
        assert_eq!(result, Ok(exit::READ));
        assert!(stderr.contains("no pages"), "{stderr}");
    }

    #[test]
    fn report_writes_one_line_and_flushes() {
        let mut sink = Vec::new();
        report(&mut sink, "hello").expect("report");
        report(&mut sink, "line\n").expect("report");
        assert_eq!(String::from_utf8_lossy(&sink), "hello\nline\n");
    }

    /// A writer whose sink always fails, for the unreportable-error path.
    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("sink full"))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn an_unwritable_error_report_returns_the_error() {
        let (dir, _file) = sample_file();
        let command = Command::Inspect(Inspect {
            file: dir.join("missing.vsdx"),
            json: false,
            page: None,
        });
        let result = run_to(&command, &mut Vec::new(), &mut FailingWriter);
        match result {
            Err(Error::Read(message)) => assert!(message.contains("missing.vsdx")),
            other => panic!("expected Error::Read, got {other:?}"),
        }
    }

    #[test]
    fn an_unwritable_inspect_output_is_an_io_error() {
        let (_dir, file) = sample_file();
        let command = Command::Inspect(Inspect {
            file,
            json: false,
            page: None,
        });
        // The failure is reported to stderr and mapped to the output error
        // code (ADR 0026).
        let result = run_to(&command, &mut FailingWriter, &mut Vec::new());
        assert_eq!(result, Ok(exit::RENDER));
        // When the report cannot be written either, `run_to` returns the
        // error itself.
        let result = run_to(&command, &mut FailingWriter, &mut FailingWriter);
        match result {
            Err(Error::Io(message)) => assert!(message.contains("sink full"), "{message}"),
            other => panic!("expected Error::Io, got {other:?}"),
        }
    }

    #[test]
    fn main_from_maps_usage_errors_to_exit_one() {
        let result = crate::cli::main_from(["inspect"].map(OsString::from).into_iter());
        assert_eq!(result, Ok(exit::USAGE));
    }

    #[test]
    fn main_from_maps_help_to_exit_zero() {
        let result = crate::cli::main_from(["--help"].map(OsString::from).into_iter());
        assert_eq!(result, Ok(exit::OK));
    }

    #[test]
    fn main_from_runs_a_command() {
        let (_dir, file) = sample_file();
        let mut args: Vec<OsString> = vec!["inspect".into(), "--json".into()];
        args.push(file.into_os_string());
        let result = crate::cli::main_from(args.into_iter());
        assert_eq!(result, Ok(exit::OK));
    }

    #[test]
    fn usage_text_names_the_commands() {
        for name in ["inspect", "render", "export", "Exit codes:"] {
            assert!(USAGE_TEXT.contains(name), "usage text misses {name}");
        }
    }

    #[test]
    fn strict_exit_only_fails_with_warnings() {
        assert_eq!(strict_exit(true, 0), exit::OK);
        assert_eq!(strict_exit(true, 1), exit::WARNINGS);
        assert_eq!(strict_exit(false, 1), exit::OK);
    }
}
