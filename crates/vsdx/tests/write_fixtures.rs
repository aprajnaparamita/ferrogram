//! Round-tripping the hand-authored fixtures through the writer (task
//! P2-07, ADR 0016).
//!
//! The drawing, stencil and template fixtures (P2-08) must survive
//! `read -> write -> read -> write` with no semantic difference, the right
//! content types and the right part sets: a stencil carries no page
//! contents, a template carries both pages and masters.

mod common;

use common::{Fixture, build};
use ferrogram_model::{DocumentKind, diff};
use ferrogram_vsdx::{Limits, Package, PartData, PartName, read_package, to_model, write_document};

/// The names of the parts of a package, in order.
fn part_names(package: &Package) -> Vec<&str> {
    package
        .parts
        .iter()
        .map(|part| part.name.as_str())
        .collect()
}

/// The display name of a root, for failure messages.
fn root_name(node: &ferrogram_model::XmlNode) -> String {
    match &node.name.prefix {
        Some(prefix) => format!("{}:{}", prefix, node.name.local),
        None => node.name.local.clone(),
    }
}

/// Reads, writes, re-reads and re-writes `bytes`, asserting semantic
/// equivalence of the first write and byte-identity of the two writes.
///
/// Returns the twice-written package for the caller's part-set assertions.
#[allow(clippy::panic)] // test helper: the assertions report the failing fixture
fn round_trip(name: &str, bytes: &[u8]) -> Package {
    let source = read_package(bytes, &Limits::DEFAULT).unwrap_or_else(|error| {
        panic!("{name}: source package does not read: {error}");
    });
    let document = to_model(&source).unwrap_or_else(|error| {
        panic!("{name}: source does not map: {error}");
    });
    let first = write_document(&document, &source).unwrap_or_else(|error| {
        panic!("{name}: write failed: {error}");
    });
    let reparsed = read_package(&first, &Limits::DEFAULT).unwrap_or_else(|error| {
        panic!("{name}: written package does not read: {error}");
    });
    let remodeled = to_model(&reparsed).unwrap_or_else(|error| {
        panic!("{name}: written package does not map: {error}");
    });
    let second = write_document(&remodeled, &reparsed).unwrap_or_else(|error| {
        panic!("{name}: second write failed: {error}");
    });
    assert_eq!(first, second, "{name}: the second write differs");

    assert_eq!(
        part_names(&source),
        part_names(&reparsed),
        "{name}: part lists differ"
    );
    assert_eq!(
        source.content_types, reparsed.content_types,
        "{name}: content types differ"
    );
    assert_eq!(
        source.relationships, reparsed.relationships,
        "{name}: relationships differ"
    );
    for (before, after) in source.parts.iter().zip(reparsed.parts.iter()) {
        assert_eq!(
            before.content_type, after.content_type,
            "{name}: content types"
        );
        match (&before.data, &after.data) {
            (PartData::Xml(a), PartData::Xml(b)) => {
                let differences: Vec<String> = diff(a, b)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|difference| difference.to_string())
                    .collect();
                assert!(
                    differences.is_empty(),
                    "{name}: part {} ({}) differs:\n{}",
                    before.name.as_str(),
                    root_name(a),
                    differences.join("\n")
                );
            }
            (PartData::Binary(a), PartData::Binary(b)) => {
                assert_eq!(a, b, "{name}: binary part differs");
            }
            _ => panic!("{name}: part kind changed"),
        }
    }
    reparsed
}

#[test]
fn the_drawing_fixture_round_trips() {
    let package = round_trip("drawing", &build(Fixture::Drawing));
    let main = package
        .part(&PartName::new("/visio/document.xml"))
        .expect("main part");
    assert_eq!(
        main.content_type.as_str(),
        "application/vnd.ms-visio.drawing.main+xml"
    );
    assert!(
        package
            .part(&PartName::new("/visio/pages/page1.xml"))
            .is_some()
    );
}

#[test]
fn the_stencil_fixture_round_trips_with_no_page_contents() {
    let package = round_trip("stencil", &build(Fixture::Stencil));
    let main = package
        .part(&PartName::new("/visio/document.xml"))
        .expect("main part");
    assert_eq!(
        main.content_type.as_str(),
        "application/vnd.ms-visio.stencil.main+xml"
    );
    // A stencil keeps its empty page list but no page contents (ADR 0016).
    assert!(
        package
            .part(&PartName::new("/visio/pages/pages.xml"))
            .is_some()
    );
    assert!(
        package
            .part(&PartName::new("/visio/pages/page1.xml"))
            .is_none()
    );
    assert!(
        package
            .part(&PartName::new("/visio/masters/master1.xml"))
            .is_some()
    );
}

#[test]
fn the_template_fixture_round_trips_with_pages_and_masters() {
    let package = round_trip("template", &build(Fixture::Template));
    let main = package
        .part(&PartName::new("/visio/document.xml"))
        .expect("main part");
    assert_eq!(
        main.content_type.as_str(),
        "application/vnd.ms-visio.template.main+xml"
    );
    assert!(
        package
            .part(&PartName::new("/visio/pages/page1.xml"))
            .is_some()
    );
    assert!(
        package
            .part(&PartName::new("/visio/masters/master1.xml"))
            .is_some()
    );
}

#[test]
fn fixture_kinds_survive_the_round_trip() {
    for (fixture, kind) in [
        (Fixture::Drawing, DocumentKind::Drawing),
        (Fixture::Stencil, DocumentKind::Stencil),
        (Fixture::Template, DocumentKind::Template),
    ] {
        let source = read_package(&build(fixture), &Limits::DEFAULT).expect("read");
        let document = to_model(&source).expect("model");
        assert_eq!(document.kind, kind);
    }
}
