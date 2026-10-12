//! MS-VSDX schema inventory extractor (P1-05).
//!
//! Reads the XSD embedded in
//! `reference/ms-vsdx/office-developer-client-docs/docs/visio/schema-mapvisio-xml.md`
//! (and the `*-complextypevisio-xml.md` reference pages for the
//! documentation cross-links) and writes `docs/vsdx-inventory.md`: every
//! OPC part, XML element and attribute of the Visio file format with its
//! Phase 2 MVP status (`supported`, `preserved-only`, `later`).
//!
//! Sources (recorded in `docs/sources.md`):
//!
//! - `schema-mapvisio-xml.md` — Visio 2013 XSD in a fenced code block,
//!   `office-developer-client-docs` commit
//!   `f8c5dfed613fafd2fb8c8f9b1e43454f29517831`.
//! - `[MS-VSDX].pdf` v20250520 — canonical part enumeration and content
//!   types; cited as `source_revision` in the inventory header.
//!
//! Clean-room (AGENTS.md section 4): the extractor never copies XSD
//! prose. The output carries only derived facts — element names, type
//! names, attribute names, parent-child relationships, cardinalities,
//! part names and content types — plus our own labels. The
//! `<xsd:annotation>` copyright notice in the source XSD is never
//! written anywhere; [`clean_room_check`] refuses to write output that
//! contains annotation text, `<xsd:`, `Copyright` or `©`.
//!
//! Determinism (ADR 0009): every collection is a `BTreeMap`/`BTreeSet`
//! and rows are sorted by the keys named in the task. The output
//! contains no wall-clock values, so re-running the extractor produces
//! byte-identical output (asserted by a unit test).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use roxmltree::Node;

use super::markdown;
use super::{ExtractError, ExtractResult};

/// Relative path of the schema source under `reference/`.
const SCHEMA_MD: &str = "ms-vsdx/office-developer-client-docs/docs/visio/schema-mapvisio-xml.md";

/// Source URL written to the inventory header.
const SOURCE_URL: &str = "https://github.com/MicrosoftDocs/office-developer-client-docs (docs/visio/schema-mapvisio-xml.md)";

/// Commit of `office-developer-client-docs` the schema markdown was
/// retrieved at (P1-01, `docs/sources.md` source 4).
const XSD_COMMIT: &str = "f8c5dfed613fafd2fb8c8f9b1e43454f29517831 (2024-04-12)";

/// Canonical specification revision cited in the header.
const SOURCE_REVISION: &str = "[MS-VSDX] v20250520 (release 2025-05-20)";

/// "Extraction timestamp" recorded in the header. It is the retrieval
/// date of the source, not a wall clock reading, so that the output is
/// byte-identical across runs.
const EXTRACTED: &str = "2026-10-12";

/// File-name suffix of the per-type reference pages.
const TYPE_PAGE_SUFFIX: &str = "-complextypevisio-xml.md";

/// MVP status of one inventory row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    /// Phase 2 reads and writes this (models it).
    Supported,
    /// Phase 2 keeps this verbatim and never models it.
    PreservedOnly,
    /// Known, but not in the MVP.
    Later,
}

impl Status {
    /// The status as written in the inventory.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::PreservedOnly => "preserved-only",
            Self::Later => "later",
        }
    }
}

/// Element names promoted to `supported` by the task's conservative
/// rule, in addition to the document-properties subtree.
const SUPPORTED_ELEMENTS: &[&str] = &[
    "Pages", "Page", "Shape", "Cell", "Section", "Row", "Connect", "Masters", "Master",
];

/// Element names whose subtree belongs to a deferred "part" group
/// (Comments, Validation, DataRecordset); the whole subtree is `later`.
const DEFERRED_ROOT_ELEMENTS: &[&str] = &["Comments", "Validation", "DataRecordSets"];

/// Complex types that are the entry points of the deferred "Theme" and
/// "User" groups. The `User_Type` section (and everything under it) and
/// the theme cells are `later`.
const DEFERRED_ENTRY_TYPES: &[&str] = &["User_Type", "Theme_Type", "ThemeModern_Type"];

/// One hard-coded OPC part row. The part list is a closed set per
/// [MS-VSDX] section 2.3.1 (part enumeration) and the content types are
/// recorded in each part's section (2.3.3); desktop Visio adds the
/// windows, solutions and validation parts observed in the test corpus.
struct PartRow {
    /// Part path pattern; `{N}` stands for the 1-based instance index.
    path: &'static str,
    /// OPC content type (or a note when unknown).
    content_type: &'static str,
    /// Short label in our own words.
    label: &'static str,
    /// MVP status.
    status: Status,
}

/// The canonical part list. Sorted by path at render time; the catch-all
/// row is always last.
const PARTS: &[PartRow] = &[
    PartRow {
        path: "/[Content_Types].xml",
        content_type: "application/xml",
        label: "Package content-type map",
        status: Status::Supported,
    },
    PartRow {
        path: "/_rels/.rels",
        content_type: "application/vnd.openxmlformats-package.relationships+xml",
        label: "Package relationships",
        status: Status::Supported,
    },
    PartRow {
        path: "/customXml/*",
        content_type: "application/xml",
        label: "Custom XML data parts",
        status: Status::PreservedOnly,
    },
    PartRow {
        path: "/docProps/app.xml",
        content_type: "application/vnd.openxmlformats-officedocument.extended-properties+xml",
        label: "Extended document properties",
        status: Status::Supported,
    },
    PartRow {
        path: "/docProps/core.xml",
        content_type: "application/vnd.openxmlformats-package.core-properties+xml",
        label: "Core document properties",
        status: Status::Supported,
    },
    PartRow {
        path: "/docProps/custom.xml",
        content_type: "application/vnd.openxmlformats-officedocument.custom-properties+xml",
        label: "Custom document properties",
        status: Status::PreservedOnly,
    },
    PartRow {
        path: "/docProps/thumbnail.emf",
        content_type: "image/x-emf",
        label: "Document thumbnail",
        status: Status::PreservedOnly,
    },
    PartRow {
        path: "/visio/_rels/document.xml.rels",
        content_type: "application/vnd.openxmlformats-package.relationships+xml",
        label: "Document relationships",
        status: Status::Supported,
    },
    PartRow {
        path: "/visio/comments.xml",
        content_type: "application/vnd.ms-visio.comments+xml",
        label: "Comments and authors",
        status: Status::Later,
    },
    PartRow {
        path: "/visio/data/connections.xml",
        content_type: "application/vnd.ms-visio.connections+xml",
        label: "Data connections",
        status: Status::PreservedOnly,
    },
    PartRow {
        path: "/visio/data/recordsets.xml",
        content_type: "application/vnd.ms-visio.recordsets+xml",
        label: "Data recordsets and bindings",
        status: Status::Later,
    },
    PartRow {
        path: "/visio/document.xml",
        content_type: "application/vnd.ms-visio.drawing.main+xml",
        label: "Main drawing document",
        status: Status::Supported,
    },
    PartRow {
        path: "/visio/extensions.xml",
        content_type: "application/vnd.ms-visio.extensions+xml",
        label: "Extension definitions",
        status: Status::PreservedOnly,
    },
    PartRow {
        path: "/visio/masters/_rels/masters.xml.rels",
        content_type: "application/vnd.openxmlformats-package.relationships+xml",
        label: "Masters relationships",
        status: Status::Supported,
    },
    PartRow {
        path: "/visio/masters/_rels/master{N}.xml.rels",
        content_type: "application/vnd.openxmlformats-package.relationships+xml",
        label: "Master relationships",
        status: Status::Supported,
    },
    PartRow {
        path: "/visio/masters/masters.xml",
        content_type: "application/vnd.ms-visio.masters+xml",
        label: "Master index",
        status: Status::Supported,
    },
    PartRow {
        path: "/visio/masters/master{N}.xml",
        content_type: "application/vnd.ms-visio.master+xml",
        label: "Master contents (N = 1..count)",
        status: Status::Supported,
    },
    PartRow {
        path: "/visio/moderncomments.xml",
        content_type: "— (not in [MS-VSDX] v20250520)",
        label: "Modern comments",
        status: Status::Later,
    },
    PartRow {
        path: "/visio/pages/_rels/pages.xml.rels",
        content_type: "application/vnd.openxmlformats-package.relationships+xml",
        label: "Pages relationships",
        status: Status::Supported,
    },
    PartRow {
        path: "/visio/pages/_rels/page{N}.xml.rels",
        content_type: "application/vnd.openxmlformats-package.relationships+xml",
        label: "Page relationships (images)",
        status: Status::Supported,
    },
    PartRow {
        path: "/visio/pages/pages.xml",
        content_type: "application/vnd.ms-visio.pages+xml",
        label: "Page index",
        status: Status::Supported,
    },
    PartRow {
        path: "/visio/pages/page{N}.xml",
        content_type: "application/vnd.ms-visio.page+xml",
        label: "Page contents (N = 1..count)",
        status: Status::Supported,
    },
    PartRow {
        path: "/visio/persons.xml",
        content_type: "— (not in [MS-VSDX] v20250520)",
        label: "Modern comment authors",
        status: Status::Later,
    },
    PartRow {
        path: "/visio/solutions/solutions.xml",
        content_type: "application/vnd.ms-visio.solutions+xml",
        label: "Solution index",
        status: Status::PreservedOnly,
    },
    PartRow {
        path: "/visio/solutions/solution{N}.xml",
        content_type: "application/vnd.ms-visio.solution+xml",
        label: "Solution contents",
        status: Status::PreservedOnly,
    },
    PartRow {
        path: "/visio/theme.xml",
        content_type: "application/vnd.openxmlformats-officedocument.theme+xml",
        label: "Theme part ([MS-VSDX] naming)",
        status: Status::PreservedOnly,
    },
    PartRow {
        path: "/visio/theme/theme{N}.xml",
        content_type: "application/vnd.openxmlformats-officedocument.theme+xml",
        label: "Theme parts (desktop Visio naming)",
        status: Status::PreservedOnly,
    },
    PartRow {
        path: "/visio/validation.xml",
        content_type: "application/vnd.ms-visio.validation+xml",
        label: "Validation rules and issues",
        status: Status::Later,
    },
    PartRow {
        path: "/visio/validation_issues.xml",
        content_type: "— (not observed)",
        label: "Validation issues (separate part)",
        status: Status::Later,
    },
    PartRow {
        path: "/visio/windows.xml",
        content_type: "application/vnd.ms-visio.windows+xml",
        label: "Window and view state",
        status: Status::PreservedOnly,
    },
    PartRow {
        path: "(any other part)",
        content_type: "varies",
        label: "Unlisted parts (images, unknown)",
        status: Status::PreservedOnly,
    },
];

/// Key of one element declaration: `(parent type, element name)`. The
/// parent type is empty for root-level elements.
type DeclKey = (String, String);

/// One `<xsd:element>` declaration in the schema.
#[derive(Debug, Clone)]
struct ElementDecl {
    /// Declaring type name ("" for root-level elements).
    parent: String,
    /// Element name.
    name: String,
    /// The `type` reference.
    type_ref: String,
    /// Effective minimum occurrences (product over particles).
    min: u64,
    /// Effective maximum occurrences; `None` is unbounded.
    max: Option<u64>,
    /// Type names selected by `<xsd:alternative>` children.
    alternatives: Vec<String>,
    /// True for the root-level element declarations.
    is_root: bool,
}

impl ElementDecl {
    /// Cardinality as written in the inventory (`0..*`, `1..1`, ...).
    fn cardinality(&self) -> String {
        let max = match self.max {
            Some(m) => m.to_string(),
            None => "*".to_string(),
        };
        format!("{}..{max}", self.min)
    }

    /// The types this declaration may carry: its own type and the
    /// transitive extension bases, plus every alternative type and its
    /// bases.
    fn effective_types(&self, model: &XsdModel) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        for t in model.type_chain(&self.type_ref) {
            out.insert(t);
        }
        for alt in &self.alternatives {
            for t in model.type_chain(alt) {
                out.insert(t);
            }
        }
        out
    }
}

/// One `<xsd:attribute>` declaration in the schema.
#[derive(Debug, Clone)]
struct AttrDecl {
    /// Declaring type name.
    declaring: String,
    /// Attribute name.
    name: String,
    /// The `type` reference.
    type_ref: String,
}

/// One `<xsd:complexType>` declaration in the schema.
#[derive(Debug, Clone, Default)]
struct TypeDecl {
    /// Extension or restriction base, when declared.
    base: Option<String>,
}

/// The parsed schema: every type, element declaration and attribute,
/// keyed deterministically.
#[derive(Debug, Default)]
struct XsdModel {
    /// Complex types by name.
    types: BTreeMap<String, TypeDecl>,
    /// Element declarations by `(parent type, name)`.
    elements: BTreeMap<DeclKey, ElementDecl>,
    /// Attribute declarations by `(declaring type, name)`.
    attributes: BTreeMap<(String, String), AttrDecl>,
    /// Type name -> declaration keys selecting it via `xsd:alternative`.
    alt_uses: BTreeMap<String, BTreeSet<DeclKey>>,
}

impl XsdModel {
    /// Parses the schema document. Total: unknown constructs are
    /// skipped, anonymous nested types are folded into their parent.
    fn parse(doc: &roxmltree::Document<'_>) -> Self {
        let mut model = Self::default();
        let root = doc.root_element();
        for child in root.children().filter(|n| n.is_element()) {
            match tag_name(&child) {
                "element" => {
                    if let Some(name) = child.attribute("name").map(str::to_string) {
                        let decl = ElementDecl {
                            parent: String::new(),
                            name,
                            type_ref: child
                                .attribute("type")
                                .map(str::to_string)
                                .unwrap_or_default(),
                            min: 1,
                            max: Some(1),
                            alternatives: Vec::new(),
                            is_root: true,
                        };
                        model.insert_decl(decl);
                    }
                }
                "complexType" => {
                    let name = child.attribute("name").map(str::to_string);
                    let name = name.unwrap_or_default();
                    let base = find_base(&child);
                    model.types.entry(name.clone()).or_default().base = base;
                    model.collect(&child, &name, 1, Some(1));
                }
                _ => {}
            }
        }
        model
    }

    /// Walks the children of `node`, collecting element and attribute
    /// declarations under `type_name`. `min`/`max` accumulate the
    /// particle cardinalities seen so far.
    fn collect(&mut self, node: &Node<'_, '_>, type_name: &str, min: u64, max: Option<u64>) {
        for child in node.children().filter(|n| n.is_element()) {
            match tag_name(&child) {
                "element" => {
                    if let Some(name) = child.attribute("name").map(str::to_string) {
                        let (cmin, cmax) = combine(
                            min,
                            max,
                            child.attribute("minOccurs"),
                            child.attribute("maxOccurs"),
                        );
                        let mut alternatives = Vec::new();
                        for alt in child.children().filter(|n| n.is_element()) {
                            if tag_name(&alt) == "alternative"
                                && let Some(t) = alt.attribute("type")
                            {
                                alternatives.push(t.to_string());
                            }
                        }
                        let decl = ElementDecl {
                            parent: type_name.to_string(),
                            name,
                            type_ref: child
                                .attribute("type")
                                .map(str::to_string)
                                .unwrap_or_default(),
                            min: cmin,
                            max: cmax,
                            alternatives,
                            is_root: false,
                        };
                        self.insert_decl(decl);
                    }
                    // Anonymous nested types fold into the parent.
                    for nested in child.children().filter(|n| n.is_element()) {
                        let tag = tag_name(&nested);
                        if tag == "complexType" || tag == "simpleType" {
                            self.collect(&nested, type_name, 1, Some(1));
                        }
                    }
                }
                "attribute" => {
                    if let Some(name) = child.attribute("name").map(str::to_string) {
                        let decl = AttrDecl {
                            declaring: type_name.to_string(),
                            name,
                            type_ref: child
                                .attribute("type")
                                .map(str::to_string)
                                .unwrap_or_default(),
                        };
                        self.attributes
                            .entry((type_name.to_string(), decl.name.clone()))
                            .or_insert(decl);
                    }
                }
                "complexType" | "simpleType" => {
                    // Named or anonymous nested type; fold its content
                    // into the enclosing type's declarations.
                    self.collect(&child, type_name, 1, Some(1));
                }
                _ => {
                    let (cmin, cmax) = combine(
                        min,
                        max,
                        child.attribute("minOccurs"),
                        child.attribute("maxOccurs"),
                    );
                    self.collect(&child, type_name, cmin, cmax);
                }
            }
        }
    }

    /// Inserts an element declaration and records alternative-type use.
    fn insert_decl(&mut self, decl: ElementDecl) {
        let key: DeclKey = (decl.parent.clone(), decl.name.clone());
        for alt in &decl.alternatives {
            self.alt_uses
                .entry(alt.clone())
                .or_default()
                .insert(key.clone());
        }
        self.elements.entry(key).or_insert(decl);
    }

    /// `type_name` plus its transitive extension bases that are model
    /// types. Bounded: a base cycle cannot loop forever.
    fn type_chain(&self, type_name: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        let mut cur = type_name.to_string();
        loop {
            if !seen.insert(cur.clone()) {
                break;
            }
            out.push(cur.clone());
            match self.types.get(&cur).and_then(|t| t.base.as_ref()) {
                Some(base) => cur = base.clone(),
                None => break,
            }
            if out.len() > 64 {
                break;
            }
        }
        out
    }

    /// Element declarations declared inside `type_name`.
    fn children_of(&self, type_name: &str) -> Vec<DeclKey> {
        self.elements
            .values()
            .filter(|d| d.parent == type_name)
            .map(|d| (d.parent.clone(), d.name.clone()))
            .collect()
    }

    /// Child declarations reachable from one declaration (its own type,
    /// its bases and its alternative types).
    fn child_keys(&self, decl: &ElementDecl) -> Vec<DeclKey> {
        let mut out = BTreeSet::new();
        for t in decl.effective_types(self) {
            for key in self.children_of(&t) {
                out.insert(key);
            }
        }
        out.into_iter().collect()
    }
}

/// Element/attribute/particle tag local name.
fn tag_name<'a, 'input>(node: &Node<'a, 'input>) -> &'input str {
    node.tag_name().name()
}

/// Multiplies the accumulated particle cardinality with a node's own
/// `minOccurs`/`maxOccurs`. `None` max means unbounded.
fn combine(
    min: u64,
    max: Option<u64>,
    min_attr: Option<&str>,
    max_attr: Option<&str>,
) -> (u64, Option<u64>) {
    let nmin = min_attr.and_then(|v| v.parse::<u64>().ok()).unwrap_or(1);
    let nmax = match max_attr {
        Some("unbounded") => None,
        Some(v) => v.parse::<u64>().ok(),
        None => Some(1),
    };
    let cmin = min.saturating_mul(nmin);
    let cmax = match (max, nmax) {
        (Some(a), Some(b)) => Some(a.saturating_mul(b)),
        _ => None,
    };
    (cmin, cmax)
}

/// Finds the `base` of the closest `<xsd:extension>` or
/// `<xsd:restriction>` inside a complex type.
fn find_base(node: &Node<'_, '_>) -> Option<String> {
    node.descendants()
        .find(|n| {
            let t = tag_name(n);
            (t == "extension" || t == "restriction") && n.attribute("base").is_some()
        })
        .and_then(|n| n.attribute("base"))
        .map(str::to_string)
}

/// The documentation pages that map onto schema types.
#[derive(Debug, Default)]
struct DocIndex {
    /// Type name -> page file names (sorted).
    by_type: BTreeMap<String, Vec<String>>,
    /// Pages that map to no type in the embedded XSD (sorted).
    unmapped: BTreeSet<String>,
}

/// What a `*-complextypevisio-xml.md` page documents. (Microsoft names
/// a few element-occurrence pages with the type-page suffix.)
#[derive(Debug, PartialEq, Eq)]
enum PageKind {
    /// A page for one complex type: `Foo_Type complexType (Visio XML)`.
    TypePage(String),
    /// A page for one element occurrence: `Foo element (Bar_Type
    /// complexType) (Visio XML)`.
    ElementPage { name: String, parent: String },
}

impl DocIndex {
    /// Reads and classifies every page in `dir`. `model` decides whether
    /// a page maps to a schema type; pages that do not are listed as
    /// unmapped.
    fn read(dir: &Path, model: &XsdModel) -> ExtractResult<Self> {
        let mut index = Self::default();
        let mut names: Vec<String> = Vec::new();
        let entries = fs::read_dir(dir).map_err(|e| ExtractError::Io {
            path: dir.to_path_buf(),
            source: e.to_string(),
        })?;
        for entry in entries {
            let entry = entry.map_err(|e| ExtractError::Io {
                path: dir.to_path_buf(),
                source: e.to_string(),
            })?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(TYPE_PAGE_SUFFIX) {
                names.push(name);
            }
        }
        names.sort();
        for name in names {
            let path = dir.join(&name);
            let text = fs::read_to_string(&path).map_err(|e| ExtractError::Io {
                path: path.clone(),
                source: e.to_string(),
            })?;
            let Some(kind) = page_kind(&text) else {
                index.unmapped.insert(name);
                continue;
            };
            let type_name = match kind {
                PageKind::TypePage(t) => Some(t),
                PageKind::ElementPage { name, parent } => model
                    .elements
                    .get(&(parent.clone(), name.clone()))
                    .map(|d| d.type_ref.clone())
                    .filter(|t| !t.is_empty())
                    .or_else(|| kv_value(&text, "Element type")),
            };
            match type_name {
                Some(t) if model.types.contains_key(&t) => {
                    index.by_type.entry(t).or_default().push(name);
                }
                _ => {
                    index.unmapped.insert(name);
                }
            }
        }
        Ok(index)
    }
}

/// Classifies a page from its `# title` line.
fn page_kind(text: &str) -> Option<PageKind> {
    let title = text.lines().find_map(|l| l.strip_prefix("# "))?.trim();
    let rest = title.strip_suffix(" (Visio XML)")?;
    if let Some(name) = rest.strip_suffix(" complexType") {
        return Some(PageKind::TypePage(name.to_string()));
    }
    let rest = rest.strip_suffix(" complexType)")?;
    let (name, parent) = rest.split_once(" element (")?;
    Some(PageKind::ElementPage {
        name: name.to_string(),
        parent: parent.to_string(),
    })
}

/// Returns the value of one key in the page's metadata tables.
fn kv_value(text: &str, key: &str) -> Option<String> {
    markdown::kv_pairs(text)
        .into_iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
}

/// One inventory row per complex type.
#[derive(Debug)]
struct TypeRow {
    /// Type name.
    name: String,
    /// Extension/restriction base, or `-`.
    parent: String,
    /// Number of child element declarations.
    children: usize,
    /// MVP status.
    status: Status,
    /// Documenting page file names, comma separated, or `-`.
    doc_pages: String,
}

/// One inventory row per element declaration.
#[derive(Debug)]
struct ElementRow {
    /// Element name.
    name: String,
    /// Declaring type name, or `(root)`.
    parent: String,
    /// The `type` reference.
    type_ref: String,
    /// Effective cardinality.
    cardinality: String,
    /// MVP status.
    status: Status,
}

/// One inventory row per attribute declaration.
#[derive(Debug)]
struct AttrRow {
    /// Attribute name.
    name: String,
    /// Names of the elements carrying the attribute, comma separated.
    parent_element: String,
    /// Declaring type name.
    declaring: String,
    /// The `type` reference.
    type_ref: String,
    /// MVP status.
    status: Status,
}

/// The whole generated inventory.
#[derive(Debug, Default)]
struct Inventory {
    /// Per-type rows, sorted by name.
    types: Vec<TypeRow>,
    /// Per-declaration rows, sorted by `(parent, name)`.
    elements: Vec<ElementRow>,
    /// Per-declaration rows, sorted by `(parent element, name,
    /// declaring)`.
    attributes: Vec<AttrRow>,
    /// Pages with no matching schema type (sorted).
    unmapped_pages: Vec<String>,
}

impl Inventory {
    /// Computes every row and every status from the parsed schema.
    fn build(model: &XsdModel, docs: DocIndex) -> Self {
        let later_decls = later_decl_set(model);
        let doc_props = doc_props_set(model);

        // Status of every element declaration, keyed for reuse.
        let mut decl_status: BTreeMap<DeclKey, Status> = BTreeMap::new();
        for (key, decl) in &model.elements {
            let status = if later_decls.contains(key) {
                Status::Later
            } else if SUPPORTED_ELEMENTS.contains(&decl.name.as_str()) || doc_props.contains(key) {
                Status::Supported
            } else {
                Status::PreservedOnly
            };
            decl_status.insert(key.clone(), status);
        }

        // Types: rule order is documented in the inventory header.
        let mut type_status: BTreeMap<String, Status> = BTreeMap::new();
        for name in model.types.keys() {
            type_status.insert(name.clone(), model.type_status(name, &decl_status));
        }

        let mut types = Vec::new();
        for (name, decl) in &model.types {
            let children = model.children_of(name).len();
            let doc_pages = docs
                .by_type
                .get(name)
                .map(|v| v.join(", "))
                .unwrap_or_else(|| "-".to_string());
            types.push(TypeRow {
                name: name.clone(),
                parent: decl.base.clone().unwrap_or_else(|| "-".to_string()),
                children,
                status: type_status_for(&type_status, name),
                doc_pages,
            });
        }

        let mut elements = Vec::new();
        for (key, decl) in &model.elements {
            elements.push(ElementRow {
                name: decl.name.clone(),
                parent: if decl.parent.is_empty() {
                    "(root)".to_string()
                } else {
                    decl.parent.clone()
                },
                type_ref: if decl.type_ref.is_empty() {
                    "-".to_string()
                } else {
                    decl.type_ref.clone()
                },
                cardinality: decl.cardinality(),
                status: status_for(&decl_status, key),
            });
        }

        let mut attributes = Vec::new();
        for attr in model.attributes.values() {
            let declaring = &attr.declaring;
            let mut carriers: BTreeSet<String> = BTreeSet::new();
            for decl in model.elements.values() {
                let chain = model.type_chain(&decl.type_ref);
                if chain.iter().any(|t| t == declaring) {
                    carriers.insert(decl.name.clone());
                }
            }
            let parent_element = if carriers.is_empty() {
                "-".to_string()
            } else {
                carriers.into_iter().collect::<Vec<_>>().join(", ")
            };
            attributes.push(AttrRow {
                name: attr.name.clone(),
                parent_element,
                declaring: declaring.clone(),
                type_ref: if attr.type_ref.is_empty() {
                    "-".to_string()
                } else {
                    attr.type_ref.clone()
                },
                status: type_status_for(&type_status, declaring),
            });
        }

        // Sort: parts by path (render time), types by name, elements by
        // (parent, name), attributes by (parent element, name,
        // declaring).
        types.sort_by(|a, b| a.name.cmp(&b.name));
        elements.sort_by(|a, b| (&a.parent, &a.name).cmp(&(&b.parent, &b.name)));
        attributes.sort_by(|a, b| {
            (&a.parent_element, &a.name, &a.declaring).cmp(&(
                &b.parent_element,
                &b.name,
                &b.declaring,
            ))
        });

        Self {
            types,
            elements,
            attributes,
            unmapped_pages: docs.unmapped.into_iter().collect(),
        }
    }

    /// Renders the inventory markdown. Deterministic.
    fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("# MS-VSDX schema inventory (generated)\n\n");
        out.push_str(
            "<!-- Generated by `cargo xtask extract` (xtask/src/extract/vsdx_schema.rs).\n     Do not edit by hand; edit the extractor and re-run. -->\n\n",
        );
        out.push_str(
            "Planning document for Phase 2 (`.vsdx` read/write) and Phase 3 (rendering).\n\n",
        );
        out.push_str("## 1. Header\n\n");
        out.push_str(&format!("- **Source URL**: {SOURCE_URL}\n"));
        out.push_str(&format!("- **XSD commit**: {XSD_COMMIT}\n"));
        out.push_str(&format!("- **Source revision**: {SOURCE_REVISION}\n"));
        out.push_str(&format!(
            "- **Extraction timestamp**: {EXTRACTED} (fixed retrieval date, not a wall clock: the\n  extractor writes no run-time values so output is byte-identical across runs)\n"
        ));
        out.push_str(
            "- **Clean-room**: names and structure only. No schema text, annotation text or\n  descriptions from the source are copied (AGENTS.md section 4).\n\n",
        );

        out.push_str("### Status legend\n\n");
        out.push_str("- `supported` — Phase 2 reads and writes this (models it).\n");
        out.push_str("- `preserved-only` — Phase 2 keeps this verbatim and never models it.\n");
        out.push_str("- `later` — known, but not in the MVP.\n\n");

        out.push_str("### Status rules\n\n");
        out.push_str(
            "Element declarations are `preserved-only` by default and promoted to `supported`\nonly for the MVP shape surface: the `Pages`, `Page`, `Shape`, `Cell`, `Section`,\n`Row`, `Connect`, `Masters` and `Master` elements and the `DocumentProperties`\nsubtree. Elements under a deferred group — the Comments, Validation and\nDataRecordset subtrees, the `User` section and the theme cells — are `later`,\nwhich overrides the `supported` list inside those groups.\n\n",
        );
        out.push_str(
            "A type is `supported` when a `supported` element carries it (directly or via an\nextension base); a type is `later` when it is a deferred group entry point or all\nof its element usages are `later`; everything else is `preserved-only`. An\nattribute's status is its declaring type's status.\n\n",
        );

        out.push_str("## 2. OPC parts\n\n");
        out.push_str("One row per part kind; `{N}` is the 1-based instance index. The last row is the\ncatch-all for parts not listed here.\n\n");
        let mut part_rows: Vec<Vec<String>> = PARTS
            .iter()
            .filter(|p| p.path != "(any other part)")
            .map(|p| {
                vec![
                    p.path.to_string(),
                    p.content_type.to_string(),
                    p.label.to_string(),
                    p.status.as_str().to_string(),
                ]
            })
            .collect();
        part_rows.sort_by(|a, b| a[0].cmp(&b[0]));
        for p in PARTS.iter().filter(|p| p.path == "(any other part)") {
            part_rows.push(vec![
                p.path.to_string(),
                p.content_type.to_string(),
                p.label.to_string(),
                p.status.as_str().to_string(),
            ]);
        }
        out.push_str(&md_table(
            &["Part", "Content type", "Label", "Status"],
            &part_rows,
        ));

        out.push_str("\n## 3. Complex types\n\n");
        out.push_str("One row per XSD complexType declaration in the embedded XSD, with the reference\npage(s) that document it under `reference/ms-vsdx/office-developer-client-docs/docs/visio/`.\n`Children` counts the element declarations the type declares.\n\n");
        let type_rows: Vec<Vec<String>> = self
            .types
            .iter()
            .map(|t| {
                vec![
                    t.name.clone(),
                    t.parent.clone(),
                    t.children.to_string(),
                    t.status.as_str().to_string(),
                    t.doc_pages.clone(),
                ]
            })
            .collect();
        out.push_str(&md_table(
            &["Type", "Parent type", "Children", "Status", "Doc page"],
            &type_rows,
        ));
        if !self.unmapped_pages.is_empty() {
            out.push_str("\n### Doc pages with no matching schema type\n\n");
            out.push_str(
                "These `*-complextypevisio-xml.md` pages describe a later schema revision and have\nno entry in the embedded XSD:\n\n",
            );
            for page in &self.unmapped_pages {
                out.push_str(&format!("- `{page}`\n"));
            }
        }

        out.push_str("\n## 4. Elements\n\n");
        out.push_str("One row per XSD element declaration, sorted by `(parent type, element name)`.\n`(root)` marks a root-level element; cardinality is the effective\n`minOccurs..maxOccurs` (`*` = unbounded).\n\n");
        let element_rows: Vec<Vec<String>> = self
            .elements
            .iter()
            .map(|e| {
                vec![
                    e.name.clone(),
                    e.parent.clone(),
                    e.type_ref.clone(),
                    e.cardinality.clone(),
                    e.status.as_str().to_string(),
                ]
            })
            .collect();
        out.push_str(&md_table(
            &[
                "Element",
                "Parent type",
                "Type ref",
                "Cardinality",
                "Status",
            ],
            &element_rows,
        ));

        out.push_str("\n## 5. Attributes\n\n");
        out.push_str("One row per XSD attribute declaration, sorted by `(parent element, attribute\nname, declaring type)`. `Parent element` lists the elements carrying the\nattribute (directly or via an extension base).\n\n");
        let attr_rows: Vec<Vec<String>> = self
            .attributes
            .iter()
            .map(|a| {
                vec![
                    a.name.clone(),
                    a.parent_element.clone(),
                    a.declaring.clone(),
                    a.type_ref.clone(),
                    a.status.as_str().to_string(),
                ]
            })
            .collect();
        out.push_str(&md_table(
            &[
                "Attribute",
                "Parent element",
                "Declared in",
                "Type",
                "Status",
            ],
            &attr_rows,
        ));

        out.push_str("\n## 6. Counts\n\n");
        let counts: Vec<Vec<String>> = vec![
            count_row("OPC parts", &part_status_counts()),
            count_row(
                "Complex types",
                &status_counts(self.types.iter().map(|t| t.status)),
            ),
            count_row(
                "Elements",
                &status_counts(self.elements.iter().map(|e| e.status)),
            ),
            count_row(
                "Attributes",
                &status_counts(self.attributes.iter().map(|a| a.status)),
            ),
        ];
        out.push_str(&md_table(
            &["Table", "Total", "supported", "preserved-only", "later"],
            &counts,
        ));
        out.push('\n');
        out
    }
}

/// Status lookup that cannot fail (rows are built together).
fn status_for(map: &BTreeMap<DeclKey, Status>, key: &DeclKey) -> Status {
    map.get(key).copied().unwrap_or(Status::PreservedOnly)
}

/// Status lookup that cannot fail (rows are built together).
fn type_status_for(map: &BTreeMap<String, Status>, name: &str) -> Status {
    map.get(name).copied().unwrap_or(Status::PreservedOnly)
}

impl XsdModel {
    /// Status of one complex type:
    ///
    /// 1. a deferred group entry point is `later`;
    /// 2. otherwise the status is the strongest of the statuses of the
    ///    element declarations carrying the type (`supported` beats
    ///    `preserved-only` beats `later`);
    /// 3. a type carried by no element but selected only by `later`
    ///    alternatives is `later`;
    /// 4. everything else is `preserved-only`.
    fn type_status(&self, name: &str, decl_status: &BTreeMap<DeclKey, Status>) -> Status {
        if DEFERRED_ENTRY_TYPES.contains(&name) {
            return Status::Later;
        }
        let mut saw_supported = false;
        let mut saw_preserved = false;
        let mut saw_later = false;
        for (key, decl) in &self.elements {
            let chain = self.type_chain(&decl.type_ref);
            if chain.iter().any(|t| t == name) {
                match status_for(decl_status, key) {
                    Status::Supported => saw_supported = true,
                    Status::PreservedOnly => saw_preserved = true,
                    Status::Later => saw_later = true,
                }
            }
        }
        if saw_supported || saw_preserved || saw_later {
            return if saw_supported {
                Status::Supported
            } else if saw_preserved {
                Status::PreservedOnly
            } else {
                Status::Later
            };
        }
        if let Some(uses) = self.alt_uses.get(name)
            && !uses.is_empty()
            && uses
                .iter()
                .all(|k| status_for(decl_status, k) == Status::Later)
        {
            return Status::Later;
        }
        Status::PreservedOnly
    }
}

/// The set of element declarations that are `later`: everything under a
/// deferred group, computed so that shared base types do not leak into
/// the groups (see the tests).
fn later_decl_set(model: &XsdModel) -> BTreeSet<DeclKey> {
    let is_deferred_node = |key: &DeclKey| DEFERRED_ROOT_ELEMENTS.contains(&key.1.as_str());
    let is_deferred_edge = |child: &DeclKey| DEFERRED_ENTRY_TYPES.contains(&child.0.as_str());
    let roots: Vec<DeclKey> = model
        .elements
        .values()
        .filter(|d| d.is_root)
        .map(|d| (d.parent.clone(), d.name.clone()))
        .collect();

    // Reachable from the roots without crossing a deferred node or
    // edge: those declarations have a non-deferred path and are not
    // `later`.
    let mut plain: BTreeSet<DeclKey> = BTreeSet::new();
    let mut queue: Vec<DeclKey> = roots
        .iter()
        .filter(|k| !is_deferred_node(k))
        .cloned()
        .collect();
    while let Some(key) = queue.pop() {
        if !plain.insert(key.clone()) {
            continue;
        }
        if let Some(decl) = model.elements.get(&key) {
            for child in model.child_keys(decl) {
                if !is_deferred_edge(&child) && !is_deferred_node(&child) {
                    queue.push(child);
                }
            }
        }
    }

    // Everything reachable at all.
    let mut all: BTreeSet<DeclKey> = BTreeSet::new();
    let mut queue = roots.clone();
    while let Some(key) = queue.pop() {
        if !all.insert(key.clone()) {
            continue;
        }
        if let Some(decl) = model.elements.get(&key) {
            for child in model.child_keys(decl) {
                queue.push(child);
            }
        }
    }

    model
        .elements
        .keys()
        .filter(|key| is_deferred_node(key) || (all.contains(*key) && !plain.contains(*key)))
        .cloned()
        .collect()
}

/// The `DocumentProperties` subtree: the element itself and every
/// declaration reachable below it. All of these are `supported`.
fn doc_props_set(model: &XsdModel) -> BTreeSet<DeclKey> {
    let mut out: BTreeSet<DeclKey> = BTreeSet::new();
    let mut queue: Vec<DeclKey> = model
        .elements
        .values()
        .filter(|d| d.name == "DocumentProperties")
        .map(|d| (d.parent.clone(), d.name.clone()))
        .collect();
    while let Some(key) = queue.pop() {
        if !out.insert(key.clone()) {
            continue;
        }
        if let Some(decl) = model.elements.get(&key) {
            for child in model.child_keys(decl) {
                queue.push(child);
            }
        }
    }
    out
}

/// Counts per status for the parts table.
fn part_status_counts() -> [usize; 4] {
    status_counts(PARTS.iter().map(|p| p.status))
}

/// `[total, supported, preserved-only, later]` for a status iterator.
fn status_counts(iter: impl Iterator<Item = Status>) -> [usize; 4] {
    let mut counts = [0usize, 0, 0, 0];
    for status in iter {
        counts[0] += 1;
        match status {
            Status::Supported => counts[1] += 1,
            Status::PreservedOnly => counts[2] += 1,
            Status::Later => counts[3] += 1,
        }
    }
    counts
}

/// One summary row of the counts table.
fn count_row(kind: &str, counts: &[usize; 4]) -> Vec<String> {
    let [total, supported, preserved, later] = *counts;
    vec![
        kind.to_string(),
        total.to_string(),
        supported.to_string(),
        preserved.to_string(),
        later.to_string(),
    ]
}

/// Renders a markdown table with a header row.
fn md_table(header: &[&str], rows: &[Vec<String>]) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "| {} |\n",
        header
            .iter()
            .map(|h| format!("**{h}**"))
            .collect::<Vec<_>>()
            .join(" | ")
    ));
    out.push_str(&format!(
        "|{}|\n",
        header.iter().map(|_| "---").collect::<Vec<_>>().join("|")
    ));
    for row in rows {
        out.push_str(&format!("| {} |\n", row.join(" | ")));
    }
    out
}

/// Extracts the first fenced block that contains an `<xsd:schema`.
fn extract_xsd(text: &str) -> Option<String> {
    markdown::fenced_blocks(text)
        .into_iter()
        .find(|b| b.body.contains("<xsd:schema"))
        .map(|b| b.body)
}

/// Collects strings that must never appear in the output: every
/// non-empty line and every long word of an `<xsd:documentation>` text
/// in the schema's annotations, plus the fixed forbidden substrings.
fn forbidden_strings(doc: &roxmltree::Document<'_>) -> Vec<String> {
    let mut out = vec![
        "<xsd:".to_string(),
        "Copyright".to_string(),
        "©".to_string(),
    ];
    for node in doc.descendants() {
        if tag_name(&node) == "documentation" {
            let text = node.text().unwrap_or_default();
            for line in text.lines() {
                let line = line.trim();
                if !line.is_empty() {
                    out.push(line.to_string());
                }
            }
            for word in text.split_whitespace() {
                let word = word.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-');
                // Long words only: short words are common English.
                if word.len() >= 12 {
                    out.push(word.to_string());
                }
            }
        }
    }
    out
}

/// Refuses to return output that carries Microsoft-owned text.
///
/// One wrinkle: the schema itself names a ShapeSheet cell type
/// `Copyright_Type`. Schema identifiers are structural data (AGENTS.md
/// section 4), so an occurrence of a forbidden word is accepted when the
/// word it sits in is one of `identifiers` (the schema's own names).
/// Everything else — annotation lines, long annotation words, `©`, and
/// any `<xsd:` markup — is rejected outright.
fn clean_room_check(
    markdown: &str,
    forbidden: &[String],
    identifiers: &BTreeSet<String>,
) -> ExtractResult<()> {
    for needle in forbidden {
        let mut from = 0;
        while let Some(offset) = markdown[from..].find(needle.as_str()) {
            let at = from + offset;
            let word = enclosing_word(markdown, at, needle.len());
            if !identifiers.contains(&word) {
                return Err(ExtractError::Parse {
                    path: PathBuf::from("docs/vsdx-inventory.md"),
                    message: format!(
                        "clean-room check failed: output contains a forbidden source string ({})",
                        short_label(needle)
                    ),
                });
            }
            from = at.saturating_add(needle.len());
        }
    }
    Ok(())
}

/// The identifier-ish word (`[A-Za-z0-9_]+`) enclosing the byte range
/// `at..at + len` of `s`. `at` and `at + len` must be char boundaries.
fn enclosing_word(s: &str, at: usize, len: usize) -> String {
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let start = s[..at]
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_ident(*c))
        .last()
        .map(|(i, _)| i)
        .unwrap_or(at);
    let end = s[at + len..]
        .char_indices()
        .take_while(|(_, c)| is_ident(*c))
        .last()
        .map(|(i, c)| at + len + i + c.len_utf8())
        .unwrap_or(at + len);
    s[start..end].to_string()
}

/// A short, safe label for an error message (never echoes the source).
fn short_label(needle: &str) -> String {
    let word = needle.split_whitespace().next().unwrap_or("(empty)");
    format!("first word `{word}`")
}

/// The schema's own names: every type, element and attribute name.
fn identifiers(model: &XsdModel) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for name in model.types.keys() {
        out.insert(name.clone());
    }
    for decl in model.elements.values() {
        out.insert(decl.name.clone());
    }
    for attr in model.attributes.values() {
        out.insert(attr.name.clone());
    }
    out
}

/// Per-extractor entry point: writes `docs/vsdx-inventory.md` under
/// `out_dir` and returns the paths written.
pub fn run(in_dir: &Path, out_dir: &Path) -> ExtractResult<Vec<PathBuf>> {
    let schema_md = in_dir.join(SCHEMA_MD);
    if !schema_md.is_file() {
        return Err(ExtractError::Missing { path: schema_md });
    }
    let text = fs::read_to_string(&schema_md).map_err(|e| ExtractError::Io {
        path: schema_md.clone(),
        source: e.to_string(),
    })?;
    let xsd = extract_xsd(&text).ok_or_else(|| ExtractError::Parse {
        path: schema_md.clone(),
        message: "no fenced code block containing <xsd:schema".to_string(),
    })?;
    let doc = roxmltree::Document::parse(&xsd).map_err(|e| ExtractError::Parse {
        path: schema_md.clone(),
        message: format!("invalid XSD: {e}"),
    })?;
    let model = XsdModel::parse(&doc);
    if model.types.is_empty() || model.elements.is_empty() {
        return Err(ExtractError::Parse {
            path: schema_md.clone(),
            message: "no complexType/element declarations found in the XSD".to_string(),
        });
    }
    let pages_dir = schema_md.parent().unwrap_or(in_dir).to_path_buf();
    let docs = DocIndex::read(&pages_dir, &model)?;
    let inventory = Inventory::build(&model, docs);
    let rendered = inventory.render();
    clean_room_check(&rendered, &forbidden_strings(&doc), &identifiers(&model))?;

    let out_path = out_dir.join("docs").join("vsdx-inventory.md");
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).map_err(|e| ExtractError::Io {
            path: parent.to_path_buf(),
            source: e.to_string(),
        })?;
    }
    fs::write(&out_path, rendered).map_err(|e| ExtractError::Io {
        path: out_path.clone(),
        source: e.to_string(),
    })?;
    Ok(vec![out_path])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A hand-written miniature of the Visio XSD shape: root elements,
    /// extension bases, particle cardinalities, attributes,
    /// alternatives, and one deferred group of each kind.
    const FIXTURE_XSD: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<xsd:schema xmlns:xsd="http://www.w3.org/2001/XMLSchema">
    <xsd:annotation>
        <xsd:documentation>
            FIXTURE-ANNOTATION-MARKER: this line must never appear in the output.
            Copyright (c) Fixture Authors. All rights reserved.
        </xsd:documentation>
    </xsd:annotation>
    <xsd:element name="VisioDocument" type="VisioDocument_Type" />
    <xsd:element name="Pages" type="Pages_Type" />
    <xsd:element name="DataRecordSets" type="DataRecordSets_Type" />
    <xsd:element name="Validation" type="Validation_Type" />
    <xsd:complexType name="VisioDocument_Type">
        <xsd:sequence>
            <xsd:element name="DocumentProperties" type="DocumentProperties_Type" minOccurs="0" maxOccurs="1" />
            <xsd:element name="Comments" type="Comments_Type" minOccurs="0" maxOccurs="1" />
            <xsd:element name="Pages" type="Pages_Type" minOccurs="0" maxOccurs="1" />
        </xsd:sequence>
    </xsd:complexType>
    <xsd:complexType name="DocumentProperties_Type">
        <xsd:all>
            <xsd:element name="Title" type="Title_Type" minOccurs="0" maxOccurs="1" />
            <xsd:element name="CustomProps" type="CustomProps_Type" minOccurs="0" maxOccurs="1" />
        </xsd:all>
    </xsd:complexType>
    <xsd:complexType name="Title_Type">
        <xsd:simpleContent>
            <xsd:extension base="xsd:string" />
        </xsd:simpleContent>
    </xsd:complexType>
    <xsd:complexType name="CustomProps_Type">
        <xsd:sequence>
            <xsd:element name="CustomProp" type="CustomProp_Type" minOccurs="0" maxOccurs="unbounded" />
        </xsd:sequence>
    </xsd:complexType>
    <xsd:complexType name="CustomProp_Type">
        <xsd:simpleContent>
            <xsd:extension base="xsd:string">
                <xsd:attribute name="Name" type="xsd:string" />
            </xsd:extension>
        </xsd:simpleContent>
    </xsd:complexType>
    <xsd:complexType name="Comments_Type">
        <xsd:sequence>
            <xsd:element name="CommentEntry" type="CommentEntry_Type" minOccurs="0" maxOccurs="unbounded" />
        </xsd:sequence>
    </xsd:complexType>
    <xsd:complexType name="CommentEntry_Type">
        <xsd:attribute name="AuthorID" type="xsd:unsignedInt" use="required" />
    </xsd:complexType>
    <xsd:complexType name="Pages_Type">
        <xsd:sequence>
            <xsd:element name="Page" type="Page_Type" minOccurs="0" maxOccurs="unbounded" />
        </xsd:sequence>
    </xsd:complexType>
    <xsd:complexType name="Page_Type">
        <xsd:all>
            <xsd:element name="Shapes" type="Shapes_Type" minOccurs="0" maxOccurs="1" />
            <xsd:element name="PageSheet" type="PageSheet_Type" minOccurs="0" maxOccurs="1" />
            <xsd:element name="Rel" type="Rel_Type" minOccurs="1" maxOccurs="1" />
        </xsd:all>
        <xsd:attribute name="ID" type="xsd:unsignedInt" use="required" />
        <xsd:attribute name="Name" type="xsd:string" />
    </xsd:complexType>
    <xsd:complexType name="Shapes_Type">
        <xsd:sequence>
            <xsd:element name="Shape" type="Shape_Type" minOccurs="0" maxOccurs="unbounded" />
        </xsd:sequence>
    </xsd:complexType>
    <xsd:complexType name="Shape_Type">
        <xsd:complexContent>
            <xsd:extension base="ShapeSheet_Type">
                <xsd:attribute name="ID" type="xsd:unsignedInt" use="required" />
            </xsd:extension>
        </xsd:complexContent>
    </xsd:complexType>
    <xsd:complexType name="PageSheet_Type">
        <xsd:complexContent>
            <xsd:extension base="ShapeSheet_Type">
                <xsd:attribute name="UniqueID" type="xsd:string" />
            </xsd:extension>
        </xsd:complexContent>
    </xsd:complexType>
    <xsd:complexType name="ShapeSheet_Type">
        <xsd:choice minOccurs="0" maxOccurs="unbounded">
            <xsd:element name="Cell" type="Cell_Type" minOccurs="0" maxOccurs="unbounded">
                <xsd:alternative test="@N = 'PinX'" type="PinX_Type" />
            </xsd:element>
            <xsd:element name="Section" type="Section_Type" minOccurs="0" maxOccurs="unbounded">
                <xsd:alternative test="@N = 'User'" type="User_Type" />
            </xsd:element>
        </xsd:choice>
    </xsd:complexType>
    <xsd:complexType name="Cell_Type">
        <xsd:sequence>
            <xsd:element name="RefBy" type="RefBy_Type" minOccurs="0" maxOccurs="unbounded" />
        </xsd:sequence>
        <xsd:attribute name="N" type="xsd:string" use="required" />
        <xsd:attribute name="V" type="xsd:string" />
    </xsd:complexType>
    <xsd:complexType name="RefBy_Type">
        <xsd:attribute name="T" type="xsd:string" use="required" />
        <xsd:attribute name="ID" type="xsd:unsignedInt" use="required" />
    </xsd:complexType>
    <xsd:complexType name="PinX_Type">
        <xsd:complexContent>
            <xsd:extension base="Cell_Type">
                <xsd:assert test="@N = 'PinX'" />
            </xsd:extension>
        </xsd:complexContent>
    </xsd:complexType>
    <xsd:complexType name="Section_Type">
        <xsd:attribute name="N" type="xsd:string" use="required" />
        <xsd:attribute name="IX" type="xsd:unsignedInt" />
    </xsd:complexType>
    <xsd:complexType name="User_Type">
        <xsd:complexContent>
            <xsd:extension base="Section_Type">
                <xsd:sequence minOccurs="0" maxOccurs="unbounded">
                    <xsd:element name="Row" type="UserRow_Type" />
                </xsd:sequence>
            </xsd:extension>
        </xsd:complexContent>
    </xsd:complexType>
    <xsd:complexType name="UserRow_Type">
        <xsd:all>
            <xsd:element name="Cell" type="Cell_Type" minOccurs="0" maxOccurs="unbounded" />
        </xsd:all>
        <xsd:attribute name="N" type="xsd:string" />
    </xsd:complexType>
    <xsd:complexType name="Validation_Type">
        <xsd:sequence>
            <xsd:element name="Issue" type="Issue_Type" minOccurs="0" maxOccurs="unbounded" />
        </xsd:sequence>
    </xsd:complexType>
    <xsd:complexType name="Issue_Type">
        <xsd:attribute name="ID" type="xsd:unsignedInt" use="required" />
    </xsd:complexType>
    <xsd:complexType name="DataRecordSets_Type">
        <xsd:sequence>
            <xsd:element name="DataRecordSet" type="DataRecordSet_Type" minOccurs="1" maxOccurs="unbounded" />
        </xsd:sequence>
        <xsd:attribute name="NextID" type="xsd:unsignedInt" use="required" />
    </xsd:complexType>
    <xsd:complexType name="DataRecordSet_Type">
        <xsd:sequence>
            <xsd:element name="Rel" type="Rel_Type" minOccurs="1" maxOccurs="1" />
            <xsd:element name="DataColumns" type="DataColumns_Type" minOccurs="0" maxOccurs="1" />
        </xsd:sequence>
    </xsd:complexType>
    <xsd:complexType name="Rel_Type" />
    <xsd:complexType name="DataColumns_Type">
        <xsd:sequence>
            <xsd:element name="DataColumn" type="DataColumn_Type" minOccurs="1" maxOccurs="unbounded" />
        </xsd:sequence>
        <xsd:attribute name="SortColumn" type="xsd:string" />
    </xsd:complexType>
    <xsd:complexType name="DataColumn_Type">
        <xsd:attribute name="Name" type="xsd:string" use="required" />
    </xsd:complexType>
</xsd:schema>
"#;

    const FIXTURE_PAGE_TYPE: &str = "# Cell_Type complexType (Visio XML)\n\n## Type information\n\n||Value |\n|:-----|:-----|\n|**Extension base** <br/> |xsd:anyType  <br/> |\n";

    const FIXTURE_PAGE_ELEMENT: &str = "# RefBy element (Cell_Type complexType) (Visio XML)\n\n## Element information\n\n||Value |\n|:-----|:-----|\n|**Element type** <br/> |[RefBy_Type](refby_type-complextypevisio-xml.md) <br/> |\n";

    const FIXTURE_PAGE_UNMAPPED: &str = "# Ghost_Type complexType (Visio XML)\n\nnothing here\n";

    /// Builds a fixture `reference/` tree in a unique temp directory.
    /// Every call gets its own directory: tests run in parallel and
    /// must not rewrite each other's fixture files.
    fn fixture_tree(name: &str) -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("ferrogram-p105-{}-{name}-{n}", std::process::id()));
        let visio = dir.join("ms-vsdx/office-developer-client-docs/docs/visio");
        fs::create_dir_all(&visio).expect("fixture dir");
        fs::write(
            visio.join("schema-mapvisio-xml.md"),
            format!("# Schema map (fixture)\n\n```XML\n{FIXTURE_XSD}\n```\n"),
        )
        .expect("fixture schema");
        fs::write(
            visio.join("cell_type-complextypevisio-xml.md"),
            FIXTURE_PAGE_TYPE,
        )
        .expect("fixture type page");
        fs::write(
            visio.join("refby-element-cell_type-complextypevisio-xml.md"),
            FIXTURE_PAGE_ELEMENT,
        )
        .expect("fixture element page");
        fs::write(
            visio.join("ghost_type-complextypevisio-xml.md"),
            FIXTURE_PAGE_UNMAPPED,
        )
        .expect("fixture unmapped page");
        dir
    }

    fn fixture_model() -> XsdModel {
        let doc = roxmltree::Document::parse(FIXTURE_XSD).expect("fixture parses");
        XsdModel::parse(&doc)
    }

    fn fixture_inventory() -> Inventory {
        let model = fixture_model();
        let docs = DocIndex::read(
            &fixture_tree("inv").join("ms-vsdx/office-developer-client-docs/docs/visio"),
            &model,
        )
        .expect("pages");
        Inventory::build(&model, docs)
    }

    fn element_status(inv: &Inventory, parent: &str, name: &str) -> Status {
        inv.elements
            .iter()
            .find(|e| e.parent == parent && e.name == name)
            .expect("element row")
            .status
    }

    fn type_status(inv: &Inventory, name: &str) -> Status {
        inv.types
            .iter()
            .find(|t| t.name == name)
            .expect("type row")
            .status
    }

    #[test]
    fn run_writes_inventory() {
        let root = fixture_tree("run");
        let out = root.join("out");
        let written = run(&root, &out).expect("run");
        assert_eq!(written.len(), 1);
        assert_eq!(written[0], out.join("docs/vsdx-inventory.md"));
        let text = fs::read_to_string(&written[0]).expect("output");
        assert!(text.contains("# MS-VSDX schema inventory (generated)"));
        assert!(text.contains("## 2. OPC parts"));
        assert!(text.contains("## 3. Complex types"));
        assert!(text.contains("## 4. Elements"));
        assert!(text.contains("## 5. Attributes"));
        assert!(text.contains("## 6. Counts"));
    }

    #[test]
    fn default_status_is_preserved_only() {
        let inv = fixture_inventory();
        // Elements outside the promotion list keep the XML verbatim.
        assert_eq!(
            element_status(&inv, "VisioDocument_Type", "Pages"),
            Status::Supported
        );
        assert_eq!(
            element_status(&inv, "Page_Type", "PageSheet"),
            Status::PreservedOnly
        );
        assert_eq!(
            element_status(&inv, "Shapes_Type", "Shape"),
            Status::Supported
        );
        assert_eq!(
            element_status(&inv, "(root)", "VisioDocument"),
            Status::PreservedOnly
        );
    }

    #[test]
    fn promotions_match_the_task_rule() {
        let inv = fixture_inventory();
        assert_eq!(
            element_status(&inv, "Pages_Type", "Page"),
            Status::Supported
        );
        assert_eq!(
            element_status(&inv, "Shapes_Type", "Shape"),
            Status::Supported
        );
        assert_eq!(
            element_status(&inv, "ShapeSheet_Type", "Cell"),
            Status::Supported
        );
        assert_eq!(
            element_status(&inv, "ShapeSheet_Type", "Section"),
            Status::Supported
        );
        // Document-properties subtree is supported.
        assert_eq!(
            element_status(&inv, "VisioDocument_Type", "DocumentProperties"),
            Status::Supported
        );
        assert_eq!(
            element_status(&inv, "DocumentProperties_Type", "Title"),
            Status::Supported
        );
        assert_eq!(
            element_status(&inv, "CustomProps_Type", "CustomProp"),
            Status::Supported
        );
    }

    #[test]
    fn deferred_groups_are_later_and_override_promotions() {
        let inv = fixture_inventory();
        // Comments / Validation / DataRecordset subtrees.
        assert_eq!(
            element_status(&inv, "VisioDocument_Type", "Comments"),
            Status::Later
        );
        assert_eq!(
            element_status(&inv, "Comments_Type", "CommentEntry"),
            Status::Later
        );
        assert_eq!(element_status(&inv, "(root)", "Validation"), Status::Later);
        assert_eq!(
            element_status(&inv, "Validation_Type", "Issue"),
            Status::Later
        );
        assert_eq!(
            element_status(&inv, "(root)", "DataRecordSets"),
            Status::Later
        );
        assert_eq!(
            element_status(&inv, "DataRecordSets_Type", "DataRecordSet"),
            Status::Later
        );
        assert_eq!(
            element_status(&inv, "DataRecordSet_Type", "DataColumns"),
            Status::Later
        );
        // The User section: `Row` and `Cell` are `later` here even
        // though the bare names are `supported` elsewhere.
        assert_eq!(element_status(&inv, "User_Type", "Row"), Status::Later);
        assert_eq!(element_status(&inv, "UserRow_Type", "Cell"), Status::Later);
    }

    #[test]
    fn shared_base_types_do_not_leak_into_deferred_groups() {
        let inv = fixture_inventory();
        // `Cell_Type` is shared with the shape surface: its `RefBy`
        // child must stay out of the User group.
        assert_eq!(
            element_status(&inv, "Cell_Type", "RefBy"),
            Status::PreservedOnly
        );
        assert_eq!(
            element_status(&inv, "ShapeSheet_Type", "Cell"),
            Status::Supported
        );
        // `Rel_Type` appears both in the recordsets group and outside.
        assert_eq!(
            element_status(&inv, "DataRecordSet_Type", "Rel"),
            Status::Later
        );
    }

    #[test]
    fn type_status_rules() {
        let inv = fixture_inventory();
        // Deferred group entry points.
        assert_eq!(type_status(&inv, "User_Type"), Status::Later);
        // All usages later.
        assert_eq!(type_status(&inv, "UserRow_Type"), Status::Later);
        assert_eq!(type_status(&inv, "CommentEntry_Type"), Status::Later);
        assert_eq!(type_status(&inv, "DataRecordSet_Type"), Status::Later);
        // Carried by a supported element.
        assert_eq!(type_status(&inv, "Cell_Type"), Status::Supported);
        assert_eq!(type_status(&inv, "ShapeSheet_Type"), Status::Supported);
        assert_eq!(type_status(&inv, "CustomProp_Type"), Status::Supported);
        // Mixed usage keeps the part we round-trip and model.
        assert_eq!(type_status(&inv, "Rel_Type"), Status::PreservedOnly);
        // Alternative-only cell type: preserved by default.
        assert_eq!(type_status(&inv, "PinX_Type"), Status::PreservedOnly);
    }

    #[test]
    fn attribute_rows_and_status() {
        let inv = fixture_inventory();
        let attr = |name: &str, declaring: &str| {
            inv.attributes
                .iter()
                .find(|a| a.name == name && a.declaring == declaring)
                .expect("attr row")
        };
        let n = attr("N", "Cell_Type");
        assert_eq!(n.parent_element, "Cell");
        assert_eq!(n.status, Status::Supported);
        assert_eq!(attr("AuthorID", "CommentEntry_Type").status, Status::Later);
        assert_eq!(attr("ID", "Issue_Type").status, Status::Later);
        assert_eq!(attr("NextID", "DataRecordSets_Type").status, Status::Later);
        assert_eq!(attr("Name", "CustomProp_Type").status, Status::Supported);
        assert_eq!(attr("N", "UserRow_Type").status, Status::Later);
        // Inherited usage: `PageSheet_Type` extends `ShapeSheet_Type`
        // but `PageSheet` is not one of the promoted names.
        let u = attr("UniqueID", "PageSheet_Type");
        assert_eq!(u.parent_element, "PageSheet");
        assert_eq!(u.status, Status::PreservedOnly);
    }

    #[test]
    fn cardinality_multiplies_particles() {
        let inv = fixture_inventory();
        let card = |parent: &str, name: &str| {
            inv.elements
                .iter()
                .find(|e| e.parent == parent && e.name == name)
                .expect("element row")
                .cardinality
                .clone()
        };
        // choice 0..unbounded wrapping 0..unbounded.
        assert_eq!(card("ShapeSheet_Type", "Cell"), "0..*");
        assert_eq!(card("Pages_Type", "Page"), "0..*");
        // sequence 0..unbounded wrapping 1..1.
        assert_eq!(card("User_Type", "Row"), "0..*");
        // plain 0..1 and 1..1.
        assert_eq!(card("VisioDocument_Type", "Pages"), "0..1");
        assert_eq!(card("DataRecordSet_Type", "Rel"), "1..1");
        assert_eq!(card("DataRecordSets_Type", "DataRecordSet"), "1..*");
    }

    #[test]
    fn counts_match_table_rows() {
        let inv = fixture_inventory();
        let rendered = inv.render();
        let counts = rendered
            .lines()
            .skip_while(|l| !l.starts_with("## 6. Counts"))
            .filter(|l| l.starts_with("| "))
            .skip(1)
            .collect::<Vec<_>>();
        assert_eq!(counts.len(), 4);
        assert_eq!(
            counts[0],
            format!(
                "| OPC parts | {} | {} | {} | {} |",
                PARTS.len(),
                part_status_counts()[1],
                part_status_counts()[2],
                part_status_counts()[3]
            )
        );
        assert_eq!(
            counts[1],
            format!(
                "| Complex types | {} | {} | {} | {} |",
                inv.types.len(),
                status_counts(inv.types.iter().map(|t| t.status))[1],
                status_counts(inv.types.iter().map(|t| t.status))[2],
                status_counts(inv.types.iter().map(|t| t.status))[3]
            )
        );
        assert_eq!(
            counts[2],
            format!(
                "| Elements | {} | {} | {} | {} |",
                inv.elements.len(),
                status_counts(inv.elements.iter().map(|e| e.status))[1],
                status_counts(inv.elements.iter().map(|e| e.status))[2],
                status_counts(inv.elements.iter().map(|e| e.status))[3]
            )
        );
        assert_eq!(
            counts[3],
            format!(
                "| Attributes | {} | {} | {} | {} |",
                inv.attributes.len(),
                status_counts(inv.attributes.iter().map(|a| a.status))[1],
                status_counts(inv.attributes.iter().map(|a| a.status))[2],
                status_counts(inv.attributes.iter().map(|a| a.status))[3]
            )
        );
    }

    #[test]
    fn output_is_sorted() {
        let inv = fixture_inventory();
        for pair in inv.types.windows(2) {
            assert!(pair[0].name <= pair[1].name);
        }
        for pair in inv.elements.windows(2) {
            assert!(
                (pair[0].parent.as_str(), pair[0].name.as_str())
                    <= (pair[1].parent.as_str(), pair[1].name.as_str())
            );
        }
        for pair in inv.attributes.windows(2) {
            assert!(
                (
                    pair[0].parent_element.as_str(),
                    pair[0].name.as_str(),
                    pair[0].declaring.as_str()
                ) <= (
                    pair[1].parent_element.as_str(),
                    pair[1].name.as_str(),
                    pair[1].declaring.as_str()
                )
            );
        }
    }

    #[test]
    fn determinism_two_runs_are_byte_equal() {
        let root = fixture_tree("det");
        let first = run(&root, &root.join("out1")).expect("run 1");
        let second = run(&root, &root.join("out2")).expect("run 2");
        let a = fs::read(&first[0]).expect("read 1");
        let b = fs::read(&second[0]).expect("read 2");
        assert_eq!(a, b);
    }

    #[test]
    fn clean_room_no_ms_text_in_output() {
        let root = fixture_tree("clean");
        let out = root.join("out");
        run(&root, &out).expect("run");
        let text = fs::read_to_string(out.join("docs/vsdx-inventory.md")).expect("output");
        for forbidden in [
            "<xsd:",
            "Copyright",
            "©",
            "FIXTURE-ANNOTATION-MARKER",
            "this line must never appear",
        ] {
            assert!(!text.contains(forbidden), "found {forbidden:?}");
        }
    }

    #[test]
    fn clean_room_check_rejects_annotation_text() {
        let doc = roxmltree::Document::parse(FIXTURE_XSD).expect("fixture parses");
        let model = XsdModel::parse(&doc);
        let forbidden = forbidden_strings(&doc);
        let ids = identifiers(&model);
        assert!(
            forbidden
                .iter()
                .any(|f| f.contains("FIXTURE-ANNOTATION-MARKER"))
        );
        assert!(clean_room_check("safe output", &forbidden, &ids).is_ok());
        let err =
            clean_room_check("oops FIXTURE-ANNOTATION-MARKER here", &forbidden, &ids).unwrap_err();
        assert!(err.to_string().contains("clean-room check failed"), "{err}");
    }

    #[test]
    fn clean_room_check_allows_schema_identifiers() {
        // The schema can name a cell type `Copyright_Type`; the name is
        // structural data, the copyright notice is not.
        let mut ids = BTreeSet::new();
        ids.insert("Copyright_Type".to_string());
        let forbidden = vec!["Copyright".to_string()];
        assert!(clean_room_check("row: Copyright_Type |", &forbidden, &ids).is_ok());
        assert!(clean_room_check("Copyright (c) X", &forbidden, &ids).is_err());
    }

    #[test]
    fn doc_pages_are_mapped_and_unmapped_listed() {
        let inv = fixture_inventory();
        let cell = inv
            .types
            .iter()
            .find(|t| t.name == "Cell_Type")
            .expect("Cell_Type");
        assert_eq!(cell.doc_pages, "cell_type-complextypevisio-xml.md");
        let refby = inv
            .types
            .iter()
            .find(|t| t.name == "RefBy_Type")
            .expect("RefBy_Type");
        assert_eq!(
            refby.doc_pages,
            "refby-element-cell_type-complextypevisio-xml.md"
        );
        assert_eq!(
            inv.unmapped_pages,
            vec!["ghost_type-complextypevisio-xml.md".to_string()]
        );
    }

    #[test]
    fn missing_input_is_a_clean_error() {
        let err = run(
            Path::new("/nonexistent/never/reference"),
            Path::new("/tmp/out"),
        )
        .unwrap_err();
        assert!(err.to_string().contains("missing input"), "{err}");
    }

    #[test]
    fn malformed_xsd_is_a_parse_error() {
        let root = fixture_tree("bad");
        let visio = root.join("ms-vsdx/office-developer-client-docs/docs/visio");
        fs::write(
            visio.join("schema-mapvisio-xml.md"),
            "# title\n\n```XML\n<xsd:schema><unclosed></xsd:schema>\n```\n",
        )
        .expect("write");
        let err = run(&root, &root.join("out")).unwrap_err();
        assert!(err.to_string().contains("invalid XSD"), "{err}");
    }

    #[test]
    fn missing_xml_block_is_a_parse_error() {
        let root = fixture_tree("noblock");
        let visio = root.join("ms-vsdx/office-developer-client-docs/docs/visio");
        fs::write(
            visio.join("schema-mapvisio-xml.md"),
            "# title\n\nno block\n",
        )
        .expect("write");
        let err = run(&root, &root.join("out")).unwrap_err();
        assert!(err.to_string().contains("no fenced code block"), "{err}");
    }

    #[test]
    fn empty_schema_is_a_parse_error() {
        let root = fixture_tree("empty");
        let visio = root.join("ms-vsdx/office-developer-client-docs/docs/visio");
        fs::write(
            visio.join("schema-mapvisio-xml.md"),
            "```XML\n<xsd:schema xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\"></xsd:schema>\n```\n",
        )
        .expect("write");
        let err = run(&root, &root.join("out")).unwrap_err();
        assert!(err.to_string().contains("no complexType"), "{err}");
    }

    #[test]
    fn page_kind_parsing() {
        assert_eq!(
            page_kind("# Actions_Type complexType (Visio XML)\n"),
            Some(PageKind::TypePage("Actions_Type".into()))
        );
        assert_eq!(
            page_kind(
                "# AttachedToolbars element (DocumentSettings_Type complexType) (Visio XML)\n"
            ),
            Some(PageKind::ElementPage {
                name: "AttachedToolbars".into(),
                parent: "DocumentSettings_Type".into(),
            })
        );
        assert_eq!(page_kind("# A page about nothing\n"), None);
    }

    #[test]
    fn status_labels() {
        assert_eq!(Status::Supported.as_str(), "supported");
        assert_eq!(Status::PreservedOnly.as_str(), "preserved-only");
        assert_eq!(Status::Later.as_str(), "later");
    }

    #[test]
    fn parts_are_sorted_and_have_a_catch_all() {
        let concrete: Vec<&str> = PARTS
            .iter()
            .filter(|p| p.path != "(any other part)")
            .map(|p| p.path)
            .collect();
        let mut sorted = concrete.clone();
        sorted.sort();
        assert_eq!(concrete, sorted);
        assert_eq!(PARTS.last().expect("last").path, "(any other part)");
    }

    /// The committed inventory must match a fresh extraction of the
    /// real source. Skipped when `reference/` is not present (CI).
    #[test]
    fn committed_inventory_matches_the_real_source() {
        let root = crate::util::root();
        let reference = root.join("reference");
        let schema = reference.join(SCHEMA_MD);
        if !schema.is_file() {
            return;
        }
        let out = std::env::temp_dir().join(format!("ferrogram-p105-real-{}", std::process::id()));
        let written = run(&reference, &out).expect("run on real source");
        let fresh = fs::read_to_string(&written[0]).expect("fresh output");
        // The strict three; `Copyright` may only appear as the schema's
        // own `Copyright_Type` identifier (checked by `run` itself).
        for forbidden in ["<xsd:", "©", "Copyright ("] {
            assert!(!fresh.contains(forbidden), "found {forbidden:?}");
        }
        let committed = root.join("docs/vsdx-inventory.md");
        if committed.is_file() {
            let committed = fs::read_to_string(&committed).expect("committed output");
            assert_eq!(
                fresh, committed,
                "docs/vsdx-inventory.md is stale; run `cargo xtask extract`"
            );
        }
    }
}
