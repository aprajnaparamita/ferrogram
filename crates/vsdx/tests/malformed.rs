//! Malformed-input tests for `read_document` (task P2-09, ADR 0017 section 3).
//!
//! Every hand-authored malformed fixture goes through the full reader and must
//! either read (two recorded exceptions) or fail with an error: never a panic
//! (AGENTS.md section 6). One test per [`Limits`] cap proves each cap is
//! enforced by `read_document` before the input it guards is used.
//!
//! The expected outcome of each fixture is recorded here (the corpus note of
//! the task): `read_package` accepts a package without `[Content_Types].xml`
//! and infers the declarations, and a dangling relationship is recorded, not
//! rejected.

mod common;

use common::{Fixture, Malformed, build};
use larrygram_vsdx::{Error, Limits, read_document};

/// The recorded outcome of one malformed fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Expect {
    /// The reader accepts the package.
    Reads,
    /// The reader rejects it with an error.
    Rejected,
}

/// Every fixture variant with its recorded outcome (task note from the
/// master: `NoContentTypes` and `DanglingRelationship` read).
const OUTCOMES: &[(Malformed, Expect)] = &[
    (Malformed::TruncatedZip, Expect::Rejected),
    (Malformed::BadCentralDirectory, Expect::Rejected),
    (Malformed::NoContentTypes, Expect::Reads),
    (Malformed::DanglingRelationship, Expect::Reads),
    (Malformed::MissingMainPart, Expect::Rejected),
    (Malformed::HugeDeclaredSize, Expect::Rejected),
    (Malformed::DeepNesting, Expect::Rejected),
    (Malformed::BadContentTypes, Expect::Rejected),
    (Malformed::ZipBomb, Expect::Rejected),
    (Malformed::ManyEntries, Expect::Rejected),
];

/// Reads one fixture and returns the outcome as a printable summary. The
/// call itself is the never-panic assertion: a panic fails the test.
fn read(fixture: Fixture) -> String {
    let bytes = build(fixture);
    match read_document(&bytes, &Limits::DEFAULT) {
        Ok(document) => format!(
            "reads: {} pages, {} masters",
            document.pages.len(),
            document.masters.len()
        ),
        Err(error) => {
            // The message must exist and never carry a panic.
            let message = error.to_string();
            assert!(!message.is_empty(), "empty error message");
            format!("rejected: {message}")
        }
    }
}

/// The `limit` name of an error, when it is a [`Error::LimitExceeded`].
fn limit_name(error: &Error) -> Option<&'static str> {
    match error {
        Error::LimitExceeded { limit, .. } => Some(limit),
        _ => None,
    }
}

/// Runs `read_document` with `limits` and returns the error.
#[allow(clippy::panic)] // clippy.toml exempts `#[test]` functions only
fn read_with(bytes: &[u8], limits: &Limits) -> Error {
    match read_document(bytes, limits) {
        Ok(_) => panic!("expected an error"),
        Err(error) => error,
    }
}

/// Every `Malformed` fixture matches its recorded outcome, and none panics.
#[test]
fn every_malformed_fixture_matches_its_recorded_outcome() {
    for (malformed, expect) in OUTCOMES {
        let fixture = Fixture::Malformed(*malformed);
        let outcome = read(fixture);
        match expect {
            Expect::Reads => assert!(
                outcome.starts_with("reads"),
                "{malformed:?}: expected Ok, got {outcome}"
            ),
            Expect::Rejected => assert!(
                outcome.starts_with("rejected"),
                "{malformed:?}: expected Err, got {outcome}"
            ),
        }
    }
}

/// `Limits::max_entries`: an archive with more entries than the cap.
#[test]
fn max_entries_cap_rejects_an_archive_with_too_many_entries() {
    let bytes = build(Fixture::Malformed(Malformed::ManyEntries));
    let error = read_with(&bytes, &Limits::DEFAULT);
    assert_eq!(limit_name(&error), Some("max_entries"), "{error}");
}

/// `Limits::max_part_uncompressed`: one entry declares more bytes than the
/// per-part cap.
#[test]
fn max_part_uncompressed_cap_rejects_a_huge_declared_size() {
    let bytes = build(Fixture::Malformed(Malformed::HugeDeclaredSize));
    let error = read_with(&bytes, &Limits::DEFAULT);
    assert_eq!(limit_name(&error), Some("max_part_uncompressed"), "{error}");
}

/// `Limits::max_compression_ratio`: one entry inflates far beyond its stored
/// size (zip bomb).
#[test]
fn max_compression_ratio_cap_rejects_a_zip_bomb() {
    let bytes = build(Fixture::Malformed(Malformed::ZipBomb));
    let error = read_with(&bytes, &Limits::DEFAULT);
    assert_eq!(limit_name(&error), Some("max_compression_ratio"), "{error}");
}

/// `Limits::max_xml_depth`: one part nests elements past the depth cap.
#[test]
fn max_xml_depth_cap_rejects_deep_nesting() {
    let bytes = build(Fixture::Malformed(Malformed::DeepNesting));
    let error = read_with(&bytes, &Limits::DEFAULT);
    assert_eq!(limit_name(&error), Some("max_xml_depth"), "{error}");
}

/// `Limits::max_total_uncompressed`: the package as a whole inflates past the
/// total cap. The well-formed drawing read under a tight cap exercises the
/// same check as a huge file would, without the megabytes.
#[test]
fn max_total_uncompressed_cap_rejects_a_package_past_the_total() {
    let bytes = build(Fixture::Drawing);
    let limits = Limits {
        max_total_uncompressed: 64,
        ..Limits::DEFAULT
    };
    let error = read_with(&bytes, &limits);
    assert_eq!(
        limit_name(&error),
        Some("max_total_uncompressed"),
        "{error}"
    );
}

/// `Limits::max_attributes_per_element`: one element carries more attributes
/// than the cap. The well-formed drawing read under a tight cap exercises the
/// same check as a crowded element would.
#[test]
fn max_attributes_per_element_cap_rejects_a_crowded_element() {
    let bytes = build(Fixture::Drawing);
    let limits = Limits {
        max_attributes_per_element: 1,
        ..Limits::DEFAULT
    };
    let error = read_with(&bytes, &limits);
    assert_eq!(
        limit_name(&error),
        Some("max_attributes_per_element"),
        "{error}"
    );
}

/// Every `Limits` field is covered by a test above. The destructuring is the
/// tripwire: a new cap breaks this test until it has its own test here.
#[test]
fn every_limits_cap_has_a_test() {
    let Limits {
        max_entries,
        max_part_uncompressed,
        max_total_uncompressed,
        max_compression_ratio,
        max_xml_depth,
        max_attributes_per_element,
    } = Limits::DEFAULT;
    assert_eq!(max_entries, 8192);
    assert_eq!(max_part_uncompressed, 64 * 1024 * 1024);
    assert_eq!(max_total_uncompressed, 512 * 1024 * 1024);
    assert_eq!(max_compression_ratio, 200);
    assert_eq!(max_xml_depth, 128);
    assert_eq!(max_attributes_per_element, 256);
}
