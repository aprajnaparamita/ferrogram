//! Schema-validation harness (task P2-09, ADR 0017 section 6).
//!
//! Rust has no mature XSD validator, so the check shells out to
//! `xmllint --schema`. The schema itself is **never committed** (AGENTS.md
//! section 4): the test extracts the XSD that ships inside
//! `reference/ms-vsdx/office-developer-client-docs/docs/visio/schema-mapvisio-xml.md`
//! (a fenced code block, local only, gitignored) into a temp file at run time,
//! and the licence finding is recorded in `docs/sources.md`.
//!
//! The check is exposed as [`validate_package`], which takes a [`Package`] so
//! a later test (P2-10) can run it on written packages. Today it runs on the
//! **source** parts of the corpus, which proves the harness and records which
//! source parts the XSD accepts; a source part that fails is recorded data,
//! not a failure of this test.
//!
//! Two deviations from ADR 0017, both recorded in the task's Result and both
//! kept inside this file:
//!
//! 1. The embedded XSD is an **XSD 1.1** schema (`xsd:assert`,
//!    `xsd:alternative`, `maxOccurs` above 1 inside `xsd:all`), and `xmllint`
//!    is an XSD 1.0 tool: it refuses to load the schema as extracted.
//!    [`adjust_for_xmllint`] derives a temp-only, mechanically reduced copy
//!    that strips those constructs so `xmllint` can load it.
//! 2. The schema's target namespace is
//!    `http://schemas.microsoft.com/office/visio/2011/1/core`, while every
//!    `.vsdx` instance (and MS-VSDX) writes
//!    `http://schemas.microsoft.com/office/visio/2012/main`. The derived copy
//!    retargets the two namespace strings, without which no instance can ever
//!    match a global declaration.
//!
//! The test runs the extracted schema and the derived one side by side and
//! prints both verdicts; the derived one is what produces per-part results.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use larrygram_vsdx::{
    Limits, Package, PartData, read_document, read_package, write_document, write_package,
};
use zip::ZipArchive;

/// The namespace the XSD is written against, as extracted.
const SCHEMA_NAMESPACE: &str = "http://schemas.microsoft.com/office/visio/2011/1/core";
/// The namespace every `.vsdx` instance writes (MS-VSDX).
const INSTANCE_NAMESPACE: &str = "http://schemas.microsoft.com/office/visio/2012/main";

/// The root elements of the parts the model regenerates: `document.xml`,
/// `pages.xml`, `page{N}.xml`, `masters.xml`, `master{N}.xml` (ADR 0017).
const REGENERATED_ROOTS: &[&str] = &[
    "VisioDocument",
    "Pages",
    "PageContents",
    "Masters",
    "MasterContents",
];

/// What `xmllint` said about one part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaVerdict {
    /// The part validates against the schema.
    Valid,
    /// The schema loaded and the part failed validation.
    Invalid(String),
    /// `xmllint` could not load the schema at all, so nothing was validated.
    SchemaUnusable(String),
}

/// The validation result for one part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartVerdict {
    /// The part name as written in the package.
    pub part: String,
    /// What `xmllint` said.
    pub verdict: SchemaVerdict,
}

/// Validates the regenerated XML parts of `package` against the schema at
/// `schema` with `xmllint`.
///
/// The package is serialized with `write_package` and each regenerated XML
/// part is fed to `xmllint --noout --schema`. A part that fails is a recorded
/// [`SchemaVerdict::Invalid`], never a panic: this is a reporting harness and
/// the caller decides what is fatal.
pub fn validate_package(package: &Package, schema: &Path) -> Result<Vec<PartVerdict>, String> {
    let bytes = write_package(package).map_err(|why| format!("write_package failed: {why}"))?;
    let mut written = ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|why| format!("written package does not reopen: {why}"))?;
    let xmllint = find_xmllint().ok_or_else(|| "xmllint is not installed".to_owned())?;
    let scratch = ScratchDir::new();
    let mut verdicts = Vec::new();
    for part in &package.parts {
        let PartData::Xml(root) = &part.data else {
            continue;
        };
        if !REGENERATED_ROOTS.contains(&root.name.local.as_str()) {
            continue;
        }
        let entry_name = part.name.as_str().trim_start_matches('/').to_owned();
        let text = read_entry(&mut written, &entry_name)?;
        let file = scratch.write(&entry_name, &text);
        verdicts.push(PartVerdict {
            part: entry_name,
            verdict: run_xmllint(&xmllint, schema, &file),
        });
    }
    Ok(verdicts)
}

/// Reads one zip entry of the written package as bytes.
fn read_entry(
    archive: &mut ZipArchive<std::io::Cursor<Vec<u8>>>,
    name: &str,
) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut out = Vec::new();
    archive
        .by_name(name)
        .map_err(|why| format!("{name}: {why}"))?
        .read_to_end(&mut out)
        .map_err(|why| format!("{name}: {why}"))?;
    Ok(out)
}

/// Every normalized validation error of one part (P2-10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartErrors {
    /// The part name as written in the package.
    pub part: String,
    /// The normalized validation errors, in `xmllint`'s order.
    pub errors: Vec<String>,
}

/// Validates the regenerated XML parts of `package` like [`validate_package`]
/// but returns **every** validation error of each part, normalized for
/// comparison (P2-10 rule).
///
/// One error normalizes to its element name and error kind with the file name
/// and line number removed, so the same error in a source part and in a
/// written part normalizes to the same string and only genuinely new problems
/// differ. A schema `xmllint` cannot load is an `Err`, never a silent pass.
pub fn validate_package_errors(
    package: &Package,
    schema: &Path,
) -> Result<Vec<PartErrors>, String> {
    let bytes = write_package(package).map_err(|why| format!("write_package failed: {why}"))?;
    let mut written = ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|why| format!("written package does not reopen: {why}"))?;
    let xmllint = find_xmllint().ok_or_else(|| "xmllint is not installed".to_owned())?;
    let scratch = ScratchDir::new();
    let mut out = Vec::new();
    for part in &package.parts {
        let PartData::Xml(root) = &part.data else {
            continue;
        };
        if !REGENERATED_ROOTS.contains(&root.name.local.as_str()) {
            continue;
        }
        let entry_name = part.name.as_str().trim_start_matches('/').to_owned();
        let text = read_entry(&mut written, &entry_name)?;
        let file = scratch.write(&entry_name, &text);
        let message = xmllint_message(&xmllint, schema, &file)?;
        let errors = message.lines().filter_map(normalize_error).collect();
        out.push(PartErrors {
            part: entry_name,
            errors,
        });
    }
    Ok(out)
}

/// The full `xmllint --noout --schema` message for one file, or the reason
/// the schema is unusable.
fn xmllint_message(xmllint: &Path, schema: &Path, file: &Path) -> Result<String, String> {
    let output = Command::new(xmllint)
        .args(["--noout", "--schema"])
        .arg(schema)
        .arg(file)
        .output()
        .map_err(|why| why.to_string())?;
    let mut message = String::from_utf8_lossy(&output.stderr).into_owned();
    message.push_str(&String::from_utf8_lossy(&output.stdout));
    if message.contains("parser error") {
        return Err(first_line(&message).to_owned());
    }
    Ok(message)
}

/// One `xmllint` message line as a comparable error, or `None` for anything
/// that is not a validation error (the trailing "fails to validate" line,
/// blank lines).
///
/// The normalization drops the file name and the line number (`P2-10` rule:
/// compare by element path and error kind). A line reads
/// `<file>:<line>: element <name>: Schemas validity error : <detail>`; what
/// is left is `element <name>: Schemas validity error : <detail>`.
fn normalize_error(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.contains("Schemas validity error") {
        return None;
    }
    match trimmed.split_once(": element ") {
        Some((_, rest)) => Some(format!("element {rest}")),
        // No element name (a schema-level complaint): strip only the
        // `<file>:<line>: ` prefix.
        None => {
            let after_line = trimmed.split_once(": ").and_then(|(head, rest)| {
                head.rsplit(':')
                    .next()
                    .is_some_and(|line| {
                        !line.is_empty() && line.chars().all(|c| c.is_ascii_digit())
                    })
                    .then_some(rest)
            });
            after_line.map(str::to_owned)
        }
    }
}

/// Runs `xmllint --noout --schema` on one file and classifies the outcome.
fn run_xmllint(xmllint: &Path, schema: &Path, file: &Path) -> SchemaVerdict {
    let output = match Command::new(xmllint)
        .args(["--noout", "--schema"])
        .arg(schema)
        .arg(file)
        .output()
    {
        Ok(output) => output,
        Err(why) => return SchemaVerdict::SchemaUnusable(why.to_string()),
    };
    let mut message = String::from_utf8_lossy(&output.stderr).into_owned();
    message.push_str(&String::from_utf8_lossy(&output.stdout));
    if message.contains("parser error") {
        return SchemaVerdict::SchemaUnusable(first_line(&message).to_owned());
    }
    if output.status.success() {
        SchemaVerdict::Valid
    } else {
        SchemaVerdict::Invalid(first_line(&message).to_owned())
    }
}

/// The first non-empty line of a message.
fn first_line(message: &str) -> &str {
    message
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("(no message)")
}

/// Extracts the XSD from the markdown's first fenced code block.
///
/// Only the fenced schema body is taken: no annotation text, no prose leaves
/// the source (AGENTS.md section 4).
pub fn extract_xsd(markdown: &str) -> Option<String> {
    let mut lines = markdown.lines();
    lines.position(|line| line.trim_start().starts_with("```"))?;
    let mut body = Vec::new();
    for line in lines {
        if line.trim_start().starts_with("```") {
            break;
        }
        body.push(line);
    }
    Some(body.join("\n"))
}

/// Derives a temp-only copy of the XSD that `xmllint` (XSD 1.0) can load.
///
/// The source is XSD 1.1 and namespaced `2011/1/core`; this strips the 1.1
/// constructs (`xsd:assert`, `xsd:alternative`, `maxOccurs` above 1 inside
/// `xsd:all`) and retargets the namespace to the one instances write. The
/// result is never committed and never leaves the temp directory; it is a
/// diagnostic aid, not a corrected schema.
pub fn adjust_for_xmllint(xsd: &str) -> String {
    let mut out = xsd.to_owned();
    for prefix in ["xsd", "xs"] {
        out = strip_paired(
            &out,
            &format!("<{prefix}:assert"),
            &format!("</{prefix}:assert>"),
        );
        out = strip_self_closing(&out, &format!("<{prefix}:assert"));
        out = strip_paired(
            &out,
            &format!("<{prefix}:alternative"),
            &format!("</{prefix}:alternative>"),
        );
        out = strip_self_closing(&out, &format!("<{prefix}:alternative"));
        out = relax_all_blocks(&out, prefix);
    }
    out.replace(SCHEMA_NAMESPACE, INSTANCE_NAMESPACE)
}

/// Removes every `<open ...> ... </close>` block, nesting-free by
/// construction (the constructs it is used on cannot nest).
fn strip_paired(text: &str, open: &str, close: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(open) {
        out.push_str(rest.get(..start).unwrap_or_default());
        let tail = rest.get(start..).unwrap_or_default();
        match tail.find(close) {
            Some(end) => rest = tail.get(end + close.len()..).unwrap_or_default(),
            None => {
                // No closing tag: keep what is left untouched.
                out.push_str(tail);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Removes every self-closing `<open .../>` element.
fn strip_self_closing(text: &str, open: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(open) {
        let before = rest.get(..start).unwrap_or_default();
        let tail = rest.get(start..).unwrap_or_default();
        let self_closing = tail
            .find('>')
            .is_some_and(|end| tail.get(..end).is_some_and(|tag| tag.ends_with('/')));
        if !self_closing {
            // Not a self-closing tag: keep what is left untouched.
            out.push_str(rest);
            return out;
        }
        out.push_str(before);
        rest = tail
            .find('>')
            .and_then(|end| tail.get(end + 1..))
            .unwrap_or_default();
    }
    out.push_str(rest);
    out
}

/// Drops `maxOccurs` attributes inside `<prefix:all>` blocks (XSD 1.0 allows
/// only 0 or 1 there).
fn relax_all_blocks(text: &str, prefix: &str) -> String {
    let open = format!("<{prefix}:all");
    let close = format!("</{prefix}:all>");
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(&open) {
        let before = rest.get(..start).unwrap_or_default();
        out.push_str(before);
        let tail = rest.get(start..).unwrap_or_default();
        let (block, after) = match tail.find(&close) {
            Some(end) => (
                tail.get(..end + close.len()).unwrap_or_default(),
                tail.get(end + close.len()..).unwrap_or_default(),
            ),
            None => (tail, ""),
        };
        out.push_str(&drop_max_occurs(block));
        rest = after;
    }
    out.push_str(rest);
    out
}

/// Drops every ` maxOccurs="..."` from one block of text.
fn drop_max_occurs(block: &str) -> String {
    let mut out = String::with_capacity(block.len());
    let mut rest = block;
    while let Some(start) = rest.find(" maxOccurs=\"") {
        out.push_str(rest.get(..start).unwrap_or_default());
        let tail = rest.get(start..).unwrap_or_default();
        let after_value = tail.find('"').and_then(|open_quote| {
            tail.get(open_quote + 1..)
                .and_then(|value| value.find('"'))
                .map(|close_quote| open_quote + 1 + close_quote + 1)
        });
        match after_value {
            Some(next) => rest = tail.get(next..).unwrap_or_default(),
            None => {
                // Unterminated attribute: keep the rest untouched.
                out.push_str(tail);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

/// `xmllint` on `PATH`, or `None`.
fn find_xmllint() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    find_program("xmllint", &std::env::split_paths(&path).collect::<Vec<_>>())
}

/// The first directory on `paths` holding `name`.
fn find_program(name: &str, paths: &[PathBuf]) -> Option<PathBuf> {
    for dir in paths {
        for candidate in [dir.join(name), dir.join(format!("{name}.exe"))] {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// A temporary directory for extracted schemas and parts. Unique per call, so
/// nested users never delete each other's files.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new() -> ScratchDir {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let unique = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("larrygram-schema-{}-{unique}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        ScratchDir(dir)
    }

    /// Writes `bytes` under `name` (slashes flattened) and returns the path.
    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name.replace('/', "_"));
        let _ = std::fs::write(&path, bytes);
        path
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The markdown file with the embedded XSD and `xmllint`, or the reason the
/// test must skip. `LARRYGRAM_REQUIRE_SCHEMA=1` turns a skip into a failure.
fn schema_tools() -> Result<(PathBuf, PathBuf), String> {
    let markdown = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("reference")
        .join("ms-vsdx")
        .join("office-developer-client-docs")
        .join("docs")
        .join("visio")
        .join("schema-mapvisio-xml.md");
    if !markdown.is_file() {
        return Err(format!(
            "{} is absent; fetch `reference/` (AGENTS.md section 3)",
            markdown.display()
        ));
    }
    let xmllint = find_xmllint().ok_or_else(|| "xmllint is not installed".to_owned())?;
    Ok((markdown, xmllint))
}

/// Skips with a message, or fails when `LARRYGRAM_REQUIRE_SCHEMA=1`.
fn skip_or_fail(why: &str) {
    assert!(
        std::env::var_os("LARRYGRAM_REQUIRE_SCHEMA").is_none(),
        "LARRYGRAM_REQUIRE_SCHEMA=1 but the schema harness cannot run: {why}"
    );
    eprintln!("skipping schema validation: {why}");
}

/// The harness itself: the XSD extracts from the markdown, and the derived
/// copy loads in `xmllint`.
#[test]
fn the_embedded_xsd_extracts_and_the_derived_copy_loads() {
    let (markdown, xmllint) = match schema_tools() {
        Ok(tools) => tools,
        Err(why) => return skip_or_fail(&why),
    };
    let text = std::fs::read_to_string(&markdown).expect("the markdown reads");
    let xsd = extract_xsd(&text).expect("the markdown has a fenced schema");
    assert!(
        xsd.contains("VisioDocument"),
        "the schema names VisioDocument"
    );

    let scratch = ScratchDir::new();
    let as_is = scratch.write("schema-as-is.xsd", xsd.as_bytes());
    let derived = scratch.write("schema-derived.xsd", adjust_for_xmllint(&xsd).as_bytes());

    // The record: what `xmllint` makes of the schema as published.
    match schema_loads(&xmllint, &as_is) {
        Ok(()) => eprintln!("the extracted schema loads in xmllint as-is"),
        Err(why) => eprintln!("the extracted schema does not load in xmllint: {why}"),
    }
    // The teeth: the derived copy must load, or no part verdict means
    // anything.
    schema_loads(&xmllint, &derived).expect("the derived schema loads in xmllint");
}

/// Whether `xmllint` can load a schema at all (a probe document that cannot
/// possibly validate).
fn schema_loads(xmllint: &Path, schema: &Path) -> Result<(), String> {
    let scratch = ScratchDir::new();
    let probe = scratch.write("probe.xml", b"<probe/>");
    match run_xmllint(xmllint, schema, &probe) {
        SchemaVerdict::SchemaUnusable(why) => Err(why),
        _ => Ok(()),
    }
}

/// Validates the source XML parts of every corpus package, with the schema as
/// extracted and with the derived copy, and records every verdict.
#[test]
fn source_parts_validate_against_the_embedded_xsd() {
    let (markdown, _xmllint) = match schema_tools() {
        Ok(tools) => tools,
        Err(why) => return skip_or_fail(&why),
    };
    let files = corpus_files();
    if files.is_empty() {
        assert!(
            std::env::var_os("LARRYGRAM_REQUIRE_CORPUS").is_none(),
            "corpus/vsdx is missing and LARRYGRAM_REQUIRE_CORPUS=1 is set"
        );
        eprintln!("skipping: corpus/vsdx is absent; run `cargo xtask corpus`");
        return;
    }
    let text = std::fs::read_to_string(&markdown).expect("the markdown reads");
    let xsd = extract_xsd(&text).expect("the markdown has a fenced schema");
    let scratch = ScratchDir::new();
    let as_is = scratch.write("schema-as-is.xsd", xsd.as_bytes());
    let derived = scratch.write("schema-derived.xsd", adjust_for_xmllint(&xsd).as_bytes());

    let mut valid = 0;
    let mut invalid = 0;
    let mut unusable = 0;
    for path in files {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let bytes = std::fs::read(&path).expect("corpus file reads");
        let package = read_package(&bytes, &larrygram_vsdx::Limits::DEFAULT)
            .unwrap_or_else(|why| panic!("{name}: package reads: {why}"));
        let expected = package
            .parts
            .iter()
            .filter(|part| matches!(&part.data, PartData::Xml(root) if REGENERATED_ROOTS.contains(&root.name.local.as_str())))
            .count();
        for (label, schema) in [("as-is", &as_is), ("derived", &derived)] {
            let verdicts =
                validate_package(&package, schema).unwrap_or_else(|why| panic!("{name}: {why}"));
            assert_eq!(
                verdicts.len(),
                expected,
                "{name}: every regenerated part gets a verdict"
            );
            for verdict in &verdicts {
                match &verdict.verdict {
                    SchemaVerdict::Valid => {
                        if label == "derived" {
                            valid += 1;
                        }
                        eprintln!("{name} {} [{label}]: valid", verdict.part);
                    }
                    SchemaVerdict::Invalid(why) => {
                        if label == "derived" {
                            invalid += 1;
                        }
                        eprintln!("{name} {} [{label}]: invalid: {why}", verdict.part);
                    }
                    SchemaVerdict::SchemaUnusable(why) => {
                        if label == "derived" {
                            unusable += 1;
                        }
                        eprintln!("{name} {} [{label}]: schema unusable: {why}", verdict.part)
                    }
                }
            }
        }
    }
    eprintln!("derived-schema verdicts: {valid} valid, {invalid} invalid (recorded, not asserted)");
    // The teeth: the derived schema must load, or every verdict above is
    // vacuous. Per-part pass/fail is recorded data (upstream file content),
    // but a schema that does not load is a broken harness.
    assert_eq!(
        unusable, 0,
        "the derived schema did not load in xmllint; see \
         `the_embedded_xsd_extracts_and_the_derived_copy_loads`"
    );
}

/// The writer adds no new schema problems (P2-10 rule, master's note).
///
/// The docs XSD is out of sync with what Visio writes (the P2-09 finding:
/// only 3 of 106 source parts validate against the derived schema), so the
/// test does **not** require written parts to be valid. It requires that a
/// P2-07 round-trip adds no new problems: for every regenerated part, each
/// validation error of the written part must also occur for the source part,
/// compared by element path and error kind with the line numbers and the
/// file name removed (see [`normalize_error`]). A new error fails the test
/// and names the part.
#[test]
fn written_parts_add_no_new_schema_errors() {
    let (markdown, _xmllint) = match schema_tools() {
        Ok(tools) => tools,
        Err(why) => return skip_or_fail(&why),
    };
    let files = corpus_files();
    if files.is_empty() {
        assert!(
            std::env::var_os("LARRYGRAM_REQUIRE_CORPUS").is_none(),
            "corpus/vsdx is missing and LARRYGRAM_REQUIRE_CORPUS=1 is set"
        );
        eprintln!("skipping: corpus/vsdx is absent; run `cargo xtask corpus`");
        return;
    }
    let text = std::fs::read_to_string(&markdown).expect("the markdown reads");
    let xsd = extract_xsd(&text).expect("the markdown has a fenced schema");
    let scratch = ScratchDir::new();
    let derived = scratch.write("schema-derived.xsd", adjust_for_xmllint(&xsd).as_bytes());

    let mut checked = 0;
    let mut recorded = 0;
    for path in files {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let bytes = std::fs::read(&path).expect("corpus file reads");
        let source =
            read_package(&bytes, &Limits::DEFAULT).unwrap_or_else(|why| panic!("{name}: {why}"));
        let document =
            read_document(&bytes, &Limits::DEFAULT).unwrap_or_else(|why| panic!("{name}: {why}"));
        let written_bytes = write_document(&document, &source)
            .unwrap_or_else(|why| panic!("{name}: write_document: {why}"));
        let written = read_package(&written_bytes, &Limits::DEFAULT)
            .unwrap_or_else(|why| panic!("{name}: written package reads: {why}"));

        let source_errors = validate_package_errors(&source, &derived)
            .unwrap_or_else(|why| panic!("{name}: {why}"));
        let written_errors = validate_package_errors(&written, &derived)
            .unwrap_or_else(|why| panic!("{name}: {why}"));
        for part in &written_errors {
            let source_part = source_errors
                .iter()
                .find(|candidate| candidate.part == part.part);
            let known = source_part
                .map(|part| &part.errors)
                .cloned()
                .unwrap_or_default();
            let mut fresh: Vec<&String> = part
                .errors
                .iter()
                .filter(|error| !known.contains(error))
                .collect();
            fresh.dedup();
            assert!(
                fresh.is_empty(),
                "{name}: the writer added new schema problems in {}:\n  \
                 new: {fresh:#?}\n  source errors: {known:#?}",
                part.part
            );
            checked += 1;
            recorded += part.errors.len();
        }
    }
    eprintln!(
        "written parts checked against their source errors: {checked} ({recorded} validation \
         errors recorded)"
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_xsd_takes_the_first_fenced_block() {
        let markdown = "intro\n\n```XML\n<xsd:schema>\n  body\n</xsd:schema>\n```\n\nmore\n";
        assert_eq!(
            extract_xsd(markdown),
            Some("<xsd:schema>\n  body\n</xsd:schema>".to_owned())
        );
        assert_eq!(extract_xsd("no fence here"), None);
        assert_eq!(extract_xsd("```XML\n```\n"), Some(String::new()));
    }

    #[test]
    fn adjust_for_xmllint_strips_xsd_11_constructs_and_retargets_the_namespace() {
        let xsd = "<xsd:schema targetNamespace=\"http://schemas.microsoft.com/office/visio/2011/1/core\">\
                   <xsd:complexType><xsd:complexContent><xsd:extension base=\"Cell_Type\">\
                   <xsd:assert test=\"@N = 'Trigger'\" /></xsd:extension></xsd:complexContent></xsd:complexType>\
                   <xsd:complexType><xsd:all><xsd:element name=\"x\" maxOccurs=\"unbounded\" /></xsd:all></xsd:complexType>\
                   <xsd:element name=\"e\"><xsd:alternative test=\"x\" type=\"T\" /></xsd:element></xsd:schema>";
        let adjusted = adjust_for_xmllint(xsd);
        assert!(!adjusted.contains("assert"));
        assert!(!adjusted.contains("alternative"));
        assert!(!adjusted.contains("maxOccurs"));
        assert!(!adjusted.contains(SCHEMA_NAMESPACE));
        assert!(adjusted.contains(INSTANCE_NAMESPACE));
        assert!(adjusted.contains("<xsd:all>"));
    }

    #[test]
    fn find_program_searches_the_given_paths_only() {
        let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        assert!(find_program("Cargo.toml", std::slice::from_ref(&here)).is_some());
        assert!(find_program("definitely-not-here", &[here]).is_none());
        assert!(find_program("xmllint", &[]).is_none());
    }

    #[test]
    fn first_line_skips_leading_blanks() {
        assert_eq!(first_line("\n\nboom\nmore"), "boom");
        assert_eq!(first_line(""), "(no message)");
    }

    #[test]
    fn normalize_error_strips_the_file_name_and_line_number() {
        let line = "/tmp/scratch_visio_pages.xml:12: element Pages: Schemas validity error : \
                   Element 'Pages': The attribute 'xml:space' is not allowed.";
        assert_eq!(
            normalize_error(line),
            Some(
                "element Pages: Schemas validity error : Element 'Pages': The attribute \
                 'xml:space' is not allowed."
                    .to_owned()
            )
        );
        // The same error in another part at another line compares equal.
        assert_eq!(
            normalize_error("/tmp/a.xml:3: element Cell: Schemas validity error : boom"),
            normalize_error("other.xml:99: element Cell: Schemas validity error : boom")
        );
    }

    #[test]
    fn normalize_error_keeps_only_validation_errors() {
        assert_eq!(normalize_error("/tmp/a.xml fails to validate"), None);
        assert_eq!(normalize_error("parser error : ..."), None);
        assert_eq!(normalize_error(""), None);
        assert_eq!(
            normalize_error("3: Schemas validity error : schema-level complaint"),
            Some("Schemas validity error : schema-level complaint".to_owned())
        );
    }
}
