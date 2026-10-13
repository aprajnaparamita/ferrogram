//! Reading parts into the model (task P2-06).
//!
//! [`to_model`] maps the parts `docs/vsdx-inventory.md` marks `supported`
//! into model entities; every other part stays in the [`Package`] untouched
//! (ADR 0012). Inheritance is never flattened: master and stylesheet
//! resolution happens later in the lookup (ADR 0013).
//!
//! # What maps where
//!
//! | Part | Model |
//! |---|---|
//! | `/visio/document.xml` | [`Document::kind`], the stylesheet, document properties and the root [`Preserved`] |
//! | `/docProps/core.xml`, `/docProps/app.xml` | [`Document::properties`] |
//! | `/visio/pages/pages.xml` | [`Document::pages`] |
//! | `/visio/pages/page{N}.xml` | the page's shapes (the shape arena), its `Connect` records and its [`Preserved`] |
//! | `/visio/masters/masters.xml` | [`Document::masters`] |
//! | `/visio/masters/master{N}.xml` | the master's top-level shapes (the shape arena) |
//! | everything else | untouched in the package |
//!
//! The main document part is found through the package root relationship
//! whose relationship type ends in `/document`, falling back to the part
//! whose content type names a main document part. That keeps packages without
//! `[Content_Types].xml` (two corpus files, whose declarations are inferred as
//! `application/xml`) readable, and [`Document::kind`] defaults to
//! [`DocumentKind::Drawing`] for those (ADR 0016).
//!
//! # Sheet elements
//!
//! A `Shape`, `PageSheet` or `StyleSheet` element carries its ShapeSheet
//! content as children: `Cell` elements (direct cells) and `Section` elements
//! (repeated-row sections, each with `Row` children). They map to
//! [`Sheet`](ferrogram_model::Sheet): a `Cell` becomes a
//! [`Cell`](ferrogram_model::Cell), a `Section` a
//! [`Section`](ferrogram_model::Section), a `Row` a [`Row`](ferrogram_model::Row)
//! and a `Cell` inside a row becomes that row's cell. A cell keeps the file's
//! spelling of the name (`N`), the formula (`F`), the cached value (`V`, `E`)
//! and the declared unit (`U`) separately (ADR 0011, and the table in
//! [`value`]). A cell or section with no `N` attribute cannot be named and is
//! kept verbatim instead.
//!
//! The cells a `Section` writes *outside* its rows (`Geometry` writes
//! `NoFill`, `NoLine`, `NoShow`, `NoSnap`, `NoQuickDrag` there) map to
//! [`Section::cells`](ferrogram_model::Section) (P2-06b decision 1), where
//! the lookup sees them before the row cells.
//!
//! # Preservation (ADR 0012)
//!
//! Every attribute a model field does not take goes to the entity's
//! [`Preserved::attributes`], and every child element no field takes goes to
//! its [`Preserved::children`], in document order. Namespace declarations
//! (`xmlns`, `xmlns:*`) are neither mapped nor preserved: they are not data
//! ([`diff`](ferrogram_model::diff) ignores them) and a write synthesizes
//! them. Where several source elements map to one entity (a page and its page
//! sheet, a page and its page-contents root), their unmapped content merges
//! into that one [`Preserved`] in depth-first document order.
//!
//! Unmapped content of a sheet element belongs to the entity that owns the
//! sheet: a shape's `Text` and `Data1` children land in
//! [`Shape::preserved`](ferrogram_model::Shape), a page sheet's `Trigger`
//! children in [`Page::preserved`](ferrogram_model::Page). The `Shape`
//! children of a `Shapes` wrapper map to the group's sub-shapes
//! ([`Shape::children`](ferrogram_model::Shape)); the wrapper's own attributes
//! follow the wrapper's owner.
//!
//! # Degrading, never panicking
//!
//! Malformed-but-recoverable input degrades (AGENTS.md section 6); a scalar
//! value the model must interpret but cannot parse is an
//! [`Error::InvalidPackage`] naming the part and the attribute or cell, never
//! a panic:
//!
//! - a missing or dangling part, relationship or `Rel` reference leaves the
//!   entity empty (`Master::shapes` is empty, a page has no shapes);
//! - a `Master` or `BackPage` reference that names nothing stays in
//!   `Preserved` instead of being resolved;
//! - a `Connect` element carrying anything the model does not hold (there is
//!   no preserved slot on `Connect`) is kept verbatim rather than modeled;
//! - a numeric or boolean value that does not parse is
//!   [`Error::InvalidPackage`] naming the part and the cell.

mod node;
mod sheet;
mod value;

use ferrogram_model::{
    CellName, Connect, Document, DocumentKind, Master, MasterId, Page, PageId, Preserved, Property,
    Shape, ShapeId, Style, StyleId, XmlNode,
};

use crate::Error;
use crate::package::{Limits, Package, PartData, PartName, TargetMode};
use crate::read::node::{attribute, elements, is_namespace_declaration};

/// The content type of the main drawing document part.
const CT_DRAWING_MAIN: &str = "application/vnd.ms-visio.drawing.main+xml";
/// The content type of the main stencil document part.
const CT_STENCIL_MAIN: &str = "application/vnd.ms-visio.stencil.main+xml";
/// The content type of the main template document part.
const CT_TEMPLATE_MAIN: &str = "application/vnd.ms-visio.template.main+xml";
/// The content type the package reader infers for an XML part when the
/// package has no `[Content_Types].xml` (task P2-05).
const CT_XML: &str = "application/xml";
/// The part name every package must name through its root relationships.
const MAIN_PART: &str = "/visio/document.xml";
/// The core document properties part.
const CORE_PROPS_PART: &str = "/docProps/core.xml";
/// The extended document properties part.
const APP_PROPS_PART: &str = "/docProps/app.xml";

/// Reads a whole document from the bytes of a file.
///
/// Convenience composition: [`crate::read_package`] followed by
/// [`to_model`]. The same [`Limits`] caps apply (ADR 0017).
pub fn read_document(bytes: &[u8], limits: &Limits) -> Result<Document, Error> {
    let package = crate::package::read_package(bytes, limits)?;
    to_model(&package)
}

/// Maps a package to a document.
///
/// Known parts (inventory status `supported`, `docs/vsdx-inventory.md`) are
/// mapped to model entities; everything else stays in the package untouched
/// (ADR 0012). Inheritance is not flattened: master and stylesheet resolution
/// happens later in the lookup (ADR 0013).
pub fn to_model(package: &Package) -> Result<Document, Error> {
    let main = main_part(package)?;
    let mut mapper = Mapper {
        package,
        document: Document::new(),
    };
    mapper.document.kind = document_kind(package, &main)?;
    mapper.read_properties_part(&PartName::new(CORE_PROPS_PART))?;
    mapper.read_properties_part(&PartName::new(APP_PROPS_PART))?;
    let root = xml_part(package, &main)?;
    mapper.read_visio_document(root)?;
    mapper.read_masters(&main)?;
    mapper.read_pages(&main)?;
    Ok(mapper.document)
}

/// The main document part of the package.
///
/// The package root relationship whose type ends in `/document` wins (the
/// content type alone cannot find the part of a package with no
/// `[Content_Types].xml`); a part whose content type names a main document
/// part is the fallback. A root relationship to a missing part is skipped.
fn main_part(package: &Package) -> Result<PartName, Error> {
    let root = PartName::new("/");
    for relationship in package.relationships_from(&root) {
        if relationship.target_mode != TargetMode::Internal {
            continue;
        }
        if rel_type_local(&relationship.rel_type) != "document" {
            continue;
        }
        let name = resolve_target(root.as_str(), &relationship.target);
        if package.part(&name).is_some() {
            return Ok(name);
        }
    }
    for part in &package.parts {
        if is_main_content_type(part.content_type.as_str()) {
            return Ok(part.name.clone());
        }
    }
    Err(Error::MissingPart(MAIN_PART.to_owned()))
}

/// Whether a content type names a main document part (ADR 0016).
fn is_main_content_type(content_type: &str) -> bool {
    matches!(
        content_type,
        CT_DRAWING_MAIN | CT_STENCIL_MAIN | CT_TEMPLATE_MAIN
    )
}

/// The document kind named by the main part's content type (ADR 0016).
///
/// An inferred `application/xml` content type (a package without
/// `[Content_Types].xml`) is a drawing; any other unknown content type is an
/// error.
fn document_kind(package: &Package, main: &PartName) -> Result<DocumentKind, Error> {
    let content_type = match package.part(main) {
        Some(part) => part.content_type.as_str(),
        None => return Err(Error::MissingPart(main.as_str().to_owned())),
    };
    match content_type {
        CT_DRAWING_MAIN | CT_XML => Ok(DocumentKind::Drawing),
        CT_STENCIL_MAIN => Ok(DocumentKind::Stencil),
        CT_TEMPLATE_MAIN => Ok(DocumentKind::Template),
        _ => Err(Error::InvalidPackage(
            "main part content type is not a Visio document".to_owned(),
        )),
    }
}

/// The root element of an XML part.
fn xml_part<'p>(package: &'p Package, name: &PartName) -> Result<&'p XmlNode, Error> {
    let part = package
        .part(name)
        .ok_or_else(|| Error::MissingPart(name.as_str().to_owned()))?;
    match &part.data {
        PartData::Xml(root) => Ok(root),
        PartData::Binary(_) => Err(Error::InvalidPackage(format!(
            "{}: part is not XML",
            name.as_str()
        ))),
    }
}

/// The last segment of a relationship type, e.g. `document` for the Visio
/// document relationship.
fn rel_type_local(rel_type: &str) -> &str {
    match rel_type.rsplit('/').next() {
        Some(local) => local,
        None => rel_type,
    }
}

/// The absolute part name a relationship `target` points at, resolved
/// against the directory of the part holding the relationship.
///
/// A target that is already absolute is used as written; `.` and `..`
/// segments are resolved. The result is not checked against the package.
fn resolve_target(from: &str, target: &str) -> PartName {
    if target.starts_with('/') {
        return PartName::new(target.to_owned());
    }
    let directory = from.rsplit_once('/').map_or("", |(dir, _)| dir);
    let mut path: Vec<&str> = Vec::new();
    for segment in directory.split('/').chain(target.split('/')) {
        match segment {
            "" | "." => {}
            ".." => {
                path.pop();
            }
            other => path.push(other),
        }
    }
    PartName::new(format!("/{}", path.join("/")))
}

/// An integer attribute value, or [`Error::InvalidPackage`] naming the part
/// and the attribute.
fn parse_attr_int(value: &str, part: &str, attribute: &str) -> Result<i32, Error> {
    parse_int(value).map_err(|()| {
        Error::InvalidPackage(format!("{part}: attribute `{attribute}`: not an integer"))
    })
}

/// A boolean attribute value (`0`/`1`, `false`/`true` in either case), or
/// [`Error::InvalidPackage`] naming the part and the attribute.
fn parse_attr_bool(value: &str, part: &str, attribute: &str) -> Result<bool, Error> {
    parse_bool(value).map_err(|()| {
        Error::InvalidPackage(format!("{part}: attribute `{attribute}`: not a boolean"))
    })
}

/// An integer, or `Err(())` when the text is not one.
fn parse_int(value: &str) -> Result<i32, ()> {
    value.trim().parse::<i32>().map_err(|_| ())
}

/// A boolean: `0`/`1` and `false`/`true` in either case.
fn parse_bool(value: &str) -> Result<bool, ()> {
    match value.trim() {
        "0" | "false" | "FALSE" => Ok(false),
        "1" | "true" | "TRUE" => Ok(true),
        _ => Err(()),
    }
}

/// The base style of a style: its own `Style` cell (ADR 0011).
///
/// The cell is consumed: it moves from the sheet to [`Style::base_style`]. A
/// `Style` cell without a string value is not a base style and stays a plain
/// cell.
fn take_base_style(style: &mut Style) {
    let position = style.sheet.cells.iter().position(|cell| {
        cell.name == CellName::from_static("Style")
            && matches!(cell.value, Some(ferrogram_model::CellValue::Str(_)))
    });
    if let Some(position) = position {
        let removed = style.sheet.cells.remove(position);
        if let Some(ferrogram_model::CellValue::Str(name)) = removed.value {
            style.base_style = Some(name);
        }
    }
}

/// The concatenated text of an element and its descendants.
fn text_content(node: &XmlNode) -> String {
    let mut text = String::new();
    let mut stack = vec![node];
    while let Some(node) = stack.pop() {
        for child in node.children.iter().rev() {
            match child {
                ferrogram_model::XmlChild::Text(value) => text.push_str(value),
                ferrogram_model::XmlChild::Element(element) => stack.push(element),
                _ => {}
            }
        }
    }
    text
}

/// The state of one read: the package being mapped and the document built.
struct Mapper<'p> {
    /// The package parts are read from (never modified).
    package: &'p Package,
    /// The document being built.
    document: Document,
}

impl Mapper<'_> {
    /// The target part of the relationship with the type `rel_local` from
    /// `source`, or `None` when there is no such relationship or its target
    /// part is missing (a dangling reference degrades).
    fn related(&self, source: &PartName, rel_local: &str) -> Option<PartName> {
        self.package
            .relationships_from(source)
            .filter(|relationship| {
                relationship.target_mode == TargetMode::Internal
                    && rel_type_local(&relationship.rel_type) == rel_local
            })
            .map(|relationship| resolve_target(source.as_str(), &relationship.target))
            .find(|name| self.package.part(name).is_some())
    }

    /// The part a `Rel` element points at, through the relationships of
    /// `source`, or `None` when the id or its target part is missing.
    fn rel_target(&self, source: &PartName, rel_id: &str) -> Option<PartName> {
        self.package
            .relationships_from(source)
            .filter(|relationship| {
                relationship.target_mode == TargetMode::Internal
                    && relationship.id.as_str() == rel_id
            })
            .map(|relationship| resolve_target(source.as_str(), &relationship.target))
            .find(|name| self.package.part(name).is_some())
    }

    /// Keeps the attributes no model field takes, skipping namespace
    /// declarations.
    fn keep_attributes(attributes: &[ferrogram_model::XmlAttribute], into: &mut Preserved) {
        for attribute in attributes.iter().filter(|a| !is_namespace_declaration(a)) {
            into.attributes.push(attribute.clone());
        }
    }

    /// Maps the document properties of one `docProps` part.
    ///
    /// A missing part is fine (a drawing may have none). A property element
    /// with element children (the `HeadingPairs` and `TitlesOfParts` of
    /// `docProps/app.xml`) has no [`Property`] shape and is kept verbatim.
    fn read_properties_part(&mut self, name: &PartName) -> Result<(), Error> {
        let Some(part) = self.package.part(name) else {
            return Ok(());
        };
        let PartData::Xml(root) = &part.data else {
            return Ok(());
        };
        Self::keep_attributes(&root.attributes, &mut self.document.preserved);
        for element in elements(root) {
            self.map_property(element);
        }
        Ok(())
    }

    /// Maps one property element: text content becomes a [`Property`] (the
    /// element's local name is its name), anything with child elements is
    /// kept verbatim in the document's preserved content.
    ///
    /// Attributes the model does not carry (the `xsi:type` of
    /// `dcterms:created` and `dcterms:modified`) go to the document's
    /// preserved attributes like any other unmapped attribute.
    fn map_property(&mut self, element: &XmlNode) {
        if elements(element).next().is_some() {
            self.document.preserved.children.push(element.clone());
            return;
        }
        let name = element.name.local.clone();
        self.document
            .properties
            .push(Property::new(name, text_content(element)));
        Self::keep_attributes(&element.attributes, &mut self.document.preserved);
    }

    /// Maps the main document part: the stylesheet, the `DocumentProperties`
    /// subtree and the root's own preserved content.
    fn read_visio_document(&mut self, root: &XmlNode) -> Result<(), Error> {
        Self::keep_attributes(&root.attributes, &mut self.document.preserved);
        for element in elements(root) {
            match element.name.local.as_str() {
                "StyleSheets" => self.read_styles(element)?,
                "DocumentProperties" => {
                    for property in elements(element) {
                        self.map_property(property);
                    }
                }
                _ => self.document.preserved.children.push(element.clone()),
            }
        }
        Ok(())
    }

    /// Maps the `StyleSheet` children of a `StyleSheets` element to the
    /// stylesheet.
    fn read_styles(&mut self, node: &XmlNode) -> Result<(), Error> {
        for element in elements(node) {
            if element.name.local != "StyleSheet" {
                self.document.preserved.children.push(element.clone());
                continue;
            }
            let index = self.document.stylesheet.styles.len();
            let mut style = Style::new(StyleId {
                index: u32::try_from(index).unwrap_or(u32::MAX),
            });
            for attribute in element
                .attributes
                .iter()
                .filter(|a| !is_namespace_declaration(a))
            {
                match attribute.name.local.as_str() {
                    "Name" => style.name = attribute.value.clone(),
                    "NameU" => style.name_u = attribute.value.clone(),
                    _ => style.preserved.attributes.push(attribute.clone()),
                }
            }
            sheet::map_sheet(
                element,
                &mut style.sheet,
                &mut style.preserved,
                MAIN_PART,
                &[],
            )?;
            take_base_style(&mut style);
            self.document.stylesheet.styles.push(style);
        }
        Ok(())
    }

    /// Maps the master list and the master contents parts.
    fn read_masters(&mut self, main: &PartName) -> Result<(), Error> {
        let index_name = match self.related(main, "masters") {
            Some(name) => name,
            None => PartName::new("/visio/masters/masters.xml"),
        };
        let Ok(index) = xml_part(self.package, &index_name) else {
            // A document without a master list has no masters.
            return Ok(());
        };
        let mut contents: Vec<(usize, Option<String>)> = Vec::new();
        for element in elements(index) {
            if element.name.local != "Master" {
                self.document.preserved.children.push(element.clone());
                continue;
            }
            let position = self.document.masters.len();
            let (master, rel) = map_master(element, position)?;
            self.document.masters.push(master);
            contents.push((position, rel));
        }
        for (position, rel) in contents {
            let Some(rel) = rel else { continue };
            let Some(target) = self.rel_target(&index_name, &rel) else {
                continue;
            };
            let Ok(root) = xml_part(self.package, &target) else {
                continue;
            };
            if root.name.local != "MasterContents" {
                // Not a master contents part (a cycle in the corpus points a
                // master at an index part): the master keeps no shape.
                continue;
            }
            let mut shapes = Vec::new();
            let mut preserved = Preserved::new();
            Self::keep_attributes(&root.attributes, &mut preserved);
            for element in elements(root) {
                if element.name.local == "Shapes" {
                    let (ids, extras) = self.map_shapes(element, target.as_str())?;
                    shapes.extend(ids);
                    preserved.attributes.extend(extras.attributes);
                    preserved.children.extend(extras.children);
                } else {
                    preserved.children.push(element.clone());
                }
            }
            if let Some(master) = self.document.masters.get_mut(position) {
                master.preserved.attributes.extend(preserved.attributes);
                master.preserved.children.extend(preserved.children);
                // Every top-level shape of the master part is kept, in
                // document order (P2-06b decision 2).
                master.shapes = shapes;
            }
        }
        Ok(())
    }

    /// Maps the page list and the page contents parts.
    fn read_pages(&mut self, main: &PartName) -> Result<(), Error> {
        let index_name = match self.related(main, "pages") {
            Some(name) => name,
            None => PartName::new("/visio/pages/pages.xml"),
        };
        let Ok(index) = xml_part(self.package, &index_name) else {
            // A stencil has no page list; a document without one has no
            // pages.
            return Ok(());
        };
        // Every page id first: `BackPage` resolves against the whole list.
        let ids: Vec<i32> = elements(index)
            .filter(|element| element.name.local == "Page")
            .filter_map(|element| attribute(element, "ID").and_then(|id| parse_int(id).ok()))
            .collect();
        let mut contents: Vec<(usize, Option<String>)> = Vec::new();
        for element in elements(index) {
            if element.name.local != "Page" {
                self.document.preserved.children.push(element.clone());
                continue;
            }
            let position = self.document.pages.len();
            let (page, rel) = map_page(element, position, &ids)?;
            self.document.pages.push(page);
            contents.push((position, rel));
        }
        for (position, rel) in contents {
            let Some(rel) = rel else { continue };
            let Some(target) = self.rel_target(&index_name, &rel) else {
                continue;
            };
            let Ok(root) = xml_part(self.package, &target) else {
                continue;
            };
            if root.name.local != "PageContents" {
                // Not a page contents part (a cycle in the corpus points a
                // page at an index part): the page keeps no shapes.
                continue;
            }
            self.read_page_contents(position, root, target.as_str())?;
        }
        Ok(())
    }

    /// Maps one page contents part into the page at `position`.
    fn read_page_contents(
        &mut self,
        position: usize,
        root: &XmlNode,
        part: &str,
    ) -> Result<(), Error> {
        let mut shapes = Vec::new();
        let mut connects = Vec::new();
        let mut preserved = Preserved::new();
        Self::keep_attributes(&root.attributes, &mut preserved);
        for element in elements(root) {
            match element.name.local.as_str() {
                "Shapes" => {
                    let (ids, extras) = self.map_shapes(element, part)?;
                    shapes.extend(ids);
                    preserved.attributes.extend(extras.attributes);
                    preserved.children.extend(extras.children);
                }
                "Connects" => map_connects(element, &mut connects, &mut preserved, part)?,
                _ => preserved.children.push(element.clone()),
            }
        }
        if let Some(page) = self.document.pages.get_mut(position) {
            page.shapes = shapes;
            page.connects = connects;
            page.preserved.attributes.extend(preserved.attributes);
            page.preserved.children.extend(preserved.children);
        }
        Ok(())
    }

    /// Maps the `Shape` children of a `Shapes` wrapper, and their sub-shapes,
    /// into the arena. Returns the top-level shape ids in document order and
    /// the wrapper's own unmapped attributes and children, which belong to
    /// the entity that owns the wrapper.
    ///
    /// The walk uses an explicit stack, so a cyclic or very deep tree cannot
    /// overflow the stack; every wrapper is walked once.
    fn map_shapes(
        &mut self,
        wrapper: &XmlNode,
        part: &str,
    ) -> Result<(Vec<ShapeId>, Preserved), Error> {
        let mut top = Vec::new();
        let mut extras = Preserved::new();
        Self::keep_attributes(&wrapper.attributes, &mut extras);
        let mut stack: Vec<(&XmlNode, Option<ShapeId>)> = vec![(wrapper, None)];
        while let Some((container, parent)) = stack.pop() {
            for element in elements(container) {
                if element.name.local != "Shape" {
                    // The wrapper's own unmapped content follows its owner.
                    match parent {
                        None => extras.children.push(element.clone()),
                        Some(group) => {
                            if let Some(group) = self.document.shapes.get_mut(group.index as usize)
                            {
                                group.preserved.children.push(element.clone());
                            }
                        }
                    }
                    continue;
                }
                let id = self.add_shape(element, parent, part)?;
                match parent {
                    None => top.push(id),
                    Some(group) => {
                        if let Some(group) = self.document.shapes.get_mut(group.index as usize) {
                            group.children.push(id);
                        }
                    }
                }
                for sub in elements(element).filter(|e| e.name.local == "Shapes") {
                    if let Some(shape) = self.document.shapes.get_mut(id.index as usize) {
                        Self::keep_attributes(&sub.attributes, &mut shape.preserved);
                    }
                    stack.push((sub, Some(id)));
                }
            }
        }
        Ok((top, extras))
    }

    /// Maps one `Shape` element into a new arena shape.
    fn add_shape(
        &mut self,
        node: &XmlNode,
        parent: Option<ShapeId>,
        part: &str,
    ) -> Result<ShapeId, Error> {
        // The `Master` reference is resolved before the attribute pass, so an
        // unknown reference stays in `Preserved` in document order.
        let master = match node::attribute_node(node, "Master") {
            None => None,
            Some(attribute) => {
                let visio_id = parse_attr_int(&attribute.value, part, "Master")?;
                self.master_id(visio_id)
            }
        };
        let mut shape = Shape::new(ShapeId { index: 0 }, 0);
        shape.parent = parent;
        for attribute in &node.attributes {
            if is_namespace_declaration(attribute) {
                continue;
            }
            match attribute.name.local.as_str() {
                "ID" => shape.visio_id = parse_attr_int(&attribute.value, part, "ID")?,
                "Name" => shape.name = attribute.value.clone(),
                "NameU" => shape.name_u = attribute.value.clone(),
                "LineStyle" => shape.line_style = Some(attribute.value.clone()),
                "FillStyle" => shape.fill_style = Some(attribute.value.clone()),
                "TextStyle" => shape.text_style = Some(attribute.value.clone()),
                "Master" => {
                    if master.is_none() {
                        shape.preserved.attributes.push(attribute.clone());
                    }
                }
                _ => shape.preserved.attributes.push(attribute.clone()),
            }
        }
        shape.master = master;
        sheet::map_sheet(
            node,
            &mut shape.sheet,
            &mut shape.preserved,
            part,
            &["Shapes"],
        )?;
        Ok(self.document.add_shape(shape))
    }

    /// The master list id of the master with this Visio `ID`.
    fn master_id(&self, visio_id: i32) -> Option<MasterId> {
        let index = self
            .document
            .masters
            .iter()
            .position(|master| master.visio_id == visio_id)?;
        Some(MasterId {
            index: u32::try_from(index).unwrap_or(u32::MAX),
        })
    }
}

/// Maps one `Master` element of the master list at list position `index`.
///
/// All children (`PageSheet`, `Icon`, `Rel`) are kept verbatim: a master has
/// no sheet field, and the `Rel` element is both used (to find the contents
/// part) and preserved. Returns the master and the `Rel` relationship id its
/// contents part hangs off, if any.
fn map_master(node: &XmlNode, index: usize) -> Result<(Master, Option<String>), Error> {
    let id = MasterId {
        index: u32::try_from(index).unwrap_or(u32::MAX),
    };
    let mut master = Master::new(id, 0);
    let mut rel = None;
    for attribute in &node.attributes {
        if is_namespace_declaration(attribute) {
            continue;
        }
        match attribute.name.local.as_str() {
            "ID" => master.visio_id = parse_attr_int(&attribute.value, MAIN_PART, "ID")?,
            "Name" => master.name = attribute.value.clone(),
            "NameU" => master.name_u = attribute.value.clone(),
            "UniqueID" => master.unique_id = Some(attribute.value.clone()),
            "Prompt" => master.prompt = Some(attribute.value.clone()),
            "Hidden" => master.hidden = parse_attr_bool(&attribute.value, MAIN_PART, "Hidden")?,
            _ => master.preserved.attributes.push(attribute.clone()),
        }
    }
    for element in elements(node) {
        if element.name.local == "Rel"
            && rel.is_none()
            && let Some(id) = attribute(element, "id")
        {
            rel = Some(id.to_owned());
        }
        master.preserved.children.push(element.clone());
    }
    Ok((master, rel))
}

/// Maps one `Page` element of the page list at list position `index`.
///
/// Returns the page and the `Rel` relationship id its contents part hangs
/// off, if any. `ids` is every page's `ID`, in list order, for `BackPage`.
fn map_page(node: &XmlNode, index: usize, ids: &[i32]) -> Result<(Page, Option<String>), Error> {
    let id = PageId {
        index: u32::try_from(index).unwrap_or(u32::MAX),
    };
    let mut page = Page::new(id, 0);
    let mut back_page = None;
    let mut rel = None;
    for attribute in &node.attributes {
        if is_namespace_declaration(attribute) {
            continue;
        }
        match attribute.name.local.as_str() {
            "ID" => page.visio_id = parse_attr_int(&attribute.value, MAIN_PART, "ID")?,
            "Name" => page.name = attribute.value.clone(),
            "NameU" => page.name_u = attribute.value.clone(),
            "BackPage" => back_page = Some(attribute.clone()),
            _ => page.preserved.attributes.push(attribute.clone()),
        }
    }
    // `BackPage` names a page by its `ID`; an unknown page keeps the
    // attribute verbatim instead.
    if let Some(attribute) = back_page {
        let wanted = parse_attr_int(&attribute.value, MAIN_PART, "BackPage")?;
        let resolved = ids
            .iter()
            .position(|seen| *seen == wanted)
            .map(|position| PageId {
                index: u32::try_from(position).unwrap_or(u32::MAX),
            });
        match resolved {
            Some(background) => page.background = Some(background),
            None => page.preserved.attributes.push(attribute),
        }
    }
    for element in elements(node) {
        match element.name.local.as_str() {
            "PageSheet" => {
                sheet::map_sheet(
                    element,
                    &mut page.sheet,
                    &mut page.preserved,
                    MAIN_PART,
                    &[],
                )?;
                Mapper::keep_attributes(&element.attributes, &mut page.preserved);
            }
            "Rel" => {
                if rel.is_none()
                    && let Some(id) = attribute(element, "id")
                {
                    rel = Some(id.to_owned());
                }
                page.preserved.children.push(element.clone());
            }
            _ => page.preserved.children.push(element.clone()),
        }
    }
    Ok((page, rel))
}

/// Maps the `Connect` children of a `Connects` element.
///
/// A `Connect` element carrying anything the model does not hold (there is
/// no preserved slot on `Connect`) is kept verbatim instead of being modeled.
fn map_connects(
    node: &XmlNode,
    connects: &mut Vec<Connect>,
    preserved: &mut Preserved,
    part: &str,
) -> Result<(), Error> {
    for element in elements(node) {
        if element.name.local != "Connect" {
            preserved.children.push(element.clone());
            continue;
        }
        let mut connect = Connect::new();
        let mut modeled = element.children.is_empty();
        for attribute in &element.attributes {
            if is_namespace_declaration(attribute) {
                continue;
            }
            match attribute.name.local.as_str() {
                "FromSheet" => {
                    connect.from_sheet = parse_attr_int(&attribute.value, part, "FromSheet")?
                }
                "FromCell" => connect.from_cell = attribute.value.clone(),
                "FromPart" => {
                    connect.from_part = parse_attr_int(&attribute.value, part, "FromPart")?
                }
                "ToSheet" => connect.to_sheet = parse_attr_int(&attribute.value, part, "ToSheet")?,
                "ToCell" => connect.to_cell = attribute.value.clone(),
                "ToPart" => connect.to_part = parse_attr_int(&attribute.value, part, "ToPart")?,
                // No slot on `Connect`: the whole element is kept verbatim.
                _ => modeled = false,
            }
        }
        if modeled {
            connects.push(connect);
        } else {
            preserved.children.push(element.clone());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use ferrogram_model::{CellLookup, QName, XmlAttribute, XmlChild};

    use super::*;
    use crate::package::{ContentType, Part, Relationship, RelationshipId};

    /// Builds a qualified name from `prefix:local` or `local`.
    fn qname(name: &str) -> QName {
        match name.split_once(':') {
            Some((prefix, local)) => QName {
                prefix: Some(prefix.to_owned()),
                uri: None,
                local: local.to_owned(),
            },
            None => QName::local(name),
        }
    }

    /// Builds an element from its name, attributes and child elements.
    fn element(name: &str, attrs: &[(&str, &str)], children: &[XmlNode]) -> XmlNode {
        XmlNode {
            name: qname(name),
            attributes: attrs
                .iter()
                .map(|(name, value)| XmlAttribute {
                    name: qname(name),
                    value: (*value).to_owned(),
                })
                .collect(),
            children: children.iter().cloned().map(XmlChild::Element).collect(),
            self_closing: false,
        }
    }

    /// Builds an element with one text child.
    fn text_element(name: &str, attrs: &[(&str, &str)], text: &str) -> XmlNode {
        let mut node = element(name, attrs, &[]);
        node.children.push(XmlChild::Text(text.to_owned()));
        node
    }

    /// Builds a package from (part name, root element) pairs and
    /// relationships (source, id, target).
    fn package(parts: &[(&str, XmlNode)], relationships: &[(&str, &str, &str)]) -> Package {
        let mut package = Package::new();
        for (name, root) in parts {
            package.parts.push(Part {
                name: PartName::new(*name),
                content_type: ContentType::new(CT_XML),
                data: PartData::Xml(root.clone()),
            });
        }
        for (source, id, target) in relationships {
            let kind = match target.rsplit('/').next().unwrap_or(target) {
                "document.xml" => "document",
                "pages.xml" => "pages",
                "masters.xml" => "masters",
                "page1.xml" => "page",
                "master1.xml" => "master",
                _ => "other",
            };
            package.relationships.push(Relationship {
                source: PartName::new(*source),
                id: RelationshipId::new(*id),
                rel_type: format!("http://schemas.microsoft.com/visio/2010/relationships/{kind}"),
                target: (*target).to_owned(),
                target_mode: TargetMode::Internal,
                preserved: Preserved::new(),
            });
        }
        package
    }

    /// A minimal drawing: one page with one rectangle, one master.
    fn drawing() -> Package {
        let shape = element(
            "Shape",
            &[("ID", "1"), ("Type", "Shape"), ("LineStyle", "0")],
            &[
                element("Cell", &[("N", "PinX"), ("V", "2")], &[]),
                element("Cell", &[("N", "PinY"), ("U", "MM"), ("V", "2")], &[]),
                element("Text", &[], &[]),
            ],
        );
        let page_contents = element(
            "PageContents",
            &[("xml:space", "preserve")],
            &[element("Shapes", &[], &[shape])],
        );
        let page_sheet = element(
            "PageSheet",
            &[("LineStyle", "0")],
            &[
                element("Cell", &[("N", "PageWidth"), ("V", "8.5")], &[]),
                element("Trigger", &[("N", "RecalcColor")], &[]),
            ],
        );
        let pages = element(
            "Pages",
            &[("xml:space", "preserve")],
            &[element(
                "Page",
                &[("ID", "0"), ("NameU", "Page-1"), ("Name", "Page-1")],
                &[page_sheet, element("Rel", &[("r:id", "rId1")], &[])],
            )],
        );
        let master_contents = element(
            "MasterContents",
            &[],
            &[element(
                "Shapes",
                &[],
                &[element("Shape", &[("ID", "1")], &[])],
            )],
        );
        let masters = element(
            "Masters",
            &[],
            &[element(
                "Master",
                &[
                    ("ID", "2"),
                    ("NameU", "Box"),
                    ("Name", "Box"),
                    ("Hidden", "0"),
                ],
                &[
                    element("Rel", &[("r:id", "rId1")], &[]),
                    element("Icon", &[], &[]),
                ],
            )],
        );
        let styles = element(
            "StyleSheets",
            &[],
            &[element(
                "StyleSheet",
                &[("ID", "0"), ("NameU", "No Style"), ("Name", "No Style")],
                &[
                    element("Cell", &[("N", "LineWeight"), ("V", "0.01")], &[]),
                    element(
                        "Section",
                        &[("N", "Geometry")],
                        &[
                            element("Cell", &[("N", "NoFill"), ("V", "0")], &[]),
                            element(
                                "Row",
                                &[("T", "RelMoveTo"), ("IX", "1")],
                                &[element("Cell", &[("N", "X"), ("V", "0")], &[])],
                            ),
                        ],
                    ),
                ],
            )],
        );
        let document = element(
            "VisioDocument",
            &[("xml:space", "preserve")],
            &[
                element("DocumentSettings", &[("TopPage", "0")], &[]),
                styles,
            ],
        );
        let core = element(
            "coreProperties",
            &[],
            &[
                text_element("title", &[], "Hello"),
                text_element("created", &[("xsi:type", "dcterms:W3CDTF")], "2015-05-12"),
                element("HeadingPairs", &[], &[text_element("Pages", &[], "1")]),
            ],
        );
        package(
            &[
                ("/visio/document.xml", document),
                ("/visio/pages/pages.xml", pages),
                ("/visio/pages/page1.xml", page_contents),
                ("/visio/masters/masters.xml", masters),
                ("/visio/masters/master1.xml", master_contents),
                ("/docProps/core.xml", core),
            ],
            &[
                ("/", "rId1", "visio/document.xml"),
                ("/visio/document.xml", "rId1", "pages/pages.xml"),
                ("/visio/document.xml", "rId2", "masters/masters.xml"),
                ("/visio/pages/pages.xml", "rId1", "page1.xml"),
                ("/visio/masters/masters.xml", "rId1", "master1.xml"),
            ],
        )
    }

    #[test]
    fn resolve_target_resolves_relative_and_absolute_targets() {
        assert_eq!(
            resolve_target("/visio/pages/pages.xml", "page1.xml").as_str(),
            "/visio/pages/page1.xml"
        );
        assert_eq!(
            resolve_target("/", "visio/document.xml").as_str(),
            "/visio/document.xml"
        );
        assert_eq!(
            resolve_target("/visio/pages/pages.xml", "../masters/masters.xml").as_str(),
            "/visio/masters/masters.xml"
        );
        assert_eq!(
            resolve_target("/visio/pages/pages.xml", "./page1.xml").as_str(),
            "/visio/pages/page1.xml"
        );
        assert_eq!(
            resolve_target("/visio/pages/pages.xml", "/docProps/core.xml").as_str(),
            "/docProps/core.xml"
        );
        assert_eq!(resolve_target("/", "../../x.xml").as_str(), "/x.xml");
    }

    #[test]
    fn rel_type_local_is_the_last_segment() {
        assert_eq!(
            rel_type_local("http://schemas.microsoft.com/visio/2010/relationships/document"),
            "document"
        );
        assert_eq!(rel_type_local("document"), "document");
    }

    #[test]
    fn integers_and_booleans_parse_or_fail() {
        assert_eq!(parse_int(" 42 "), Ok(42));
        assert_eq!(parse_int("x"), Err(()));
        assert_eq!(parse_bool("1"), Ok(true));
        assert_eq!(parse_bool("false"), Ok(false));
        assert_eq!(parse_bool("yes"), Err(()));
        assert!(matches!(
            parse_attr_int("x", "/p", "ID"),
            Err(Error::InvalidPackage(_))
        ));
        assert!(matches!(
            parse_attr_bool("x", "/p", "Hidden"),
            Err(Error::InvalidPackage(_))
        ));
    }

    #[test]
    fn text_content_joins_descendant_text() {
        assert_eq!(text_content(&text_element("Title", &[], "Hello")), "Hello");
        assert_eq!(text_content(&element("Empty", &[], &[])), "");
    }

    #[test]
    fn main_part_prefers_the_root_relationship() {
        let package = drawing();
        assert_eq!(
            main_part(&package).expect("main part").as_str(),
            "/visio/document.xml"
        );
    }

    #[test]
    fn main_part_skips_a_dangling_root_relationship() {
        let mut package = package(
            &[("/visio/document.xml", element("VisioDocument", &[], &[]))],
            &[],
        );
        package.relationships.push(Relationship {
            source: PartName::new("/"),
            id: RelationshipId::new("rId1"),
            rel_type: "http://schemas.microsoft.com/visio/2010/relationships/document".to_owned(),
            target: "visio/gone.xml".to_owned(),
            target_mode: TargetMode::Internal,
            preserved: Preserved::new(),
        });
        if let Some(part) = package.parts.first_mut() {
            part.content_type = ContentType::new(CT_DRAWING_MAIN);
        }
        assert_eq!(
            main_part(&package).expect("main part").as_str(),
            "/visio/document.xml"
        );
    }

    #[test]
    fn a_package_without_a_main_part_is_an_error() {
        assert_eq!(
            main_part(&package(&[], &[])),
            Err(Error::MissingPart(MAIN_PART.to_owned()))
        );
    }

    #[test]
    fn document_kind_follows_the_main_content_type() {
        for (content_type, want) in [
            (CT_DRAWING_MAIN, DocumentKind::Drawing),
            (CT_STENCIL_MAIN, DocumentKind::Stencil),
            (CT_TEMPLATE_MAIN, DocumentKind::Template),
            (CT_XML, DocumentKind::Drawing),
        ] {
            let mut package = drawing();
            if let Some(part) = package.parts.first_mut() {
                part.content_type = ContentType::new(content_type);
            }
            assert_eq!(
                document_kind(&package, &PartName::new("/visio/document.xml")),
                Ok(want),
                "content type {content_type}"
            );
        }
    }

    #[test]
    fn an_unknown_main_content_type_is_an_error() {
        let mut package = drawing();
        if let Some(part) = package.parts.first_mut() {
            part.content_type = ContentType::new("text/xml");
        }
        assert!(matches!(
            document_kind(&package, &PartName::new("/visio/document.xml")),
            Err(Error::InvalidPackage(_))
        ));
    }

    #[test]
    fn xml_part_reports_missing_and_non_xml_parts() {
        let mut package = drawing();
        assert!(xml_part(&package, &PartName::new("/visio/document.xml")).is_ok());
        assert_eq!(
            xml_part(&package, &PartName::new("/nope.xml")),
            Err(Error::MissingPart("/nope.xml".to_owned()))
        );
        if let Some(part) = package.parts.first_mut() {
            part.data = PartData::Binary(vec![0]);
        }
        assert!(matches!(
            xml_part(&package, &PartName::new("/visio/document.xml")),
            Err(Error::InvalidPackage(_))
        ));
    }

    #[test]
    fn to_model_maps_pages_shapes_and_masters() {
        let document = to_model(&drawing()).expect("document");
        assert_eq!(document.kind, DocumentKind::Drawing);
        assert_eq!(document.pages.len(), 1);
        assert_eq!(document.masters.len(), 1);

        let page = &document.pages[0];
        assert_eq!(page.visio_id, 0);
        assert_eq!(page.name, "Page-1");
        assert_eq!(page.name_u, "Page-1");
        assert_eq!(page.shapes.len(), 1);
        assert_eq!(page.sheet.cells.len(), 1);

        let shape = document.shape(page.shapes[0]).expect("shape");
        assert_eq!(shape.visio_id, 1);
        assert_eq!(shape.sheet.cells.len(), 2);
        assert_eq!(
            shape.sheet.cells.first().map(|cell| cell.name.as_str()),
            Some("PinX")
        );

        let master = &document.masters[0];
        assert_eq!(master.visio_id, 2);
        assert_eq!(master.name, "Box");
        assert!(!master.hidden);
        assert_eq!(master.shapes.len(), 1);
    }

    #[test]
    fn a_master_keeps_every_top_level_shape() {
        let mut package = drawing();
        // Append a second top-level shape to the master's `Shapes` wrapper.
        if let Some(part) = package
            .parts
            .iter_mut()
            .find(|part| part.name.as_str() == "/visio/masters/master1.xml")
            && let PartData::Xml(root) = &mut part.data
            && let Some(XmlChild::Element(shapes)) = root.children.first_mut()
        {
            shapes
                .children
                .push(XmlChild::Element(element("Shape", &[("ID", "2")], &[])));
        }
        let document = to_model(&package).expect("document");
        // Both shapes are kept, in document order (P2-06b decision 2).
        let master = &document.masters[0];
        assert_eq!(master.shapes.len(), 2);
        let first = document.shape(master.shapes[0]).expect("shape");
        let second = document.shape(master.shapes[1]).expect("shape");
        assert_eq!(first.visio_id, 1);
        assert_eq!(second.visio_id, 2);
    }

    #[test]
    fn to_model_keeps_unknown_content_preserved() {
        let document = to_model(&drawing()).expect("document");
        // `DocumentSettings` is unmapped; `xml:space` on `VisioDocument` too.
        let root_children: Vec<&str> = document
            .preserved
            .children
            .iter()
            .map(|child| child.name.local.as_str())
            .collect();
        assert!(root_children.contains(&"DocumentSettings"));
        // Unmapped attributes: the `xsi:type` of `dcterms:created` and
        // `xml:space` on `VisioDocument`.
        let root_attrs: Vec<&str> = document
            .preserved
            .attributes
            .iter()
            .map(|attribute| attribute.name.local.as_str())
            .collect();
        assert_eq!(root_attrs, ["type", "space"]);

        let page = &document.pages[0];
        // The `Rel` element and the page sheet's `Trigger` are unmapped.
        let names: Vec<&str> = page
            .preserved
            .children
            .iter()
            .map(|child| child.name.local.as_str())
            .collect();
        assert_eq!(names, ["Trigger", "Rel"]);
        // The master's `Rel` and `Icon` children are all preserved.
        assert_eq!(document.masters[0].preserved.children.len(), 2);
    }

    #[test]
    fn to_model_maps_document_properties() {
        let document = to_model(&drawing()).expect("document");
        assert_eq!(document.properties.len(), 2);
        assert_eq!(document.properties[0].name, "title");
        assert_eq!(document.properties[0].value, "Hello");
        assert_eq!(document.properties[1].name, "created");
        // `HeadingPairs` has child elements and stays preserved.
        let names: Vec<&str> = document
            .preserved
            .children
            .iter()
            .map(|child| child.name.local.as_str())
            .collect();
        assert!(names.contains(&"HeadingPairs"));
    }

    #[test]
    fn to_model_maps_section_level_cells() {
        let document = to_model(&drawing()).expect("document");
        let style = document
            .style(ferrogram_model::StyleId { index: 0 })
            .expect("style");
        let section = style.sheet.sections.first().expect("section");
        assert_eq!(section.name.as_str(), "Geometry");
        assert_eq!(section.rows.len(), 1);
        // `NoFill` is a section-level cell (P2-06b decision 1).
        assert_eq!(section.cells.len(), 1);
        assert_eq!(section.cells[0].name.as_str(), "NoFill");
        assert!(section.preserved.children.is_empty());
    }

    #[test]
    fn take_base_style_consumes_the_style_cell() {
        let mut style = Style::new(StyleId { index: 0 });
        style
            .sheet
            .cells
            .push(ferrogram_model::Cell::new(CellName::from_static(
                "LineWeight",
            )));
        let mut cell = ferrogram_model::Cell::new(CellName::from_static("Style"));
        cell.value = Some(ferrogram_model::CellValue::Str("Normal".to_owned()));
        style.sheet.cells.push(cell);
        take_base_style(&mut style);
        assert_eq!(style.base_style.as_deref(), Some("Normal"));
        assert_eq!(style.sheet.cells.len(), 1);
    }

    #[test]
    fn take_base_style_keeps_a_non_string_style_cell() {
        let mut style = Style::new(StyleId { index: 0 });
        let mut cell = ferrogram_model::Cell::new(CellName::from_static("Style"));
        cell.value = Some(ferrogram_model::CellValue::Number(3.0));
        style.sheet.cells.push(cell);
        take_base_style(&mut style);
        assert_eq!(style.base_style, None);
        assert_eq!(style.sheet.cells.len(), 1);
    }

    /// The first `Shape` element of one part, for test surgery.
    fn first_shape_mut<'n>(package: &'n mut Package, part_name: &str) -> Option<&'n mut XmlNode> {
        let part = package
            .parts
            .iter_mut()
            .find(|part| part.name.as_str() == part_name)?;
        let PartData::Xml(root) = &mut part.data else {
            return None;
        };
        let shapes = match root.children.first_mut() {
            Some(XmlChild::Element(shapes)) => shapes,
            _ => return None,
        };
        match shapes.children.first_mut() {
            Some(XmlChild::Element(shape)) => Some(shape),
            _ => None,
        }
    }

    #[test]
    fn a_dangling_master_reference_stays_preserved() {
        let mut package = drawing();
        // Point the page's shape at a master that does not exist.
        if let Some(shape) = first_shape_mut(&mut package, "/visio/pages/page1.xml") {
            shape.attributes.push(XmlAttribute {
                name: QName::local("Master"),
                value: "99".to_owned(),
            });
        }
        let document = to_model(&package).expect("document");
        let shape = document.shape(document.pages[0].shapes[0]).expect("shape");
        assert_eq!(shape.master, None);
        assert!(
            shape
                .preserved
                .attributes
                .iter()
                .any(|attribute| attribute.name.local == "Master")
        );
    }

    #[test]
    fn a_bad_attribute_value_is_an_error_naming_the_part() {
        let mut package = drawing();
        if let Some(shape) = first_shape_mut(&mut package, "/visio/pages/page1.xml") {
            shape.attributes[0].value = "not-an-int".to_owned();
        }
        assert!(matches!(to_model(&package), Err(Error::InvalidPackage(_))));
    }

    #[test]
    fn a_page_without_contents_has_no_shapes() {
        let mut package = drawing();
        package.relationships.clear();
        package.relationships.push(Relationship {
            source: PartName::new("/"),
            id: RelationshipId::new("rId1"),
            rel_type: "http://schemas.microsoft.com/visio/2010/relationships/document".to_owned(),
            target: "visio/document.xml".to_owned(),
            target_mode: TargetMode::Internal,
            preserved: Preserved::new(),
        });
        let document = to_model(&package).expect("document");
        assert_eq!(document.pages.len(), 1);
        assert!(document.pages[0].shapes.is_empty());
        assert_eq!(document.masters.len(), 1);
        assert!(document.masters[0].shapes.is_empty());
    }

    /// Pushes a `Width` cell onto the master's content shape and turns the
    /// page's shape into an instance of that master.
    fn with_master_instance(package: &mut Package) {
        if let Some(shape) = first_shape_mut(package, "/visio/masters/master1.xml") {
            shape.children.push(XmlChild::Element(element(
                "Cell",
                &[("N", "Width"), ("V", "2")],
                &[],
            )));
        }
        if let Some(shape) = first_shape_mut(package, "/visio/pages/page1.xml") {
            shape.attributes.push(XmlAttribute {
                name: QName::local("Master"),
                value: "2".to_owned(),
            });
        }
    }

    #[test]
    fn master_and_style_cells_are_not_flattened_into_instances() {
        let mut package = drawing();
        with_master_instance(&mut package);
        let document = to_model(&package).expect("document");

        let instance = document
            .shape(document.pages[0].shapes[0])
            .expect("instance");
        assert_eq!(instance.master, Some(MasterId { index: 0 }));
        // The instance keeps only the cells it was written with: the master's
        // `Width` and the style's `LineWeight` are not copied in.
        let names: Vec<&str> = instance
            .sheet
            .cells
            .iter()
            .map(|cell| cell.name.as_str())
            .collect();
        assert_eq!(names, ["PinX", "PinY"]);

        // The lookup still sees the master and the style (ADR 0013).
        let resolved = document
            .resolve(document.pages[0].shapes[0], &CellName::from_static("Width"))
            .expect("resolve");
        assert_eq!(
            resolved.source,
            ferrogram_model::ValueSource::Master(MasterId { index: 0 })
        );
        assert_eq!(
            resolved.value,
            Some(&ferrogram_model::CellValue::Number(2.0))
        );
    }
}
