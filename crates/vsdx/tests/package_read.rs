//! Malformed-input and cap tests for `read_package` (task P2-05).
//!
//! Every fixture is built here (ADR 0017 hand-authored fixtures): a bad
//! container yields `Err` and never a panic.

use std::io::Write as _;

use ferrogram_vsdx::{ContentTypeKind, Error, Limits, PartData, read_package};
use zip::CompressionMethod;
use zip::write::{SimpleFileOptions, ZipWriter};

mod common;

const TYPES_XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"xml\" ContentType=\"application/xml\"/><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"bin\" ContentType=\"application/octet-stream\"/></Types>";

const ROOT_RELS_XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://rel/officeDocument\" Target=\"visio/document.xml\"/></Relationships>";

const DOCUMENT_XML: &str = "<VisioDocument xmlns=\"urn:ms-visio\"><Page ID=\"1\"><Cell N=\"PinX\" V=\"1\"/></Page></VisioDocument>";

/// Builds zip bytes from entries, deflate-compressed.
fn build_zip(entries: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut writer = ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, data) in entries {
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        if writer.start_file(*name, options).is_err() {
            return Vec::new();
        }
        if writer.write_all(data).is_err() {
            return Vec::new();
        }
    }
    writer.finish().map(|c| c.into_inner()).unwrap_or_default()
}

/// Deterministic incompressible bytes, so size caps do not trip the
/// compression-ratio cap by accident.
fn noise(len: usize) -> Vec<u8> {
    let mut state: u64 = 0x2545_F491_4F6C_DD1D;
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        out.push((state >> 33) as u8);
    }
    out
}

/// A minimal valid package: content types, root relationships, one XML part.
fn minimal_package() -> Vec<u8> {
    build_zip(&[
        ("[Content_Types].xml", TYPES_XML.as_bytes().to_vec()),
        ("_rels/.rels", ROOT_RELS_XML.as_bytes().to_vec()),
        ("visio/document.xml", DOCUMENT_XML.as_bytes().to_vec()),
    ])
}

#[test]
fn zero_byte_input_is_an_error() {
    assert!(matches!(
        read_package(&[], &Limits::DEFAULT),
        Err(Error::InvalidPackage(_))
    ));
}

#[test]
fn minimal_package_reads_parts_and_relationships() {
    let package = read_package(&minimal_package(), &Limits::DEFAULT).expect("valid package");
    assert_eq!(package.parts.len(), 1);
    assert_eq!(package.content_types.len(), 3);
    assert_eq!(package.relationships.len(), 1);
    assert_eq!(package.relationships[0].target, "visio/document.xml");
    assert!(matches!(package.parts[0].data, PartData::Xml(_)));
}

#[test]
fn truncated_zip_is_an_error() {
    let bytes = minimal_package();
    let cut = &bytes[..bytes.len() / 2];
    assert!(matches!(
        read_package(cut, &Limits::DEFAULT),
        Err(Error::InvalidPackage(_))
    ));
}

#[test]
fn bad_central_directory_is_an_error() {
    let mut bytes = minimal_package();
    let signature: Vec<u8> = b"PK\x01\x02".to_vec();
    let at = bytes
        .windows(4)
        .position(|window| window == signature.as_slice());
    let at = at.expect("central directory signature in a zip file");
    bytes[at] = b'X';
    assert!(matches!(
        read_package(&bytes, &Limits::DEFAULT),
        Err(Error::InvalidPackage(_))
    ));
}

#[test]
fn missing_content_types_part_infers_declarations() {
    // Two corpus packages omit `[Content_Types].xml`; they must still read
    // (the declarations are inferred and recorded, never invented silently).
    let bytes = build_zip(&[
        ("_rels/.rels", ROOT_RELS_XML.as_bytes().to_vec()),
        ("visio/document.xml", DOCUMENT_XML.as_bytes().to_vec()),
        ("image.bin", noise(16)),
    ]);
    let package = read_package(&bytes, &Limits::DEFAULT).expect("inferred package");
    assert_eq!(package.parts.len(), 2);
    assert_eq!(package.parts[0].content_type.as_str(), "application/xml");
    assert_eq!(
        package.parts[1].content_type.as_str(),
        "application/octet-stream"
    );
    assert_eq!(package.content_types.len(), 2);
}

#[test]
fn wrong_content_types_part_is_an_error() {
    let cases: [&[u8]; 3] = [
        b"<Types><bad",
        b"<Nope/>",
        b"<Types><Default Extension=\"xml\"/></Types>", // no ContentType attribute
    ];
    for types in cases {
        let bytes = build_zip(&[("[Content_Types].xml", types.to_vec())]);
        assert!(matches!(
            read_package(&bytes, &Limits::DEFAULT),
            Err(Error::InvalidPackage(_))
        ));
    }
}

#[test]
fn missing_content_type_declaration_is_an_error() {
    let types = "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"xml\" ContentType=\"application/xml\"/></Types>";
    let bytes = build_zip(&[
        ("[Content_Types].xml", types.as_bytes().to_vec()),
        ("image.png", noise(16)),
    ]);
    assert!(matches!(
        read_package(&bytes, &Limits::DEFAULT),
        Err(Error::InvalidPackage(_))
    ));
}

#[test]
fn dangling_relationship_is_recorded_not_an_error() {
    let rels = "<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId9\" Type=\"http://rel/x\" Target=\"visio/missing.xml\"/><Relationship Id=\"rId8\" Type=\"http://rel/x\" Target=\"http://example.com/x\" TargetMode=\"External\"/></Relationships>";
    let bytes = build_zip(&[
        ("[Content_Types].xml", TYPES_XML.as_bytes().to_vec()),
        ("_rels/.rels", rels.as_bytes().to_vec()),
        ("visio/document.xml", DOCUMENT_XML.as_bytes().to_vec()),
    ]);
    let package = read_package(&bytes, &Limits::DEFAULT).expect("dangling targets are legal");
    assert_eq!(package.relationships.len(), 2);
    assert_eq!(package.relationships[0].target, "visio/missing.xml");
    assert_eq!(
        package.relationships[1].target_mode,
        ferrogram_vsdx::TargetMode::External
    );
}

#[test]
fn bad_relationships_part_is_an_error() {
    let cases: [&[u8]; 3] = [
        b"<Relationships><Relationship Id=\"rId1\" Type=\"t\" Target=\"x\" TargetMode=\"Weird\"/></Relationships>",
        b"<Relationships><Relationship Id=\"rId1\" Type=\"t\"/></Relationships>", // no Target
        b"<Relationships><Other/></Relationships>",
    ];
    for rels in cases {
        let bytes = build_zip(&[
            ("[Content_Types].xml", TYPES_XML.as_bytes().to_vec()),
            ("_rels/.rels", rels.to_vec()),
        ]);
        assert!(matches!(
            read_package(&bytes, &Limits::DEFAULT),
            Err(Error::InvalidPackage(_))
        ));
    }
}

#[test]
fn malformed_xml_part_is_an_error() {
    let cases: [&[u8]; 5] = [
        b"<a>",              // unclosed
        b"<a/><b/>",         // two roots
        b"<a>&nope;</a>",    // unresolved entity
        b"<!DOCTYPE a><a/>", // doctype is not supported
        b"text without any element",
    ];
    for document in cases {
        let bytes = build_zip(&[
            ("[Content_Types].xml", TYPES_XML.as_bytes().to_vec()),
            ("visio/document.xml", document.to_vec()),
        ]);
        assert!(matches!(
            read_package(&bytes, &Limits::DEFAULT),
            Err(Error::InvalidPackage(_))
        ));
    }
}

#[test]
fn max_entries_cap_is_enforced() {
    let bytes = build_zip(&[
        ("[Content_Types].xml", TYPES_XML.as_bytes().to_vec()),
        ("visio/document.xml", DOCUMENT_XML.as_bytes().to_vec()),
        ("extra.bin", noise(32)),
    ]);
    let mut limits = Limits::DEFAULT;
    limits.max_entries = 2;
    assert!(matches!(
        read_package(&bytes, &limits),
        Err(Error::LimitExceeded {
            limit: "max_entries",
            ..
        })
    ));
}

#[test]
fn max_part_uncompressed_cap_is_enforced() {
    let bytes = build_zip(&[
        ("[Content_Types].xml", TYPES_XML.as_bytes().to_vec()),
        ("big.bin", noise(500)),
    ]);
    let mut limits = Limits::DEFAULT;
    limits.max_part_uncompressed = 400;
    assert!(matches!(
        read_package(&bytes, &limits),
        Err(Error::LimitExceeded {
            limit: "max_part_uncompressed",
            ..
        })
    ));
}

#[test]
fn max_total_uncompressed_cap_is_enforced() {
    let bytes = build_zip(&[
        ("[Content_Types].xml", TYPES_XML.as_bytes().to_vec()),
        ("one.bin", noise(500)),
        ("two.bin", noise(500)),
    ]);
    let mut limits = Limits::DEFAULT;
    limits.max_part_uncompressed = 1_000;
    limits.max_total_uncompressed = 700;
    assert!(matches!(
        read_package(&bytes, &limits),
        Err(Error::LimitExceeded {
            limit: "max_total_uncompressed",
            ..
        })
    ));
}

#[test]
fn zip_bomb_beyond_max_compression_ratio_is_refused() {
    // The bomb fixture is built here: a highly compressible payload whose
    // declared uncompressed size is beyond the 200:1 cap of `Limits::DEFAULT`.
    let bomb = vec![0u8; 256 * 1024];
    let bytes = build_zip(&[
        ("[Content_Types].xml", TYPES_XML.as_bytes().to_vec()),
        ("bomb.bin", bomb),
    ]);
    let package = read_package(&bytes, &Limits::DEFAULT);
    assert!(matches!(
        package,
        Err(Error::LimitExceeded {
            limit: "max_compression_ratio",
            ..
        })
    ));
}

#[test]
fn max_xml_depth_cap_is_enforced() {
    let mut document = String::new();
    for _ in 0..200 {
        document.push_str("<a>");
    }
    for _ in 0..200 {
        document.push_str("</a>");
    }
    let bytes = build_zip(&[
        ("[Content_Types].xml", TYPES_XML.as_bytes().to_vec()),
        ("visio/document.xml", document.into_bytes()),
    ]);
    assert!(matches!(
        read_package(&bytes, &Limits::DEFAULT),
        Err(Error::LimitExceeded {
            limit: "max_xml_depth",
            ..
        })
    ));
}

#[test]
fn max_attributes_per_element_cap_is_enforced() {
    let mut document = String::from("<a");
    for i in 0..300 {
        document.push_str(&format!(" a{i}=\"v\""));
    }
    document.push_str("/>");
    let bytes = build_zip(&[
        ("[Content_Types].xml", TYPES_XML.as_bytes().to_vec()),
        ("visio/document.xml", document.into_bytes()),
    ]);
    assert!(matches!(
        read_package(&bytes, &Limits::DEFAULT),
        Err(Error::LimitExceeded {
            limit: "max_attributes_per_element",
            ..
        })
    ));
}

#[test]
fn binary_parts_stay_binary() {
    let payload = noise(64);
    let bytes = build_zip(&[
        ("[Content_Types].xml", TYPES_XML.as_bytes().to_vec()),
        ("image.bin", payload.clone()),
    ]);
    let package = read_package(&bytes, &Limits::DEFAULT).expect("valid package");
    assert_eq!(package.parts.len(), 1);
    match &package.parts[0].data {
        PartData::Binary(data) => assert_eq!(data, &payload),
        PartData::Xml(_) => panic!("binary part was parsed as XML"),
        _ => panic!("unknown part data kind"),
    }
}

#[test]
fn cap_errors_never_leak_file_content() {
    let bytes = build_zip(&[
        ("[Content_Types].xml", TYPES_XML.as_bytes().to_vec()),
        ("visio/document.xml", b"<secret><a/></secret>".to_vec()),
    ]);
    let mut limits = Limits::DEFAULT;
    limits.max_xml_depth = 1;
    let message = match read_package(&bytes, &limits) {
        Err(Error::InvalidPackage(why)) => why,
        Err(Error::LimitExceeded { limit, .. }) => limit.to_owned(),
        other => panic!("expected a limit error, got {other:?}"),
    };
    assert!(!message.contains("secret"));
}

#[test]
fn the_streaming_size_cap_refuses_a_lying_declared_size() {
    // Coverage (audit minor 1): the declared size passes the pre-checks, so
    // only the inflate loop's own cap can stop the entry.
    let bytes = build_zip(&[
        ("[Content_Types].xml", TYPES_XML.as_bytes().to_vec()),
        ("big.bin", noise(4096)),
    ]);
    let bytes = common::declare_huge_size(bytes, "big.bin", 8);
    let mut limits = Limits::DEFAULT;
    limits.max_part_uncompressed = 1024;
    // Either the inflate loop's cap stops the entry, or zip (9.x and
    // later) refuses to inflate past the declared size first.
    let result = read_package(&bytes, &limits);
    assert!(
        matches!(
            result,
            Err(Error::LimitExceeded {
                limit: "max_part_uncompressed",
                max: 1024,
                ..
            }) | Err(Error::InvalidPackage(_))
        ),
        "{result:?}"
    );
}

#[test]
fn an_extensionless_part_infers_an_override() {
    // Coverage (audit minor 3): a package without `[Content_Types].xml` and
    // a part with no usable extension gets an `Override` for it.
    let bytes = build_zip(&[
        ("_rels/.rels", ROOT_RELS_XML.as_bytes().to_vec()),
        ("weird", noise(16)),
    ]);
    let package = read_package(&bytes, &Limits::DEFAULT).expect("inferred package");
    assert_eq!(
        package.parts[0].content_type.as_str(),
        "application/octet-stream"
    );
    assert!(
        package
            .content_types
            .iter()
            .any(|decl| { decl.kind == ContentTypeKind::Override && decl.name == "/weird" })
    );
}

#[test]
fn a_relationships_part_with_no_directory_part_maps_to_the_root() {
    // Coverage (audit minor 3): `_rels/page.xml.rels` names `/page.xml` and
    // `_rels/.rels` names the package root.
    let bytes = build_zip(&[
        ("[Content_Types].xml", TYPES_XML.as_bytes().to_vec()),
        ("_rels/.rels", ROOT_RELS_XML.as_bytes().to_vec()),
        ("_rels/page.xml.rels", ROOT_RELS_XML.as_bytes().to_vec()),
    ]);
    let package = read_package(&bytes, &Limits::DEFAULT).expect("reads");
    let sources: Vec<&str> = package
        .relationships
        .iter()
        .map(|relationship| relationship.source.as_str())
        .collect();
    assert_eq!(sources, ["/", "/page.xml"]);
}

#[test]
fn a_relationships_part_with_an_unmappable_name_is_an_error() {
    // Coverage (audit minor 3): a relationships part outside any `_rels`
    // directory names no source and cannot be mapped.
    let types = "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/></Types>";
    let bytes = build_zip(&[
        ("[Content_Types].xml", types.as_bytes().to_vec()),
        ("weird.rels", ROOT_RELS_XML.as_bytes().to_vec()),
    ]);
    assert!(matches!(
        read_package(&bytes, &Limits::DEFAULT),
        Err(Error::InvalidPackage(_))
    ));
}
