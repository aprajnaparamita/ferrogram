//! Round-trip tests for the package layer (task P2-05).
//!
//! Every corpus package must survive `read -> write -> read` unchanged:
//! same parts (byte-equal binaries, semantically equal XML trees, which is
//! `diff`-empty under ADR 0012), same content types, same relationships. The
//! writer is a function of the package, so a second write is byte-identical.
//! Corpus files are fetched, never committed (ADR 0005); the tests skip with
//! a message when `corpus/vsdx/` is absent, and fail instead when
//! `FERROGRAM_REQUIRE_CORPUS=1` is set.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use ferrogram_vsdx::{
    ContentType, ContentTypeDecl, ContentTypeKind, Limits, Package, Part, PartData, PartName,
    Relationship, RelationshipId, TargetMode, read_package, write_package,
};

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
///
/// Fails the calling test (with a message) when `FERROGRAM_REQUIRE_CORPUS=1`
/// is set and the corpus is absent; that check lives in the test functions.
fn corpus_missing(files: &[PathBuf]) -> bool {
    if files.is_empty() {
        eprintln!("skipping: corpus/vsdx is absent; run `cargo xtask corpus`");
    }
    files.is_empty()
}

/// A hand-built package: XML and binary parts, root and part relationships.
fn hand_built_package() -> Option<Package> {
    let mut package = Package::new();
    package.content_types.push(ContentTypeDecl {
        kind: ContentTypeKind::Default,
        name: "xml".to_owned(),
        content_type: "application/xml".to_owned(),
    });
    package.content_types.push(ContentTypeDecl {
        kind: ContentTypeKind::Default,
        name: "png".to_owned(),
        content_type: "image/png".to_owned(),
    });
    package.content_types.push(ContentTypeDecl {
        kind: ContentTypeKind::Override,
        name: "/visio/document.xml".to_owned(),
        content_type: "application/vnd.ms-visio.drawing.main+xml".to_owned(),
    });

    let tree = build_tree(
        "<v:VisioDocument xmlns:v=\"urn:ms-visio\" b=\"2\" a=\"1\">\n\
         <Page ID=\"1\"><Cell N=\"PinX\" V=\"0.5\"/></Page>\n\
         <!--kept--><?keep data?>\n\
         </v:VisioDocument>",
    )?;
    package.parts.push(Part {
        name: PartName::new("/visio/document.xml"),
        content_type: ContentType::new("application/vnd.ms-visio.drawing.main+xml"),
        data: PartData::Xml(tree),
    });
    package.parts.push(Part {
        name: PartName::new("/docProps/thumbnail.png"),
        content_type: ContentType::new("image/png"),
        data: PartData::Binary(vec![0x89, 0x50, 0x4E, 0x47, 0, 1, 2, 255]),
    });

    package.relationships.push(Relationship {
        source: PartName::new("/"),
        id: RelationshipId::new("rId1"),
        rel_type: "http://rel/officeDocument".to_owned(),
        target: "visio/document.xml".to_owned(),
        target_mode: TargetMode::Internal,
        preserved: Default::default(),
    });
    package.relationships.push(Relationship {
        source: PartName::new("/visio/document.xml"),
        id: RelationshipId::new("rId1"),
        rel_type: "http://rel/pages".to_owned(),
        target: "pages/pages.xml".to_owned(),
        target_mode: TargetMode::Internal,
        preserved: Default::default(),
    });
    Some(package)
}

/// Builds a preserved tree with the same parser `read_package` uses.
fn build_tree(text: &str) -> Option<ferrogram_model::XmlNode> {
    let types = "<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"xml\" ContentType=\"application/xml\"/></Types>";
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, data) in [
        ("[Content_Types].xml", types.as_bytes().to_vec()),
        ("part.xml", text.as_bytes().to_vec()),
    ] {
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        if writer.start_file(name, options).is_err() || writer.write_all(&data).is_err() {
            return None;
        }
    }
    let bytes = writer.finish().ok()?.into_inner();
    let package = read_package(&bytes, &Limits::DEFAULT).ok()?;
    match package.parts.into_iter().next().map(|part| part.data) {
        Some(PartData::Xml(tree)) => Some(tree),
        _ => None,
    }
}

/// Asserts the two packages agree on parts, content types and relationships.
fn assert_packages_equal(expected: &Package, actual: &Package) -> Result<(), String> {
    if expected.content_types != actual.content_types {
        return Err("content types differ".to_owned());
    }
    if expected.relationships != actual.relationships {
        return Err("relationships differ".to_owned());
    }
    if expected.parts.len() != actual.parts.len() {
        return Err("part count differs".to_owned());
    }
    for (left, right) in expected.parts.iter().zip(actual.parts.iter()) {
        if left.name != right.name {
            return Err("part names differ".to_owned());
        }
        if left.content_type != right.content_type {
            return Err(format!("content type of `{}` differs", left.name.as_str()));
        }
        match (&left.data, &right.data) {
            (PartData::Binary(a), PartData::Binary(b)) => {
                if a != b {
                    return Err(format!(
                        "binary part `{}` is not byte-equal",
                        left.name.as_str()
                    ));
                }
            }
            // Equal trees are semantically equivalent (empty `diff`), which is
            // the ADR 0012 criterion.
            (PartData::Xml(a), PartData::Xml(b)) => {
                if a != b {
                    return Err(format!("XML part `{}` differs", left.name.as_str()));
                }
            }
            _ => return Err(format!("part `{}` changed kind", left.name.as_str())),
        }
    }
    Ok(())
}

/// One package through `read -> write -> read -> write`.
fn round_trip(bytes: &[u8]) -> Result<(), String> {
    let first_read =
        read_package(bytes, &Limits::DEFAULT).map_err(|e| format!("first read: {e}"))?;
    let first_write = write_package(&first_read).map_err(|e| format!("first write: {e}"))?;
    let second_read =
        read_package(&first_write, &Limits::DEFAULT).map_err(|e| format!("second read: {e}"))?;
    assert_packages_equal(&first_read, &second_read)?;
    let second_write = write_package(&second_read).map_err(|e| format!("second write: {e}"))?;
    if first_write != second_write {
        return Err("writer is not stable".to_owned());
    }
    Ok(())
}

#[test]
fn hand_built_package_round_trips() {
    let package = hand_built_package().expect("fixture builds");
    let bytes = write_package(&package).expect("write");
    let reread = read_package(&bytes, &Limits::DEFAULT).expect("read");
    assert_packages_equal(&package, &reread).expect("packages equal");
}

#[test]
fn hand_built_writer_is_stable() {
    let package = hand_built_package().expect("fixture builds");
    let first = write_package(&package).expect("write");
    let second =
        write_package(&read_package(&first, &Limits::DEFAULT).expect("read")).expect("write");
    assert_eq!(first, second);
}

#[test]
fn corpus_packages_round_trip() {
    let files = corpus_files();
    if corpus_missing(&files) {
        assert!(
            std::env::var_os("FERROGRAM_REQUIRE_CORPUS").is_none(),
            "corpus/vsdx is missing and FERROGRAM_REQUIRE_CORPUS=1 is set"
        );
        return;
    }
    for path in files {
        let bytes = std::fs::read(&path).expect("corpus file reads");
        round_trip(&bytes).unwrap_or_else(|why| panic!("{}: {why}", path.display()));
    }
}

#[test]
fn corpus_packages_keep_every_xml_and_binary_part() {
    let files = corpus_files();
    if corpus_missing(&files) {
        assert!(
            std::env::var_os("FERROGRAM_REQUIRE_CORPUS").is_none(),
            "corpus/vsdx is missing and FERROGRAM_REQUIRE_CORPUS=1 is set"
        );
        return;
    }
    for path in files {
        let bytes = std::fs::read(&path).expect("corpus file reads");
        let source = read_package(&bytes, &Limits::DEFAULT).expect("source reads");
        assert!(!source.parts.is_empty(), "package has no parts");
        let written = write_package(&source).expect("write");
        let reread = read_package(&written, &Limits::DEFAULT).expect("reread");
        assert_packages_equal(&source, &reread).unwrap_or_else(|why| panic!("{}", why));
        // The `[Content_Types].xml` and relationships parts are regenerated
        // from the lists, never stored as parts.
        for part in &reread.parts {
            assert_ne!(part.name.as_str(), "/[Content_Types].xml");
            assert!(!part.name.as_str().ends_with(".rels"));
        }
    }
}
