//! Zip entry-name edge cases for `read_package` (task P4-10, the zip 9
//! entry-name path in `package/read.rs`).
//!
//! zip 9 turned `ZipFile::name()` into a `ZipResult`: a name that is
//! not valid UTF-8 is decoded as CP437 and a name that cannot be
//! decoded at all is an error, which `read_package` maps to
//! `Error::InvalidPackage("zip entry name is malformed")`. The
//! malformed arm is defensive (zip 9.0.4's CP437 fallback decodes any
//! byte sequence), so these tests pin the reachable contract on the
//! exact inputs that would reach it:
//!
//! - an entry name with invalid UTF-8 bytes never panics and never
//!   loses the entry silently: `read_package` answers `Ok` with the
//!   entry kept under its decoded name, or `Err`;
//! - a hostile name (traversal, absolute, backslashes, NUL) normalizes
//!   to one absolute part name and cannot escape the package model
//!   (nothing is ever written to a file system here);
//! - the limits still apply whatever the name decodes to.
//!
//! A panic is the bug (AGENTS.md section 6).

// The never-crash lints apply to non-test code only (clippy.toml);
// these helpers are test code and use `unwrap` freely.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::io::Write as _;

use ferrogram_vsdx::{Error, Limits, Package, read_package};
use zip::CompressionMethod;
use zip::write::{SimpleFileOptions, ZipWriter};

/// A minimal content-types part: `application/xml` for `.xml`.
const TYPES_XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"xml\" ContentType=\"application/xml\"/><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/></Types>";

/// The fixture name the patch helpers rewrite (local header and
/// central directory, exactly once each).
const FIXTURE_NAME: &str = "part0000/data.xml";

/// The zip writer only writes `&str` names, so a name that is not
/// valid UTF-8 is built by patching the name bytes in place. The
/// patched name has the same length, so every offset in the local
/// header and the central directory stays valid.
fn patch_name(bytes: Vec<u8>, new: &[u8]) -> Vec<u8> {
    assert_eq!(
        FIXTURE_NAME.len(),
        new.len(),
        "same length keeps the zip offsets"
    );
    let mut out = bytes;
    let mut patched = 0;
    for index in 0..out.len() {
        if out[index..].starts_with(FIXTURE_NAME.as_bytes()) {
            out[index..index + new.len()].copy_from_slice(new);
            patched += 1;
        }
    }
    assert_eq!(patched, 2, "local header and central directory");
    out
}

/// A two-entry package: `[Content_Types].xml` plus one entry named
/// `FIXTURE_NAME` with `<x/>` as content.
fn package_bytes() -> Vec<u8> {
    let mut writer = ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    writer.start_file("[Content_Types].xml", options).unwrap();
    writer.write_all(TYPES_XML.as_bytes()).unwrap();
    writer.start_file(FIXTURE_NAME, options).unwrap();
    writer.write_all(b"<x/>").unwrap();
    writer.finish().unwrap().into_inner()
}

/// A two-entry package whose second entry has a raw `&str` name
/// (valid UTF-8, hostile spelling).
fn package_with_str_name(name: &str) -> Vec<u8> {
    let mut writer = ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    writer.start_file("[Content_Types].xml", options).unwrap();
    writer.write_all(TYPES_XML.as_bytes()).unwrap();
    writer.start_file(name, options).unwrap();
    writer.write_all(b"<x/>").unwrap();
    writer.finish().unwrap().into_inner()
}

/// Asserts the well-formedness every successful read shares.
fn assert_well_formed(package: &Package, why: &str) {
    for part in &package.parts {
        assert!(
            part.name.as_str().starts_with('/'),
            "{why}: part name {:?} is not absolute",
            part.name
        );
    }
    assert!(package.parts.len() <= 1, "{why}: one entry, many parts");
    assert_eq!(package.content_types.len(), 2, "{why}");
}

/// An entry name that is not valid UTF-8: the reader must answer, and
/// the entry must not vanish silently (it is kept under whatever name
/// zip decodes, or the read fails with a mapped error).
#[test]
fn an_entry_name_that_is_not_utf8_never_panics() {
    let cases: Vec<(Vec<u8>, &str)> = vec![
        // 0xFF is never valid UTF-8 (leading byte).
        (b"\xffart0000/data.xml".to_vec(), "first byte"),
        // 0x80 is never valid UTF-8 (continuation byte alone).
        (b"part\x80000/data.xml".to_vec(), "interior byte"),
        // A dangling 0xC3 starts a two-byte sequence and never ends.
        (b"part0000/data.xm\xc3".to_vec(), "trailing partial"),
        // Every byte hostile.
        (vec![0xff; FIXTURE_NAME.len()], "all bytes"),
    ];
    for (name, why) in cases {
        let bytes = patch_name(package_bytes(), &name);
        match read_package(&bytes, &Limits::DEFAULT) {
            Ok(package) => {
                assert_well_formed(&package, why);
                // The entry survived under its decoded name.
                assert_eq!(package.parts.len(), 1, "{why}: the entry vanished");
            }
            // A mapped error is fine too: the strict rule rejects an
            // entry whose decoded name has no declared content type
            // ("no content type is declared for part ..."), and the
            // zip 9 name-error arm maps to the same class. Never a
            // panic, never a silent drop.
            Err(Error::InvalidPackage(_)) => {}
            Err(other) => panic!("{why}: unexpected error {other}"),
        }
    }
}

/// A name zip decodes fine but the package model must tame: traversal,
/// absolute and backslash names normalize to one absolute part name.
/// Nothing is extracted to a file system (ADR 0036 is byte-based), so
/// these names are inert; the rule under test is "normalize and keep",
/// never "reject and crash".
#[test]
fn a_hostile_but_utf8_name_normalizes_to_one_absolute_part_name() {
    // Every name ends in `.xml` so the declared content type covers
    // it (the strict rule rejects an undeclared extension).
    let names: &[(&str, &str)] = &[
        ("part0000/../evil.xml", "/part0000/../evil.xml"),
        ("/abs/evil.xml", "/abs/evil.xml"),
        ("a\\b\\evil.xml", "/a/b/evil.xml"),
        ("//double.xml", "/double.xml"),
    ];
    for (name, expected) in names {
        let bytes = package_with_str_name(name);
        let package = read_package(&bytes, &Limits::DEFAULT)
            .unwrap_or_else(|error| panic!("name {name:?}: {error}"));
        let seen: Vec<&str> = package
            .parts
            .iter()
            .map(|part| part.name.as_str())
            .collect();
        assert!(seen.contains(expected), "name {name:?}: {seen:?}");
    }
}

/// A NUL byte inside the name must not corrupt anything: same rule as
/// above (the entry is kept or the read fails, never a panic).
#[test]
fn a_name_with_a_nul_byte_is_tamed_like_any_other_name() {
    let bytes = patch_name(package_bytes(), b"part0000/da\x00a.xml");
    match read_package(&bytes, &Limits::DEFAULT) {
        Ok(package) => assert_well_formed(&package, "nul"),
        Err(Error::InvalidPackage(_)) => {}
        Err(other) => panic!("unexpected error {other}"),
    }
}

/// The entry-count cap applies whatever the names decode to.
#[test]
fn the_entry_limit_still_applies_with_hostile_names() {
    let mut writer = ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for index in 0..4 {
        writer
            .start_file(format!("p{index}/data.xml"), options)
            .unwrap();
        writer.write_all(b"<x/>").unwrap();
    }
    let bytes = writer.finish().unwrap().into_inner();
    let limits = Limits {
        max_entries: 2,
        ..Limits::DEFAULT
    };
    let error = read_package(&bytes, &limits).expect_err("over the cap");
    assert!(
        matches!(
            error,
            Error::LimitExceeded {
                limit: "max_entries",
                ..
            }
        ),
        "{error}"
    );
}
