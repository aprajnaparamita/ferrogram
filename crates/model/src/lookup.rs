//! The inheritance lookup API (ADR 0013).
//!
//! Master/instance and stylesheet inheritance are resolved through this
//! lookup, never flattened on load. A lookup returns where the value came
//! from, so a caller can show provenance ("from master", "from style
//! `Normal`"). A cyclic style chain is cut and reported in `via`, never
//! looped on.
//!
//! Resolution order (ADR 0013): the shape's own cell (**local**), the
//! master's corresponding cell (**master**, with the sub-shape rule), the
//! style chain selected by the cell's section (**style**), the ShapeSheet
//! table's `default_value` (**section default**) and the default of the
//! cell's type (**hard default**). Every consulted source appears in
//! [`Resolved::via`]; the entry that defines the value is last.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use ferrogram_geom::Unit;

use crate::cell::{Cell, CellValue, Color};
use crate::document::Document;
use crate::generated::{CELLS, CellDef};
use crate::id::{MasterId, ShapeId, StyleId};
use crate::name::CellName;
use crate::sheet::Sheet;

/// Where a resolved cell value came from (ADR 0013).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ValueSource {
    /// The shape's own cell.
    Local,
    /// The cell of the master shape this shape is an instance of.
    Master(MasterId),
    /// A cell of a stylesheet row in the style chain.
    Style(StyleId),
    /// The default value of the cell in the ShapeSheet table
    /// (`ui-spec/shapesheet.toml`, ADR 0008).
    SectionDefault,
    /// The hard default of the cell's type, when nothing else defines it.
    HardDefault,
}

/// The result of a lookup: the resolved value and its provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved<'a> {
    /// The resolved cached value, or `None` when no source defines the cell.
    pub value: Option<&'a CellValue>,
    /// Where the value came from.
    pub source: ValueSource,
    /// The formula of the source that defines the cell, if any.
    pub formula: Option<&'a str>,
    /// Every source consulted, in resolution order (ADR 0013). The last
    /// entry is `source`. A cyclic style chain appears here as a repeated
    /// [`ValueSource`] and the chain stops there.
    pub via: Vec<ValueSource>,
}

impl<'a> Resolved<'a> {
    /// Creates a resolution with no value, no formula and no chain.
    pub fn empty(source: ValueSource) -> Self {
        Self {
            value: None,
            source,
            formula: None,
            via: Vec::new(),
        }
    }
}

/// Resolves cell values through master/instance and stylesheet inheritance
/// (ADR 0013).
pub trait CellLookup {
    /// Resolves `cell` on `shape`, following the resolution order of
    /// ADR 0013: the shape's own cell, the master's cell, the style chain
    /// (each style's own chain included), the section default, the hard
    /// default.
    ///
    /// Returns [`crate::Error::UnknownShape`] when `shape` is not an arena
    /// id of the document. A cycle never errors: the chain stops at the
    /// first repeated source (ADR 0013).
    fn resolve(&self, shape: ShapeId, cell: &CellName) -> Result<Resolved<'_>, crate::Error>;
}

impl CellLookup for Document {
    fn resolve(&self, shape: ShapeId, cell: &CellName) -> Result<Resolved<'_>, crate::Error> {
        self.resolve_with_table(shape, cell, CELLS)
    }
}

impl Document {
    /// Resolves `cell` on `shape` against `table`, the ShapeSheet cell
    /// catalogue (the generated table in production).
    ///
    /// The table is a parameter so the section-default rung can be tested
    /// with a table that records `default_value` strings; the shipped
    /// catalogue (`ui-spec/shapesheet.toml`) records none yet.
    pub(crate) fn resolve_with_table(
        &self,
        shape: ShapeId,
        cell: &CellName,
        table: &[CellDef],
    ) -> Result<Resolved<'_>, crate::Error> {
        let entity = self.shape(shape).ok_or(crate::Error::UnknownShape(shape))?;
        let mut via = Vec::new();
        // The section the cell lives in, when a find saw it; it selects the
        // style and disambiguates table entries.
        let mut section_hint: Option<&str> = None;

        // 1. Local: the cell on the shape itself.
        via.push(ValueSource::Local);
        if let Some((found, hint)) = sheet_cell(&entity.sheet, cell) {
            section_hint = hint;
            if defines(found) {
                return Ok(Resolved {
                    value: found.value.as_ref(),
                    source: ValueSource::Local,
                    formula: found.formula.as_deref(),
                    via,
                });
            }
        }

        // 2. Master chain, with the sub-shape rule (ADR 0013) and the
        // position rule for masters with several top-level shapes (P2-06b
        // decision 2).
        let mut visited_masters: Vec<MasterId> = Vec::new();
        if let Some((first, root, mut path)) = self.instance_chain_start(shape) {
            let position = self.sibling_position(root);
            let mut current = first;
            loop {
                if visited_masters.contains(&current) {
                    // A cyclic master chain: record the repeat and stop it.
                    via.push(ValueSource::Master(current));
                    break;
                }
                visited_masters.push(current);
                via.push(ValueSource::Master(current));
                let mut chained = None;
                if let Some(content) = self
                    .master(current)
                    .and_then(|m| self.master_shape(m, position))
                {
                    let target = walk_down(self, content, &path).unwrap_or(content);
                    if let Some((found, hint)) = sheet_cell(&target.sheet, cell) {
                        section_hint = hint;
                        if defines(found) {
                            return Ok(Resolved {
                                value: found.value.as_ref(),
                                source: ValueSource::Master(current),
                                formula: found.formula.as_deref(),
                                via,
                            });
                        }
                    }
                    // A master shape that is itself an instance continues
                    // the chain at that master's own root.
                    chained = target.master;
                    path.clear();
                }
                match chained {
                    Some(next) => current = next,
                    None => break,
                }
            }
        }

        // 3. Style chain selected by the cell's section (a Geometry cell
        // never comes from a style). A style reference is a Visio style id
        // (P2-06b decision 3) and each style continues the chain through
        // its own link of the same kind.
        let mut visited_styles: Vec<StyleId> = Vec::new();
        if let Some(kind) = style_kind_for(table, cell.as_str(), section_hint) {
            let mut link = match kind {
                StyleKind::Line => entity.line_style,
                StyleKind::Fill => entity.fill_style,
                StyleKind::Text => entity.text_style,
            };
            while let Some(visio_id) = link {
                let Some(style) = self.style_by_visio_id(visio_id) else {
                    // A dangling style reference ends the chain.
                    break;
                };
                let id = style.id;
                if visited_styles.contains(&id) {
                    // A cyclic style chain: record the repeat and stop it.
                    via.push(ValueSource::Style(id));
                    break;
                }
                visited_styles.push(id);
                via.push(ValueSource::Style(id));
                if let Some((found, hint)) = sheet_cell(&style.sheet, cell) {
                    section_hint = hint;
                    if defines(found) {
                        return Ok(Resolved {
                            value: found.value.as_ref(),
                            source: ValueSource::Style(id),
                            formula: found.formula.as_deref(),
                            via,
                        });
                    }
                }
                link = match kind {
                    StyleKind::Line => style.line_style,
                    StyleKind::Fill => style.fill_style,
                    StyleKind::Text => style.text_style,
                };
            }
        }

        // 4. Section default from the ShapeSheet table.
        via.push(ValueSource::SectionDefault);
        let def = find_def(table, cell.as_str(), section_hint);
        if let Some(value) = def
            .and_then(|d| d.default_value.map(|text| (text, d)))
            .and_then(|(text, d)| interned_default(text, d.default_unit, d.kind))
        {
            return Ok(Resolved {
                value: Some(value),
                source: ValueSource::SectionDefault,
                formula: None,
                via,
            });
        }

        // 5. Hard default: the default of the cell's type.
        via.push(ValueSource::HardDefault);
        let value = def.and_then(|d| hard_default(d.kind));
        Ok(Resolved {
            value,
            source: ValueSource::HardDefault,
            formula: None,
            via,
        })
    }

    /// The master chain that applies to `shape`: the master of the nearest
    /// shape that is an instance (the shape itself or an ancestor), that
    /// instance root, and the child positions from the root down to `shape`.
    ///
    /// Returns `None` when no ancestor is a master instance, and when the
    /// parent chain is missing or cyclic (a malformed file must not loop).
    fn instance_chain_start(&self, shape: ShapeId) -> Option<(MasterId, ShapeId, Vec<usize>)> {
        let mut path = Vec::new();
        let mut visited = Vec::new();
        let mut current = shape;
        loop {
            if visited.contains(&current) {
                return None;
            }
            visited.push(current);
            let shape = self.shape(current)?;
            if let Some(master) = shape.master {
                path.reverse();
                return Some((master, current, path));
            }
            let parent = shape.parent?;
            let position = self
                .shape(parent)?
                .children
                .iter()
                .position(|c| *c == current)?;
            path.push(position);
            current = parent;
        }
    }

    /// The position of `shape` among its siblings: its parent's children, or
    /// the page's top-level shapes when it has no parent. `None` when the
    /// shape is unreachable from any page (a malformed file).
    fn sibling_position(&self, shape: ShapeId) -> Option<usize> {
        let node = self.shape(shape)?;
        if let Some(parent) = node.parent {
            return self
                .shape(parent)?
                .children
                .iter()
                .position(|c| *c == shape);
        }
        self.pages
            .iter()
            .find_map(|page| page.shapes.iter().position(|s| *s == shape))
    }

    /// The master shape an instance at `position` matches (P2-06b decision
    /// 2): the master's only shape when it has exactly one top-level shape,
    /// the shape at `position` otherwise. A position the master does not
    /// have degrades to the master's first shape.
    fn master_shape(
        &self,
        master: &crate::master::Master,
        position: Option<usize>,
    ) -> Option<&crate::shape::Shape> {
        match master.shapes.as_slice() {
            [] => None,
            [only] => self.shape(*only),
            many => {
                let picked = position
                    .and_then(|p| many.get(p))
                    .or_else(|| many.first())?;
                self.shape(*picked)
            }
        }
    }
}

/// The corresponding shape of `root` at the child positions `path`, or
/// `None` when the structure does not match. The walk is bounded by
/// `path.len()`.
fn walk_down<'a>(
    doc: &'a Document,
    root: &'a crate::shape::Shape,
    path: &[usize],
) -> Option<&'a crate::shape::Shape> {
    let mut current = root;
    for position in path {
        let child = *current.children.get(*position)?;
        current = doc.shape(child)?;
    }
    Some(current)
}

/// Whether a cell defines the value of its source: it carries a formula or
/// a cached value (ADR 0013, "the first that defines the cell").
fn defines(cell: &Cell) -> bool {
    cell.value.is_some() || cell.formula.is_some()
}

/// The cell named `name` in `sheet` and the section it lives in (`None` for
/// a direct cell). Direct cells come first, then each section's own cells
/// before its row cells, in file order (P2-06b decision 1).
fn sheet_cell<'a>(sheet: &'a Sheet, name: &CellName) -> Option<(&'a Cell, Option<&'a str>)> {
    if let Some(found) = sheet.cells.iter().find(|c| c.name == *name) {
        return Some((found, None));
    }
    for section in &sheet.sections {
        if let Some(found) = section.cells.iter().find(|c| c.name == *name) {
            return Some((found, Some(section.name.as_str())));
        }
        for row in &section.rows {
            if let Some(found) = row.cells.iter().find(|c| c.name == *name) {
                return Some((found, Some(section.name.as_str())));
            }
        }
    }
    None
}

/// Which stylesheet applies to a cell of one section (ADR 0013).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StyleKind {
    /// `LineStyle` selects the style for `Line Format` cells.
    Line,
    /// `FillStyle` selects the style for `Fill Format` cells.
    Fill,
    /// `TextStyle` selects the style for `Character` and `Paragraph` cells.
    Text,
}

/// The style kind of the cells of one section, or `None` when that
/// section's cells never come from a style (Geometry, Shape Transform, ...).
fn style_kind_of_section(section: &str) -> Option<StyleKind> {
    match section {
        "Line Format" | "Line" => Some(StyleKind::Line),
        "Fill Format" | "Fill" => Some(StyleKind::Fill),
        "Character" | "Paragraph" => Some(StyleKind::Text),
        _ => None,
    }
}

/// The style kind for one cell: its section when known, otherwise the kind
/// of the table sections the cell name appears in (a name shared by several
/// kinds has no style).
///
/// Names are translated from their `.vsdx` spelling to their ShapeSheet-table
/// spelling first (crate::aliases), so file spellings find the same entries.
fn style_kind_for(table: &[CellDef], name: &str, section_hint: Option<&str>) -> Option<StyleKind> {
    let name = crate::aliases::table_name(name);
    if let Some(hint) = section_hint {
        return style_kind_of_section(crate::aliases::table_name(hint));
    }
    let mut found = None;
    for def in table {
        if def.name != name {
            continue;
        }
        if let Some(kind) = style_kind_of_section(def.section) {
            if found.is_some_and(|seen| seen != kind) {
                return None;
            }
            found = Some(kind);
        }
    }
    found
}

/// The catalogue entry of one cell: its section when known, otherwise the
/// first entry with this name (the table is sorted, so the choice is
/// deterministic).
///
/// Names are translated from their `.vsdx` spelling to their ShapeSheet-table
/// spelling first (crate::aliases), so file spellings find the same entries.
fn find_def<'t>(
    table: &'t [CellDef],
    name: &str,
    section_hint: Option<&str>,
) -> Option<&'t CellDef> {
    let name = crate::aliases::table_name(name);
    let section_hint = section_hint.map(crate::aliases::table_name);
    let mut fallback = None;
    for def in table {
        if def.name != name {
            continue;
        }
        if Some(def.section) == section_hint {
            return Some(def);
        }
        if fallback.is_none() {
            fallback = Some(def);
        }
    }
    fallback
}

/// The hard default of a cell type (ADR 0013: 0, empty string, false,
/// black). A list's hard default is the empty list; a type the table does
/// not know has none.
///
/// The values are interned once so a lookup can hand out `&'static`
/// references without allocating per call.
fn hard_default(kind: &str) -> Option<&'static CellValue> {
    static NUMBER: OnceLock<CellValue> = OnceLock::new();
    static STRING: OnceLock<CellValue> = OnceLock::new();
    static BOOL: OnceLock<CellValue> = OnceLock::new();
    static COLOR: OnceLock<CellValue> = OnceLock::new();
    static LIST: OnceLock<CellValue> = OnceLock::new();
    let value = match kind {
        "Number" => NUMBER.get_or_init(|| CellValue::Number(0.0)),
        "String" => STRING.get_or_init(|| CellValue::Str(String::new())),
        "Bool" => BOOL.get_or_init(|| CellValue::Bool(false)),
        "Color" => COLOR.get_or_init(|| CellValue::Color(Color::new(0, 0, 0))),
        "List" => LIST.get_or_init(|| CellValue::List(String::new())),
        _ => return None,
    };
    Some(value)
}

/// Parses a table `default_value` and interns the result, so a lookup can
/// hand out a `&'static` reference (the borrowed value outlives the lookup).
///
/// The cache is keyed by the catalogue strings it was parsed from and is
/// bounded by their number (the table is static), so the interning leak is
/// bounded and happens once per distinct default.
fn interned_default(
    text: &'static str,
    default_unit: &'static str,
    kind: &'static str,
) -> Option<&'static CellValue> {
    type Key = (&'static str, &'static str, &'static str);
    static CACHE: OnceLock<Mutex<HashMap<Key, &'static CellValue>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = (text, default_unit, kind);
    if let Ok(guard) = cache.lock()
        && let Some(value) = guard.get(&key)
    {
        return Some(*value);
    }
    let parsed = parse_default_value(text, default_unit, kind)?;
    let interned: &'static CellValue = Box::leak(Box::new(parsed));
    match cache.lock() {
        Ok(mut guard) => Some(*guard.entry(key).or_insert(interned)),
        Err(_) => Some(interned),
    }
}

/// Parses a `default_value` string of the ShapeSheet table into a cached
/// value in canonical units (ADR 0011), using the cell type and its
/// `default_unit` (ADR 0008).
///
/// - `"String"`: the text, without its quotes when quoted.
/// - `"Bool"`: `TRUE` or `FALSE`, case-insensitive.
/// - `"Color"`: `#RRGGBB` or `#RGB`.
/// - `"List"`: the raw text.
/// - `"Number"` (and any other type): a number literal with an optional
///   unit suffix (`"0 in"`, `"2.5"`, `"50"` for a percent cell), converted
///   to the canonical unit of the cell's `default_unit` kind.
///
/// Returns `None` when the text does not fit the type.
fn parse_default_value(text: &str, default_unit: &str, kind: &str) -> Option<CellValue> {
    let trimmed = text.trim();
    match kind {
        "String" => Some(CellValue::Str(unquote(trimmed))),
        "Bool" => match trimmed.to_ascii_uppercase().as_str() {
            "TRUE" => Some(CellValue::Bool(true)),
            "FALSE" => Some(CellValue::Bool(false)),
            _ => None,
        },
        "Color" => parse_color(trimmed),
        "List" => Some(CellValue::List(trimmed.to_owned())),
        _ => parse_number_default(trimmed, default_unit),
    }
}

/// Removes one layer of matching quotes.
fn unquote(text: &str) -> String {
    for quote in ['"', '\''] {
        if text.len() >= 2
            && text.starts_with(quote)
            && text.ends_with(quote)
            && let Some(inner) = text.get(1..text.len() - 1)
        {
            return inner.to_owned();
        }
    }
    text.to_owned()
}

/// Parses `#RRGGBB` or `#RGB`.
fn parse_color(text: &str) -> Option<CellValue> {
    let hex = text.strip_prefix('#')?;
    let (r, g, b) = match hex.len() {
        6 => (
            u8::from_str_radix(hex.get(0..2)?, 16).ok()?,
            u8::from_str_radix(hex.get(2..4)?, 16).ok()?,
            u8::from_str_radix(hex.get(4..6)?, 16).ok()?,
        ),
        3 => (
            u8::from_str_radix(hex.get(0..1)?, 16).ok()? * 17,
            u8::from_str_radix(hex.get(1..2)?, 16).ok()? * 17,
            u8::from_str_radix(hex.get(2..3)?, 16).ok()? * 17,
        ),
        _ => return None,
    };
    Some(CellValue::Color(Color::new(r, g, b)))
}

/// Parses a numeric default: a number literal with an optional unit suffix,
/// converted to the canonical unit of the cell's `default_unit` kind
/// (`ferrogram_geom::Unit` and `UnitKind`). A suffix must measure the same
/// kind as the cell's default unit.
fn parse_number_default(text: &str, default_unit: &str) -> Option<CellValue> {
    let (value, suffix) = crate::raw::parse_number_value(text)?;
    let base = unit_from_name(default_unit).unwrap_or(Unit::Unitsless);
    let unit = if suffix.trim().is_empty() {
        base
    } else {
        unit_from_name(suffix)?
    };
    let canonical = base.kind().to_canonical(unit, value).ok()?;
    Some(CellValue::Number(canonical))
}

/// The [`Unit`] a name or suffix spells, case-insensitively. The table's
/// unit-less spellings (`NoCast`, `Number`, `String`, `Color`) are plain
/// numbers ([`Unit::Unitsless`]).
fn unit_from_name(name: &str) -> Option<Unit> {
    let lower = name.trim().to_ascii_lowercase();
    match lower.as_str() {
        "" | "nocast" | "number" | "string" | "color" | "unitsless" | "unitless" | "no" => {
            Some(Unit::Unitsless)
        }
        "in" | "inch" | "inches" => Some(Unit::Inches),
        "ft" | "foot" | "feet" => Some(Unit::Feet),
        "yd" | "yard" | "yards" => Some(Unit::Yards),
        "mi" | "mile" | "miles" => Some(Unit::Miles),
        "mm" | "millimeter" | "millimeters" => Some(Unit::Millimeters),
        "cm" | "centimeter" | "centimeters" => Some(Unit::Centimeters),
        "m" | "meter" | "meters" => Some(Unit::Meters),
        "km" | "kilometer" | "kilometers" => Some(Unit::Kilometers),
        "pt" | "point" | "points" => Some(Unit::Points),
        "pica" | "picas" => Some(Unit::Picas),
        "cicero" | "ciceros" => Some(Unit::Ciceros),
        "ang" | "angstrom" | "angstroms" => Some(Unit::Angstroms),
        "nm" | "nanometer" | "nanometers" => Some(Unit::Nanometers),
        "micron" | "microns" | "micrometer" | "micrometers" => Some(Unit::Microns),
        "millipoint" | "millipoints" => Some(Unit::Millipoints),
        "deg" | "degree" | "degrees" => Some(Unit::Degrees),
        "rad" | "radian" | "radians" => Some(Unit::Radians),
        "%" | "percent" => Some(Unit::Percent),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Style;

    #[test]
    fn empty_resolution_has_no_value() {
        let r = Resolved::empty(ValueSource::HardDefault);
        assert_eq!(r.value, None);
        assert_eq!(r.source, ValueSource::HardDefault);
        assert_eq!(r.formula, None);
        assert!(r.via.is_empty());
    }

    #[test]
    fn value_sources_carry_their_ids() {
        assert_ne!(
            ValueSource::Master(MasterId { index: 1 }),
            ValueSource::Master(MasterId { index: 2 })
        );
        assert_ne!(ValueSource::Style(StyleId { index: 1 }), ValueSource::Local);
    }

    // --- fixtures ------------------------------------------------------

    /// A cell with a cached number value.
    fn number_cell(name: &str, value: f64) -> Cell {
        let mut cell = Cell::new(CellName::new(name));
        cell.value = Some(CellValue::Number(value));
        cell
    }

    /// A table whose `PinX` has a section default of `2 in`.
    const DEFAULT_TABLE: &[CellDef] = &[CellDef {
        name: "PinX",
        section: "Shape Transform",
        cell_index: Some(0),
        kind: "Number",
        default_unit: "Inches",
        default_value: Some("2 in"),
    }];

    /// A table with one cell per type and no defaults.
    const KIND_TABLE: &[CellDef] = &[
        CellDef {
            name: "Num",
            section: "Shape Transform",
            cell_index: Some(0),
            kind: "Number",
            default_unit: "Inches",
            default_value: None,
        },
        CellDef {
            name: "Str",
            section: "Character",
            cell_index: Some(0),
            kind: "String",
            default_unit: "NoCast",
            default_value: None,
        },
        CellDef {
            name: "Bool",
            section: "Protection",
            cell_index: Some(0),
            kind: "Bool",
            default_unit: "NoCast",
            default_value: None,
        },
        CellDef {
            name: "Col",
            section: "Fill Format",
            cell_index: Some(0),
            kind: "Color",
            default_unit: "Color",
            default_value: None,
        },
        CellDef {
            name: "Lst",
            section: "Shape Data",
            cell_index: Some(0),
            kind: "List",
            default_unit: "NoCast",
            default_value: None,
        },
        CellDef {
            name: "Mystery",
            section: "Miscellaneous",
            cell_index: Some(0),
            kind: "Something",
            default_unit: "NoCast",
            default_value: None,
        },
    ];

    /// Builds a document with one page and one top-level shape.
    fn document_with_shape() -> (Document, ShapeId) {
        let mut doc = Document::new();
        let id = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 1));
        doc.pages
            .push(crate::page::Page::new(crate::id::PageId { index: 0 }, 1));
        doc.pages[0].shapes.push(id);
        (doc, id)
    }

    // --- the five sources, in order ------------------------------------

    #[test]
    fn local_cell_resolves_first() {
        let (mut doc, id) = document_with_shape();
        doc.shape_mut(id)
            .expect("shape exists")
            .sheet
            .cells
            .push(number_cell("PinX", 1.5));
        let r = doc.resolve(id, &CellName::from_static("PinX")).unwrap();
        assert_eq!(r.value, Some(&CellValue::Number(1.5)));
        assert_eq!(r.source, ValueSource::Local);
        assert_eq!(r.via, vec![ValueSource::Local]);
    }

    #[test]
    fn local_formula_is_reported_without_a_value() {
        let (mut doc, id) = document_with_shape();
        let mut cell = Cell::new(CellName::from_static("LocPinX"));
        cell.set_formula(Some("Width*0.5".to_owned()));
        doc.shape_mut(id)
            .expect("shape exists")
            .sheet
            .cells
            .push(cell);
        let r = doc.resolve(id, &CellName::from_static("LocPinX")).unwrap();
        assert_eq!(r.value, None);
        assert_eq!(r.source, ValueSource::Local);
        assert_eq!(r.formula, Some("Width*0.5"));
    }

    #[test]
    fn empty_local_cell_does_not_define() {
        let (mut doc, id) = document_with_shape();
        doc.shape_mut(id)
            .expect("shape exists")
            .sheet
            .cells
            .push(Cell::new(CellName::from_static("PinX")));
        let r = doc
            .resolve_with_table(id, &CellName::from_static("PinX"), DEFAULT_TABLE)
            .unwrap();
        assert_eq!(r.value, Some(&CellValue::Number(2.0)));
        assert_eq!(r.source, ValueSource::SectionDefault);
        assert_eq!(r.via, vec![ValueSource::Local, ValueSource::SectionDefault]);
    }

    #[test]
    fn master_cell_resolves_for_an_instance() {
        let (mut doc, id) = document_with_shape();
        let mid = doc.masters.len();
        let mut master = crate::master::Master::new(MasterId { index: mid as u32 }, 1);
        let content = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 1));
        doc.shape_mut(content)
            .expect("content exists")
            .sheet
            .cells
            .push(number_cell("PinX", 7.0));
        master.shapes = vec![content];
        master.name_u = "Box".to_owned();
        doc.masters.push(master);
        doc.shape_mut(id).expect("shape exists").master = Some(MasterId { index: mid as u32 });

        let r = doc
            .resolve_with_table(id, &CellName::from_static("PinX"), DEFAULT_TABLE)
            .unwrap();
        assert_eq!(r.value, Some(&CellValue::Number(7.0)));
        assert_eq!(
            r.source,
            ValueSource::Master(MasterId { index: mid as u32 })
        );
        assert_eq!(
            r.via,
            vec![
                ValueSource::Local,
                ValueSource::Master(MasterId { index: mid as u32 })
            ]
        );
    }

    #[test]
    fn sub_shape_resolves_against_the_master_sub_shape_at_the_same_position() {
        let (mut doc, root) = document_with_shape();
        let sibling = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 3));
        let sub = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 2));
        doc.shape_mut(root)
            .expect("root exists")
            .children
            .extend([sibling, sub]);
        doc.shape_mut(sibling).expect("sibling exists").parent = Some(root);
        doc.shape_mut(sub).expect("sub exists").parent = Some(root);

        // Master content: root with two children; only the second defines
        // PinX.
        let mid = doc.masters.len();
        let mut master = crate::master::Master::new(MasterId { index: mid as u32 }, 1);
        let m_root = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 1));
        let m_first = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 2));
        let m_second = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 3));
        doc.shape_mut(m_root).expect("exists").children = vec![m_first, m_second];
        doc.shape_mut(m_first).expect("exists").parent = Some(m_root);
        doc.shape_mut(m_second).expect("exists").parent = Some(m_root);
        doc.shape_mut(m_first)
            .expect("exists")
            .sheet
            .cells
            .push(number_cell("PinX", 1.0));
        doc.shape_mut(m_second)
            .expect("exists")
            .sheet
            .cells
            .push(number_cell("PinX", 2.0));
        master.shapes = vec![m_root];
        doc.masters.push(master);
        doc.shape_mut(root).expect("exists").master = Some(MasterId { index: mid as u32 });

        // The instance's own second child resolves to the master's second
        // child (position 1).
        let r = doc
            .resolve_with_table(sub, &CellName::from_static("PinX"), DEFAULT_TABLE)
            .unwrap();
        assert_eq!(r.value, Some(&CellValue::Number(2.0)));
        assert_eq!(
            r.source,
            ValueSource::Master(MasterId { index: mid as u32 })
        );

        // The instance root resolves to the master root (no definition
        // there): the section default shows the master rung was consulted.
        let r = doc
            .resolve_with_table(root, &CellName::from_static("PinX"), DEFAULT_TABLE)
            .unwrap();
        assert_eq!(r.source, ValueSource::SectionDefault);
        assert_eq!(
            r.via,
            vec![
                ValueSource::Local,
                ValueSource::Master(MasterId { index: mid as u32 }),
                ValueSource::SectionDefault
            ]
        );
    }

    #[test]
    fn structure_mismatch_falls_back_to_the_master_own_sheet() {
        let (mut doc, root) = document_with_shape();
        let sibling = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 3));
        let sub = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 2));
        doc.shape_mut(root)
            .expect("exists")
            .children
            .extend([sibling, sub]);
        doc.shape_mut(sibling).expect("exists").parent = Some(root);
        doc.shape_mut(sub).expect("exists").parent = Some(root);

        // The master has only one child, so the position 1 does not exist.
        let mid = doc.masters.len();
        let mut master = crate::master::Master::new(MasterId { index: mid as u32 }, 1);
        let m_root = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 1));
        let m_only = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 2));
        doc.shape_mut(m_root).expect("exists").children = vec![m_only];
        doc.shape_mut(m_only).expect("exists").parent = Some(m_root);
        doc.shape_mut(m_root)
            .expect("exists")
            .sheet
            .cells
            .push(number_cell("PinX", 9.0));
        master.shapes = vec![m_root];
        doc.masters.push(master);
        doc.shape_mut(root).expect("exists").master = Some(MasterId { index: mid as u32 });

        let r = doc
            .resolve_with_table(sub, &CellName::from_static("PinX"), DEFAULT_TABLE)
            .unwrap();
        assert_eq!(r.value, Some(&CellValue::Number(9.0)));
        assert_eq!(
            r.source,
            ValueSource::Master(MasterId { index: mid as u32 })
        );
    }

    #[test]
    fn a_master_with_one_shape_matches_any_instance_position() {
        // The first page shape is a plain shape; the instance sits at
        // position 1 and still matches the master's only shape.
        let (mut doc, plain) = document_with_shape();
        let id = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 2));
        doc.pages[0].shapes.push(id);
        let mid = doc.masters.len();
        let mut master = crate::master::Master::new(MasterId { index: mid as u32 }, 1);
        let content = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 1));
        doc.shape_mut(content)
            .expect("content exists")
            .sheet
            .cells
            .push(number_cell("PinX", 7.0));
        master.shapes = vec![content];
        doc.masters.push(master);
        doc.shape_mut(id).expect("exists").master = Some(MasterId { index: mid as u32 });
        assert_eq!(doc.sibling_position(id), Some(1));
        assert_eq!(doc.sibling_position(plain), Some(0));

        let r = doc
            .resolve_with_table(id, &CellName::from_static("PinX"), DEFAULT_TABLE)
            .unwrap();
        assert_eq!(r.value, Some(&CellValue::Number(7.0)));
        assert_eq!(
            r.source,
            ValueSource::Master(MasterId { index: mid as u32 })
        );
    }

    #[test]
    fn a_master_with_several_shapes_matches_by_position() {
        let (mut doc, first) = document_with_shape();
        let second = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 2));
        doc.pages[0].shapes.push(second);
        let mid = doc.masters.len();
        let mut master = crate::master::Master::new(MasterId { index: mid as u32 }, 1);
        let m_first = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 1));
        let m_second = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 2));
        doc.shape_mut(m_first)
            .expect("exists")
            .sheet
            .cells
            .push(number_cell("PinX", 1.0));
        doc.shape_mut(m_second)
            .expect("exists")
            .sheet
            .cells
            .push(number_cell("PinX", 2.0));
        master.shapes = vec![m_first, m_second];
        doc.masters.push(master);
        doc.shape_mut(first).expect("exists").master = Some(MasterId { index: mid as u32 });
        doc.shape_mut(second).expect("exists").master = Some(MasterId { index: mid as u32 });

        let r = doc
            .resolve_with_table(first, &CellName::from_static("PinX"), DEFAULT_TABLE)
            .unwrap();
        assert_eq!(r.value, Some(&CellValue::Number(1.0)));
        let r = doc
            .resolve_with_table(second, &CellName::from_static("PinX"), DEFAULT_TABLE)
            .unwrap();
        assert_eq!(r.value, Some(&CellValue::Number(2.0)));
    }

    #[test]
    fn an_out_of_range_position_degrades_to_the_first_master_shape() {
        let (mut doc, _) = document_with_shape();
        let id = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 2));
        doc.pages[0].shapes.push(id);
        let mid = doc.masters.len();
        let mut master = crate::master::Master::new(MasterId { index: mid as u32 }, 1);
        let m_first = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 1));
        let m_second = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 2));
        doc.shape_mut(m_first)
            .expect("exists")
            .sheet
            .cells
            .push(number_cell("PinX", 1.0));
        doc.shape_mut(m_second)
            .expect("exists")
            .sheet
            .cells
            .push(number_cell("PinX", 2.0));
        master.shapes = vec![m_first, m_second];
        doc.masters.push(master);
        // The instance sits at position 1, the master has a shape there;
        // a master with one shape keeps its single shape for any position.
        doc.shape_mut(id).expect("exists").master = Some(MasterId { index: mid as u32 });
        assert_eq!(doc.sibling_position(id), Some(1));

        let r = doc
            .resolve_with_table(id, &CellName::from_static("PinX"), DEFAULT_TABLE)
            .unwrap();
        assert_eq!(r.value, Some(&CellValue::Number(2.0)));

        // A shape no page reaches has no position: the master's first shape.
        let orphan = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 3));
        doc.shape_mut(orphan).expect("exists").master = Some(MasterId { index: mid as u32 });
        assert_eq!(doc.sibling_position(orphan), None);
        let r = doc
            .resolve_with_table(orphan, &CellName::from_static("PinX"), DEFAULT_TABLE)
            .unwrap();
        assert_eq!(r.value, Some(&CellValue::Number(1.0)));
    }

    #[test]
    fn nested_instance_continues_the_master_chain() {
        let (mut doc, id) = document_with_shape();
        // Master 1 content is itself an instance of master 2.
        let m2 = doc.masters.len();
        let m2_content = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 1));
        doc.shape_mut(m2_content)
            .expect("exists")
            .sheet
            .cells
            .push(number_cell("PinX", 4.0));
        let mut master2 = crate::master::Master::new(MasterId { index: m2 as u32 }, 2);
        master2.shapes = vec![m2_content];
        doc.masters.push(master2);

        let m1 = doc.masters.len();
        let m1_content = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 1));
        doc.shape_mut(m1_content).expect("exists").master = Some(MasterId { index: m2 as u32 });
        let mut master1 = crate::master::Master::new(MasterId { index: m1 as u32 }, 3);
        master1.shapes = vec![m1_content];
        doc.masters.push(master1);

        doc.shape_mut(id).expect("exists").master = Some(MasterId { index: m1 as u32 });
        let r = doc
            .resolve_with_table(id, &CellName::from_static("PinX"), DEFAULT_TABLE)
            .unwrap();
        assert_eq!(r.value, Some(&CellValue::Number(4.0)));
        assert_eq!(r.source, ValueSource::Master(MasterId { index: m2 as u32 }));
        assert_eq!(
            r.via,
            vec![
                ValueSource::Local,
                ValueSource::Master(MasterId { index: m1 as u32 }),
                ValueSource::Master(MasterId { index: m2 as u32 })
            ]
        );
    }

    #[test]
    fn cyclic_master_chain_terminates_and_shows_the_repeat() {
        let (mut doc, id) = document_with_shape();
        // Master 0 content is an instance of master 1 and back.
        let m0 = doc.masters.len();
        let c0 = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 1));
        let m1 = m0 + 1;
        let c1 = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 1));
        doc.shape_mut(c0).expect("exists").master = Some(MasterId { index: m1 as u32 });
        doc.shape_mut(c1).expect("exists").master = Some(MasterId { index: m0 as u32 });
        let mut master0 = crate::master::Master::new(MasterId { index: m0 as u32 }, 1);
        master0.shapes = vec![c0];
        let mut master1 = crate::master::Master::new(MasterId { index: m1 as u32 }, 2);
        master1.shapes = vec![c1];
        doc.masters.push(master0);
        doc.masters.push(master1);
        doc.shape_mut(id).expect("exists").master = Some(MasterId { index: m0 as u32 });

        let r = doc
            .resolve_with_table(id, &CellName::from_static("PinX"), DEFAULT_TABLE)
            .unwrap();
        assert_eq!(r.source, ValueSource::SectionDefault);
        assert_eq!(
            r.via,
            vec![
                ValueSource::Local,
                ValueSource::Master(MasterId { index: m0 as u32 }),
                ValueSource::Master(MasterId { index: m1 as u32 }),
                ValueSource::Master(MasterId { index: m0 as u32 }),
                ValueSource::SectionDefault
            ]
        );
    }

    #[test]
    fn style_chain_resolves_through_the_style_links() {
        let (mut doc, id) = document_with_shape();
        doc.shape_mut(id).expect("exists").line_style = Some(1);

        let mut red = Style::new(StyleId { index: 0 });
        red.visio_id = 1;
        red.name = "Red".to_owned();
        red.line_style = Some(0);
        doc.stylesheet.styles.push(red);

        let mut normal = Style::new(StyleId { index: 1 });
        normal.visio_id = 0;
        normal.name = "Normal".to_owned();
        normal.sheet.cells.push(number_cell("LineWeight", 0.05));
        doc.stylesheet.styles.push(normal);

        let r = doc
            .resolve(id, &CellName::from_static("LineWeight"))
            .unwrap();
        assert_eq!(r.value, Some(&CellValue::Number(0.05)));
        assert_eq!(r.source, ValueSource::Style(StyleId { index: 1 }));
        assert_eq!(
            r.via,
            vec![
                ValueSource::Local,
                ValueSource::Style(StyleId { index: 0 }),
                ValueSource::Style(StyleId { index: 1 })
            ]
        );
    }

    #[test]
    fn the_chain_follows_the_style_link_of_the_cells_kind() {
        // Style 0 links line cells to style 1 and text cells to style 2:
        // one chain per kind (P2-06b decision 3).
        let (mut doc, id) = document_with_shape();
        doc.shape_mut(id).expect("exists").line_style = Some(0);
        doc.shape_mut(id).expect("exists").text_style = Some(0);
        let mut head = Style::new(StyleId { index: 0 });
        head.visio_id = 0;
        head.line_style = Some(1);
        head.text_style = Some(2);
        doc.stylesheet.styles.push(head);
        let mut line = Style::new(StyleId { index: 1 });
        line.visio_id = 1;
        line.sheet.cells.push(number_cell("LineWeight", 0.05));
        doc.stylesheet.styles.push(line);
        let mut text = Style::new(StyleId { index: 2 });
        text.visio_id = 2;
        text.sheet.cells.push(number_cell("Size", 12.0));
        doc.stylesheet.styles.push(text);

        let r = doc
            .resolve(id, &CellName::from_static("LineWeight"))
            .unwrap();
        assert_eq!(r.value, Some(&CellValue::Number(0.05)));
        assert_eq!(r.source, ValueSource::Style(StyleId { index: 1 }));
        let r = doc.resolve(id, &CellName::from_static("Size")).unwrap();
        assert_eq!(r.value, Some(&CellValue::Number(12.0)));
        assert_eq!(r.source, ValueSource::Style(StyleId { index: 2 }));
    }

    #[test]
    fn fill_and_text_cells_use_their_own_styles() {
        let (mut doc, id) = document_with_shape();
        doc.shape_mut(id).expect("exists").fill_style = Some(1);
        doc.shape_mut(id).expect("exists").text_style = Some(2);
        let mut fill = Style::new(StyleId { index: 0 });
        fill.visio_id = 1;
        fill.name = "Fill".to_owned();
        fill.sheet.cells.push(number_cell("FillForegnd", 1.0));
        doc.stylesheet.styles.push(fill);
        let mut text = Style::new(StyleId { index: 1 });
        text.visio_id = 2;
        text.name = "TextStyle".to_owned();
        text.sheet.cells.push(number_cell("Size", 12.0));
        doc.stylesheet.styles.push(text);

        let r = doc
            .resolve(id, &CellName::from_static("FillForegnd"))
            .unwrap();
        assert_eq!(r.source, ValueSource::Style(StyleId { index: 0 }));
        let r = doc.resolve(id, &CellName::from_static("Size")).unwrap();
        assert_eq!(r.source, ValueSource::Style(StyleId { index: 1 }));
    }

    #[test]
    fn geometry_cells_never_come_from_a_style() {
        // `X` is a Geometry cell here; the same name also appears in
        // sections with no style kind.
        const GEO_TABLE: &[CellDef] = &[
            CellDef {
                name: "X",
                section: "Geometry",
                cell_index: Some(0),
                kind: "Number",
                default_unit: "Inches",
                default_value: None,
            },
            CellDef {
                name: "X",
                section: "Scratch",
                cell_index: Some(0),
                kind: "Number",
                default_unit: "Inches",
                default_value: None,
            },
        ];
        let (mut doc, id) = document_with_shape();
        doc.shape_mut(id).expect("exists").line_style = Some(1);
        let mut red = Style::new(StyleId { index: 0 });
        red.visio_id = 1;
        red.name = "Red".to_owned();
        red.sheet.cells.push(number_cell("X", 3.0));
        doc.stylesheet.styles.push(red);

        // No style is consulted: `via` holds no `Style` entry and the value
        // falls through to the hard default.
        let r = doc
            .resolve_with_table(id, &CellName::from_static("X"), GEO_TABLE)
            .unwrap();
        assert_eq!(r.source, ValueSource::HardDefault);
        assert_eq!(r.value, Some(&CellValue::Number(0.0)));
        assert_eq!(
            r.via,
            vec![
                ValueSource::Local,
                ValueSource::SectionDefault,
                ValueSource::HardDefault
            ]
        );
    }

    #[test]
    fn row_cells_carry_their_section_into_the_style_lookup() {
        let (mut doc, id) = document_with_shape();
        doc.shape_mut(id).expect("exists").text_style = Some(1);
        let mut text = Style::new(StyleId { index: 0 });
        text.visio_id = 1;
        text.name = "TextStyle".to_owned();
        text.sheet.cells.push(number_cell("Size", 12.0));
        doc.stylesheet.styles.push(text);

        // The local cell exists but defines nothing; its section (`Character`)
        // selects `TextStyle`.
        let mut section =
            crate::section::Section::new(crate::name::SectionName::from_static("Character"));
        let mut row = crate::section::Row::new();
        row.cells.push(Cell::new(CellName::from_static("Size")));
        section.rows.push(row);
        doc.shape_mut(id)
            .expect("exists")
            .sheet
            .sections
            .push(section);

        let r = doc.resolve(id, &CellName::from_static("Size")).unwrap();
        assert_eq!(r.value, Some(&CellValue::Number(12.0)));
        assert_eq!(r.source, ValueSource::Style(StyleId { index: 0 }));
    }

    #[test]
    fn cyclic_style_chain_terminates_and_falls_through_to_defaults() {
        let (mut doc, id) = document_with_shape();
        doc.shape_mut(id).expect("exists").line_style = Some(0);
        let mut a = Style::new(StyleId { index: 0 });
        a.visio_id = 0;
        a.name = "A".to_owned();
        a.line_style = Some(1);
        doc.stylesheet.styles.push(a);
        let mut b = Style::new(StyleId { index: 1 });
        b.visio_id = 1;
        b.name = "B".to_owned();
        b.line_style = Some(0);
        doc.stylesheet.styles.push(b);

        let r = doc
            .resolve(id, &CellName::from_static("LineWeight"))
            .unwrap();
        assert_eq!(r.source, ValueSource::HardDefault);
        assert_eq!(
            r.via,
            vec![
                ValueSource::Local,
                ValueSource::Style(StyleId { index: 0 }),
                ValueSource::Style(StyleId { index: 1 }),
                ValueSource::Style(StyleId { index: 0 }),
                ValueSource::SectionDefault,
                ValueSource::HardDefault
            ]
        );
    }

    #[test]
    fn self_referencing_style_terminates() {
        let (mut doc, id) = document_with_shape();
        doc.shape_mut(id).expect("exists").fill_style = Some(0);
        let mut solo = Style::new(StyleId { index: 0 });
        solo.visio_id = 0;
        solo.name = "Solo".to_owned();
        solo.fill_style = Some(0);
        doc.stylesheet.styles.push(solo);

        let r = doc
            .resolve(id, &CellName::from_static("FillForegnd"))
            .unwrap();
        assert_eq!(r.source, ValueSource::HardDefault);
        assert_eq!(
            r.via,
            vec![
                ValueSource::Local,
                ValueSource::Style(StyleId { index: 0 }),
                ValueSource::Style(StyleId { index: 0 }),
                ValueSource::SectionDefault,
                ValueSource::HardDefault
            ]
        );
    }

    #[test]
    fn dangling_master_and_style_references_do_not_error() {
        let (mut doc, id) = document_with_shape();
        doc.shape_mut(id).expect("exists").master = Some(MasterId { index: 9 });
        doc.shape_mut(id).expect("exists").line_style = Some(42);
        let r = doc
            .resolve(id, &CellName::from_static("LineWeight"))
            .unwrap();
        assert_eq!(r.source, ValueSource::HardDefault);
        assert_eq!(
            r.via,
            vec![
                ValueSource::Local,
                ValueSource::Master(MasterId { index: 9 }),
                ValueSource::SectionDefault,
                ValueSource::HardDefault
            ]
        );
    }

    #[test]
    fn master_without_content_shape_is_consulted_but_defines_nothing() {
        let (mut doc, id) = document_with_shape();
        let mid = doc.masters.len();
        doc.masters.push(crate::master::Master::new(
            MasterId { index: mid as u32 },
            1,
        ));
        doc.shape_mut(id).expect("exists").master = Some(MasterId { index: mid as u32 });
        let r = doc
            .resolve_with_table(id, &CellName::from_static("PinX"), DEFAULT_TABLE)
            .unwrap();
        assert_eq!(r.source, ValueSource::SectionDefault);
    }

    #[test]
    fn unknown_shape_is_an_error() {
        let doc = Document::new();
        let err = doc
            .resolve(ShapeId { index: 3 }, &CellName::from_static("PinX"))
            .unwrap_err();
        assert_eq!(err, crate::Error::UnknownShape(ShapeId { index: 3 }));
    }

    #[test]
    fn cyclic_parent_chain_skips_the_master_rung() {
        let mut doc = Document::new();
        let a = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 1));
        let b = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 2));
        doc.shape_mut(a).expect("exists").parent = Some(b);
        doc.shape_mut(b).expect("exists").parent = Some(a);
        let r = doc
            .resolve_with_table(a, &CellName::from_static("PinX"), DEFAULT_TABLE)
            .unwrap();
        assert_eq!(r.via, vec![ValueSource::Local, ValueSource::SectionDefault]);
    }

    #[test]
    fn broken_parent_link_skips_the_master_rung() {
        let mut doc = Document::new();
        let a = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 1));
        // The parent does not list the shape among its children.
        let p = doc.add_shape(crate::shape::Shape::new(ShapeId { index: 0 }, 2));
        doc.shape_mut(a).expect("exists").parent = Some(p);
        let r = doc
            .resolve_with_table(a, &CellName::from_static("PinX"), DEFAULT_TABLE)
            .unwrap();
        assert_eq!(r.via, vec![ValueSource::Local, ValueSource::SectionDefault]);
    }

    // --- the default rungs ---------------------------------------------

    #[test]
    fn section_default_parses_the_table_value() {
        let (doc, id) = document_with_shape();
        let r = doc
            .resolve_with_table(id, &CellName::from_static("PinX"), DEFAULT_TABLE)
            .unwrap();
        assert_eq!(r.value, Some(&CellValue::Number(2.0)));
        assert_eq!(r.source, ValueSource::SectionDefault);
        assert_eq!(r.formula, None);
        assert_eq!(r.via, vec![ValueSource::Local, ValueSource::SectionDefault]);
    }

    #[test]
    fn section_default_of_a_bad_literal_falls_through() {
        const BAD: &[CellDef] = &[CellDef {
            name: "PinX",
            section: "Shape Transform",
            cell_index: Some(0),
            kind: "Number",
            default_unit: "Inches",
            default_value: Some("not a number"),
        }];
        let (doc, id) = document_with_shape();
        let r = doc
            .resolve_with_table(id, &CellName::from_static("PinX"), BAD)
            .unwrap();
        assert_eq!(r.source, ValueSource::HardDefault);
        assert_eq!(r.value, Some(&CellValue::Number(0.0)));
        assert_eq!(
            r.via,
            vec![
                ValueSource::Local,
                ValueSource::SectionDefault,
                ValueSource::HardDefault
            ]
        );
    }

    #[test]
    fn hard_defaults_match_the_cell_type() {
        let (doc, id) = document_with_shape();
        for (name, want) in [
            ("Num", CellValue::Number(0.0)),
            ("Str", CellValue::Str(String::new())),
            ("Bool", CellValue::Bool(false)),
            ("Col", CellValue::Color(Color::new(0, 0, 0))),
            ("Lst", CellValue::List(String::new())),
        ] {
            let r = doc
                .resolve_with_table(id, &CellName::new(name), KIND_TABLE)
                .unwrap();
            assert_eq!(r.source, ValueSource::HardDefault);
            assert_eq!(r.value, Some(&want), "{name}");
        }
    }

    #[test]
    fn unknown_cell_type_has_no_hard_default() {
        let (doc, id) = document_with_shape();
        let r = doc
            .resolve_with_table(id, &CellName::from_static("Mystery"), KIND_TABLE)
            .unwrap();
        assert_eq!(r.source, ValueSource::HardDefault);
        assert_eq!(r.value, None);
        assert_eq!(r.formula, None);
    }

    #[test]
    fn unknown_cell_name_has_no_table_default() {
        let (doc, id) = document_with_shape();
        let r = doc
            .resolve_with_table(id, &CellName::from_static("NoSuchCell"), DEFAULT_TABLE)
            .unwrap();
        assert_eq!(r.source, ValueSource::HardDefault);
        assert_eq!(r.value, None);
    }

    // --- default parsing ------------------------------------------------

    #[test]
    fn parse_default_value_by_kind() {
        assert_eq!(
            parse_default_value("hello", "NoCast", "String"),
            Some(CellValue::Str("hello".to_owned()))
        );
        assert_eq!(
            parse_default_value("\"q\"", "NoCast", "String"),
            Some(CellValue::Str("q".to_owned()))
        );
        assert_eq!(
            parse_default_value("TRUE", "NoCast", "Bool"),
            Some(CellValue::Bool(true))
        );
        assert_eq!(
            parse_default_value("false", "NoCast", "Bool"),
            Some(CellValue::Bool(false))
        );
        assert_eq!(
            parse_default_value("#ff8000", "Color", "Color"),
            Some(CellValue::Color(Color::new(255, 128, 0)))
        );
        assert_eq!(
            parse_default_value("#f80", "Color", "Color"),
            Some(CellValue::Color(Color::new(255, 136, 0)))
        );
        assert_eq!(
            parse_default_value("1;2", "NoCast", "List"),
            Some(CellValue::List("1;2".to_owned()))
        );
    }

    #[test]
    fn parse_default_numbers_convert_to_canonical_units() {
        assert_eq!(
            parse_default_value("0.5", "Inches", "Number"),
            Some(CellValue::Number(0.5))
        );
        assert_eq!(
            parse_default_value("0.5 ft", "Inches", "Number"),
            Some(CellValue::Number(6.0))
        );
        assert_eq!(
            parse_default_value("50", "Percent", "Number"),
            Some(CellValue::Number(0.5))
        );
        assert_eq!(
            parse_default_value("2", "NoCast", "Number"),
            Some(CellValue::Number(2.0))
        );
    }

    #[test]
    fn parse_default_angle_values_are_radians() {
        // The canonical angle unit is radians (ADR 0011), so a `Degrees`
        // default converts to radians and a `Radians` default stays as
        // written. `parse_default_value` itself does not change.
        match parse_default_value("90", "Degrees", "Number") {
            Some(CellValue::Number(value)) => {
                assert!(
                    (value - core::f64::consts::FRAC_PI_2).abs() <= 1.0e-12,
                    "{value}"
                );
            }
            other => panic!("expected a number, got {other:?}"),
        }
        assert_eq!(
            parse_default_value("1.5", "Radians", "Number"),
            Some(CellValue::Number(1.5))
        );
    }

    #[test]
    fn parse_default_rejects_bad_literals() {
        assert_eq!(parse_default_value("maybe", "NoCast", "Bool"), None);
        assert_eq!(parse_default_value("#12", "Color", "Color"), None);
        assert_eq!(parse_default_value("#xyzxyz", "Color", "Color"), None);
        assert_eq!(parse_default_value("word", "Inches", "Number"), None);
        // A suffix of a different kind than the cell's unit is rejected.
        assert_eq!(parse_default_value("1 deg", "Inches", "Number"), None);
        assert_eq!(parse_default_value("1 furlong", "Inches", "Number"), None);
    }

    #[test]
    fn unit_names_cover_the_table_and_suffix_spellings() {
        assert_eq!(unit_from_name("Inches"), Some(Unit::Inches));
        assert_eq!(unit_from_name("in"), Some(Unit::Inches));
        assert_eq!(unit_from_name("NoCast"), Some(Unit::Unitsless));
        assert_eq!(unit_from_name("Number"), Some(Unit::Unitsless));
        assert_eq!(unit_from_name("%"), Some(Unit::Percent));
        assert_eq!(unit_from_name("deg"), Some(Unit::Degrees));
        assert_eq!(unit_from_name("furlong"), None);
    }

    #[test]
    fn unquote_strips_one_layer_of_quotes() {
        assert_eq!(unquote("\"a\""), "a");
        assert_eq!(unquote("'a'"), "a");
        assert_eq!(unquote("\"\""), "");
        assert_eq!(unquote("\""), "\"");
        assert_eq!(unquote("'\""), "'\"");
        assert_eq!(unquote("plain"), "plain");
    }

    #[test]
    fn parse_color_rejects_bad_hex() {
        assert_eq!(parse_color("nope"), None);
        assert_eq!(parse_color("#12345"), None);
    }

    #[test]
    fn interned_defaults_are_stable_references() {
        let first = interned_default("2 in", "Inches", "Number").expect("parses");
        let second = interned_default("2 in", "Inches", "Number").expect("parses");
        assert!(std::ptr::eq(first, second));
        assert_eq!(interned_default("word", "Inches", "Number"), None);
    }

    // --- sheet and style helpers ---------------------------------------

    #[test]
    fn sheet_cell_prefers_direct_cells_and_reports_row_sections() {
        let mut sheet = Sheet::new();
        sheet.cells.push(number_cell("PinX", 1.0));
        let mut section =
            crate::section::Section::new(crate::name::SectionName::from_static("Geometry"));
        let mut row = crate::section::Row::new();
        row.cells.push(number_cell("X", 2.0));
        section.rows.push(row);
        sheet.sections.push(section);

        let (found, hint) = sheet_cell(&sheet, &CellName::from_static("PinX")).expect("found");
        assert_eq!(found.value.as_ref(), Some(&CellValue::Number(1.0)));
        assert_eq!(hint, None);
        let (found, hint) = sheet_cell(&sheet, &CellName::from_static("X")).expect("found");
        assert_eq!(found.value.as_ref(), Some(&CellValue::Number(2.0)));
        assert_eq!(hint, Some("Geometry"));
        assert!(sheet_cell(&sheet, &CellName::from_static("Y")).is_none());
    }

    #[test]
    fn section_level_cells_match_before_row_cells() {
        let mut sheet = Sheet::new();
        let mut section =
            crate::section::Section::new(crate::name::SectionName::from_static("Geometry"));
        section.cells.push(number_cell("X", 5.0));
        let mut row = crate::section::Row::new();
        row.cells.push(number_cell("X", 2.0));
        section.rows.push(row);
        sheet.sections.push(section);

        let (found, hint) = sheet_cell(&sheet, &CellName::from_static("X")).expect("found");
        assert_eq!(found.value.as_ref(), Some(&CellValue::Number(5.0)));
        assert_eq!(hint, Some("Geometry"));
    }

    #[test]
    fn a_section_level_cell_resolves_as_a_local_cell() {
        let (mut doc, id) = document_with_shape();
        let mut section =
            crate::section::Section::new(crate::name::SectionName::from_static("Geometry"));
        section.cells.push(number_cell("NoShow", 1.0));
        let mut row = crate::section::Row::new();
        row.cells.push(number_cell("NoShow", 0.0));
        section.rows.push(row);
        doc.shape_mut(id)
            .expect("exists")
            .sheet
            .sections
            .push(section);

        let r = doc
            .resolve(id, &CellName::from_static("NoShow"))
            .expect("resolves");
        assert_eq!(r.value, Some(&CellValue::Number(1.0)));
        assert_eq!(r.source, ValueSource::Local);
    }

    #[test]
    fn style_kind_covers_the_four_sections() {
        assert_eq!(style_kind_of_section("Line Format"), Some(StyleKind::Line));
        assert_eq!(style_kind_of_section("Fill Format"), Some(StyleKind::Fill));
        assert_eq!(style_kind_of_section("Character"), Some(StyleKind::Text));
        assert_eq!(style_kind_of_section("Paragraph"), Some(StyleKind::Text));
        assert_eq!(style_kind_of_section("Geometry"), None);
    }

    #[test]
    fn style_kind_of_a_name_shared_by_kinds_is_none() {
        // `ColorTrans` is a Character cell and something else: a name that
        // maps to two kinds selects no style.
        const TWO: &[CellDef] = &[
            CellDef {
                name: "Shared",
                section: "Character",
                cell_index: Some(0),
                kind: "Number",
                default_unit: "NoCast",
                default_value: None,
            },
            CellDef {
                name: "Shared",
                section: "Fill Format",
                cell_index: Some(0),
                kind: "Number",
                default_unit: "NoCast",
                default_value: None,
            },
        ];
        assert_eq!(style_kind_for(TWO, "Shared", None), None);
        assert_eq!(
            style_kind_for(TWO, "Shared", Some("Character")),
            Some(StyleKind::Text)
        );
        assert_eq!(style_kind_for(TWO, "NoSuch", None), None);
    }

    #[test]
    fn find_def_prefers_the_hinted_section() {
        const TWO: &[CellDef] = &[
            CellDef {
                name: "Shared",
                section: "Character",
                cell_index: Some(0),
                kind: "Number",
                default_unit: "NoCast",
                default_value: Some("1"),
            },
            CellDef {
                name: "Shared",
                section: "Fill Format",
                cell_index: Some(0),
                kind: "Number",
                default_unit: "NoCast",
                default_value: Some("2"),
            },
        ];
        assert_eq!(
            find_def(TWO, "Shared", Some("Fill Format")).and_then(|d| d.default_value),
            Some("2")
        );
        assert_eq!(
            find_def(TWO, "Shared", None).and_then(|d| d.default_value),
            Some("1")
        );
        assert_eq!(find_def(TWO, "Other", None), None);
    }
}
