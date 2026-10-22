//! Reference resolution and inheritance (ADR 0054).
//!
//! A formula's references resolve through the rungs of ADR 0013 (local,
//! master, style chain, section default, hard default), and an inherited
//! formula is evaluated against the sheet it is evaluated *for*: an
//! instance's `Width` in a master's geometry row is the instance's
//! width. [`ResolvedCell`] names where the formula and the cached value
//! came from, so the evaluator and the recalc pass can report provenance.
//!
//! The resolver is the only place that knows the row shapes of the
//! reference forms (ADR 0054): `Char.Size` is a column of the `Character`
//! section's rows while `Prop.Cost` is a named row's default cell. Names
//! match case-insensitively; the file's own spelling is what later gets
//! written back. A name that resolves to nothing becomes `#REF!` with one
//! warning ([`EvalWarningKind::UnsupportedReference`], the `MissingReference`
//! of "About Error Values"), never a Rust error.
//!
//! The value side prefers the local cell's own cached value, snapshot of
//! an `Inh` cell included (ADR 0054 amendment, ADR 0057): the snapshot is
//! the value for *this* sheet at save time. Provenance still comes from
//! the rung that defines the formula, so a snapshot cannot hide an
//! inheritance bug.

use ferrogram_geom::Unit;
use ferrogram_model::{
    Cell, CellLookup, CellName, CellValue, Document, Master, MasterId, PageId, Row, Section, Shape,
    ShapeId, Sheet, SheetRef, StyleId, ValueSource,
};

use crate::Error;
use crate::eval::{EvalContext, EvalWarning, EvalWarningKind};
use crate::formula::{CellRef, Formula};
use crate::value::{ErrorCode, Value};

/// The formula `.vsdx` writes on a cell whose value is inherited. It
/// defines nothing; its cached value is the sheet's own snapshot.
const INHERIT_FORMULA: &str = "Inh";

/// How many nested formula evaluations one `cell_value` call may start
/// (ADR 0051 bounds nesting inside one formula; this bounds the chain
/// across formulas). Recursion never runs away.
const MAX_EVAL_DEPTH: u32 = 32;

/// The default cell of a named row (`Prop.Cost` reads `Cost`'s `Value`).
const DEFAULT_ROW_CELL: &str = "Value";

/// One resolved cell: the formula that defines it and the value to use
/// (ADR 0054).
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedCell {
    /// The defining formula's source text, or `None` when no rung has one
    /// (a literal or an absent cell).
    pub formula: Option<String>,
    /// The value to use: the cached value of the defining rung, or the
    /// local snapshot of an `Inh` cell (ADR 0054 amendment), or `None`.
    pub value: Option<Value>,
    /// Which rung defines the cell.
    pub source: ValueSource,
}

/// Resolves references against a [`Document`] (ADR 0054).
///
/// The resolver is the [`EvalContext`] the evaluator runs against: it
/// reads cells with inheritance, applies basic `SETF` writes and collects
/// the structured warnings of ADR 0053.
pub struct DocumentResolver<'d> {
    document: &'d mut Document,
    warnings: Vec<EvalWarning>,
    depth: u32,
}

impl<'d> DocumentResolver<'d> {
    /// A resolver over `document`.
    pub fn new(document: &'d mut Document) -> Self {
        Self {
            document,
            warnings: Vec::new(),
            depth: 0,
        }
    }

    /// The document the resolver reads.
    pub fn document(&self) -> &Document {
        self.document
    }

    /// The warnings collected so far.
    pub fn warnings(&self) -> &[EvalWarning] {
        &self.warnings
    }

    /// The collected warnings, taking them out of the resolver.
    pub fn into_warnings(self) -> Vec<EvalWarning> {
        self.warnings
    }

    /// Resolves one referenced cell (ADR 0054).
    ///
    /// `sheet` is the sheet the formula being evaluated lives on: local
    /// names bind to it, so a master's formula evaluated for an instance
    /// reads the instance's `Width`. A name that resolves to nothing
    /// yields `#REF!` with one warning; it is never an error.
    pub fn resolve(&mut self, sheet: SheetRef, reference: &CellRef) -> Result<ResolvedCell, Error> {
        let found = self.find(sheet, reference);
        Ok(match found {
            Some(found) => self.resolved_cell(found),
            None => self.missing(sheet, reference),
        })
    }

    /// The `#REF!` answer for a name that resolves to nothing, with
    /// exactly one warning (ADR 0054, "About Error Values").
    fn missing(&mut self, sheet: SheetRef, reference: &CellRef) -> ResolvedCell {
        self.warnings.push(EvalWarning {
            sheet,
            kind: EvalWarningKind::UnsupportedReference(describe(reference)),
        });
        ResolvedCell {
            formula: None,
            value: Some(Value::Err(ErrorCode::Ref)),
            source: ValueSource::HardDefault,
        }
    }

    /// Maps what a resolution found to the answer, reading the cached
    /// value through `Value::from_cached` (P2-14, P6-03).
    fn resolved_cell(&self, found: Found) -> ResolvedCell {
        let value = found
            .value
            .and_then(|(value, unit)| Value::from_cached(&value, unit).ok());
        ResolvedCell {
            formula: found.formula,
            value,
            source: found.source,
        }
    }

    /// The [`Target`] the reference's sheet parts name, or `None` when
    /// one of them names nothing (ADR 0054).
    fn target(&self, sheet: SheetRef, parts: &[String]) -> Option<Target> {
        let mut current = Target::Sheet(sheet);
        let mut index = 0;
        while let Some(part) = parts.get(index) {
            let part = unquote_part(part);
            index += 1;
            if eq_ci(&part, "Styles") {
                // `Styles!Manager`: the stylesheet row named next.
                let name = unquote_part(parts.get(index)?);
                index += 1;
                current = Target::Style(self.style_named(&name)?);
                continue;
            }
            current = self.step(current, &part)?;
        }
        Some(current)
    }

    /// One sheet part of a reference, applied to `current`.
    fn step(&self, current: Target, part: &str) -> Option<Target> {
        if eq_ci(part, "ThePage") {
            return Some(Target::Sheet(SheetRef::Page(self.page_context(current)?)));
        }
        if eq_ci(part, "TheDoc") {
            return Some(Target::Document);
        }
        if let Some(name) = bracketed(part, "Masters") {
            let master = self.master_named(&name)?;
            let content = master_content(self.document, master)?;
            return Some(Target::Sheet(SheetRef::Shape(content)));
        }
        if let Some(name) = bracketed(part, "Pages") {
            return Some(Target::Sheet(SheetRef::Page(self.page_named(&name)?)));
        }
        let (name, visio_id) = split_numbered(part);
        if let Some(visio_id) = visio_id {
            let page = self.page_context(current);
            let found = page
                .and_then(|page| shape_by_visio_id(self.document, page, visio_id, name.as_str()))
                .or_else(|| self.descendant_by_visio_id(current, visio_id));
            return found.map(|shape| Target::Sheet(SheetRef::Shape(shape)));
        }
        self.named_shape(current, &name)
            .map(|shape| Target::Sheet(SheetRef::Shape(shape)))
    }

    /// The named sub-shape or page shape one bare sheet part names: the
    /// target's descendants first (a master's `Shaft`), then the shapes of
    /// the target's page (`Star` on the same page).
    fn named_shape(&self, current: Target, name: &str) -> Option<ShapeId> {
        self.descendant_named(current, name).or_else(|| {
            let page = self.page_context(current)?;
            let roots: Vec<ShapeId> = self
                .document
                .page(page)
                .map(|page| page.shapes.clone())
                .unwrap_or_default();
            first_named(self.document, &roots, name)
        })
    }

    /// The named descendant of the current shape's sheet: the shape's own
    /// children first (a master's `Shaft`), then the root itself.
    fn descendant_named(&self, current: Target, name: &str) -> Option<ShapeId> {
        let root = self.sheet_shape(current)?;
        let children = self
            .document
            .shape(root)
            .map(|shape| shape.children.clone())
            .unwrap_or_default();
        // The root's own name never names the root: a reference part names
        // a shape *on* the current sheet.
        first_named(self.document, &children, name).or_else(|| {
            let roots = vec![root];
            first_named(self.document, &roots, name)
        })
    }

    /// The descendant of the current sheet whose Visio `ID` is `visio_id`
    /// (a `Sheet.8` part on a master's sheet, which sits on no page).
    fn descendant_by_visio_id(&self, current: Target, visio_id: i32) -> Option<ShapeId> {
        let root = self.sheet_shape(current)?;
        let children: Vec<ShapeId> = self
            .document
            .shape(root)
            .map(|shape| shape.children.clone())
            .unwrap_or_default();
        first_by_visio_id(self.document, &children, visio_id)
    }

    /// The shape whose sheet the current target carries, if any.
    fn sheet_shape(&self, current: Target) -> Option<ShapeId> {
        match current {
            Target::Sheet(SheetRef::Shape(shape)) => Some(shape),
            _ => None,
        }
    }

    /// The page the current target sits on.
    fn page_context(&self, current: Target) -> Option<PageId> {
        match current {
            Target::Sheet(SheetRef::Page(page)) => Some(page),
            Target::Sheet(SheetRef::Shape(shape)) => self.document.page_of(shape),
            Target::Style(_) | Target::Document => None,
        }
    }

    /// The stylesheet row named `name`, matched case-insensitively.
    fn style_named(&self, name: &str) -> Option<StyleId> {
        self.document
            .stylesheet
            .styles
            .iter()
            .find(|style| eq_ci(&style.name, name) || eq_ci(&style.name_u, name))
            .map(|style| style.id)
    }

    /// The master named `name`, matched case-insensitively.
    fn master_named(&self, name: &str) -> Option<MasterId> {
        self.document
            .masters
            .iter()
            .enumerate()
            .find(|(_, master)| eq_ci(&master.name, name) || eq_ci(&master.name_u, name))
            .map(|(index, _)| MasterId {
                index: u32::try_from(index).unwrap_or(u32::MAX),
            })
    }

    /// The page named `name`, matched case-insensitively.
    fn page_named(&self, name: &str) -> Option<PageId> {
        self.document
            .pages
            .iter()
            .find(|page| eq_ci(&page.name, name) || eq_ci(&page.name_u, name))
            .map(|page| page.id)
    }

    /// What a reference names inside its target (ADR 0054).
    fn find(&self, sheet: SheetRef, reference: &CellRef) -> Option<Found> {
        let target = self.target(sheet, &reference.sheets)?;
        let (section, names) = split_names(reference);
        match target {
            Target::Document => self.find_document_cell(section, names),
            Target::Style(style) => {
                let style = self.document.style(style)?;
                match section {
                    Some(section) => self.find_section_in(&style.sheet, &[], section, names),
                    None => self.find_bare_in(&style.sheet, names),
                }
            }
            Target::Sheet(target) => self.find_sheet_cell(target, section, names),
        }
    }

    /// A document-sheet cell (`TheDoc!PreviewQuality`). The model keeps
    /// the document sheet as its `properties`, so a property by that name
    /// is the cell.
    fn find_document_cell(&self, section: Option<&str>, names: &[String]) -> Option<Found> {
        if section.is_some() {
            return None;
        }
        let name = names.first()?;
        let property = self
            .document
            .properties
            .iter()
            .find(|property| eq_ci(&property.name, name));
        if let Some(property) = property {
            return Some(Found {
                formula: None,
                source: ValueSource::Local,
                value: Some((CellValue::Str(property.value.clone()), Unit::Unitsless)),
            });
        }
        Some(Found {
            formula: None,
            source: ValueSource::HardDefault,
            value: self.table_default(None, name),
        })
    }

    /// A bare cell name on a sheet with no rungs behind it (a style's
    /// sheet): the sheet's own entry, then the table default.
    fn find_bare_in(&self, sheet: &Sheet, names: &[String]) -> Option<Found> {
        let name = names.first()?;
        if let Some((cell, _)) = find_entry(sheet, name) {
            let value = cell.value.clone().map(|value| (value, cell.unit));
            if defines(cell) {
                return Some(Found {
                    formula: cell.formula.clone(),
                    source: ValueSource::Local,
                    value,
                });
            }
            if value.is_some() {
                // An `Inh` snapshot is this sheet's own value (ADR 0054).
                return Some(Found {
                    formula: None,
                    source: ValueSource::HardDefault,
                    value,
                });
            }
        }
        self.table_default(None, name).map(|value| Found {
            formula: None,
            source: ValueSource::HardDefault,
            value: Some(value),
        })
    }

    /// One cell of a shape or page sheet.
    fn find_sheet_cell(
        &self,
        target: SheetRef,
        section: Option<&str>,
        names: &[String],
    ) -> Option<Found> {
        match section {
            Some(section) => self.find_section_cell(target, section, names),
            None => self.find_bare_cell(target, names),
        }
    }

    /// A bare cell name (`Width`, `Sheet.8!FillForegnd`): the model's
    /// lookup runs the rungs of ADR 0013 for it. The local entry's own
    /// cached value wins over the rung's (the `Inh` snapshot rule), and a
    /// case-only spelling difference still resolves.
    fn find_bare_cell(&self, target: SheetRef, names: &[String]) -> Option<Found> {
        let name = names.first()?;
        let sheet = self.sheet_of(target)?;
        let own = find_entry(sheet, name);
        let spelling = own
            .map(|(cell, _)| cell.name.as_str().to_owned())
            .or_else(|| table_spelling(name))
            .unwrap_or_else(|| name.clone());
        let cell_name = CellName::new(spelling);
        let resolved = match target {
            SheetRef::Shape(shape) => self.document.resolve(shape, &cell_name).ok()?,
            SheetRef::Page(page) => self.document.resolve_page_cell(page, &cell_name).ok()?,
        };
        if resolved.value.is_none() && resolved.formula.is_none() {
            // No rung and no table entry: the name resolves to nothing.
            return None;
        }
        let snapshot = own
            .and_then(|(cell, _)| cell.value.clone().map(|value| (value, cell.unit)))
            .or_else(|| {
                resolved
                    .value
                    .cloned()
                    .map(|value| (value, self.rung_unit(target, &cell_name)))
            });
        Some(Found {
            formula: resolved.formula.map(str::to_owned),
            source: resolved.source,
            value: snapshot,
        })
    }

    /// The unit the rung that defines `cell` recorded, when the resolver
    /// can find that rung's cell. A style or default rung keeps no cell
    /// the resolver can name, so its value reads back unit-less.
    fn rung_unit(&self, target: SheetRef, cell: &CellName) -> Unit {
        let Some(sheet) = self.sheet_of(target) else {
            return Unit::Unitsless;
        };
        if let Some((found, _)) = find_entry(sheet, cell.as_str())
            && defines(found)
        {
            return found.unit;
        }
        if let SheetRef::Shape(shape) = target {
            for (_, rung) in master_rungs(self.document, shape) {
                let Some(rung_sheet) = self.document.shape(rung).map(|shape| &shape.sheet) else {
                    continue;
                };
                if let Some((found, _)) = find_entry(rung_sheet, cell.as_str())
                    && defines(found)
                {
                    return found.unit;
                }
            }
        }
        Unit::Unitsless
    }

    /// A section-form reference (`Char.Size`, `Prop.Cost`, `Scratch.A5`,
    /// `Geometry1.X2`): the section's row shape decides whether the name
    /// is a column or a named row (ADR 0054).
    fn find_section_cell(
        &self,
        target: SheetRef,
        section: &str,
        names: &[String],
    ) -> Option<Found> {
        let sheet = self.sheet_of(target)?;
        let rungs = match target {
            SheetRef::Shape(shape) => master_rungs(self.document, shape),
            SheetRef::Page(_) => Vec::new(),
        };
        self.find_section_in(sheet, &rungs, section, names)
    }

    /// The section walk behind [`Self::find_section_cell`], over one
    /// sheet and the master rungs behind it (empty for a page sheet or a
    /// style's sheet).
    fn find_section_in(
        &self,
        sheet: &Sheet,
        rungs: &[(MasterId, ShapeId)],
        section: &str,
        names: &[String],
    ) -> Option<Found> {
        let name = names.first()?;
        let explicit = names.get(1).map(String::as_str);
        let (base, instance) = split_section_name(section);
        let canon = canonical_section(base.as_str());
        let parts = split_row_name(name);

        // The row shape: a column of the section's rows (the ShapeSheet
        // table, or the cells the file itself writes in those rows), else
        // a named row of a named-row section.
        let local_section = match_section(sheet, canon.as_str(), instance);
        let column = column_known(canon.as_str(), local_section, parts.base.as_str());
        let kind = if column {
            RowKind::Column {
                column: parts.base.clone(),
                index: parts.index,
            }
        } else {
            RowKind::Named(parts.full.clone())
        };

        let mut snapshot = None;
        let mut saw_row = false;
        let mut saw_cell = false;

        // 1. Local rung.
        if let Some(section) = local_section {
            if row_known(section, &kind) {
                saw_row = true;
            }
            if let Some(cell) = section_cell(section, &kind, explicit) {
                saw_cell = true;
                if cell.value.is_some() {
                    snapshot = cell.value.clone().map(|value| (value, cell.unit));
                }
                if defines(cell) {
                    return Some(Found {
                        formula: cell.formula.clone(),
                        source: ValueSource::Local,
                        value: cell.value.clone().map(|value| (value, cell.unit)),
                    });
                }
            }
        }

        // 2. Master rungs, with the sub-shape and position rules of
        //    ADR 0013 (no style rung for a section cell, ADR 0019).
        for (master, rung) in rungs {
            let Some(rung_sheet) = self.document.shape(*rung).map(|shape| &shape.sheet) else {
                continue;
            };
            let Some(section) = match_section(rung_sheet, canon.as_str(), instance) else {
                continue;
            };
            if row_known(section, &kind) {
                saw_row = true;
            }
            if let Some(cell) = section_cell(section, &kind, explicit) {
                saw_cell = true;
                if defines(cell) {
                    let value =
                        snapshot.or_else(|| cell.value.clone().map(|value| (value, cell.unit)));
                    return Some(Found {
                        formula: cell.formula.clone(),
                        source: ValueSource::Master(*master),
                        value,
                    });
                }
            }
        }

        // 3./4. The section default and the hard default, for a name the
        //       row shape knows. Row 0 is the section's own first row and
        //       exists conceptually; a named row or a higher row must
        //       exist on some rung (ADR 0054: a missing row is `#REF!`).
        let row_ok = match &kind {
            RowKind::Column { index, .. } => *index == 0 || saw_row,
            RowKind::Named(_) => saw_row,
        };
        if !row_ok {
            return None;
        }
        let known = saw_cell || table_column(canon.as_str(), kind.cell_name());
        if !known {
            return None;
        }
        Some(Found {
            formula: None,
            source: ValueSource::HardDefault,
            value: self.table_default(Some(canon.as_str()), kind.cell_name()),
        })
    }

    /// The sheet of a shape or page target.
    fn sheet_of(&self, target: SheetRef) -> Option<&Sheet> {
        match target {
            SheetRef::Shape(shape) => self.document.shape(shape).map(|shape| &shape.sheet),
            SheetRef::Page(page) => self.document.page(page).map(|page| &page.sheet),
        }
    }

    /// The hard default of a table cell, as a cached value.
    fn table_default(&self, section: Option<&str>, name: &str) -> Option<(CellValue, Unit)> {
        let kind = column_kind(section, name)?;
        hard_default(kind)
    }

    /// Evaluates the formula a reference's cell defines, in `sheet`'s
    /// context (ADR 0054: an inherited formula binds local names to the
    /// sheet it is evaluated *for*).
    fn eval_reference(&mut self, sheet: SheetRef, reference: &CellRef) -> Result<Value, Error> {
        let resolved = self.resolve(sheet, reference)?;
        if let Some(source) = resolved.formula.as_deref() {
            let formula = Formula::new(source);
            if let Ok(value) = crate::eval::eval_formula(&formula, sheet, self) {
                return Ok(value);
            }
            // An engine that is not ready yet, or a stored formula that
            // does not parse: keep the cached value (ADR 0053's rule).
        }
        Ok(resolved.value.unwrap_or(Value::Err(ErrorCode::Ref)))
    }
}

impl EvalContext for DocumentResolver<'_> {
    fn cell_value(&mut self, sheet: SheetRef, reference: &CellRef) -> Result<Value, Error> {
        if self.depth >= MAX_EVAL_DEPTH {
            // A formula chain deeper than the bound keeps its cached
            // values: the resolver never recurses away.
            return Ok(Value::Err(ErrorCode::Ref));
        }
        self.depth += 1;
        let outcome = self.eval_reference(sheet, reference);
        self.depth -= 1;
        outcome
    }

    fn set_cell(
        &mut self,
        sheet: SheetRef,
        reference: &CellRef,
        value: Value,
    ) -> Result<(), Error> {
        let cached = value.to_cached()?;
        let target = self
            .target(sheet, &reference.sheets)
            .ok_or(Error::Conversion("SETF target does not exist"))?;
        let target = match target {
            Target::Sheet(target) => target,
            Target::Style(_) | Target::Document => {
                return Err(Error::Conversion("SETF target is not writable"));
            }
        };
        let (section, names) = split_names(reference);
        let slot = {
            let found_sheet = self
                .sheet_of(target)
                .ok_or(Error::Conversion("SETF target does not exist"))?;
            match section {
                Some(section) => {
                    let name = names
                        .first()
                        .ok_or(Error::Conversion("SETF target does not exist"))?;
                    let explicit = names.get(1).map(String::as_str);
                    let (base, instance) = split_section_name(section);
                    let canon = canonical_section(base.as_str());
                    let parts = split_row_name(name);
                    let column = column_known(canon.as_str(), None, parts.base.as_str());
                    let kind = if column {
                        RowKind::Column {
                            column: parts.base.clone(),
                            index: parts.index,
                        }
                    } else {
                        RowKind::Named(parts.full.clone())
                    };
                    section_slot(found_sheet, canon.as_str(), instance, &kind, explicit)
                        .ok_or(Error::Conversion("SETF target does not exist"))?
                }
                None => {
                    let name = names
                        .first()
                        .ok_or(Error::Conversion("SETF target does not exist"))?;
                    let (_, slot) = find_entry(found_sheet, name)
                        .ok_or(Error::Conversion("SETF target does not exist"))?;
                    slot
                }
            }
        };
        let found_sheet = sheet_mut(self.document, target)
            .ok_or(Error::Conversion("SETF target does not exist"))?;
        write_slot(found_sheet, &slot, cached)
            .ok_or(Error::Conversion("SETF target does not exist"))
    }

    fn warn(&mut self, warning: EvalWarning) {
        self.warnings.push(warning);
    }
}

/// What one resolution found (ADR 0054): the formula of the rung that
/// defines the cell, which rung that is, and the value to use.
#[derive(Debug, Clone, PartialEq)]
struct Found {
    /// The defining formula's source text.
    formula: Option<String>,
    /// The rung that defines the cell.
    source: ValueSource,
    /// The cached value and the unit it was written with.
    value: Option<(CellValue, Unit)>,
}

/// The target a reference's sheet parts name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    /// A shape or page sheet.
    Sheet(SheetRef),
    /// A stylesheet row's sheet.
    Style(StyleId),
    /// The document sheet.
    Document,
}

/// Where a section-form name points (ADR 0054): a column of the section's
/// row shape at a row index, or a named row.
#[derive(Debug, Clone, PartialEq)]
enum RowKind {
    /// A column of the row shape at a row index (`X2`, `Font[3]`, `Size`).
    Column { column: String, index: usize },
    /// A named row (`Cost`).
    Named(String),
}

impl RowKind {
    /// The cell name this kind reads: the column, or a named row's default
    /// cell (`Value`).
    fn cell_name(&self) -> &str {
        match self {
            RowKind::Column { column, .. } => column.as_str(),
            RowKind::Named(_) => DEFAULT_ROW_CELL,
        }
    }
}

/// The parts of one dotted name after the section (`A5`, `Font[3]`,
/// `Cost`): the leading name and the row index its trailing digits or
/// `[n]` carry (ADR 0054).
#[derive(Debug, Clone, PartialEq)]
struct NameParts {
    /// The name without the row index.
    base: String,
    /// The row index (default 0).
    index: usize,
    /// The name as written.
    full: String,
}

/// Where one cell sits on one sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    /// A direct cell of the sheet, at this index.
    Direct(usize),
    /// A section-level cell: section index, cell index.
    SectionLevel(usize, usize),
    /// A row cell: section index, row index, cell index.
    Row(usize, usize, usize),
}

/// Splits a written name into its row-shape base and row index.
fn split_row_name(part: &str) -> NameParts {
    let full = part.to_owned();
    if part.ends_with(']')
        && let Some(open) = part.rfind('[')
        && let Some(inside) = part.get(open + 1..part.len().saturating_sub(1))
        && let Ok(index) = inside.parse::<usize>()
        && open > 0
    {
        return NameParts {
            base: part.get(..open).unwrap_or("").to_owned(),
            index,
            full,
        };
    }
    let digits = part
        .chars()
        .rev()
        .take_while(|character| character.is_ascii_digit())
        .count();
    if digits > 0 && digits < part.len() {
        let (base, tail) = part.split_at(part.len() - digits);
        if let Ok(index) = tail.parse::<usize>() {
            return NameParts {
                base: base.to_owned(),
                index,
                full,
            };
        }
    }
    NameParts {
        base: full.clone(),
        index: 0,
        full,
    }
}

/// Splits a written section name into its base and its instance index
/// (`Geometry1` is the second `Geometry` section).
fn split_section_name(part: &str) -> (String, usize) {
    let digits = part
        .chars()
        .rev()
        .take_while(|character| character.is_ascii_digit())
        .count();
    if digits > 0 && digits < part.len() {
        let (base, tail) = part.split_at(part.len() - digits);
        if let Ok(index) = tail.parse::<usize>() {
            return (base.to_owned(), index);
        }
    }
    (part.to_owned(), 0)
}

/// The section a written section name names: the file spelling and the
/// ShapeSheet-table spelling match, and the short formula spellings
/// (`Char`, `Prop`) map to their sections.
fn canonical_section(name: &str) -> String {
    let lowered = name.to_lowercase();
    let mapped: &str = match lowered.as_str() {
        "char" | "character" => "Character",
        "prop" | "property" | "shape data" => "Shape Data",
        "user" | "user-defined cells" => "User-defined Cells",
        other => other,
    };
    if mapped == lowered {
        // A file spelling with its own alias (`Property` -> `Shape Data`).
        return ferrogram_model::aliases::table_name(name).to_owned();
    }
    mapped.to_owned()
}

/// Whether two written names are the same name (ADR 0054: names match
/// case-insensitively).
fn eq_ci(left: &str, right: &str) -> bool {
    left.to_lowercase() == right.to_lowercase()
}

/// Removes the quoting of a nonstandard name (`'1-D'`, with `''` as the
/// quote escape) and returns the name itself.
fn unquote_part(part: &str) -> String {
    let unescaped = part.replace("''", "'");
    if unescaped.len() >= 2
        && unescaped.starts_with('\'')
        && unescaped.ends_with('\'')
        && let Some(inner) = unescaped.get(1..unescaped.len() - 1)
    {
        return inner.to_owned();
    }
    unescaped
}

/// Splits `Masters[Gear]` into the name inside its brackets when the part
/// carries `prefix`.
fn bracketed(part: &str, prefix: &str) -> Option<String> {
    let rest = part.strip_prefix(prefix)?.strip_prefix('[')?;
    let name = rest.strip_suffix(']')?;
    Some(unquote_part(name))
}

/// Splits `Sheet.8` or `Executive.2` into the written name and the
/// numeric suffix. A part with no numeric suffix comes back whole.
fn split_numbered(part: &str) -> (String, Option<i32>) {
    let Some(dot) = part.rfind('.') else {
        return (part.to_owned(), None);
    };
    let tail = part.get(dot + 1..).unwrap_or("");
    if tail.is_empty() || !tail.chars().all(|character| character.is_ascii_digit()) {
        return (part.to_owned(), None);
    }
    match tail.parse::<i32>() {
        Ok(visio_id) => (part.get(..dot).unwrap_or("").to_owned(), Some(visio_id)),
        Err(_) => (part.to_owned(), None),
    }
}

/// The reference as written, for the warning a missing name records.
fn describe(reference: &CellRef) -> String {
    let mut text = reference.sheets.join("!");
    if !text.is_empty() {
        text.push('!');
    }
    if let Some(section) = reference.section.as_deref() {
        text.push_str(section);
        text.push('.');
    }
    text.push_str(&reference.names.join("."));
    text
}

/// Splits a reference's written names into the section part and the name
/// parts (`Prop.Cost.Value` is section `Prop`, names `Cost`, `Value`).
fn split_names(reference: &CellRef) -> (Option<&str>, &[String]) {
    if let Some(section) = reference.section.as_deref() {
        return (Some(section), &reference.names[..]);
    }
    if reference.names.len() > 1
        && let Some(first) = reference.names.first()
    {
        // A section that a parser left in the name list.
        return (
            Some(first.as_str()),
            reference.names.get(1..).unwrap_or(&[]),
        );
    }
    (None, &reference.names[..])
}

/// Whether a cell defines its source (ADR 0013): `Inh` defines nothing.
fn defines(cell: &Cell) -> bool {
    if cell.formula.as_deref() == Some(INHERIT_FORMULA) {
        return false;
    }
    cell.value.is_some() || cell.formula.is_some()
}

/// The cell of `sheet` named `name` and where it sits: the sheet's own
/// cells first, then each section's level cells and row cells, in file
/// order (the lookup's local rung, matched case-insensitively).
fn find_entry<'a>(sheet: &'a Sheet, name: &str) -> Option<(&'a Cell, Slot)> {
    if let Some(index) = sheet
        .cells
        .iter()
        .position(|cell| eq_ci(cell.name.as_str(), name))
    {
        return sheet
            .cells
            .get(index)
            .map(|cell| (cell, Slot::Direct(index)));
    }
    for (section_index, section) in sheet.sections.iter().enumerate() {
        if let Some(cell_index) = section
            .cells
            .iter()
            .position(|cell| eq_ci(cell.name.as_str(), name))
        {
            return section
                .cells
                .get(cell_index)
                .map(|cell| (cell, Slot::SectionLevel(section_index, cell_index)));
        }
        for (row_index, row) in section.rows.iter().enumerate() {
            if let Some(cell_index) = row
                .cells
                .iter()
                .position(|cell| eq_ci(cell.name.as_str(), name))
            {
                return row
                    .cells
                    .get(cell_index)
                    .map(|cell| (cell, Slot::Row(section_index, row_index, cell_index)));
            }
        }
    }
    None
}

/// The sections of `sheet` the written section name names, in file order.
fn sections_named<'a>(sheet: &'a Sheet, canon: &str) -> impl Iterator<Item = &'a Section> {
    sheet
        .sections
        .iter()
        .filter(move |section| eq_ci(canonical_section(section.name.as_str()).as_str(), canon))
}

/// The section instance a section-form name names (`Geometry1` is the
/// second `Geometry` section).
fn match_section<'a>(sheet: &'a Sheet, canon: &str, instance: usize) -> Option<&'a Section> {
    sections_named(sheet, canon).nth(instance)
}

/// The row index a [`RowKind`] names in one section (ADR 0054): a row
/// index is the row at that position, or the row carrying its `IX`; a
/// named row is the row with that `N`.
fn row_index_of(section: &Section, kind: &RowKind) -> Option<usize> {
    match kind {
        RowKind::Column { index, .. } => {
            let candidate = section.rows.get(*index)?;
            match candidate.ix {
                Some(ix) => Some(
                    section
                        .rows
                        .iter()
                        .position(|row| row.ix == Some(ix))
                        .unwrap_or(*index),
                ),
                None => Some(*index),
            }
        }
        RowKind::Named(name) => section
            .rows
            .iter()
            .position(|row| row.name.as_deref().is_some_and(|row| eq_ci(row, name))),
    }
}

/// The row a [`RowKind`] names in one section.
fn match_row<'a>(section: &'a Section, kind: &RowKind) -> Option<&'a Row> {
    let index = row_index_of(section, kind)?;
    section.rows.get(index)
}

/// Whether the section carries the row the kind names.
fn row_known(section: &Section, kind: &RowKind) -> bool {
    match_row(section, kind).is_some()
}

/// The cell a section-form name names on one section: the section's own
/// cells first (ADR 0013 amendment), then the row's cells.
fn section_cell<'a>(
    section: &'a Section,
    kind: &RowKind,
    explicit: Option<&str>,
) -> Option<&'a Cell> {
    let name = explicit.unwrap_or_else(|| kind.cell_name());
    if let Some(found) = section
        .cells
        .iter()
        .find(|cell| eq_ci(cell.name.as_str(), name))
    {
        return Some(found);
    }
    let row = match_row(section, kind)?;
    row.cells
        .iter()
        .find(|cell| eq_ci(cell.name.as_str(), name))
}

/// The slot a section-form name writes to: the section and row of the
/// sheet, and the cell [`section_cell`] would find there.
fn section_slot(
    sheet: &Sheet,
    canon: &str,
    instance: usize,
    kind: &RowKind,
    explicit: Option<&str>,
) -> Option<Slot> {
    let name = explicit.unwrap_or_else(|| kind.cell_name());
    let (section_index, section) = sections_named(sheet, canon).enumerate().nth(instance)?;
    if let Some(cell) = section
        .cells
        .iter()
        .position(|cell| eq_ci(cell.name.as_str(), name))
    {
        return Some(Slot::SectionLevel(section_index, cell));
    }
    let row_index = row_index_of(section, kind)?;
    let row = section.rows.get(row_index)?;
    let cell = row
        .cells
        .iter()
        .position(|cell| eq_ci(cell.name.as_str(), name))?;
    Some(Slot::Row(section_index, row_index, cell))
}

/// Whether the name is a column of the section's row shape: the ShapeSheet
/// table, or a cell the file itself writes in those rows (the table has
/// gaps; ADR 0060's deferred items).
fn column_known(canon: &str, section: Option<&Section>, name: &str) -> bool {
    table_column(canon, name)
        || section.is_some_and(|section| {
            section
                .cells
                .iter()
                .any(|cell| eq_ci(cell.name.as_str(), name))
                || section
                    .rows
                    .iter()
                    .any(|row| row.cells.iter().any(|cell| eq_ci(cell.name.as_str(), name)))
        })
}

/// Whether the ShapeSheet table lists `name` as a cell of `canon`.
fn table_column(canon: &str, name: &str) -> bool {
    column_kind(Some(canon), name).is_some()
}

/// The cell type the ShapeSheet table records for `name`, when it knows
/// the cell.
fn column_kind(section: Option<&str>, name: &str) -> Option<&'static str> {
    let mut fallback = None;
    for def in ferrogram_model::generated::CELLS {
        if !eq_ci(def.name, name) {
            continue;
        }
        match section {
            Some(section) if eq_ci(def.section, section) => return Some(def.kind),
            Some(_) => {}
            None if fallback.is_none() => fallback = Some(def.kind),
            None => {}
        }
    }
    fallback
}

/// The spelling the ShapeSheet table uses for a cell name, when it knows
/// one (the model's table lookup matches spelling exactly).
fn table_spelling(name: &str) -> Option<String> {
    ferrogram_model::generated::CELLS
        .iter()
        .find(|def| eq_ci(def.name, name))
        .map(|def| def.name.to_owned())
}

/// The hard default of one cell type (ADR 0013: 0, empty string, false,
/// black), in the canonical units of ADR 0011.
fn hard_default(kind: &str) -> Option<(CellValue, Unit)> {
    let value = match kind {
        "Number" => CellValue::Number(0.0),
        "String" => CellValue::Str(String::new()),
        "Bool" => CellValue::Bool(false),
        "Color" => CellValue::Color(ferrogram_model::Color::new(0, 0, 0)),
        "List" => CellValue::List(String::new()),
        _ => return None,
    };
    Some((value, Unit::Unitsless))
}

/// The master rungs of `shape` in rung order (ADR 0013): the sub-shape
/// the instance's path walks down to (or the master's own shape), once per
/// rung, with the position rule for masters with several top-level
/// shapes. A cyclic master chain is cut at the first repeat and the walk
/// is bounded.
fn master_rungs(doc: &Document, shape: ShapeId) -> Vec<(MasterId, ShapeId)> {
    const MAX_RUNGS: usize = 16;
    let mut rungs = Vec::new();
    let Some((first, root, mut path)) = instance_chain_start(doc, shape) else {
        return rungs;
    };
    let position = sibling_position(doc, root);
    let mut visited: Vec<MasterId> = Vec::new();
    let mut current = first;
    loop {
        if visited.contains(&current) || rungs.len() >= MAX_RUNGS {
            break;
        }
        visited.push(current);
        let mut chained = None;
        if let Some(content) = doc
            .master(current)
            .and_then(|master| master_shape(doc, master, position))
        {
            let target = walk_down(doc, content, &path).unwrap_or(content);
            rungs.push((current, target.id));
            chained = target.master;
            path.clear();
        }
        match chained {
            Some(next) => current = next,
            None => break,
        }
    }
    rungs
}

/// The master of the nearest shape that is an instance (the shape itself
/// or an ancestor), that instance root, and the child positions from the
/// root down to `shape` (the lookup's chain start).
fn instance_chain_start(doc: &Document, shape: ShapeId) -> Option<(MasterId, ShapeId, Vec<usize>)> {
    let mut path = Vec::new();
    let mut visited = Vec::new();
    let mut current = shape;
    loop {
        if visited.contains(&current) {
            return None;
        }
        visited.push(current);
        let node = doc.shape(current)?;
        if let Some(master) = node.master {
            path.reverse();
            return Some((master, current, path));
        }
        let parent = node.parent?;
        let position = doc
            .shape(parent)?
            .children
            .iter()
            .position(|child| *child == current)?;
        path.push(position);
        current = parent;
    }
}

/// The position of `shape` among its siblings (the lookup's position
/// rule).
fn sibling_position(doc: &Document, shape: ShapeId) -> Option<usize> {
    let node = doc.shape(shape)?;
    if let Some(parent) = node.parent {
        return doc
            .shape(parent)?
            .children
            .iter()
            .position(|child| *child == shape);
    }
    doc.pages
        .iter()
        .find_map(|page| page.shapes.iter().position(|entry| *entry == shape))
}

/// The master shape an instance at `position` matches (P2-06b decision 2):
/// the master's only shape when it has exactly one, the shape at
/// `position` otherwise, degrading to the first.
fn master_shape<'a>(
    doc: &'a Document,
    master: &Master,
    position: Option<usize>,
) -> Option<&'a Shape> {
    match master.shapes.as_slice() {
        [] => None,
        [only] => doc.shape(*only),
        many => {
            let picked = position
                .and_then(|position| many.get(position))
                .or_else(|| many.first())?;
            doc.shape(*picked)
        }
    }
}

/// The corresponding shape of `root` at the child positions `path`, or
/// `None` when the structure does not match.
fn walk_down<'a>(doc: &'a Document, root: &'a Shape, path: &[usize]) -> Option<&'a Shape> {
    let mut current = root;
    for position in path {
        let child = *current.children.get(*position)?;
        current = doc.shape(child)?;
    }
    Some(current)
}

/// The master's content shape: its only shape, else its first.
fn master_content(doc: &Document, master: MasterId) -> Option<ShapeId> {
    let master = doc.master(master)?;
    master_shape(doc, master, Some(0)).map(|shape| shape.id)
}

/// The first shape of `roots` (a depth-first walk, bounded by the arena)
/// whose name matches `name` case-insensitively.
fn first_named(doc: &Document, roots: &[ShapeId], name: &str) -> Option<ShapeId> {
    walk_shapes(doc, roots, |shape| {
        eq_ci(&shape.name, name)
            || eq_ci(&shape.name_u, name)
            || shape
                .name_u
                .rsplit_once('.')
                .is_some_and(|(base, _)| eq_ci(base, name))
    })
}

/// The first shape of `roots` whose Visio `ID` is `visio_id`, preferring
/// one whose written name matches `name`.
fn shape_by_visio_id(doc: &Document, page: PageId, visio_id: i32, name: &str) -> Option<ShapeId> {
    let roots = doc.page(page)?.shapes.clone();
    let by_id = |shape: &Shape| shape.visio_id == visio_id;
    let named = |shape: &Shape| {
        by_id(shape)
            && (name == "Sheet"
                || eq_ci(&shape.name, name)
                || shape
                    .name_u
                    .rsplit_once('.')
                    .is_some_and(|(base, _)| eq_ci(base, name)))
    };
    walk_shapes(doc, &roots, named).or_else(|| walk_shapes(doc, &roots, by_id))
}

/// The first shape of `roots` whose Visio `ID` is `visio_id` (the master
/// fallback of a `Sheet.8` part).
fn first_by_visio_id(doc: &Document, roots: &[ShapeId], visio_id: i32) -> Option<ShapeId> {
    walk_shapes(doc, roots, |shape| shape.visio_id == visio_id)
}

/// The first shape in a bounded depth-first walk of the shape tree that
/// matches `wanted`.
fn walk_shapes<F>(doc: &Document, roots: &[ShapeId], wanted: F) -> Option<ShapeId>
where
    F: Fn(&Shape) -> bool,
{
    const MAX_VISITED: usize = 100_000;
    let mut stack: Vec<ShapeId> = roots.iter().rev().copied().collect();
    let mut visited = 0;
    while let Some(id) = stack.pop() {
        visited += 1;
        if visited > MAX_VISITED {
            return None;
        }
        let Some(shape) = doc.shape(id) else {
            continue;
        };
        if wanted(shape) {
            return Some(id);
        }
        stack.extend(shape.children.iter().rev().copied());
    }
    None
}

/// The mutable sheet of a shape or page target.
fn sheet_mut(doc: &mut Document, target: SheetRef) -> Option<&mut Sheet> {
    match target {
        SheetRef::Shape(shape) => doc.shape_mut(shape).map(|shape| &mut shape.sheet),
        SheetRef::Page(page) => doc
            .pages
            .iter_mut()
            .find(|entry| entry.id == page)
            .map(|page| &mut page.sheet),
    }
}

/// Writes one cached value into the cell at `slot` (basic `SETF`, ADR
/// 0056): the cell keeps the value as a literal.
fn write_slot(sheet: &mut Sheet, slot: &Slot, cached: crate::value::CachedValue) -> Option<()> {
    let cell = match slot {
        Slot::Direct(index) => sheet.cells.get_mut(*index),
        Slot::SectionLevel(section, cell) => sheet.sections.get_mut(*section)?.cells.get_mut(*cell),
        Slot::Row(section, row, cell) => sheet
            .sections
            .get_mut(*section)?
            .rows
            .get_mut(*row)?
            .cells
            .get_mut(*cell),
    }?;
    cell.formula = None;
    cell.value = Some(cached.value);
    cell.unit = cached.unit;
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::EvalWarningKind;
    use ferrogram_model::{Page, PageId, Section, SectionName, Shape};

    fn sheet() -> SheetRef {
        SheetRef::Page(PageId { index: 0 })
    }

    fn number_cell(name: &str, value: f64) -> Cell {
        let mut cell = Cell::new(CellName::new(name));
        cell.value = Some(CellValue::Number(value));
        cell
    }

    #[test]
    fn a_resolver_holds_its_document_and_warnings() {
        let mut doc = Document::new();
        let mut resolver = DocumentResolver::new(&mut doc);
        assert!(resolver.warnings().is_empty());
        resolver.warn(EvalWarning {
            sheet: sheet(),
            kind: EvalWarningKind::Cycle,
        });
        assert_eq!(resolver.warnings().len(), 1);
        assert_eq!(resolver.into_warnings().len(), 1);
    }

    #[test]
    fn the_document_is_borrowable() {
        let mut doc = Document::new();
        let resolver = DocumentResolver::new(&mut doc);
        assert_eq!(resolver.document().pages.len(), 0);
    }

    #[test]
    fn a_resolved_cell_holds_provenance() {
        let cell = ResolvedCell {
            formula: None,
            value: None,
            source: ValueSource::Local,
        };
        assert_eq!(cell.source, ValueSource::Local);
    }

    #[test]
    fn row_names_split_the_column_and_the_row_index() {
        assert_eq!(
            split_row_name("X2"),
            NameParts {
                base: String::from("X"),
                index: 2,
                full: String::from("X2"),
            }
        );
        assert_eq!(
            split_row_name("Font[3]"),
            NameParts {
                base: String::from("Font"),
                index: 3,
                full: String::from("Font[3]"),
            }
        );
        assert_eq!(split_row_name("Size").index, 0);
        // A name whose base is not a column keeps its digits as part of
        // the row name (`Cost5` is the row `Cost5`).
        assert_eq!(split_row_name("Cost5").full, "Cost5");
        assert_eq!(split_row_name("A[").full, "A[");
    }

    #[test]
    fn section_names_split_the_instance_index() {
        assert_eq!(
            split_section_name("Geometry1"),
            (String::from("Geometry"), 1)
        );
        assert_eq!(split_section_name("Char"), (String::from("Char"), 0));
    }

    #[test]
    fn section_names_come_in_short_and_long_spellings() {
        assert_eq!(canonical_section("Char"), "Character");
        assert_eq!(canonical_section("CHAR"), "Character");
        assert_eq!(canonical_section("Prop"), "Shape Data");
        assert_eq!(canonical_section("Property"), "Shape Data");
        assert_eq!(canonical_section("User"), "User-defined Cells");
        assert_eq!(canonical_section("Scratch"), "Scratch");
        assert_eq!(canonical_section("Geometry"), "Geometry");
    }

    #[test]
    fn quoted_names_unescape_their_quotes() {
        assert_eq!(unquote_part("'1-D'"), "1-D");
        assert_eq!(unquote_part("'A''B'"), "A'B");
        assert_eq!(unquote_part("Star"), "Star");
    }

    #[test]
    fn bracketed_parts_split_the_name() {
        assert_eq!(
            bracketed("Masters[Gear]", "Masters"),
            Some(String::from("Gear"))
        );
        assert_eq!(
            bracketed("Pages[Page-3]", "Pages"),
            Some(String::from("Page-3"))
        );
        assert_eq!(bracketed("Pages[Page-3]", "Masters"), None);
        assert_eq!(bracketed("Pages[]", "Pages"), Some(String::new()));
    }

    #[test]
    fn numbered_parts_split_the_visio_id() {
        assert_eq!(split_numbered("Sheet.8"), (String::from("Sheet"), Some(8)));
        assert_eq!(
            split_numbered("Executive.2"),
            (String::from("Executive"), Some(2))
        );
        assert_eq!(split_numbered("Star"), (String::from("Star"), None));
        assert_eq!(split_numbered("A.B"), (String::from("A.B"), None));
    }

    #[test]
    fn a_missing_name_describes_itself_in_the_warning() {
        let mut doc = Document::new();
        let mut resolver = DocumentResolver::new(&mut doc);
        let reference = CellRef {
            sheets: vec![String::from("Sheet.9")],
            section: Some(String::from("Geometry1")),
            names: vec![String::from("X2")],
        };
        let resolved = resolver.resolve(sheet(), &reference).expect("resolves");
        assert_eq!(resolved.value, Some(Value::Err(ErrorCode::Ref)));
        assert_eq!(
            resolver.warnings(),
            [EvalWarning {
                sheet: sheet(),
                kind: EvalWarningKind::UnsupportedReference(String::from("Sheet.9!Geometry1.X2")),
            }]
        );
    }

    #[test]
    fn hard_defaults_follow_the_cell_type() {
        assert_eq!(
            hard_default("Number"),
            Some((CellValue::Number(0.0), Unit::Unitsless))
        );
        assert_eq!(
            hard_default("String").map(|(value, _)| value),
            Some(CellValue::Str(String::new()))
        );
        assert_eq!(
            hard_default("Bool").map(|(value, _)| value),
            Some(CellValue::Bool(false))
        );
        assert_eq!(hard_default("Nope"), None);
    }

    #[test]
    fn the_table_knows_columns_and_spelling() {
        assert!(table_column("Character", "Size"));
        assert!(table_column("Shape Data", "Value"));
        assert!(!table_column("Shape Data", "Cost"));
        assert_eq!(table_spelling("size"), Some(String::from("Size")));
        assert_eq!(table_spelling("nosuch"), None);
    }

    #[test]
    fn section_level_cells_come_before_row_cells() {
        let mut section = Section::new(SectionName::from_static("Geometry"));
        section.cells.push(number_cell("NoFill", 1.0));
        let mut row = Row::new();
        row.cells.push(number_cell("X", 2.0));
        section.rows.push(row);
        let kind = RowKind::Column {
            column: String::from("NoFill"),
            index: 0,
        };
        let cell = section_cell(&section, &kind, None).expect("cell");
        assert_eq!(cell.name.as_str(), "NoFill");
        // The row's own column cell comes second.
        let kind = RowKind::Column {
            column: String::from("X"),
            index: 0,
        };
        assert_eq!(
            section_cell(&section, &kind, None).map(|c| c.name.as_str()),
            Some("X")
        );
    }

    #[test]
    fn named_rows_match_their_row_name() {
        let mut section = Section::new(SectionName::from_static("Property"));
        let mut row = Row::new();
        row.name = Some(String::from("Cost"));
        row.cells.push(number_cell("Value", 3.0));
        section.rows.push(row);
        let kind = RowKind::Named(String::from("cost"));
        assert_eq!(
            section_cell(&section, &kind, None).map(|cell| cell.value.clone()),
            Some(Some(CellValue::Number(3.0)))
        );
        assert_eq!(
            section_cell(&section, &RowKind::Named(String::from("Nope")), None),
            None
        );
    }

    #[test]
    fn a_row_index_matches_the_row_ix_first() {
        let mut section = Section::new(SectionName::from_static("Geometry"));
        for (position, ix) in [Some(7), None, Some(3)].into_iter().enumerate() {
            let mut row = Row::new();
            row.ix = ix;
            row.cells.push(number_cell("X", (position as u32).into()));
            section.rows.push(row);
        }
        // Row 0 carries `IX` 7: the row with `IX` 7 matches wherever it
        // sits (here, row 0 itself).
        let kind = RowKind::Column {
            column: String::from("X"),
            index: 0,
        };
        assert_eq!(
            section_cell(&section, &kind, None).and_then(|cell| cell.value.clone()),
            Some(CellValue::Number(0.0))
        );
    }

    #[test]
    fn a_document_sheet_cell_reads_the_property() {
        let mut doc = Document::new();
        doc.properties
            .push(ferrogram_model::Property::new("PreviewQuality", "1"));
        let mut resolver = DocumentResolver::new(&mut doc);
        let reference = CellRef {
            sheets: vec![String::from("TheDoc")],
            section: None,
            names: vec![String::from("previewquality")],
        };
        let resolved = resolver.resolve(sheet(), &reference).expect("resolves");
        assert_eq!(resolved.source, ValueSource::Local);
        assert_eq!(resolved.formula, None);
    }

    #[test]
    fn writes_go_into_the_local_cell() {
        let mut doc = Document::new();
        let shape = doc.add_shape(Shape::new(ShapeId { index: 0 }, 1));
        doc.shape_mut(shape)
            .expect("exists")
            .sheet
            .cells
            .push(number_cell("PinX", 1.0));
        let mut page = Page::new(PageId { index: 0 }, 1);
        page.shapes.push(shape);
        doc.pages.push(page);
        let mut resolver = DocumentResolver::new(&mut doc);
        let reference = CellRef::new("PinX");
        // The value mapping belongs to P6-03: the write succeeds once the
        // mapping does, and refuses before that.
        let value = Value::Number(crate::value::Number::new(5.0, crate::units::Unit::Inches));
        match value.to_cached() {
            Ok(_) => {
                resolver
                    .set_cell(SheetRef::Shape(shape), &reference, value)
                    .expect("writes");
            }
            Err(_) => {
                assert!(
                    resolver
                        .set_cell(SheetRef::Shape(shape), &reference, value)
                        .is_err()
                );
            }
        }
    }

    #[test]
    fn a_write_to_a_missing_target_is_refused() {
        let mut doc = Document::new();
        let mut resolver = DocumentResolver::new(&mut doc);
        let value = Value::Number(crate::value::Number::new(5.0, crate::units::Unit::Inches));
        assert!(
            resolver
                .set_cell(sheet(), &CellRef::new("PinX"), value)
                .is_err()
        );
    }

    #[test]
    fn deep_formula_chains_stop_at_the_bound() {
        let mut doc = Document::new();
        let mut resolver = DocumentResolver::new(&mut doc);
        resolver.depth = MAX_EVAL_DEPTH;
        let value = resolver
            .cell_value(sheet(), &CellRef::new("Width"))
            .expect("value");
        assert_eq!(value, Value::Err(ErrorCode::Ref));
    }
}
