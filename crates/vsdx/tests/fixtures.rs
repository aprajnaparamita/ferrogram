//! Smoke tests for the hand-authored fixture set (task P2-08).
//!
//! Every fixture variant gets a test that the bytes look right (the truncated
//! one is really shorter, the bomb really bombs, ...), and the committed files
//! under `corpus/fixtures/` must equal the [`common::build`] output byte for
//! byte. Nothing here depends on `read_package`: the reader tasks (P2-05 to
//! P2-07) build on these fixtures later.

mod common;

use std::io::Cursor;

use common::{COMMITTED_FIXTURES, Fixture, Malformed, build, file_name, path};
use ferrogram_vsdx::Limits;
use zip::ZipArchive;

/// Opens the bytes as a zip archive.
///
/// `expect` is fine here: this is test scaffolding and a fixture that will not
/// open is exactly what the tests must report (clippy.toml exempts `#[test]`
/// functions only, hence the allow).
#[allow(clippy::expect_used)]
fn archive(bytes: &[u8]) -> ZipArchive<Cursor<&[u8]>> {
    ZipArchive::new(Cursor::new(bytes)).expect("fixture must open as a zip archive")
}

/// The zip entry names of a fixture.
fn names(bytes: &[u8]) -> Vec<String> {
    archive(bytes).file_names().map(str::to_owned).collect()
}

/// The raw bytes of one zip entry.
fn entry_bytes(bytes: &[u8], name: &str) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut archive = archive(bytes);
    let mut out = Vec::new();
    archive
        .by_name(name)
        .ok()
        .and_then(|mut file| file.read_to_end(&mut out).ok())
        .map(|_| out)
}

#[test]
fn drawing_is_a_valid_zip_with_the_visio_part_set() {
    // Acceptance: "at minimum it is a valid zip with the Visio part set".
    // (The Phase 1 libvisio oracle reads the binary .vsd format only, so the
    // oracle smoke path does not apply to a .vsdx fixture.)
    let bytes = build(Fixture::Drawing);
    let names = names(&bytes);
    for part in [
        "[Content_Types].xml",
        "_rels/.rels",
        "visio/document.xml",
        "visio/_rels/document.xml.rels",
        "visio/pages/pages.xml",
        "visio/pages/_rels/pages.xml.rels",
        "visio/pages/page1.xml",
    ] {
        assert!(names.iter().any(|n| n == part), "missing part {part}");
    }
    let content_types =
        String::from_utf8(entry_bytes(&bytes, "[Content_Types].xml").unwrap()).unwrap();
    assert!(content_types.contains("application/vnd.ms-visio.drawing.main+xml"));
    let document = String::from_utf8(entry_bytes(&bytes, "visio/document.xml").unwrap()).unwrap();
    assert!(document.contains("VisioDocument"));
}

#[test]
fn stencil_has_one_master_and_no_page_contents() {
    let bytes = build(Fixture::Stencil);
    let names = names(&bytes);
    assert!(names.iter().any(|n| n == "visio/masters/masters.xml"));
    assert!(names.iter().any(|n| n == "visio/masters/master1.xml"));
    assert!(!names.iter().any(|n| n == "visio/pages/page1.xml"));
    let content_types =
        String::from_utf8(entry_bytes(&bytes, "[Content_Types].xml").unwrap()).unwrap();
    assert!(content_types.contains("application/vnd.ms-visio.stencil.main+xml"));
    // The pages part is present but empty (ADR 0016).
    let pages = String::from_utf8(entry_bytes(&bytes, "visio/pages/pages.xml").unwrap()).unwrap();
    assert!(!pages.contains("<Page "));
}

#[test]
fn template_has_a_page_and_a_master() {
    let bytes = build(Fixture::Template);
    let names = names(&bytes);
    assert!(names.iter().any(|n| n == "visio/pages/page1.xml"));
    assert!(names.iter().any(|n| n == "visio/masters/master1.xml"));
    let content_types =
        String::from_utf8(entry_bytes(&bytes, "[Content_Types].xml").unwrap()).unwrap();
    assert!(content_types.contains("application/vnd.ms-visio.template.main+xml"));
}

#[test]
fn truncated_zip_is_shorter_and_unreadable() {
    let truncated = build(Fixture::Malformed(Malformed::TruncatedZip));
    let valid = build(Fixture::Drawing);
    assert!(truncated.len() < valid.len());
    assert!(truncated.len() == valid.len() / 2);
    assert!(ZipArchive::new(Cursor::new(truncated.as_slice())).is_err());
}

#[test]
fn bad_central_directory_is_unreadable() {
    let bytes = build(Fixture::Malformed(Malformed::BadCentralDirectory));
    assert!(ZipArchive::new(Cursor::new(bytes.as_slice())).is_err());
}

#[test]
fn no_content_types_drops_the_map() {
    let bytes = build(Fixture::Malformed(Malformed::NoContentTypes));
    let names = names(&bytes); // the zip container itself is fine
    assert!(!names.iter().any(|n| n == "[Content_Types].xml"));
    assert!(names.iter().any(|n| n == "visio/document.xml"));
}

#[test]
fn dangling_relationship_targets_a_missing_part() {
    let bytes = build(Fixture::Malformed(Malformed::DanglingRelationship));
    let rels =
        String::from_utf8(entry_bytes(&bytes, "visio/_rels/document.xml.rels").unwrap()).unwrap();
    assert!(rels.contains("Target=\"pages/missing.xml\""));
    assert!(!names(&bytes).iter().any(|n| n == "visio/pages/missing.xml"));
}

#[test]
fn missing_main_part_drops_document_xml() {
    let bytes = build(Fixture::Malformed(Malformed::MissingMainPart));
    assert!(!names(&bytes).iter().any(|n| n == "visio/document.xml"));
    // The package relationship still points at it: the dangling main part is
    // exactly what the reader must reject.
    let rels = String::from_utf8(entry_bytes(&bytes, "_rels/.rels").unwrap()).unwrap();
    assert!(rels.contains("Target=\"visio/document.xml\""));
}

#[test]
fn huge_declared_size_exceeds_the_cap() {
    let bytes = build(Fixture::Malformed(Malformed::HugeDeclaredSize));
    let mut archive = archive(&bytes);
    let file = archive.by_name("visio/pages/page1.xml").unwrap();
    assert!(file.size() > Limits::DEFAULT.max_part_uncompressed);
}

#[test]
fn deep_nesting_exceeds_the_xml_depth_cap() {
    let bytes = build(Fixture::Malformed(Malformed::DeepNesting));
    let document = String::from_utf8(entry_bytes(&bytes, "visio/document.xml").unwrap()).unwrap();
    let depth = document.matches("<Deep>").count() as u32;
    assert!(depth > Limits::DEFAULT.max_xml_depth);
}

#[test]
fn bad_content_types_is_not_well_formed() {
    let bytes = build(Fixture::Malformed(Malformed::BadContentTypes));
    let content_types =
        String::from_utf8(entry_bytes(&bytes, "[Content_Types].xml").unwrap()).unwrap();
    assert!(content_types.contains("<Types"));
    assert!(!content_types.contains("</Types>"));
}

#[test]
fn zip_bomb_ratio_exceeds_the_cap() {
    let bytes = build(Fixture::Malformed(Malformed::ZipBomb));
    let mut archive = archive(&bytes);
    let bomb = archive.by_name("visio/bomb.bin").unwrap();
    assert!(bomb.size() > 0);
    assert!(bomb.size() / bomb.compressed_size() > Limits::DEFAULT.max_compression_ratio);
}

#[test]
fn many_entries_exceed_the_entry_cap() {
    let bytes = build(Fixture::Malformed(Malformed::ManyEntries));
    let count = archive(&bytes).len() as u32;
    assert!(count > Limits::DEFAULT.max_entries);
}

#[test]
fn every_variant_builds_deterministically() {
    let mut all = vec![Fixture::Drawing, Fixture::Stencil, Fixture::Template];
    all.extend(
        [
            Malformed::TruncatedZip,
            Malformed::BadCentralDirectory,
            Malformed::NoContentTypes,
            Malformed::DanglingRelationship,
            Malformed::MissingMainPart,
            Malformed::HugeDeclaredSize,
            Malformed::DeepNesting,
            Malformed::BadContentTypes,
            Malformed::ZipBomb,
            Malformed::ManyEntries,
        ]
        .map(Fixture::Malformed),
    );
    assert_eq!(all.len(), 13);
    for fixture in all {
        assert_eq!(
            build(fixture),
            build(fixture),
            "not deterministic: {fixture:?}"
        );
    }
}

#[test]
fn file_names_are_distinct() {
    let mut seen = Vec::new();
    let all = [
        Fixture::Drawing,
        Fixture::Stencil,
        Fixture::Template,
        Fixture::Malformed(Malformed::TruncatedZip),
        Fixture::Malformed(Malformed::BadCentralDirectory),
        Fixture::Malformed(Malformed::NoContentTypes),
        Fixture::Malformed(Malformed::DanglingRelationship),
        Fixture::Malformed(Malformed::MissingMainPart),
        Fixture::Malformed(Malformed::HugeDeclaredSize),
        Fixture::Malformed(Malformed::DeepNesting),
        Fixture::Malformed(Malformed::BadContentTypes),
        Fixture::Malformed(Malformed::ZipBomb),
        Fixture::Malformed(Malformed::ManyEntries),
    ];
    for fixture in all {
        let name = file_name(&fixture);
        assert!(!seen.contains(&name), "duplicate file name {name}");
        seen.push(name);
    }
}

#[test]
fn committed_fixtures_match_the_builder_output() {
    for fixture in COMMITTED_FIXTURES {
        let name = file_name(fixture);
        let path = path(name).expect("committed fixture has a path");
        let on_disk = std::fs::read(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        assert_eq!(
            on_disk,
            build(*fixture),
            "{name} differs from the builder output; run the \
             `regenerate_committed_fixtures` test and commit the change"
        );
    }
}

#[test]
fn path_knows_committed_and_generated_names() {
    assert!(path("minimal-drawing.vsdx").is_some());
    assert!(path("malformed-deep-nesting.vsdx").is_some());
    // Generated at test time, never committed.
    assert!(path("malformed-zip-bomb.vsdx").is_none());
    assert!(path("malformed-many-entries.vsdx").is_none());
    assert!(path("nonesuch.vsdx").is_none());
}

#[test]
fn every_committed_fixture_has_a_provenance_row() {
    let provenance = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/PROVENANCE.md"),
    )
    .expect("corpus/PROVENANCE.md");
    for fixture in COMMITTED_FIXTURES {
        let name = file_name(fixture);
        assert!(
            provenance.contains(&format!("`fixtures/{name}`")),
            "corpus/PROVENANCE.md has no row for fixtures/{name}"
        );
    }
}

/// Rewrites the committed fixture files from the builder. Run manually after
/// changing the builder, then commit the result together with the
/// `corpus/PROVENANCE.md` rows (AGENTS.md section 4):
///
/// ```sh
/// cargo test --test fixtures regenerate_committed_fixtures -- --ignored
/// ```
#[test]
#[ignore = "regenerates committed fixture files"]
fn regenerate_committed_fixtures() {
    let dir = common::fixtures_dir();
    std::fs::create_dir_all(&dir).expect("corpus/fixtures");
    for fixture in COMMITTED_FIXTURES {
        let path = path(file_name(fixture)).expect("committed fixture has a path");
        std::fs::write(&path, build(*fixture)).expect("write fixture");
        eprintln!("wrote {}", path.display());
    }
}
