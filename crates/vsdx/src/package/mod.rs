//! Pure OPC: part names, content types, relationships and the input caps.
//!
//! Nothing here knows about Visio XML: the package layer is a zip container
//! plus `[Content_Types].xml` and relationships (task P2-05).

pub(crate) mod discovery;
mod edit;
mod read;
mod write;
mod xml;

use larrygram_model::Preserved;

/// A part name inside the package, e.g. `"/visio/document.xml"`.
///
/// Names are absolute (leading slash), with `/` separators, exactly as OPC
/// writes them.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PartName(String);

impl PartName {
    /// Creates a part name from any string.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// The name as written in the package.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A content type string, e.g. `"application/vnd.ms-visio.drawing.main+xml"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContentType(String);

impl ContentType {
    /// Creates a content type from any string.
    pub fn new(content_type: impl Into<String>) -> Self {
        Self(content_type.into())
    }

    /// The content type as written in `[Content_Types].xml`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A relationship id, e.g. `"rId1"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelationshipId(String);

impl RelationshipId {
    /// Creates a relationship id from any string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The id as written in a relationships part.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether a relationship targets a part inside the package or an external
/// resource (the `TargetMode` attribute).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum TargetMode {
    /// The target is a part of this package (the default).
    #[default]
    Internal,
    /// The target is external (a URL), never dereferenced.
    External,
}

/// One relationship entry of a relationships part.
#[derive(Debug, Clone, PartialEq)]
pub struct Relationship {
    /// The part holding this relationship (package root for `/_rels/.rels`).
    pub source: PartName,
    /// The `Id` attribute.
    pub id: RelationshipId,
    /// The `Type` attribute (the relationship type URI).
    pub rel_type: String,
    /// The `Target` attribute, as written.
    pub target: String,
    /// The `TargetMode` attribute.
    pub target_mode: TargetMode,
    /// Unmapped attributes kept verbatim (ADR 0012).
    pub preserved: Preserved,
}

/// The bytes of one part, in the least structured form that keeps them
/// lossless (ADR 0012).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum PartData {
    /// An XML part, parsed into a preserved tree. Parts the model maps are
    /// also parsed into the model; this tree is what a write puts back.
    Xml(larrygram_model::XmlNode),
    /// A non-XML part (an image, a thumbnail, anything unknown).
    Binary(Vec<u8>),
}

/// One part of the package: name, content type and data.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    /// The part name.
    pub name: PartName,
    /// The content type from `[Content_Types].xml`.
    pub content_type: ContentType,
    /// The part data.
    pub data: PartData,
}

/// One `[Content_Types].xml` declaration, kept so a write can reproduce the
/// file's `Default` and `Override` spelling exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentTypeDecl {
    /// Whether this is a `Default` (by extension) or `Override` (by part).
    pub kind: ContentTypeKind,
    /// The file extension (for `Default`) or the part name (for `Override`).
    pub name: String,
    /// The declared content type.
    pub content_type: String,
}

/// Whether a content-type declaration is a `Default` or an `Override`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ContentTypeKind {
    /// A `Default` declaration, keyed by file extension.
    Default,
    /// An `Override` declaration, keyed by part name.
    Override,
}

/// A whole OPC package: parts, relationships and content-type declarations.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Package {
    /// Every part of the package, in zip order.
    pub parts: Vec<Part>,
    /// Every relationship, in file order (package root first).
    pub relationships: Vec<Relationship>,
    /// The `[Content_Types].xml` declarations, in file order.
    pub content_types: Vec<ContentTypeDecl>,
}

impl Package {
    /// Creates an empty package.
    pub fn new() -> Self {
        Self::default()
    }

    /// The part with this name, or `None`.
    pub fn part(&self, name: &PartName) -> Option<&Part> {
        self.parts.iter().find(|p| &p.name == name)
    }

    /// The relationships whose source is this part, in file order.
    pub fn relationships_from(&self, source: &PartName) -> impl Iterator<Item = &Relationship> {
        self.relationships
            .iter()
            .filter(move |r| &r.source == source)
    }
}

/// Caps on untrusted input (ADR 0017). Every cap is checked while reading,
/// before the memory it guards is allocated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The maximum number of zip entries (parts).
    pub max_entries: u32,
    /// The maximum uncompressed size of one part, in bytes.
    pub max_part_uncompressed: u64,
    /// The maximum uncompressed size of the whole package, in bytes.
    pub max_total_uncompressed: u64,
    /// The maximum uncompressed-to-compressed ratio of one entry (zip-bomb
    /// guard).
    pub max_compression_ratio: u64,
    /// The maximum nesting depth of XML elements in one part.
    pub max_xml_depth: u32,
    /// The maximum number of attributes on one XML element.
    pub max_attributes_per_element: u32,
}

impl Limits {
    /// The caps for normal files (ADR 0017).
    pub const DEFAULT: Self = Self {
        max_entries: 8192,
        max_part_uncompressed: 64 * 1024 * 1024,
        max_total_uncompressed: 512 * 1024 * 1024,
        max_compression_ratio: 200,
        max_xml_depth: 128,
        max_attributes_per_element: 256,
    };
}

impl Default for Limits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

pub use read::read_package;
pub use write::write_package;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newtypes_hold_their_strings() {
        assert_eq!(
            PartName::new("/visio/document.xml").as_str(),
            "/visio/document.xml"
        );
        assert_eq!(
            ContentType::new("application/xml").as_str(),
            "application/xml"
        );
        assert_eq!(RelationshipId::new("rId1").as_str(), "rId1");
    }

    #[test]
    fn target_mode_defaults_to_internal() {
        assert_eq!(TargetMode::default(), TargetMode::Internal);
    }

    #[test]
    fn package_finds_parts_and_relationships() {
        let mut p = Package::new();
        p.parts.push(Part {
            name: PartName::new("/visio/document.xml"),
            content_type: ContentType::new("application/xml"),
            data: PartData::Binary(Vec::new()),
        });
        p.relationships.push(Relationship {
            source: PartName::new("/visio/document.xml"),
            id: RelationshipId::new("rId1"),
            rel_type: "rel".to_owned(),
            target: "pages/pages.xml".to_owned(),
            target_mode: TargetMode::Internal,
            preserved: Preserved::new(),
        });
        p.relationships.push(Relationship {
            source: PartName::new("/"),
            id: RelationshipId::new("rId1"),
            rel_type: "rel".to_owned(),
            target: "visio/document.xml".to_owned(),
            target_mode: TargetMode::Internal,
            preserved: Preserved::new(),
        });

        assert!(p.part(&PartName::new("/visio/document.xml")).is_some());
        assert!(p.part(&PartName::new("/missing.xml")).is_none());
        assert_eq!(
            p.relationships_from(&PartName::new("/visio/document.xml"))
                .count(),
            1
        );
    }

    #[test]
    fn default_limits_are_the_adr_0017_caps() {
        let l = Limits::default();
        assert_eq!(l, Limits::DEFAULT);
        assert_eq!(l.max_entries, 8192);
        assert_eq!(l.max_part_uncompressed, 64 * 1024 * 1024);
        assert_eq!(l.max_total_uncompressed, 512 * 1024 * 1024);
        assert_eq!(l.max_compression_ratio, 200);
        assert_eq!(l.max_xml_depth, 128);
        assert_eq!(l.max_attributes_per_element, 256);
    }

    #[test]
    fn package_io_is_implemented() {
        assert!(read_package(&[], &Limits::DEFAULT).is_err());
        let bytes = write_package(&Package::new()).expect("write");
        assert!(!bytes.is_empty());
    }
}
