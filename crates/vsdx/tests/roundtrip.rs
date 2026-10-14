//! The Phase 2 exit criterion: every corpus `.vsdx` file round-trips through
//! the writer without loss (task P2-07, ADR 0012).
//!
//! For each corpus file: read, write, re-read and re-write. The first write
//! must be semantically equivalent to the source (`diff` empty on every XML
//! part, same part list, content types and relationships, byte-equal binary
//! parts) and the second write must be byte-identical to the first (writer
//! stability). The corpus is fetched, never committed (ADR 0005): the tests
//! skip with a message when `corpus/vsdx/` is absent, and fail instead when
//! `FERROGRAM_REQUIRE_CORPUS=1` is set.

use std::path::{Path, PathBuf};

use ferrogram_model::{XmlNode, diff};
use ferrogram_vsdx::{Limits, Package, PartData, read_package, to_model, write_document};

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
#[allow(clippy::panic)] // test helper: a missing corpus must be loud here
fn corpus_missing(files: &[PathBuf]) -> bool {
    if files.is_empty() {
        if std::env::var_os("FERROGRAM_REQUIRE_CORPUS").is_some_and(|v| v == "1") {
            panic!("corpus/vsdx is absent and FERROGRAM_REQUIRE_CORPUS=1");
        }
        eprintln!("skipping: corpus/vsdx is absent; run `cargo xtask corpus`");
    }
    files.is_empty()
}

/// The names of the parts of a package, in order.
fn part_names(package: &Package) -> Vec<&str> {
    package
        .parts
        .iter()
        .map(|part| part.name.as_str())
        .collect()
}

/// The paths of every XML difference between two parts, empty when the parts
/// are equivalent (ADR 0012).
fn part_differences(a: &XmlNode, b: &XmlNode) -> Vec<String> {
    diff(a, b)
        .unwrap_or_default()
        .into_iter()
        .map(|difference| difference.to_string())
        .collect()
}

/// The display name of a root, for failure messages.
fn root_name(node: &XmlNode) -> String {
    match &node.name.prefix {
        Some(prefix) => format!("{}:{}", prefix, node.name.local),
        None => node.name.local.clone(),
    }
}

/// Round-trips one package: read, write, re-read, re-write.
///
/// Asserts semantic equivalence of the first write with the source and
/// byte-identity of the two writes. Prints the difference paths on failure.
#[allow(clippy::panic)] // test helper: the assertions report the failing file
fn round_trip(name: &str, bytes: &[u8]) {
    let source = read_package(bytes, &Limits::DEFAULT).unwrap_or_else(|error| {
        panic!("{name}: source package does not read: {error}");
    });
    let document = to_model(&source).unwrap_or_else(|error| {
        panic!("{name}: source does not map to the model: {error}");
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

    assert_eq!(
        first, second,
        "{name}: the second write differs from the first"
    );

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
        assert_eq!(before.name, after.name, "{name}: part names differ");
        assert_eq!(
            before.content_type,
            after.content_type,
            "{name}: part {} has a different content type",
            before.name.as_str()
        );
        match (&before.data, &after.data) {
            (PartData::Xml(a), PartData::Xml(b)) => {
                let differences = part_differences(a, b);
                assert!(
                    differences.is_empty(),
                    "{name}: part {} ({}) differs:\n{}",
                    before.name.as_str(),
                    root_name(a),
                    differences.join("\n")
                );
            }
            (PartData::Binary(a), PartData::Binary(b)) => {
                assert_eq!(a, b, "{name}: binary part {} differs", before.name.as_str());
            }
            _ => panic!("{name}: part {} changed kind", before.name.as_str()),
        }
    }
}

#[test]
fn every_corpus_file_round_trips() {
    let files = corpus_files();
    if corpus_missing(&files) {
        return;
    }
    for file in &files {
        let name = file.file_name().map_or_else(
            || file.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let bytes = std::fs::read(file).unwrap_or_else(|error| {
            panic!("{name}: cannot read: {error}");
        });
        round_trip(&name, &bytes);
    }
}
