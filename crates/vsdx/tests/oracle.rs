//! Oracle comparison against libvisio's `vsd2raw` (task P2-09, ADR 0010 and
//! ADR 0017 section 5).
//!
//! This test file is oracle-only: it consumes the CLI tool's stdout as an
//! independent opinion on the same file, and holds no libvisio code and no
//! description of libvisio's output format beyond what the comparison needs.
//! The oracle is built by `cargo xtask oracle build` (a dev-only harness under
//! `tools/libvisio-oracle/`); the test skips with a message when
//! `FERROGRAM_LIBVISIO_ORACLE` is unset, and `FERROGRAM_REQUIRE_ORACLE=1`
//! makes a missing oracle a failure (ADR 0010).
//!
//! What the raw dump supports: a page list with a name and a size in inches.
//! It renders pages through librevenge calls; it carries no master list, no
//! shape list and no ShapeSheet cells, so master counts, shape counts and the
//! `PinX`/`PinY`/`Width`/`Height` cells cannot be compared against it. The
//! test prints what it skips and compares what the dump does carry: the page
//! count and the page-size cells `PageWidth`/`PageHeight`.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use ferrogram_model::{CellValue, Document};
use ferrogram_vsdx::{Limits, read_document};

/// The oracle tools directory when `FERROGRAM_LIBVISIO_ORACLE` names an
/// existing directory; `None` when it is unset, empty or missing (ADR 0010).
pub fn oracle_dir() -> Option<PathBuf> {
    oracle_dir_from(std::env::var_os("FERROGRAM_LIBVISIO_ORACLE"))
}

/// Whether `FERROGRAM_REQUIRE_ORACLE=1` turns a missing oracle into a test
/// failure instead of a skip (set in CI on Linux and macOS).
pub fn require_oracle() -> bool {
    require_from(std::env::var("FERROGRAM_REQUIRE_ORACLE").as_deref().ok())
}

/// [`oracle_dir`] with an explicit value for the environment variable.
fn oracle_dir_from(var: Option<std::ffi::OsString>) -> Option<PathBuf> {
    let dir = PathBuf::from(var?);
    dir.is_dir().then_some(dir)
}

/// [`require_oracle`] with an explicit value for the environment variable.
fn require_from(var: Option<&str>) -> bool {
    var == Some("1")
}

/// One page of the oracle's dump: the name it printed and the size in inches.
#[derive(Debug, Clone, PartialEq)]
struct OraclePage {
    name: String,
    height_in: f64,
    width_in: f64,
}

/// The `key: value` of one raw-dump property-list entry.
fn property<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let start = line.find(key)? + key.len();
    let rest = line.get(start..)?;
    let end = rest
        .find(", ")
        .or_else(|| rest.find(')'))
        .unwrap_or(rest.len());
    rest.get(..end)
}

/// Parses one `startPage(...)` line of the dump.
fn parse_page(line: &str) -> Option<OraclePage> {
    let number = |key: &str| -> Option<f64> {
        property(line, key)?
            .trim()
            .trim_end_matches("in")
            .parse::<f64>()
            .ok()
    };
    Some(OraclePage {
        name: property(line, "draw:name: ")?.trim().to_owned(),
        height_in: number("svg:height: ")?,
        width_in: number("svg:width: ")?,
    })
}

/// The pages of the first `startDocument()` block of a dump.
///
/// A malformed file can make the oracle emit more than one document block
/// (`recursion-cycle.vsdx` emits two); the first block is the file's document.
fn first_document_pages(dump: &str) -> Vec<OraclePage> {
    let Some(start) = dump.find("startDocument()") else {
        return Vec::new();
    };
    let block = dump.get(start..).unwrap_or(dump);
    let block = block.split("endDocument").next().unwrap_or(block);
    block
        .lines()
        .filter(|line| line.contains("startPage("))
        .filter_map(parse_page)
        .collect()
}

/// Runs `<oracle>/vsd2raw` on one file and returns its stdout.
///
/// `None` means the tool produced no document dump for this file (it does not
/// support the format, or the file broke it): that is a skip, not a failure.
fn run_vsd2raw(oracle: &Path, file: &Path) -> Option<String> {
    let output = Command::new(oracle.join("vsd2raw"))
        .arg(file)
        .output()
        .ok()?;
    if !output.status.success() {
        eprintln!(
            "skipped {}: vsd2raw exited with {}",
            file.display(),
            output.status
        );
        return None;
    }
    let dump = String::from_utf8_lossy(&output.stdout).into_owned();
    if !dump.contains("startDocument()") {
        eprintln!("skipped {}: vsd2raw printed no document", file.display());
        return None;
    }
    Some(dump)
}

/// The `PageWidth` or `PageHeight` cell of a page sheet, in inches.
fn page_size_cell(document: &Document, page_index: usize, name: &str) -> Option<f64> {
    let page = document.pages.get(page_index)?;
    page.sheet
        .cells
        .iter()
        .find(|cell| cell.name.as_str() == name)
        .and_then(|cell| match cell.value {
            Some(CellValue::Number(value)) => Some(value),
            _ => None,
        })
}

/// A temporary directory for the fixture bytes the oracle reads. Unique per
/// call, so nested users never delete each other's files.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new() -> ScratchDir {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let unique = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("ferrogram-oracle-{}-{unique}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        ScratchDir(dir)
    }

    /// Writes `bytes` under `name` and returns the path.
    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        if std::fs::write(&path, bytes).is_err() {
            eprintln!("cannot write {}: comparisons for it are skipped", name);
        }
        path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The files to compare: the hand-authored well-formed fixtures (always) plus
/// every corpus `.vsdx` (fetched, not committed; ADR 0005).
fn comparison_files(scratch: &ScratchDir) -> Vec<(String, PathBuf)> {
    let mut files = Vec::new();
    for fixture in [
        common::Fixture::Drawing,
        common::Fixture::Stencil,
        common::Fixture::Template,
    ] {
        let name = common::file_name(&fixture);
        files.push((
            name.to_owned(),
            scratch.write(name, &common::build(fixture)),
        ));
    }
    for path in corpus_files() {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        files.push((name, path));
    }
    files
}

/// The `.vsdx` files of the corpus, sorted for a stable order.
fn corpus_files() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("corpus")
        .join("vsdx");
    let mut files: Vec<PathBuf> = match std::fs::read_dir(dir) {
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

/// Compares our parse of every file against `vsd2raw`'s dump: the page count
/// and the page-size cells. Everything the dump does not carry is printed as
/// skipped.
#[test]
fn oracle_agrees_on_page_counts_and_page_sizes() {
    let Some(oracle) = oracle_dir() else {
        assert!(
            !require_oracle(),
            "FERROGRAM_REQUIRE_ORACLE=1 but no oracle: FERROGRAM_LIBVISIO_ORACLE \
             is unset or not a directory"
        );
        eprintln!(
            "skipping the oracle comparison: set FERROGRAM_LIBVISIO_ORACLE (ADR 0010) \
             to the oracle's bin directory"
        );
        return;
    };
    eprintln!(
        "skipped comparisons the raw dump cannot support: master counts (it prints \
         no master list), shape counts (its layers are render layers, not shapes), \
         PinX/PinY/Width/Height (it prints rendered paths, not ShapeSheet cells)"
    );
    let scratch = ScratchDir::new();
    for (name, path) in comparison_files(&scratch) {
        let Some(dump) = run_vsd2raw(&oracle, &path) else {
            continue;
        };
        let bytes = std::fs::read(&path).unwrap_or_default();
        let document = read_document(&bytes, &Limits::DEFAULT)
            .unwrap_or_else(|why| panic!("{name}: our reader failed: {why}"));
        let pages = first_document_pages(&dump);
        assert_eq!(
            pages.len(),
            document.pages.len(),
            "{name}: page count (oracle vs reader)"
        );
        for (index, oracle_page) in pages.iter().enumerate() {
            let ours = &document.pages[index];
            eprintln!(
                "{name} page {index}: oracle name `{}` vs model name `{}`",
                oracle_page.name, ours.name
            );
            match (
                page_size_cell(&document, index, "PageWidth"),
                page_size_cell(&document, index, "PageHeight"),
            ) {
                (Some(width), Some(height)) => {
                    assert!(
                        (width - oracle_page.width_in).abs() <= 1e-3,
                        "{name} page {index}: PageWidth {width} in vs oracle {} in",
                        oracle_page.width_in
                    );
                    assert!(
                        (height - oracle_page.height_in).abs() <= 1e-3,
                        "{name} page {index}: PageHeight {height} in vs oracle {} in",
                        oracle_page.height_in
                    );
                }
                _ => eprintln!(
                    "{name} page {index}: skipped size comparison (no PageWidth/PageHeight \
                     cell in the model; the oracle prints 0.0000in for a missing page sheet)"
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The `oracle_dir` and `require_oracle` helpers of ADR 0010 are a
    // copy-paste per consumer crate; these are their unit tests.

    #[test]
    fn oracle_dir_finds_only_an_existing_directory() {
        assert_eq!(oracle_dir_from(None), None);
        assert_eq!(oracle_dir_from(Some("".into())), None);
        assert_eq!(oracle_dir_from(Some("/nope/nope".into())), None);
        let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        assert_eq!(oracle_dir_from(Some(here.clone().into())), Some(here));
    }

    #[test]
    fn require_oracle_reads_only_the_literal_one() {
        assert!(!require_from(None));
        assert!(!require_from(Some("")));
        assert!(!require_from(Some("0")));
        assert!(!require_from(Some("true")));
        assert!(require_from(Some("1")));
    }

    #[test]
    fn parse_page_reads_the_dump_line() {
        let line = "  startPage(draw:name: Page-1, svg:height: 8.2677in, svg:width: 11.6929in)";
        assert_eq!(
            parse_page(line),
            Some(OraclePage {
                name: "Page-1".to_owned(),
                height_in: 8.2677,
                width_in: 11.6929,
            })
        );
        assert_eq!(parse_page("  startPage(noise)"), None);
    }

    #[test]
    fn first_document_pages_takes_the_first_block_only() {
        let dump = "startDocument()\n  startPage(draw:name: A, svg:height: 1.0in, svg:width: 2.0in)\n\
                    endDocument\nstartDocument()\n  startPage(draw:name: B, svg:height: 3.0in, \
                    svg:width: 4.0in)\nendDocument\n";
        let pages = first_document_pages(dump);
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].name, "A");
        assert_eq!(pages[0].width_in, 2.0);
        assert!(first_document_pages("nothing here").is_empty());
    }
}
