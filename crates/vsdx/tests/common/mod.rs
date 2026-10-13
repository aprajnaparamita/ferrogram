//! Hand-authored fixture builder for `ferrogram-vsdx` integration tests.
//!
//! Task P2-08. [`build`] generates every fixture byte by byte: a plain zip
//! archive around a minimal, hand-written Visio part set. The XML is our own
//! work (recorded in `corpus/PROVENANCE.md`); the element and attribute names,
//! content types and relationship types are structural facts from [MS-VSDX]
//! and the `docs/vsdx-inventory.md` part table.
//!
//! Determinism matters: the committed files under `corpus/fixtures/` must
//! equal the builder output byte for byte, so every archive entry is written
//! `Stored` (except the zip-bomb fixture, which needs real deflate) with a
//! fixed, clock-free timestamp.
//!
//! Later test crates (P2-09 onwards) include this module with `mod common;`
//! and call [`build`] instead of checking in blobs.

#![allow(dead_code)] // each test binary uses a different subset of the helpers

use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

/// What the builder produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fixture {
    /// A minimal `.vsdx` drawing: one page, one rectangle shape.
    Drawing,
    /// A minimal `.vssx` stencil: one master, an empty `pages.xml` and no
    /// page contents (ADR 0016).
    Stencil,
    /// A minimal `.vstx` template: one page, one rectangle shape, one master
    /// (ADR 0016).
    Template,
    /// A broken package, one per failure mode (ADR 0017 section 3).
    Malformed(Malformed),
}

/// The malformed-input failure modes the builder can produce (ADR 0017
/// section 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Malformed {
    /// The zip archive is cut in half.
    TruncatedZip,
    /// The central directory signature is destroyed.
    BadCentralDirectory,
    /// `[Content_Types].xml` is missing.
    NoContentTypes,
    /// A relationship targets a part that does not exist.
    DanglingRelationship,
    /// `/visio/document.xml` (the main part) is missing.
    MissingMainPart,
    /// An entry declares an uncompressed size far beyond
    /// `Limits::DEFAULT.max_part_uncompressed`.
    HugeDeclaredSize,
    /// One XML part nests elements deeper than
    /// `Limits::DEFAULT.max_xml_depth`.
    DeepNesting,
    /// `[Content_Types].xml` is not well-formed XML.
    BadContentTypes,
    /// One entry compresses 8 MiB of zeros, far beyond
    /// `Limits::DEFAULT.max_compression_ratio`.
    ZipBomb,
    /// The archive holds more entries than `Limits::DEFAULT.max_entries`.
    ManyEntries,
}

/// The fixtures committed under `corpus/fixtures/`, in the order the
/// regenerate test writes them. The zip-bomb and many-entries fixtures are
/// generated at test time instead (see [`path`]).
pub const COMMITTED_FIXTURES: &[Fixture] = &[
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
];

const XML_DECL: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>";
const XMLNS_VISIO: &str = "http://schemas.microsoft.com/office/visio/2012/main";
const XMLNS_R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const XMLNS_CT: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
const XMLNS_RELS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const CT_RELS: &str = "application/vnd.openxmlformats-package.relationships+xml";
const CT_XML: &str = "application/xml";
const CT_DRAWING_MAIN: &str = "application/vnd.ms-visio.drawing.main+xml";
const CT_STENCIL_MAIN: &str = "application/vnd.ms-visio.stencil.main+xml";
const CT_TEMPLATE_MAIN: &str = "application/vnd.ms-visio.template.main+xml";
const CT_PAGES: &str = "application/vnd.ms-visio.pages+xml";
const CT_PAGE: &str = "application/vnd.ms-visio.page+xml";
const CT_MASTERS: &str = "application/vnd.ms-visio.masters+xml";
const CT_MASTER: &str = "application/vnd.ms-visio.master+xml";
const REL_DOCUMENT: &str = "http://schemas.microsoft.com/visio/2010/relationships/document";
const REL_PAGES: &str = "http://schemas.microsoft.com/visio/2010/relationships/pages";
const REL_PAGE: &str = "http://schemas.microsoft.com/visio/2010/relationships/page";
const REL_MASTERS: &str = "http://schemas.microsoft.com/visio/2010/relationships/masters";
const REL_MASTER: &str = "http://schemas.microsoft.com/visio/2010/relationships/master";

/// One archive entry: name, bytes and compression method.
struct Entry {
    name: String,
    data: Vec<u8>,
    method: CompressionMethod,
}

/// Which well-formed package kind to assemble.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DocKind {
    Drawing,
    Stencil,
    Template,
}

/// Builds the raw bytes of a fixture package (see [`Fixture`]).
///
/// The output is deterministic: the same fixture always produces the same
/// bytes, so a committed file can be checked against the builder output.
pub fn build(fixture: Fixture) -> Vec<u8> {
    match fixture {
        Fixture::Drawing => zip_entries(&package_entries(DocKind::Drawing)),
        Fixture::Stencil => zip_entries(&package_entries(DocKind::Stencil)),
        Fixture::Template => zip_entries(&package_entries(DocKind::Template)),
        Fixture::Malformed(malformed) => build_malformed(malformed),
    }
}

/// The file name a fixture is committed under, e.g. `"minimal-drawing.vsdx"`.
pub fn file_name(fixture: &Fixture) -> &'static str {
    match fixture {
        Fixture::Drawing => "minimal-drawing.vsdx",
        Fixture::Stencil => "minimal-stencil.vssx",
        Fixture::Template => "minimal-template.vstx",
        Fixture::Malformed(m) => match m {
            Malformed::TruncatedZip => "malformed-truncated-zip.vsdx",
            Malformed::BadCentralDirectory => "malformed-bad-central-directory.vsdx",
            Malformed::NoContentTypes => "malformed-no-content-types.vsdx",
            Malformed::DanglingRelationship => "malformed-dangling-relationship.vsdx",
            Malformed::MissingMainPart => "malformed-missing-main-part.vsdx",
            Malformed::HugeDeclaredSize => "malformed-huge-declared-size.vsdx",
            Malformed::DeepNesting => "malformed-deep-nesting.vsdx",
            Malformed::BadContentTypes => "malformed-bad-content-types.vsdx",
            Malformed::ZipBomb => "malformed-zip-bomb.vsdx",
            Malformed::ManyEntries => "malformed-many-entries.vsdx",
        },
    }
}

/// Path of a committed fixture under `corpus/fixtures/`, if it is committed
/// rather than generated.
///
/// The zip-bomb and many-entries fixtures are generated by [`build`] at test
/// time: the bomb's deflate stream is not stable across `flate2` versions and
/// the many-entries archive is a boring 700 KiB of headers.
pub fn path(name: &str) -> Option<PathBuf> {
    let committed = COMMITTED_FIXTURES
        .iter()
        .any(|fixture| file_name(fixture) == name);
    committed.then(|| fixtures_dir().join(name))
}

/// The `corpus/fixtures/` directory in the repository root.
pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus/fixtures")
}

fn build_malformed(malformed: Malformed) -> Vec<u8> {
    match malformed {
        Malformed::TruncatedZip => {
            let bytes = build(Fixture::Drawing);
            Vec::from(bytes.get(..bytes.len() / 2).unwrap_or_default())
        }
        Malformed::BadCentralDirectory => corrupt_central_directory(build(Fixture::Drawing)),
        Malformed::NoContentTypes => {
            let entries: Vec<Entry> = package_entries(DocKind::Drawing)
                .into_iter()
                .filter(|e| e.name != "[Content_Types].xml")
                .collect();
            zip_entries(&entries)
        }
        Malformed::DanglingRelationship => {
            let mut entries = package_entries(DocKind::Drawing);
            for entry in &mut entries {
                if entry.name == "visio/_rels/document.xml.rels" {
                    entry.data = add_dangling_relationship(&entry.data);
                }
            }
            zip_entries(&entries)
        }
        Malformed::MissingMainPart => {
            let entries: Vec<Entry> = package_entries(DocKind::Drawing)
                .into_iter()
                .filter(|e| e.name != "visio/document.xml")
                .collect();
            zip_entries(&entries)
        }
        Malformed::HugeDeclaredSize => declare_huge_size(
            zip_entries(&package_entries(DocKind::Drawing)),
            "visio/pages/page1.xml",
            100 * 1024 * 1024,
        ),
        Malformed::DeepNesting => {
            let mut entries = package_entries(DocKind::Drawing);
            for entry in &mut entries {
                if entry.name == "visio/document.xml" {
                    entry.data = deep_document_xml(200).into_bytes();
                }
            }
            zip_entries(&entries)
        }
        Malformed::BadContentTypes => {
            let mut entries = package_entries(DocKind::Drawing);
            for entry in &mut entries {
                if entry.name == "[Content_Types].xml" {
                    // Well-formed opening tag, then the file just stops.
                    entry.data =
                        format!("{XML_DECL}\n<Types xmlns=\"{XMLNS_CT}\"><Default").into_bytes();
                }
            }
            zip_entries(&entries)
        }
        Malformed::ZipBomb => {
            let mut entries = package_entries(DocKind::Drawing);
            entries.push(Entry {
                name: "visio/bomb.bin".to_owned(),
                data: vec![0; 8 * 1024 * 1024],
                method: CompressionMethod::Deflated,
            });
            zip_entries(&entries)
        }
        Malformed::ManyEntries => {
            let entries: Vec<Entry> = (0..=ferrogram_vsdx::Limits::DEFAULT.max_entries)
                .map(|i| Entry {
                    name: format!("x{i:05}.bin"),
                    data: Vec::new(),
                    method: CompressionMethod::Stored,
                })
                .collect();
            zip_entries(&entries)
        }
    }
}

/// The part set of one well-formed package (ADR 0016): content types,
/// package and document relationships, `document.xml`, `pages.xml` (empty for
/// a stencil), `page1.xml` (drawings and templates) and the master parts
/// (stencils and templates).
fn package_entries(kind: DocKind) -> Vec<Entry> {
    let has_pages_contents = kind != DocKind::Stencil;
    let has_masters = kind != DocKind::Drawing;
    let stored = CompressionMethod::Stored;
    let mut entries = vec![
        Entry {
            name: "[Content_Types].xml".to_owned(),
            data: content_types(kind, has_pages_contents, has_masters).into_bytes(),
            method: stored,
        },
        Entry {
            name: "_rels/.rels".to_owned(),
            data: root_rels().into_bytes(),
            method: stored,
        },
        Entry {
            name: "visio/document.xml".to_owned(),
            data: document_xml().into_bytes(),
            method: stored,
        },
        Entry {
            name: "visio/_rels/document.xml.rels".to_owned(),
            data: document_rels(has_masters).into_bytes(),
            method: stored,
        },
        Entry {
            name: "visio/pages/pages.xml".to_owned(),
            data: pages_xml(has_pages_contents).into_bytes(),
            method: stored,
        },
    ];
    if has_pages_contents {
        entries.push(Entry {
            name: "visio/pages/_rels/pages.xml.rels".to_owned(),
            data: pages_rels().into_bytes(),
            method: stored,
        });
        entries.push(Entry {
            name: "visio/pages/page1.xml".to_owned(),
            data: page1_xml().into_bytes(),
            method: stored,
        });
    }
    if has_masters {
        entries.push(Entry {
            name: "visio/masters/masters.xml".to_owned(),
            data: masters_xml().into_bytes(),
            method: stored,
        });
        entries.push(Entry {
            name: "visio/masters/_rels/masters.xml.rels".to_owned(),
            data: masters_rels().into_bytes(),
            method: stored,
        });
        entries.push(Entry {
            name: "visio/masters/master1.xml".to_owned(),
            data: master1_xml().into_bytes(),
            method: stored,
        });
    }
    entries
}

fn content_types(kind: DocKind, has_pages_contents: bool, has_masters: bool) -> String {
    let main = match kind {
        DocKind::Drawing => CT_DRAWING_MAIN,
        DocKind::Stencil => CT_STENCIL_MAIN,
        DocKind::Template => CT_TEMPLATE_MAIN,
    };
    let mut s = String::new();
    s.push_str(XML_DECL);
    s.push_str(&format!("<Types xmlns=\"{XMLNS_CT}\">"));
    s.push_str(&format!(
        "<Default Extension=\"rels\" ContentType=\"{CT_RELS}\"/>"
    ));
    s.push_str(&format!(
        "<Default Extension=\"xml\" ContentType=\"{CT_XML}\"/>"
    ));
    override_decl(&mut s, "/visio/document.xml", main);
    override_decl(&mut s, "/visio/pages/pages.xml", CT_PAGES);
    if has_pages_contents {
        override_decl(&mut s, "/visio/pages/page1.xml", CT_PAGE);
    }
    if has_masters {
        override_decl(&mut s, "/visio/masters/masters.xml", CT_MASTERS);
        override_decl(&mut s, "/visio/masters/master1.xml", CT_MASTER);
    }
    s.push_str("</Types>");
    s
}

fn override_decl(s: &mut String, part: &str, content_type: &str) {
    s.push_str(&format!(
        "<Override PartName=\"{part}\" ContentType=\"{content_type}\"/>"
    ));
}

fn root_rels() -> String {
    format!(
        "{XML_DECL}\n<Relationships xmlns=\"{XMLNS_RELS}\">\
         <Relationship Id=\"rId1\" Type=\"{REL_DOCUMENT}\" Target=\"visio/document.xml\"/>\
         </Relationships>"
    )
}

fn document_xml() -> String {
    format!(
        "{XML_DECL}\n<VisioDocument xmlns=\"{XMLNS_VISIO}\" xmlns:r=\"{XMLNS_R}\" xml:space=\"preserve\">\
         <DocumentSettings TopPage=\"0\" DefaultTextStyle=\"0\" DefaultLineStyle=\"0\" \
         DefaultFillStyle=\"0\" DefaultGuideStyle=\"0\"/>\
         </VisioDocument>"
    )
}

fn document_rels(has_masters: bool) -> String {
    let mut s = String::new();
    s.push_str(XML_DECL);
    s.push_str(&format!("\n<Relationships xmlns=\"{XMLNS_RELS}\">"));
    s.push_str(&format!(
        "<Relationship Id=\"rId1\" Type=\"{REL_PAGES}\" Target=\"pages/pages.xml\"/>"
    ));
    if has_masters {
        s.push_str(&format!(
            "<Relationship Id=\"rId2\" Type=\"{REL_MASTERS}\" Target=\"masters/masters.xml\"/>"
        ));
    }
    s.push_str("</Relationships>");
    s
}

fn pages_xml(has_pages_contents: bool) -> String {
    if !has_pages_contents {
        // A stencil carries no pages (ADR 0016).
        return format!(
            "{XML_DECL}\n<Pages xmlns=\"{XMLNS_VISIO}\" xmlns:r=\"{XMLNS_R}\" xml:space=\"preserve\"/>"
        );
    }
    format!(
        "{XML_DECL}\n<Pages xmlns=\"{XMLNS_VISIO}\" xmlns:r=\"{XMLNS_R}\" xml:space=\"preserve\">\
         <Page ID=\"0\" NameU=\"Page-1\" Name=\"Page-1\">\
         <PageSheet LineStyle=\"0\" FillStyle=\"0\" TextStyle=\"0\">\
         <Cell N=\"PageWidth\" V=\"8.5\"/><Cell N=\"PageHeight\" V=\"11\"/>\
         </PageSheet><Rel r:id=\"rId1\"/></Page></Pages>"
    )
}

fn pages_rels() -> String {
    format!(
        "{XML_DECL}\n<Relationships xmlns=\"{XMLNS_RELS}\">\
         <Relationship Id=\"rId1\" Type=\"{REL_PAGE}\" Target=\"page1.xml\"/>\
         </Relationships>"
    )
}

fn page1_xml() -> String {
    format!(
        "{XML_DECL}\n<PageContents xmlns=\"{XMLNS_VISIO}\" xmlns:r=\"{XMLNS_R}\" xml:space=\"preserve\">\
         <Shapes>{}</Shapes></PageContents>",
        rect_shape(1)
    )
}

fn masters_xml() -> String {
    format!(
        "{XML_DECL}\n<Masters xmlns=\"{XMLNS_VISIO}\" xmlns:r=\"{XMLNS_R}\" xml:space=\"preserve\">\
         <Master ID=\"1\" NameU=\"Master.1\" Name=\"Master 1\" \
         UniqueID=\"{{5A4E1C2D-3B6F-4A21-9C0D-8E7F6A5B4C3D}}\" \
         BaseID=\"{{9F8E7D6C-5B4A-4321-8F0E-1D2C3B4A5968}}\" \
         LineStyle=\"0\" FillStyle=\"0\" TextStyle=\"0\"><Rel r:id=\"rId1\"/></Master></Masters>"
    )
}

fn masters_rels() -> String {
    format!(
        "{XML_DECL}\n<Relationships xmlns=\"{XMLNS_RELS}\">\
         <Relationship Id=\"rId1\" Type=\"{REL_MASTER}\" Target=\"master1.xml\"/>\
         </Relationships>"
    )
}

fn master1_xml() -> String {
    format!(
        "{XML_DECL}\n<MasterContents xmlns=\"{XMLNS_VISIO}\" xmlns:r=\"{XMLNS_R}\" xml:space=\"preserve\">\
         <Shapes>{}</Shapes></MasterContents>",
        rect_shape(1)
    )
}

/// A 2 by 1 rectangle with its own geometry rows. Our own values.
fn rect_shape(id: u32) -> String {
    format!(
        "<Shape ID=\"{id}\" Type=\"Shape\" LineStyle=\"0\" FillStyle=\"0\" TextStyle=\"0\">\
         <Cell N=\"PinX\" V=\"2\"/><Cell N=\"PinY\" V=\"2\"/>\
         <Cell N=\"Width\" V=\"2\"/><Cell N=\"Height\" V=\"1\"/>\
         <Cell N=\"LocPinX\" V=\"1\"/><Cell N=\"LocPinY\" V=\"0.5\"/>\
         <Section N=\"Geometry\" IX=\"0\"><Cell N=\"NoFill\" V=\"0\"/>\
         <Row T=\"RelMoveTo\" IX=\"1\"><Cell N=\"X\" V=\"0\"/><Cell N=\"Y\" V=\"0\"/></Row>\
         <Row T=\"RelLineTo\" IX=\"2\"><Cell N=\"X\" V=\"1\"/><Cell N=\"Y\" V=\"0\"/></Row>\
         <Row T=\"RelLineTo\" IX=\"3\"><Cell N=\"X\" V=\"1\"/><Cell N=\"Y\" V=\"1\"/></Row>\
         <Row T=\"RelLineTo\" IX=\"4\"><Cell N=\"X\" V=\"0\"/><Cell N=\"Y\" V=\"1\"/></Row>\
         <Row T=\"RelLineTo\" IX=\"5\"><Cell N=\"X\" V=\"0\"/><Cell N=\"Y\" V=\"0\"/></Row>\
         </Section></Shape>"
    )
}

/// `VisioDocument` with `depth` nested `<Deep>` elements: deeper than
/// `Limits::DEFAULT.max_xml_depth`.
fn deep_document_xml(depth: u32) -> String {
    let mut s = String::new();
    s.push_str(XML_DECL);
    s.push_str(&format!(
        "\n<VisioDocument xmlns=\"{XMLNS_VISIO}\" xmlns:r=\"{XMLNS_R}\" xml:space=\"preserve\">"
    ));
    for _ in 0..depth {
        s.push_str("<Deep>");
    }
    for _ in 0..depth {
        s.push_str("</Deep>");
    }
    s.push_str("</VisioDocument>");
    s
}

/// Adds a relationship whose target part does not exist.
fn add_dangling_relationship(rels: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(rels);
    let extra = "<Relationship Id=\"rId9\" Type=\"http://example.invalid/dangling\" \
                 Target=\"pages/missing.xml\"/>";
    text.replace("</Relationships>", &format!("{extra}</Relationships>"))
        .into_bytes()
}

/// Writes the entries as a plain zip archive with a fixed, clock-free
/// timestamp, so the bytes are a pure function of the entries.
fn zip_entries(entries: &[Entry]) -> Vec<u8> {
    use std::io::Write;
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut cursor);
        for entry in entries {
            let options = SimpleFileOptions::default()
                .compression_method(entry.method)
                .last_modified_time(DateTime::default());
            if zip.start_file(entry.name.as_str(), options).is_err() {
                continue;
            }
            if zip.write_all(&entry.data).is_err() {
                continue;
            }
        }
        let _ = zip.finish();
    }
    cursor.into_inner()
}

// Byte surgery on the zip container: the `zip` crate writes honest headers,
// and some failure modes need a lie.

const LOCAL_SIG: &[u8] = b"PK\x03\x04";
const CENTRAL_SIG: &[u8] = b"PK\x01\x02";
const EOCD_SIG: &[u8] = b"PK\x05\x06";

/// The start offset of the last subslice equal to `needle`.
fn rfind_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    (0..=haystack.len() - needle.len())
        .rev()
        .find(|&i| haystack.get(i..i + needle.len()) == Some(needle))
}

/// The start offset of the zip header with `sig` whose file-name field
/// (`name_at` bytes into the header) is `name`.
fn find_header(bytes: &[u8], sig: &[u8], name: &str, name_at: usize) -> Option<usize> {
    if sig.is_empty() {
        return None;
    }
    let name = name.as_bytes();
    let mut i = 0;
    while let Some(tail) = bytes.get(i..) {
        let rel = tail.windows(sig.len()).position(|w| w == sig)?;
        let start = i + rel;
        if bytes.get(start + name_at..start + name_at + name.len()) == Some(name) {
            return Some(start);
        }
        i = start + 1;
    }
    None
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let slot = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([slot[0], slot[1], slot[2], slot[3]]))
}

fn patch_u32(bytes: &mut [u8], offset: usize, value: u32) {
    if let Some(slot) = bytes.get_mut(offset..offset + 4) {
        slot.copy_from_slice(&value.to_le_bytes());
    }
}

/// Declares `declared` uncompressed bytes for `name` in both its local and
/// central-directory header, while the entry data stays as it is.
fn declare_huge_size(mut zip_bytes: Vec<u8>, name: &str, declared: u32) -> Vec<u8> {
    // Local file header: file name at +30, uncompressed size at +22.
    if let Some(start) = find_header(&zip_bytes, LOCAL_SIG, name, 30) {
        patch_u32(&mut zip_bytes, start + 22, declared);
    }
    // Central directory header: file name at +46, uncompressed size at +24.
    if let Some(start) = find_header(&zip_bytes, CENTRAL_SIG, name, 46) {
        patch_u32(&mut zip_bytes, start + 24, declared);
    }
    zip_bytes
}

/// Destroys the first central-directory header signature the end-of-central-
/// directory record points at.
fn corrupt_central_directory(mut zip_bytes: Vec<u8>) -> Vec<u8> {
    if let Some(offset) =
        rfind_subslice(&zip_bytes, EOCD_SIG).and_then(|eocd| read_u32(&zip_bytes, eocd + 16))
    {
        patch_u32(&mut zip_bytes, offset as usize, 0xFFFF_FFFF);
    }
    zip_bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    // The builder itself is exercised by `tests/fixtures.rs`; these unit
    // tests cover the zip byte-surgery helpers.

    #[test]
    fn rfind_subslice_finds_the_last_occurrence() {
        assert_eq!(rfind_subslice(b"abXabX", b"X"), Some(5));
        assert_eq!(rfind_subslice(b"abc", b"abcd"), None);
        assert_eq!(rfind_subslice(b"abc", b""), None);
    }

    #[test]
    fn find_header_matches_the_file_name_field() {
        let bytes = build(Fixture::Drawing);
        let local = find_header(&bytes, LOCAL_SIG, "[Content_Types].xml", 30);
        let central = find_header(&bytes, CENTRAL_SIG, "[Content_Types].xml", 46);
        assert!(local.is_some());
        assert!(central.is_some());
        assert_ne!(local, central);
        assert!(find_header(&bytes, LOCAL_SIG, "nope.xml", 30).is_none());
    }

    #[test]
    fn read_and_patch_u32_round_trip() {
        let mut bytes = vec![0, 1, 2, 3, 4, 5];
        patch_u32(&mut bytes, 1, 0x0640_0000);
        assert_eq!(read_u32(&bytes, 1), Some(0x0640_0000));
        assert_eq!(read_u32(&bytes, 4), None); // out of range, no panic
        patch_u32(&mut bytes, 99, 7); // out of range, no panic
    }

    #[test]
    fn corrupt_central_directory_breaks_the_signature() {
        let bytes = corrupt_central_directory(build(Fixture::Drawing));
        let eocd = rfind_subslice(&bytes, EOCD_SIG).unwrap();
        let cd_offset = read_u32(&bytes, eocd + 16).unwrap() as usize;
        assert_eq!(read_u32(&bytes, cd_offset), Some(0xFFFF_FFFF));
    }

    #[test]
    fn declare_huge_size_patches_both_headers() {
        let bytes = declare_huge_size(build(Fixture::Drawing), "visio/pages/page1.xml", 77);
        let local = find_header(&bytes, LOCAL_SIG, "visio/pages/page1.xml", 30).unwrap();
        let central = find_header(&bytes, CENTRAL_SIG, "visio/pages/page1.xml", 46).unwrap();
        assert_eq!(read_u32(&bytes, local + 22), Some(77));
        assert_eq!(read_u32(&bytes, central + 24), Some(77));
    }

    #[test]
    fn add_dangling_relationship_keeps_the_rest() {
        let rels = pages_rels().into_bytes();
        let out = add_dangling_relationship(&rels);
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("Target=\"pages/missing.xml\""));
        assert!(text.contains("Target=\"page1.xml\""));
    }
}
