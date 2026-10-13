//! Reading every corpus `.vsdx` file into the model (task P2-06).
//!
//! The corpus is fetched, never committed (ADR 0005); these tests skip with a
//! message when `corpus/vsdx/` is absent, and fail instead when
//! `FERROGRAM_REQUIRE_CORPUS=1` is set. The expected page and master counts
//! are recorded here (the corpus note of the task): they are the ground truth
//! of `pages.xml` and `masters.xml`, and the reader must reproduce them.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use ferrogram_model::aliases::table_name;
use ferrogram_model::generated::{CELLS, SECTIONS};
use ferrogram_model::{CellLookup, CellName, Document, Preserved, ValueSource, XmlChild, XmlNode};
use ferrogram_vsdx::{Limits, Package, PartData, read_document, read_package};

/// The corpus directory with the real `.vsdx` packages.
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

/// Returns `true` when no corpus is available and the run may skip.
fn corpus_missing(files: &[PathBuf]) -> bool {
    if files.is_empty() {
        eprintln!("skipping: corpus/vsdx is absent; run `cargo xtask corpus`");
    }
    files.is_empty()
}

/// The expected `(pages, masters)` counts per corpus file, read off
/// `pages.xml` and `masters.xml` (the corpus note of task P2-06).
const EXPECTED_COUNTS: &[(&str, usize, usize)] = &[
    ("bgcolor.vsdx", 1, 1),
    ("blue-box.vsdx", 1, 1),
    ("color-boxes.vsdx", 1, 1),
    ("dwg.vsdx", 1, 4),
    ("fdo86664.vsdx", 1, 3),
    ("qs-box.vsdx", 1, 1),
    ("recursion-cycle.vsdx", 1, 0),
    ("tab-short-prefix.vsdx", 1, 0),
    ("tdf136564-WhiteTextBackground.vsdx", 1, 3),
    ("tdf154379-QuickStyleFillMatrix.vsdx", 1, 23),
    ("testfile1.vsdx", 1, 4),
    ("testfile3.vsdx", 1, 4),
    ("testfile4.vsdx", 1, 1),
    ("testfile5.vsdx", 1, 1),
    ("testfile6.vsdx", 1, 2),
];

/// The cell names the corpus writes and the ShapeSheet table does not know,
/// after aliasing (task P2-06, recorded in its Result).
const UNKNOWN_CELLS: &[&str] = &[
    "AutoGen",
    "Copyright",
    "DataLinked",
    "GradientStopColor",
    "GradientStopColorTrans",
    "GradientStopPosition",
    "HelpTopic",
    "LayerMember",
    "NameUniv",
    "ShapeKeywords",
    "Status",
    "Verify",
];

/// Every corpus file reads into a document with the expected page and master
/// counts (the acceptance criterion of task P2-06).
#[test]
fn corpus_documents_read_with_the_expected_counts() {
    let files = corpus_files();
    if corpus_missing(&files) {
        assert!(
            std::env::var_os("FERROGRAM_REQUIRE_CORPUS").is_none(),
            "corpus/vsdx is missing and FERROGRAM_REQUIRE_CORPUS=1 is set"
        );
        return;
    }
    assert_eq!(files.len(), EXPECTED_COUNTS.len(), "corpus file count");
    for path in files {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let expected = EXPECTED_COUNTS
            .iter()
            .find(|(file, _, _)| *file == name)
            .unwrap_or_else(|| panic!("no recorded counts for {name}"));
        let bytes = std::fs::read(&path).expect("corpus file reads");
        let document =
            read_document(&bytes, &Limits::DEFAULT).unwrap_or_else(|why| panic!("{name}: {why}"));
        assert_eq!(
            document.kind,
            ferrogram_model::DocumentKind::Drawing,
            "{name}"
        );
        assert_eq!(document.pages.len(), expected.1, "{name}: pages");
        assert_eq!(document.masters.len(), expected.2, "{name}: masters");
    }
}

/// Every `Cell` element of the mapped parts is either a modeled cell or a
/// preserved one: nothing is dropped, and nothing is copied into another
/// sheet (which is what flattening on load would do).
#[test]
fn corpus_every_cell_is_modeled_or_preserved() {
    let files = corpus_files();
    if corpus_missing(&files) {
        assert!(
            std::env::var_os("FERROGRAM_REQUIRE_CORPUS").is_none(),
            "corpus/vsdx is missing and FERROGRAM_REQUIRE_CORPUS=1 is set"
        );
        return;
    }
    for path in files {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let bytes = std::fs::read(&path).expect("corpus file reads");
        let package = read_package(&bytes, &Limits::DEFAULT).expect("package reads");
        let document = read_document(&bytes, &Limits::DEFAULT).expect("document reads");
        assert_eq!(
            count_source_cells(&package),
            count_model_cells(&document),
            "{name}: cell count"
        );
    }
}

/// Every cell and section name of the corpus is either a name of the
/// ShapeSheet table or one of the recorded aliases' targets.
#[test]
fn corpus_cell_and_section_names_are_known_after_aliasing() {
    let files = corpus_files();
    if corpus_missing(&files) {
        assert!(
            std::env::var_os("FERROGRAM_REQUIRE_CORPUS").is_none(),
            "corpus/vsdx is missing and FERROGRAM_REQUIRE_CORPUS=1 is set"
        );
        return;
    }
    let mut unknown_cells = BTreeSet::new();
    let mut unknown_sections = BTreeSet::new();
    for path in files {
        let bytes = std::fs::read(&path).expect("corpus file reads");
        let package = read_package(&bytes, &Limits::DEFAULT).expect("package reads");
        for root in xml_roots(&package) {
            walk(root, &mut |node| match node.name.local.as_str() {
                "Cell" => {
                    if let Some(name) = name_of(node) {
                        let table = table_name(name);
                        if !CELLS.iter().any(|def| def.name == table) {
                            unknown_cells.insert(table.to_owned());
                        }
                    }
                }
                "Section" => {
                    if let Some(name) = name_of(node) {
                        let table = table_name(name);
                        if !SECTIONS.iter().any(|def| def.name == table) {
                            unknown_sections.insert(table.to_owned());
                        }
                    }
                }
                _ => {}
            });
        }
    }
    eprintln!("cell names with no table entry: {unknown_cells:?}");
    eprintln!("section names with no table entry: {unknown_sections:?}");
    let recorded: BTreeSet<String> = UNKNOWN_CELLS
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    assert_eq!(unknown_cells, recorded, "unrecorded cell names appear");
    assert!(
        unknown_sections.is_empty(),
        "unrecorded section names appear"
    );
}

/// A shape with a `LineStyle` and no local `LineWeight` resolves the cell
/// from its style, and `via` names that style (P2-06b decision 3).
#[test]
fn corpus_lineweight_resolves_from_the_shape_style() {
    let path = corpus_dir().join("blue-box.vsdx");
    if corpus_missing(&corpus_files()) {
        assert!(
            std::env::var_os("FERROGRAM_REQUIRE_CORPUS").is_none(),
            "corpus/vsdx is missing and FERROGRAM_REQUIRE_CORPUS=1 is set"
        );
        return;
    }
    let bytes = std::fs::read(&path).expect("corpus file reads");
    let document = read_document(&bytes, &Limits::DEFAULT).expect("document reads");

    // The master's content shape carries a `LineStyle` and no local
    // `LineWeight`.
    let shape_id = document.masters[0].shapes[0];
    let shape = document.shape(shape_id).expect("shape");
    let reference = shape.line_style.expect("the shape names a style");
    assert!(
        shape
            .sheet
            .cells
            .iter()
            .all(|cell| cell.name.as_str() != "LineWeight"),
        "the shape has no local LineWeight"
    );

    // The value comes from the style the reference names.
    let resolved = document
        .resolve(shape_id, &CellName::from_static("LineWeight"))
        .expect("resolves");
    let ValueSource::Style(id) = resolved.source else {
        panic!("expected the style rung, got {:?}", resolved.source);
    };
    let style = document.style(id).expect("style");
    assert_eq!(
        style.visio_id, reference,
        "the via style is the referenced one"
    );
    assert!(!style.name.is_empty());
    assert_eq!(
        resolved.via,
        vec![ValueSource::Local, ValueSource::Style(id)]
    );
    let style_cell = style
        .sheet
        .cells
        .iter()
        .find(|cell| cell.name.as_str() == "LineWeight")
        .expect("the style defines LineWeight");
    assert_eq!(resolved.value, style_cell.value.as_ref());
}

/// The `N` attribute of an element.
fn name_of(node: &XmlNode) -> Option<&str> {
    node.attributes
        .iter()
        .find(|attribute| attribute.name.local == "N")
        .map(|attribute| attribute.value.as_str())
}

/// The roots of every XML part of the package.
fn xml_roots(package: &Package) -> Vec<&XmlNode> {
    package
        .parts
        .iter()
        .filter_map(|part| match &part.data {
            PartData::Xml(root) => Some(root),
            _ => None,
        })
        .collect()
}

/// Visits `node` and every descendant element.
fn walk(node: &XmlNode, seen: &mut impl FnMut(&XmlNode)) {
    seen(node);
    for child in &node.children {
        if let XmlChild::Element(element) = child {
            walk(element, seen);
        }
    }
}

/// Whether the root element is one the reader decomposes into model entities.
fn is_mapped_root(root: &XmlNode) -> bool {
    matches!(
        root.name.local.as_str(),
        "VisioDocument"
            | "Pages"
            | "PageContents"
            | "Masters"
            | "MasterContents"
            | "coreProperties"
            | "Properties"
    )
}

/// Every `Cell` element of the parts the reader maps.
fn count_source_cells(package: &Package) -> usize {
    let mut count = 0;
    for root in xml_roots(package)
        .into_iter()
        .filter(|root| is_mapped_root(root))
    {
        walk(root, &mut |node| {
            if node.name.local == "Cell" {
                count += 1;
            }
        });
    }
    count
}

/// Every modeled cell plus every cell kept verbatim, in the whole document.
fn count_model_cells(document: &Document) -> usize {
    let mut count = 0;
    for page in &document.pages {
        count += sheet_cells(&page.sheet);
        count += preserved_cells(&page.preserved);
    }
    for master in &document.masters {
        count += preserved_cells(&master.preserved);
    }
    for style in &document.stylesheet.styles {
        count += sheet_cells(&style.sheet);
        count += preserved_cells(&style.preserved);
    }
    for shape in &document.shapes {
        count += sheet_cells(&shape.sheet);
        count += preserved_cells(&shape.preserved);
    }
    count += preserved_cells(&document.preserved);
    count
}

/// The cells of one sheet: direct cells, section-level cells, row cells, and
/// the cells kept verbatim in sections and rows.
fn sheet_cells(sheet: &ferrogram_model::Sheet) -> usize {
    let mut count = sheet.cells.len();
    for section in &sheet.sections {
        count += section.cells.len() + preserved_cells(&section.preserved);
        for row in &section.rows {
            count += row.cells.len() + preserved_cells(&row.preserved);
        }
    }
    count
}

/// The `Cell` elements kept verbatim in one preserved set, at any depth.
fn preserved_cells(preserved: &Preserved) -> usize {
    let mut count = 0;
    for child in &preserved.children {
        walk(child, &mut |node| {
            if node.name.local == "Cell" {
                count += 1;
            }
        });
    }
    count
}
