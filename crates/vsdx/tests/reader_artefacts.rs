//! The two reader artefacts P5-10 documented are gone (task P6-16).
//!
//! A cell keeps its `U` spelling in `Preserved` only when the writer
//! cannot spell the code back for it: a code the writer re-derives from
//! the value kind (`BOOL`, `STR`, `COLOR`, `LIST`) or the cell's unit
//! (the `write::value::unit_code` codes) is consumed on read, and every
//! code the writer never produces (`DATE`, `GUID`, `PNT`, `DT`, `DL`,
//! ... and any unknown code) is kept and written back unchanged. The
//! per-cell cases keep the code too: `U="STR"` on a plain string and
//! `U="COLOR"` on a colour without a unit are spellings the writer would
//! strip, so the code is their only record.
//!
//! A read `Page` or `Master` keeps no `<Rel>` child that names its
//! contents part (the writer writes the element again); any other
//! unmapped child stays preserved as before. The file writes back
//! unchanged: the same `Rel` ids in the same places and no XML
//! difference on any part (the corpus identity of `tests/roundtrip.rs`
//! covers the 15 real files).

// The never-crash lints apply to non-test code only (clippy.toml);
// these helpers are test code and use `expect`/`panic` freely.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod common;

use larrygram_model::{Document, Preserved, QName, XmlAttribute, XmlChild, XmlNode, diff};
use larrygram_vsdx::{
    ContentType, Limits, Package, Part, PartData, PartName, Relationship, RelationshipId,
    TargetMode, read_package, to_model, write_document,
};

const CT_XML: &str = "application/xml";

fn qname(name: &str) -> QName {
    match name.split_once(':') {
        Some((prefix, local)) => QName {
            prefix: Some(prefix.to_owned()),
            uri: None,
            local: local.to_owned(),
        },
        None => QName::local(name),
    }
}

fn element(name: &str, attrs: &[(&str, &str)], children: &[XmlNode]) -> XmlNode {
    XmlNode {
        name: qname(name),
        attributes: attrs
            .iter()
            .map(|(name, value)| XmlAttribute {
                name: qname(name),
                value: (*value).to_owned(),
            })
            .collect(),
        children: children.iter().cloned().map(XmlChild::Element).collect(),
        self_closing: false,
    }
}

fn cell(name: &str, code: &str, value: &str) -> XmlNode {
    element("Cell", &[("N", name), ("U", code), ("V", value)], &[])
}

fn package(parts: &[(&str, XmlNode)], relationships: &[(&str, &str, &str)]) -> Package {
    let mut package = Package::new();
    for (name, root) in parts {
        package.parts.push(Part {
            name: PartName::new(*name),
            content_type: ContentType::new(CT_XML),
            data: PartData::Xml(root.clone()),
        });
    }
    for (source, id, target) in relationships {
        let kind = match target.rsplit('/').next().unwrap_or(target) {
            "document.xml" => "document",
            "pages.xml" => "pages",
            "masters.xml" => "masters",
            "page1.xml" => "page",
            "master1.xml" => "master",
            _ => "other",
        };
        package.relationships.push(Relationship {
            source: PartName::new(*source),
            id: RelationshipId::new(*id),
            rel_type: format!("http://schemas.microsoft.com/visio/2010/relationships/{kind}"),
            target: (*target).to_owned(),
            target_mode: TargetMode::Internal,
            preserved: Preserved::new(),
        });
    }
    package
}

/// A drawing whose page shape carries one cell per probe, plus a master
/// with its own contents part (so both lists map).
fn probe_package(codes: &[(&str, &str, &str)]) -> Package {
    let cells: Vec<XmlNode> = codes
        .iter()
        .map(|(name, code, value)| cell(name, code, value))
        .collect();
    let shape = element("Shape", &[("ID", "1"), ("Type", "Shape")], &cells);
    let page_contents = element("PageContents", &[], &[element("Shapes", &[], &[shape])]);
    let pages = element(
        "Pages",
        &[],
        &[element(
            "Page",
            &[("ID", "0")],
            &[
                element("PageSheet", &[], &[]),
                element("Rel", &[("r:id", "rId1")], &[]),
            ],
        )],
    );
    let master_contents = element(
        "MasterContents",
        &[],
        &[element(
            "Shapes",
            &[],
            &[element("Shape", &[("ID", "1")], &[])],
        )],
    );
    let masters = element(
        "Masters",
        &[],
        &[element(
            "Master",
            &[("ID", "2")],
            &[
                element("Rel", &[("r:id", "rId1")], &[]),
                element("Icon", &[], &[]),
            ],
        )],
    );
    package(
        &[
            (
                "/visio/document.xml",
                element(
                    "VisioDocument",
                    &[],
                    &[
                        element("DocumentSettings", &[], &[]),
                        element("StyleSheets", &[], &[]),
                    ],
                ),
            ),
            ("/visio/pages/pages.xml", pages),
            ("/visio/pages/page1.xml", page_contents),
            ("/visio/masters/masters.xml", masters),
            ("/visio/masters/master1.xml", master_contents),
        ],
        &[
            ("/", "rId1", "visio/document.xml"),
            ("/visio/document.xml", "rId1", "pages/pages.xml"),
            ("/visio/document.xml", "rId2", "masters/masters.xml"),
            ("/visio/pages/pages.xml", "rId1", "page1.xml"),
            ("/visio/masters/masters.xml", "rId1", "master1.xml"),
        ],
    )
}

/// The model of `source` and the bytes one write of it produces.
fn read_and_write(source: &Package) -> (Document, Vec<u8>) {
    let document = to_model(source).expect("the source maps");
    let bytes = write_document(&document, source).expect("write_document succeeds");
    (document, bytes)
}

/// Every modeled cell of the document (the master's shape has none), by
/// name.
fn probe_cells(document: &Document) -> Vec<(String, Preserved)> {
    let mut cells = Vec::new();
    for shape in &document.shapes {
        for cell in &shape.sheet.cells {
            cells.push((cell.name.as_str().to_owned(), cell.preserved.clone()));
        }
    }
    cells
}

/// The local names of a preserved fragment's children.
fn names(preserved: &Preserved) -> Vec<&str> {
    preserved
        .children
        .iter()
        .map(|child| child.name.local.as_str())
        .collect()
}

/// The preserved `U` spelling of one cell's fragment, if any.
fn kept_code(preserved: &Preserved) -> Option<String> {
    preserved
        .attributes
        .iter()
        .find(|attribute| attribute.name.local == "U")
        .map(|attribute| attribute.value.clone())
}

/// The XML root of `name` in `package`.
fn root<'p>(package: &'p Package, name: &str) -> &'p XmlNode {
    let Some(root) = package
        .parts
        .iter()
        .find(|part| part.name.as_str() == name)
        .and_then(|part| match &part.data {
            PartData::Xml(root) => Some(root),
            _ => None,
        })
    else {
        panic!("{name} is an XML part");
    };
    root
}

/// Asserts the parts `names` are semantically identical in `source` and
/// `written` (`diff` empty, ADR 0012).
fn assert_unchanged(source: &Package, written: &Package, names: &[&str]) {
    for name in names {
        let differences: Vec<String> = diff(root(source, name), root(written, name))
            .expect("diff runs")
            .into_iter()
            .map(|difference| difference.to_string())
            .collect();
        assert!(
            differences.is_empty(),
            "{name} differs:\n{}",
            differences.join("\n")
        );
    }
}

#[test]
fn a_re_derived_code_reads_with_an_empty_preserved() {
    // One cell per code the writer re-derives: the three kind codes it
    // can spell for a read value (`BOOL`, `STR` on a number-like string,
    // `LIST`) and a unit code of each family.
    let source = probe_package(&[
        ("Flip", "BOOL", "1"),
        ("NumberLike", "STR", "2"),
        ("Layers", "LIST", "1;0"),
        ("Length", "IN", "1"),
        ("Angle", "DEG", "1.5707963267949"),
        ("Ratio", "PER", "0.5"),
    ]);
    let (document, _) = read_and_write(&source);
    for (name, preserved) in probe_cells(&document) {
        assert!(
            preserved.attributes.is_empty(),
            "{name}: the re-derived code is not preserved ({:?})",
            preserved.attributes
        );
    }
}

#[test]
fn a_kept_code_stays_and_writes_back_unchanged() {
    // Every code the writer never spells: a string code, two numeric
    // codes with no `Unit` variant, and an unknown one.
    let source = probe_package(&[
        ("Created", "DATE", "2015-07-06T11:05:49"),
        ("Key", "GUID", "{EFB4EACD-BEFB-4ECD-A226-1282E88A7BE7}"),
        ("Point", "PNT", "2;3"),
        ("Duration", "DT", "0.01041666666666667"),
        ("Length", "DL", "1.082677165354331"),
        ("Odd", "XYZ", "word"),
    ]);
    let (document, bytes) = read_and_write(&source);
    for (name, preserved) in probe_cells(&document) {
        assert_eq!(
            kept_code(&preserved).as_deref(),
            Some(match name.as_str() {
                "Created" => "DATE",
                "Key" => "GUID",
                "Point" => "PNT",
                "Duration" => "DT",
                "Length" => "DL",
                "Odd" => "XYZ",
                other => panic!("unexpected cell {other}"),
            }),
            "{name}: the kept code is preserved"
        );
    }
    // The codes are written back unchanged.
    let written = read_package(&bytes, &Limits::DEFAULT).expect("the written package reads");
    assert_unchanged(&source, &written, &["/visio/pages/page1.xml"]);
}

#[test]
fn a_kind_code_the_writer_would_strip_stays() {
    // The two per-cell cases: `STR` on a plain string and `COLOR` on a
    // colour without a unit read back the same value without the code,
    // so the writer drops such a code unless the reader keeps it (the
    // corpus spells `U='STR' V='Bottom'` and `U='COLOR' V='#ffffff'`).
    let source = probe_package(&[("Name", "STR", "Bottom"), ("Fill", "COLOR", "#ffffff")]);
    let (document, bytes) = read_and_write(&source);
    let kept: Vec<(String, Option<String>)> = probe_cells(&document)
        .into_iter()
        .map(|(name, preserved)| (name, kept_code(&preserved)))
        .collect();
    assert_eq!(
        kept,
        vec![
            ("Name".to_owned(), Some("STR".to_owned())),
            ("Fill".to_owned(), Some("COLOR".to_owned())),
        ]
    );
    let written = read_package(&bytes, &Limits::DEFAULT).expect("the written package reads");
    assert_unchanged(&source, &written, &["/visio/pages/page1.xml"]);
}

#[test]
fn a_read_page_and_master_keep_no_rel() {
    let source = probe_package(&[]);
    let (document, _) = read_and_write(&source);
    // The `Rel` that names the mapped contents part is gone; any other
    // unmapped child (`Icon`) is kept as before.
    assert!(names(&document.pages[0].preserved).is_empty());
    assert_eq!(names(&document.masters[0].preserved), ["Icon"]);
}

#[test]
fn the_file_writes_back_with_the_same_rel_ids() {
    let source = probe_package(&[]);
    let (_document, bytes) = read_and_write(&source);
    // Writer stability: a second write is byte-identical.
    let written = read_package(&bytes, &Limits::DEFAULT).expect("the written package reads");
    let again = write_document(&to_model(&written).expect("model"), &written).expect("rewrite");
    assert_eq!(bytes, again, "the second write differs from the first");
    // The `Rel` elements survive with their ids and no part changes.
    assert_unchanged(
        &source,
        &written,
        &[
            "/visio/pages/pages.xml",
            "/visio/masters/masters.xml",
            "/visio/pages/page1.xml",
            "/visio/masters/master1.xml",
            "/visio/document.xml",
        ],
    );
}
