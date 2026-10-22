//! `edit.*` commands and the shared undo steps (P5-04).
//!
//! Editing state: the `CellEdit`, `StructuralEdit` and `GeometryEdit`
//! undo steps every Phase 5 command builds its undo step from (ADR 0038,
//! ADR 0039, ADR 0040), and the selection commands.
//!
//! Implemented ids (task P5-04):
//!
//! - `edit.delete` — detach the selection (one undo step per subtree);
//! - `edit.select` — set the selection from Visio shape ids;
//! - `edit.select_all`, `edit.deselect` — selection housekeeping.
//!
//! `edit.undo` and `edit.redo` stay in [`crate::builtins::view`]; the
//! recording gate of ADR 0040 lives in [`crate::undo`] and
//! [`AppCtx::push_edit`](crate::AppCtx::push_edit).
//!
//! The undo steps restore **exact** prior state (an `apply`/`revert`
//! pair round-trips the model byte for byte, ADR 0040 rule 4). One
//! consequence is recorded in the task Result: reverting a cell write
//! restores the cell entries directly, because `Document::write_cell`
//! cannot express "no entry" or a formula-less literal.

use ferrogram_geom::Size;
use ferrogram_model::{
    Cell, CellName, CellReader, CellValue, CellWrite, DetachedShapes, Document, DocumentedDefault,
    PageId, Row, RowTarget, Section, ShapeId, SheetRef, documented_default,
};
use ferrogram_shapesheet::CellAddress;

use crate::app::{AppCtx, SessionId};
use crate::edit::CompositeEdit;
use crate::error::CommandError;
use crate::recalc::{formula_is_guarded, literal_number, subtree_addresses};
use crate::undo::Edit;
use crate::{Invocation, Outcome};

use super::connector::reroute_quietly_on;
use super::{
    CommandTable, edit_outcome, has_pages, has_selection, model_error, no_document, outcome,
};

// ---------------------------------------------------------------------------
// Shared helpers (also used by `builtins::shape` and the later Phase 5
// command modules).
// ---------------------------------------------------------------------------

/// The session's document, or [`CommandError::Failed`] when the session
/// is gone (an edit outlived its document).
pub(crate) fn document_mut(
    ctx: &mut AppCtx,
    session: SessionId,
) -> Result<&mut Document, CommandError> {
    ctx.documents
        .iter_mut()
        .find(|slot| slot.id == session)
        .map(|slot| &mut slot.document)
        .ok_or_else(|| CommandError::failed("the document session is gone"))
}

/// The literal formula of one number cell (ADR 0038): `0` for zero,
/// the shortest round-trip spelling otherwise. `-0.0` folds to `"0"`.
///
/// The model's one spelling rule, exported by `ferrogram_model` (task
/// P5-19 review F6: one spelling, one implementation — a local copy
/// could diverge and make a redo write a different `F` than the first
/// apply). Re-exported here under the command layer's name.
pub(crate) use ferrogram_model::mutate::literal as number_literal;

/// The exact cell entries a `Document::write_cell` of `cell` touches:
/// the cell itself plus the cells the documented-defaults table derives
/// from it by one multiplication (ADR 0038 rule 2).
///
/// This mirrors the derived-cell rule of `ferrogram_model::mutate`
/// (`DERIVED_CELLS`); the `a_width_write_refreshes_the_derived_cells`
/// test pins the mirror against the model's behaviour.
fn touched_cells(cell: &str) -> Vec<CellName> {
    const DERIVED: &[&str] = &[
        "LocPinX",
        "LocPinY",
        "TxtPinX",
        "TxtPinY",
        "TxtLocPinX",
        "TxtLocPinY",
        "TxtWidth",
        "TxtHeight",
    ];
    let mut names = vec![CellName::new(cell)];
    for name in DERIVED {
        let derives = matches!(
            documented_default(name),
            Some(DocumentedDefault::HalfOf(source) | DocumentedDefault::WholeOf(source))
                if source == cell
        );
        if derives {
            names.push(CellName::new(*name));
        }
    }
    names
}

/// The direct cells of one sheet, or an error when the id is stale.
fn sheet_cells(doc: &Document, sheet: SheetRef) -> Result<&Vec<Cell>, CommandError> {
    match sheet {
        SheetRef::Shape(id) => doc
            .shape(id)
            .map(|shape| &shape.sheet.cells)
            .ok_or_else(|| CommandError::failed(format!("unknown shape id {}", id.index))),
        SheetRef::Page(id) => doc
            .pages
            .get(id.index as usize)
            .map(|page| &page.sheet.cells)
            .ok_or_else(|| CommandError::failed(format!("unknown page id {}", id.index))),
    }
}

/// The direct cells of one sheet, mutably.
fn sheet_cells_mut(doc: &mut Document, sheet: SheetRef) -> Result<&mut Vec<Cell>, CommandError> {
    match sheet {
        SheetRef::Shape(id) => doc
            .shape_mut(id)
            .map(|shape| &mut shape.sheet.cells)
            .ok_or_else(|| CommandError::failed(format!("unknown shape id {}", id.index))),
        SheetRef::Page(id) => doc
            .pages
            .get_mut(id.index as usize)
            .map(|page| &mut page.sheet.cells)
            .ok_or_else(|| CommandError::failed(format!("unknown page id {}", id.index))),
    }
}

/// Puts the exact prior entries back: a captured entry replaces or
/// re-adds its cell, an absent one removes it again.
///
/// This is the one place that writes a `Sheet` outside
/// `Document::write_cell`: `write_cell` cannot restore "no entry", a
/// formula-less literal or the file's own `unit`, and the undo property
/// (ADR 0040 rule 4) needs the exact entry. See the task Result.
fn restore_cells(cells: &mut Vec<Cell>, old: &[(CellName, Option<Cell>)]) {
    for (name, snapshot) in old {
        let position = cells
            .iter()
            .position(|cell| cell.name.as_str() == name.as_str());
        match (position, snapshot) {
            (Some(at), Some(cell)) => {
                if let Some(slot) = cells.get_mut(at) {
                    *slot = cell.clone();
                }
            }
            (Some(at), None) => {
                cells.remove(at);
            }
            (None, Some(cell)) => cells.push(cell.clone()),
            (None, None) => {}
        }
    }
}

/// The resolved cached number of one shape cell, with the reader's
/// substitution warnings as lines (ADR 0021).
pub(crate) fn read_cell_number(
    doc: &Document,
    shape: ShapeId,
    cell: &str,
) -> Result<(f64, Vec<String>), CommandError> {
    let mut reader = CellReader::new(doc);
    let value = reader
        .number(SheetRef::Shape(shape), &CellName::new(cell))
        .map_err(model_error)?;
    let warnings = reader
        .take_warnings()
        .iter()
        .map(|warning| crate::builtins::warning_line(doc, warning))
        .collect();
    Ok((value, warnings))
}

/// The stable identity of a page (task P5-02: `PageId` is a list
/// position, so `delete_page`/`move_page` renumber every later page).
/// An undo step keys its page by Visio's `ID` and re-resolves the
/// current `PageId` when it replays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PageKey {
    /// Visio's `ID` of the page (unique per document, P5-02).
    pub visio_id: i32,
    /// The page's `NameU` (fallback when the `ID` cannot be found).
    pub name_u: String,
}

/// The stable key of `page`, or an error when the id is stale.
pub(crate) fn page_key(doc: &Document, page: PageId) -> Result<PageKey, CommandError> {
    let entry = doc
        .pages
        .get(page.index as usize)
        .ok_or_else(|| CommandError::failed(format!("unknown page id {}", page.index)))?;
    Ok(PageKey {
        visio_id: entry.visio_id,
        name_u: entry.name_u.clone(),
    })
}

/// The current `PageId` of the page `key` names: by Visio `ID`, then by
/// `NameU`, then the recorded index as a last resort.
pub(crate) fn resolve_page(doc: &Document, key: &PageKey, fallback: PageId) -> PageId {
    let by_id = doc
        .pages
        .iter()
        .enumerate()
        .find(|(_, page)| page.visio_id == key.visio_id)
        .or_else(|| {
            doc.pages
                .iter()
                .enumerate()
                .find(|(_, page)| !key.name_u.is_empty() && page.name_u == key.name_u)
        });
    match by_id {
        Some((index, _)) => u32::try_from(index)
            .map(|index| PageId { index })
            .unwrap_or(fallback),
        None => fallback,
    }
}

/// One undoable sheet-cell write (ADR 0038, ADR 0056): the exact prior
/// entries of every cell the write touches, and the write to replay.
///
/// `apply` goes through [`Document::write_cell`] (the mutation API, ADR
/// 0039), which also refreshes the derived cells; `revert` restores the
/// captured entries exactly, so `revert(apply(x)) == x` holds for the
/// model, not just for the lookups.
///
/// A write is **indirect** (a move, a resize, a style apply: a
/// programmatic write) unless it comes from a direct user edit of the
/// cell ([`CellEdit::direct`]). An indirect write is skipped for a
/// `GUARD`-ed cell, which keeps its formula, with one `Guarded` warning
/// line (ADR 0056); a direct edit replaces the formula, `GUARD` and all.
#[derive(Debug)]
pub(crate) struct CellEdit {
    session: SessionId,
    label: String,
    sheet: SheetRef,
    cell: CellName,
    /// The prior entries of the target cell and its derived cells.
    old: Vec<(CellName, Option<Cell>)>,
    /// The write to replay.
    new: CellWrite,
    /// Whether the user edited this cell directly (ADR 0056's `GUARD`
    /// rule).
    direct: bool,
    /// The cells the write touches (the recalc pass's dirty set).
    dirty: Vec<CellAddress>,
}

impl CellEdit {
    /// Stages one indirect write of `cell` on `sheet`, capturing the
    /// exact prior entries it will overwrite (the target and the cells
    /// the documented-defaults table derives from it).
    pub(crate) fn new(
        doc: &Document,
        session: SessionId,
        label: impl Into<String>,
        sheet: SheetRef,
        cell: &str,
        new: CellWrite,
    ) -> Result<Self, CommandError> {
        Self::stage(doc, session, label, sheet, cell, new, false)
    }

    /// Stages a **direct** user edit of `cell`: the one write that
    /// replaces a `GUARD`-ed formula (ADR 0056). Task P6-09's panel
    /// edits are the first non-test callers.
    #[allow(dead_code)]
    pub(crate) fn direct(
        doc: &Document,
        session: SessionId,
        label: impl Into<String>,
        sheet: SheetRef,
        cell: &str,
        new: CellWrite,
    ) -> Result<Self, CommandError> {
        Self::stage(doc, session, label, sheet, cell, new, true)
    }

    /// The shared staging tail of [`CellEdit::new`] and
    /// [`CellEdit::direct`].
    fn stage(
        doc: &Document,
        session: SessionId,
        label: impl Into<String>,
        sheet: SheetRef,
        cell: &str,
        new: CellWrite,
        direct: bool,
    ) -> Result<Self, CommandError> {
        let touched = touched_cells(cell);
        let cells = sheet_cells(doc, sheet)?;
        let old = touched
            .iter()
            .map(|name| {
                let existing = cells
                    .iter()
                    .find(|entry| entry.name.as_str() == name.as_str())
                    .cloned();
                (name.clone(), existing)
            })
            .collect();
        let dirty = touched
            .into_iter()
            .map(|name| CellAddress::new(sheet, name))
            .collect();
        Ok(Self {
            session,
            label: label.into(),
            sheet,
            cell: CellName::new(cell),
            old,
            new,
            direct,
            dirty,
        })
    }

    /// Whether the cell's current formula is `GUARD`-ed (ADR 0056).
    fn guarded(&self, doc: &Document) -> bool {
        let Ok(cells) = sheet_cells(doc, self.sheet) else {
            return false;
        };
        cells
            .iter()
            .find(|entry| entry.name.as_str() == self.cell.as_str())
            .and_then(|entry| entry.formula.as_deref())
            .is_some_and(formula_is_guarded)
    }
}

impl Edit for CellEdit {
    fn label(&self) -> &str {
        &self.label
    }

    fn dirty_cells(&self) -> &[CellAddress] {
        &self.dirty
    }

    fn apply(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError> {
        let session = self.session;
        if !self.direct {
            let guarded = {
                let doc = document_mut(ctx, session)?;
                self.guarded(doc)
            };
            if guarded {
                ctx.record_warning(format!(
                    "Guarded: {} keeps its formula; the write was skipped",
                    self.cell.as_str()
                ));
                return Ok(());
            }
        }
        let doc = document_mut(ctx, session)?;
        doc.write_cell(self.sheet, &self.cell, &self.new)
            .map_err(model_error)?;
        ctx.touch_session(session);
        Ok(())
    }

    fn revert(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError> {
        let session = self.session;
        {
            let doc = document_mut(ctx, session)?;
            let cells = sheet_cells_mut(doc, self.sheet)?;
            restore_cells(cells, &self.old);
        }
        ctx.touch_session(session);
        Ok(())
    }
}

/// Which direction one [`StructuralEdit`] runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Polarity {
    /// `apply` detaches the subtree, `revert` restores it (delete).
    Remove,
    /// `apply` restores the subtree, `revert` detaches it (insert).
    Insert,
}

/// One undoable shape subtree (ADR 0039): delete is detach plus
/// restore, insert is its mirror.
///
/// The subtree is captured by `Document::detach_shape` the first time
/// the edit detaches it, so z-order indices and the removed `Connects`
/// are the ones that really applied. The page is re-resolved through
/// [`PageKey`] on every replay (task P5-02 note: `PageId` moves when
/// pages are deleted or reordered).
#[derive(Debug)]
pub(crate) struct StructuralEdit {
    session: SessionId,
    label: String,
    root: ShapeId,
    page: PageKey,
    polarity: Polarity,
    snapshot: Option<DetachedShapes>,
    /// Every cell of the subtree (ADR 0056: an insert or delete dirties
    /// the cells it adds or removes).
    dirty: Vec<CellAddress>,
}

impl StructuralEdit {
    /// The edit that removes `root`'s subtree on `apply` and puts it
    /// back on `revert` (`edit.delete`).
    pub(crate) fn removal(
        doc: &Document,
        session: SessionId,
        label: impl Into<String>,
        root: ShapeId,
    ) -> Result<Self, CommandError> {
        Self::new(doc, session, label, root, Polarity::Remove)
    }

    /// The edit that inserts `root`'s subtree on `apply` and removes it
    /// on `revert` (the undo step of an insertion).
    ///
    /// The caller inserts the shape first (through the mutation API) and
    /// hands out this edit; `revert` captures the subtree and `apply`
    /// replays the capture. A first `apply` with nothing captured is an
    /// error, never a panic. The `insert.*`, `edit.paste` and
    /// `edit.duplicate` commands use it (P5-06..P5-08).
    pub(crate) fn insertion(
        doc: &Document,
        session: SessionId,
        label: impl Into<String>,
        root: ShapeId,
    ) -> Result<Self, CommandError> {
        Self::new(doc, session, label, root, Polarity::Insert)
    }

    fn new(
        doc: &Document,
        session: SessionId,
        label: impl Into<String>,
        root: ShapeId,
        polarity: Polarity,
    ) -> Result<Self, CommandError> {
        let page = doc.page_of(root).ok_or_else(|| {
            CommandError::failed(format!("shape {} is not on a page", root.index))
        })?;
        let dirty = subtree_addresses(doc, root);
        Ok(Self {
            session,
            label: label.into(),
            root,
            page: page_key(doc, page)?,
            polarity,
            snapshot: None,
            dirty,
        })
    }

    /// Detaches the subtree and captures it.
    fn detach(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError> {
        let session = self.session;
        let doc = document_mut(ctx, session)?;
        let snapshot = doc.detach_shape(self.root).map_err(model_error)?;
        self.snapshot = Some(snapshot);
        ctx.touch_session(session);
        Ok(())
    }

    /// Restores the captured subtree at its (re-resolved) page.
    fn restore(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError> {
        let Some(snapshot) = self.snapshot.clone() else {
            return Err(CommandError::failed(
                "nothing to restore: the subtree was never detached",
            ));
        };
        let session = self.session;
        let doc = document_mut(ctx, session)?;
        let page = resolve_page(doc, &self.page, snapshot.page);
        let mut snapshot = snapshot;
        snapshot.page = page;
        doc.restore_shapes(snapshot).map_err(model_error)?;
        ctx.touch_session(session);
        Ok(())
    }
}

impl Edit for StructuralEdit {
    fn label(&self) -> &str {
        &self.label
    }

    fn dirty_cells(&self) -> &[CellAddress] {
        &self.dirty
    }

    fn apply(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError> {
        match self.polarity {
            Polarity::Remove => self.detach(ctx),
            Polarity::Insert => self.restore(ctx),
        }
    }

    fn revert(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError> {
        match self.polarity {
            Polarity::Remove => self.restore(ctx),
            Polarity::Insert => self.detach(ctx),
        }
    }
}

/// One undoable rescale of a shape's **constant** geometry (ADR 0056).
///
/// A resize writes `Width`/`Height` and lets the recalc pass compute
/// every geometry row whose formula computes (a master's formulas,
/// materialised onto the instance). The rows nothing computes — a bare
/// literal formula or a cached value with no formula — are constant
/// geometry, and this step scales them with the box. The two writers
/// never touch the same cell (the split of ADR 0056's connector rule),
/// and this step never calls [`Document::scale_geometry`]: the model
/// function is untouched (task P6-07 criterion 3).
///
/// `revert` restores the exact prior geometry sections.
#[derive(Debug)]
pub(crate) struct GeometryEdit {
    session: SessionId,
    label: String,
    shape: ShapeId,
    from: Size,
    to: Size,
    /// The effective geometry rows at construction: the scale input, so
    /// a redo re-scales the same rows instead of the scaled ones.
    source: Vec<Section>,
    /// The shape's own geometry sections before the first rescale.
    old_sections: Option<Vec<Section>>,
    /// Every cell of the source rows (the recalc pass recomputes the
    /// ones whose formulas compute).
    dirty: Vec<CellAddress>,
}

impl GeometryEdit {
    /// The edit that rescales `shape` from the `from` box to the `to`
    /// box (inches). The caller checks the sizes; [`check_sizes`] rejects
    /// a degenerate `from` and any non-finite value.
    pub(crate) fn new(
        doc: &Document,
        session: SessionId,
        label: impl Into<String>,
        shape: ShapeId,
        from: Size,
        to: Size,
    ) -> Result<Self, CommandError> {
        if doc.shape(shape).is_none() {
            return Err(CommandError::failed(format!(
                "unknown shape id {}",
                shape.index
            )));
        }
        let source = geometry_source(doc, shape);
        let mut dirty = Vec::new();
        for section in &source {
            for (at, row) in section.rows.iter().enumerate() {
                let target = RowTarget {
                    position: Some(at),
                    ix: row.ix,
                };
                for cell in &row.cells {
                    dirty.push(CellAddress::in_row(
                        SheetRef::Shape(shape),
                        section.name.clone(),
                        target,
                        cell.name.clone(),
                    ));
                }
            }
        }
        Ok(Self {
            session,
            label: label.into(),
            shape,
            from,
            to,
            source,
            old_sections: None,
            dirty,
        })
    }
}

/// The size checks `Document::scale_geometry` applies (the model error
/// texts are kept, so the command errors read the same).
fn check_sizes(from: Size, to: Size) -> Result<(), CommandError> {
    if !from.width.is_finite()
        || !from.height.is_finite()
        || from.width <= 0.0
        || from.height <= 0.0
    {
        return Err(model_error(ferrogram_model::Error::BadCell(
            "zero or non-finite source size",
        )));
    }
    if !to.width.is_finite() || !to.height.is_finite() || to.width < 0.0 || to.height < 0.0 {
        return Err(model_error(ferrogram_model::Error::BadCell(
            "negative or non-finite target size",
        )));
    }
    Ok(())
}

/// The geometry rows a rescale works on: the shape's own `Geometry` rows
/// when they carry any, else the nearest master rung's rows (the ADR
/// 0019 row union at the row granularity this step needs).
///
/// The master rung is the master's first shape that carries rows: the
/// same approximation the dependency graph takes (P6-06 open issue 2),
/// because the model's position rule of `master_shape` is not public.
fn geometry_source(doc: &Document, shape: ShapeId) -> Vec<Section> {
    const MAX_CHAIN: usize = 32;
    let geometry = |entity: &ferrogram_model::Shape| -> Vec<Section> {
        entity
            .sheet
            .sections
            .iter()
            .filter(|section| section.name.as_str() == "Geometry")
            .cloned()
            .collect()
    };
    let mut chained = None;
    if let Some(entity) = doc.shape(shape) {
        let own = geometry(entity);
        if own.iter().any(|section| !section.rows.is_empty()) {
            return own;
        }
        chained = entity.master;
    }
    let mut seen: Vec<ferrogram_model::MasterId> = Vec::new();
    for _ in 0..MAX_CHAIN {
        let Some(master) = chained else {
            break;
        };
        if seen.contains(&master) {
            break;
        }
        seen.push(master);
        let mut next = None;
        if let Some(entry) = doc.master(master) {
            for rung in &entry.shapes {
                if let Some(rung) = doc.shape(*rung) {
                    let rows = geometry(rung);
                    if rows.iter().any(|section| !section.rows.is_empty()) {
                        return rows;
                    }
                    next = rung.master;
                }
            }
        }
        chained = next;
    }
    Vec::new()
}

/// The scaled copy of the source rows, and whether any constant cell
/// changed (a shape whose geometry all computes needs no write here).
struct ScaledSections {
    /// The rows to write.
    sections: Vec<Section>,
    /// Whether a constant cell was scaled.
    touched: bool,
}

/// Scales the constant cells of the source rows (the cells whose formula
/// is a bare literal, or that carry no formula).
fn scale_sections(source: &[Section], from: Size, to: Size) -> ScaledSections {
    let sx = to.width / from.width;
    let sy = to.height / from.height;
    let mut touched = false;
    let sections = source
        .iter()
        .cloned()
        .map(|mut section| {
            for row in &mut section.rows {
                scale_row(row, sx, sy, &mut touched);
            }
            section
        })
        .collect();
    ScaledSections { sections, touched }
}

/// Scales one row's constant cells. `X` scales by `sx` and `Y` by `sy`
/// (never on a `Rel*` row), `A`/`B` of an `Ellipse`, `EllipticalArcTo`
/// or `InfiniteLine` row and `C`/`D` of an `Ellipse` row with them. Arc
/// bows, spline knots and elliptical ratios stay (the documented P5-02
/// distortion of a non-uniform rescale).
fn scale_row(row: &mut Row, sx: f64, sy: f64, touched: &mut bool) {
    let kind = row.row_type.as_deref().unwrap_or("");
    let relative = kind.starts_with("Rel");
    for cell in &mut row.cells {
        let factor = match (kind, cell.name.as_str()) {
            (_, "X") if !relative => sx,
            (_, "Y") if !relative => sy,
            ("Ellipse" | "EllipticalArcTo" | "InfiniteLine", "A") => sx,
            ("Ellipse" | "EllipticalArcTo" | "InfiniteLine", "B") => sy,
            ("Ellipse", "C") => sx,
            ("Ellipse", "D") => sy,
            _ => continue,
        };
        if factor == 1.0 {
            continue;
        }
        let Some(value) = literal_number(cell.formula.as_deref(), cell.value.as_ref()) else {
            continue;
        };
        let fresh = value * factor;
        if !fresh.is_finite() {
            continue;
        }
        cell.value = Some(CellValue::Number(fresh));
        cell.formula = Some(number_literal(fresh));
        *touched = true;
    }
}

/// Replaces the sheet's `Geometry` sections with `fresh`: existing
/// sections keep their place (and their preserved content and section
/// cells), trailing ones the new list does not cover are dropped and
/// extra new sections land at the end. (The model's private
/// `replace_geometry_sections` rule, mirrored here: `Document::
/// scale_geometry` is not called any more.)
fn replace_geometry_sections(sections: &mut Vec<Section>, fresh: Vec<Section>) {
    let positions: Vec<usize> = sections
        .iter()
        .enumerate()
        .filter(|(_, section)| section.name.as_str() == "Geometry")
        .map(|(index, _)| index)
        .collect();
    let total = fresh.len();
    let mut incoming = fresh.into_iter();
    for (count, position) in positions.iter().enumerate() {
        if count >= total {
            break;
        }
        if let (Some(mut section), Some(slot)) = (incoming.next(), sections.get_mut(*position)) {
            section.preserved = std::mem::take(&mut slot.preserved);
            section.cells = std::mem::take(&mut slot.cells);
            *slot = section;
        }
    }
    for section in incoming {
        sections.push(section);
    }
    if positions.len() > total {
        let mut seen = 0;
        sections.retain(|section| {
            if section.name.as_str() != "Geometry" {
                return true;
            }
            seen += 1;
            seen <= total
        });
    }
}

impl Edit for GeometryEdit {
    fn label(&self) -> &str {
        &self.label
    }

    fn dirty_cells(&self) -> &[CellAddress] {
        &self.dirty
    }

    fn apply(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError> {
        check_sizes(self.from, self.to)?;
        let session = self.session;
        let shape = self.shape;
        if self.old_sections.is_none() {
            let doc = document_mut(ctx, session)?;
            let entity = doc
                .shape_mut(shape)
                .ok_or_else(|| CommandError::failed(format!("unknown shape id {}", shape.index)))?;
            self.old_sections = Some(entity.sheet.sections.clone());
        }
        let scaled = scale_sections(&self.source, self.from, self.to);
        if !scaled.touched {
            // Every row computes from its formulas: the recalc pass owns
            // them (ADR 0056) and this step writes nothing.
            return Ok(());
        }
        let doc = document_mut(ctx, session)?;
        let entity = doc
            .shape_mut(shape)
            .ok_or_else(|| CommandError::failed(format!("unknown shape id {}", shape.index)))?;
        replace_geometry_sections(&mut entity.sheet.sections, scaled.sections);
        ctx.touch_session(session);
        Ok(())
    }

    fn revert(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError> {
        let Some(sections) = self.old_sections.clone() else {
            return Err(CommandError::failed(
                "nothing to restore: the geometry was never scaled",
            ));
        };
        let session = self.session;
        let doc = document_mut(ctx, session)?;
        let entity = doc.shape_mut(self.shape).ok_or_else(|| {
            CommandError::failed(format!("unknown shape id {}", self.shape.index))
        })?;
        entity.sheet.sections = sections;
        ctx.touch_session(session);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// The `edit.*` commands.
// ---------------------------------------------------------------------------

/// The implemented `edit.*` commands (task P5-04).
pub(crate) const TABLE: CommandTable = &[
    ("edit.delete", "Delete", has_selection, run_delete),
    ("edit.select", "Select", has_pages, run_select),
    ("edit.select_all", "Select All", has_pages, run_select_all),
    ("edit.deselect", "Deselect", has_selection, run_deselect),
];

/// The shapes a selection command works on: the selected shapes that
/// are still on a page, in click order. Stale ids are skipped with a
/// warning line; nothing left is a `Failed`.
pub(super) fn selection_targets(
    ctx: &AppCtx,
) -> Result<(SessionId, Vec<ShapeId>, Vec<String>), CommandError> {
    let session = ctx.active_document().ok_or_else(no_document)?;
    let mut targets = Vec::new();
    let mut warnings = Vec::new();
    for &shape in &ctx.selection.shapes {
        if session.document.page_of(shape).is_some() {
            targets.push(shape);
        } else {
            warnings.push(format!(
                "shape {} is no longer on a page; skipped",
                shape.index
            ));
        }
    }
    if targets.is_empty() {
        return Err(CommandError::failed("no selected shape to edit"));
    }
    Ok((session.id, targets, warnings))
}

/// Whether any shape of `targets` is a strict ancestor of `id`: such a
/// target is deleted with its ancestor (one subtree, one undo step,
/// task P5-19 review F1). The parent walk is bounded (AGENTS.md section
/// 6), so a cyclic parent chain cannot loop.
fn has_selected_ancestor(doc: &Document, id: ShapeId, targets: &[ShapeId]) -> bool {
    const MAX_PARENT_WALK: usize = 1 << 16;
    let mut current = id;
    for _ in 0..MAX_PARENT_WALK {
        match doc.shape(current).and_then(|entry| entry.parent) {
            Some(parent) if targets.contains(&parent) => return true,
            Some(parent) => current = parent,
            None => return false,
        }
    }
    false
}

/// "Delete shape" / "Delete 3 shapes": the undo label of one command
/// (ADR 0040 rule 1).
pub(crate) fn count_label(verb: &str, count: usize) -> String {
    if count == 1 {
        format!("{verb} shape")
    } else {
        format!("{verb} {count} shapes")
    }
}

/// One warning line per connector glued to a shape this delete removes
/// (ADR 0045: a glue whose target is detached is dropped, with one line).
///
/// The model's `detach_shape` removes the records themselves; this counts
/// them before the edit runs so the user sees why the arrows came loose.
fn deleted_glue_warnings(ctx: &AppCtx, page: Option<PageId>, targets: &[ShapeId]) -> Vec<String> {
    let Some(page) = page else {
        return Vec::new();
    };
    let Some(session) = ctx.active_document() else {
        return Vec::new();
    };
    let doc = &session.document;
    let Some(entity) = doc.page(page) else {
        return Vec::new();
    };
    let removed: Vec<i32> = targets
        .iter()
        .filter_map(|shape| doc.shape(*shape).map(|entry| entry.visio_id))
        .collect();
    entity
        .connects
        .iter()
        .filter(|connect| removed.contains(&connect.to_sheet))
        .map(|connect| {
            format!(
                "connector {} lost its glue to detached shape {}",
                connect.from_sheet, connect.to_sheet
            )
        })
        .collect()
}

/// `edit.delete`: detaches every selected shape (one undo step).
///
/// A target whose ancestor is also a target goes with its ancestor and
/// is dropped from the list with one warning line (task P5-19 review F1:
/// a group plus its member is one subtree and one undo step, in either
/// selection order).
///
/// The removed shapes leave the selection; an entered group that went
/// with them is exited.
pub(super) fn run_delete(ctx: &mut AppCtx, _inv: &Invocation) -> Result<Outcome, CommandError> {
    let (session, selected, mut warnings) = selection_targets(ctx)?;
    let targets: Vec<ShapeId> = {
        let doc = &ctx.active_document().ok_or_else(no_document)?.document;
        let mut kept = Vec::new();
        for id in selected.iter().copied() {
            if has_selected_ancestor(doc, id, &selected) {
                warnings.push(format!(
                    "shape {} is inside another selected shape; skipped",
                    id.index
                ));
            } else {
                kept.push(id);
            }
        }
        kept
    };
    let page = ctx
        .active_document()
        .and_then(|session| session.active_page);
    let glue_warnings = deleted_glue_warnings(ctx, page, &targets);
    warnings.extend(glue_warnings);
    let mut edit = CompositeEdit::new(count_label("Delete", targets.len()));
    {
        let doc = document_mut(ctx, session)?;
        for &root in &targets {
            edit.push(Box::new(StructuralEdit::removal(
                doc,
                session,
                "Delete shape",
                root,
            )?));
        }
    }
    edit.apply(ctx)?;
    if let Some(page) = page {
        reroute_quietly_on(ctx, page);
    }

    // The detached shapes (and anything inside them) leave the selection.
    let AppCtx {
        documents,
        selection,
        ..
    } = ctx;
    if let Some(slot) = documents.iter().find(|slot| slot.id == session) {
        let doc = &slot.document;
        selection.shapes.retain(|id| doc.page_of(*id).is_some());
        if selection
            .anchor
            .is_some_and(|anchor| !selection.shapes.contains(&anchor))
        {
            selection.anchor = selection.shapes.last().copied();
        }
        if selection
            .entered
            .is_some_and(|group| doc.page_of(group).is_none())
        {
            selection.entered = None;
        }
        selection.refresh_page(doc);
    }
    Ok(edit_outcome(
        count_label("Deleted", targets.len()),
        warnings,
        Box::new(edit),
    ))
}

/// `edit.select`: sets the selection from Visio shape ids on the active
/// page.
///
/// Arguments: `shapes` is an [`ArgValue::List`](crate::ArgValue::List) of
/// Visio `ID` strings (the form the control channel uses), and `mode` is
/// `replace` (the default), `add` or `toggle`. An empty list with
/// `replace` clears the selection.
///
/// Every id must name a shape on the active page: an unknown or malformed
/// id fails the whole command and leaves the selection unchanged. The
/// command is not an edit (ADR 0040 rule 3): it returns no undo step and
/// does not bump `DocumentSession::changes`.
pub(super) fn run_select(ctx: &mut AppCtx, inv: &Invocation) -> Result<Outcome, CommandError> {
    let mode = match inv.args.get("mode") {
        None => "replace",
        Some(crate::ArgValue::Text(text)) => text.as_str(),
        Some(_) => {
            return Err(CommandError::invalid_args(
                "mode must be replace, add or toggle",
            ));
        }
    };
    if !matches!(mode, "replace" | "add" | "toggle") {
        return Err(CommandError::invalid_args(
            "mode must be replace, add or toggle",
        ));
    }
    let requested = match inv.args.get("shapes") {
        None => Vec::new(),
        Some(crate::ArgValue::List(ids)) => ids.iter().map(String::as_str).collect::<Vec<_>>(),
        Some(_) => {
            return Err(CommandError::invalid_args(
                "shapes must be a list of shape ids",
            ));
        }
    };

    let (page, resolved) = {
        let session = ctx.active_document().ok_or_else(no_document)?;
        let page = session
            .active_page
            .ok_or_else(|| CommandError::failed("no active page"))?;
        let mut resolved: Vec<ShapeId> = Vec::new();
        for text in requested {
            let visio_id = text.trim().parse::<i32>().map_err(|_| {
                CommandError::invalid_args(format!("shape id {text:?} is not an integer"))
            })?;
            let shape = session
                .document
                .shape_by_visio_id(page, visio_id)
                .ok_or_else(|| {
                    CommandError::invalid_args(format!(
                        "unknown shape id {visio_id} on the active page"
                    ))
                })?;
            if !resolved.contains(&shape) {
                resolved.push(shape);
            }
        }
        (page, resolved)
    };

    let mut shapes = match mode {
        "replace" => Vec::new(),
        _ => ctx.selection.shapes.clone(),
    };
    for shape in resolved {
        match mode {
            "add" => {
                if !shapes.contains(&shape) {
                    shapes.push(shape);
                }
            }
            "toggle" => {
                if let Some(index) = shapes.iter().position(|candidate| *candidate == shape) {
                    shapes.remove(index);
                } else {
                    shapes.push(shape);
                }
            }
            _ => shapes.push(shape),
        }
    }
    let count = shapes.len();
    let anchor = shapes.last().copied();
    let empty = shapes.is_empty();
    ctx.selection.shapes = shapes;
    ctx.selection.anchor = anchor;
    ctx.selection.page = if empty { None } else { Some(page) };
    if empty {
        ctx.selection.entered = None;
    }
    Ok(outcome(count_label("Selected", count), Vec::new()))
}

/// `edit.select_all`: selects every top-level shape of the active page,
/// or every member of the entered group while inside one (ADR 0041).
///
/// Selection only: no document change, no undo step and no change
/// counter bump (ADR 0040 rule 3).
pub(super) fn run_select_all(ctx: &mut AppCtx, _inv: &Invocation) -> Result<Outcome, CommandError> {
    let (page, shapes) = {
        let session = ctx.active_document().ok_or_else(no_document)?;
        let page = session
            .active_page
            .ok_or_else(|| CommandError::failed("no active page"))?;
        let doc = &session.document;
        let shapes = match ctx.selection.entered {
            Some(group) if doc.page_of(group).is_some() => doc
                .shape(group)
                .map(|shape| shape.children.clone())
                .unwrap_or_default(),
            _ => doc
                .pages
                .get(page.index as usize)
                .map(|entry| entry.shapes.clone())
                .unwrap_or_default(),
        };
        (page, shapes)
    };
    let count = shapes.len();
    ctx.selection.anchor = shapes.last().copied();
    ctx.selection.shapes = shapes;
    ctx.selection.page = Some(page);
    Ok(outcome(count_label("Selected", count), Vec::new()))
}

/// `edit.deselect`: clears the shapes and the anchor, keeping the page
/// and the entered group (clicking a member selects it again).
pub(super) fn run_deselect(ctx: &mut AppCtx, _inv: &Invocation) -> Result<Outcome, CommandError> {
    ctx.selection.shapes.clear();
    ctx.selection.anchor = None;
    Ok(outcome("Selection cleared", Vec::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{DocumentSession, Settings};
    use ferrogram_model::{CellValue, Preserved, Row, SectionName, Shape};

    /// One numeric cell with a literal formula and its cached value.
    fn number_cell(name: &str, value: f64) -> Cell {
        Cell {
            name: CellName::new(name),
            formula: Some(number_literal(value)),
            value: Some(CellValue::Number(value)),
            unit: ferrogram_geom::Unit::Inches,
            preserved: Preserved::new(),
        }
    }

    /// A document with one page and `count` numbered shapes, each 2x1
    /// at (n, 0).
    fn fixture(count: usize) -> (AppCtx, SessionId, PageId, Vec<ShapeId>) {
        let mut doc = Document::new();
        let page = doc.add_page("Page-1").expect("page");
        let mut shapes = Vec::new();
        for n in 0..count {
            let mut shape = Shape::new(ShapeId { index: 0 }, i32::try_from(n).unwrap_or(0) + 1);
            for (name, value) in [
                ("PinX", n as f64),
                ("PinY", 0.0),
                ("Width", 2.0),
                ("Height", 1.0),
            ] {
                shape.sheet.cells.push(Cell {
                    name: CellName::new(name),
                    formula: Some(number_literal(value)),
                    value: Some(CellValue::Number(value)),
                    unit: ferrogram_geom::Unit::Inches,
                    preserved: Preserved::new(),
                });
            }
            let mut geometry = Section::new(SectionName::from_static("Geometry"));
            let points: [(&str, f64, f64); 3] = [
                ("MoveTo", 0.0, 0.0),
                ("LineTo", 2.0, 0.0),
                ("LineTo", 2.0, 1.0),
            ];
            for (ix, (kind, x, y)) in points.into_iter().enumerate() {
                let mut row = Row::new();
                row.row_type = Some(kind.to_owned());
                row.ix = Some(i32::try_from(ix).unwrap_or(0) + 1);
                for (cell, value) in [("X", x), ("Y", y)] {
                    row.cells.push(Cell {
                        name: CellName::new(cell),
                        formula: Some(number_literal(value)),
                        value: Some(CellValue::Number(value)),
                        unit: ferrogram_geom::Unit::Inches,
                        preserved: Preserved::new(),
                    });
                }
                geometry.rows.push(row);
            }
            shape.sheet.sections.push(geometry);
            shapes.push(doc.insert_shape(page, shape, None).expect("insert"));
        }
        let mut ctx = AppCtx::new(Settings::default());
        let index = ctx.open_document(DocumentSession::new(doc, None));
        let session = ctx.documents.get(index).expect("session").id;
        (ctx, session, page, shapes)
    }

    /// The identity of a session that was never opened: no document
    /// can ever carry it.
    fn ghost_session() -> SessionId {
        DocumentSession::new(Document::new(), None).id
    }

    #[test]
    fn number_literal_spells_zero_and_round_trips() {
        assert_eq!(number_literal(0.0), "0");
        assert_eq!(number_literal(-0.0), "0");
        assert_eq!(number_literal(2.5), "2.5");
        assert_eq!(number_literal(-3.0), "-3");
    }

    #[test]
    fn touched_cells_mirror_the_derived_cell_rule() {
        let names: Vec<String> = touched_cells("Width")
            .iter()
            .map(|name| name.as_str().to_owned())
            .collect();
        assert_eq!(
            names,
            ["Width", "LocPinX", "TxtPinX", "TxtLocPinX", "TxtWidth"]
        );
        let names: Vec<String> = touched_cells("Height")
            .iter()
            .map(|name| name.as_str().to_owned())
            .collect();
        assert_eq!(
            names,
            ["Height", "LocPinY", "TxtPinY", "TxtLocPinY", "TxtHeight"]
        );
        let names: Vec<String> = touched_cells("Angle")
            .iter()
            .map(|name| name.as_str().to_owned())
            .collect();
        assert_eq!(names, ["Angle"]);
    }

    /// The mirror is pinned against the model: writing `Width` through
    /// `Document::write_cell` touches exactly the captured names.
    #[test]
    fn a_width_write_refreshes_the_derived_cells() {
        let (mut ctx, session, _page, shapes) = fixture(1);
        let shape = shapes[0];
        {
            let doc = document_mut(&mut ctx, session).expect("doc");
            doc.write_cell(
                SheetRef::Shape(shape),
                &CellName::new("Width"),
                &CellWrite::new("4", CellValue::Number(4.0)),
            )
            .expect("write");
        }
        let doc = &ctx.active_document().expect("doc").document;
        let cells = sheet_cells(doc, SheetRef::Shape(shape)).expect("cells");
        let touched: Vec<&str> = cells
            .iter()
            .map(|cell| cell.name.as_str())
            .filter(|name| touched_cells("Width").iter().any(|n| n.as_str() == *name))
            .collect();
        assert_eq!(
            touched,
            ["Width", "LocPinX", "TxtPinX", "TxtLocPinX", "TxtWidth"]
        );
    }

    #[test]
    fn cell_edits_round_trip_the_model_exactly() {
        let (mut ctx, session, _page, shapes) = fixture(1);
        let shape = shapes[0];
        let before = ctx.active_document().expect("doc").document.clone();

        let mut edit = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            CellEdit::new(
                doc,
                session,
                "Set Width",
                SheetRef::Shape(shape),
                "Width",
                CellWrite::new("4", CellValue::Number(4.0)),
            )
            .expect("capture")
        };
        assert_eq!(edit.label(), "Set Width");
        edit.apply(&mut ctx).expect("apply");
        let after = ctx.active_document().expect("doc").document.clone();
        assert_ne!(before, after);
        edit.revert(&mut ctx).expect("revert");
        assert_eq!(ctx.active_document().expect("doc").document, before);
        edit.apply(&mut ctx).expect("apply");
        assert_eq!(ctx.active_document().expect("doc").document, after);
    }

    /// Reverting a write to a cell that had no entry removes the entry
    /// again (the exact-inverse rule).
    #[test]
    fn cell_edits_restore_absence_exactly() {
        let (mut ctx, session, _page, shapes) = fixture(1);
        let shape = shapes[0];
        {
            let doc = document_mut(&mut ctx, session).expect("doc");
            let cells = sheet_cells_mut(doc, SheetRef::Shape(shape)).expect("cells");
            cells.retain(|cell| cell.name.as_str() != "Angle");
        }
        let before = ctx.active_document().expect("doc").document.clone();
        let mut edit = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            CellEdit::new(
                doc,
                session,
                "Set Angle",
                SheetRef::Shape(shape),
                "Angle",
                CellWrite::new("1.5", CellValue::Number(1.5)),
            )
            .expect("capture")
        };
        edit.apply(&mut ctx).expect("apply");
        assert!(
            sheet_cells(
                &ctx.active_document().expect("doc").document,
                SheetRef::Shape(shape)
            )
            .expect("cells")
            .iter()
            .any(|cell| cell.name.as_str() == "Angle")
        );
        edit.revert(&mut ctx).expect("revert");
        assert_eq!(ctx.active_document().expect("doc").document, before);
    }

    #[test]
    fn cell_edits_reject_a_stale_session_or_sheet() {
        let (mut ctx, session, _page, shapes) = fixture(1);
        let error = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            CellEdit::new(
                doc,
                session,
                "Set Width",
                SheetRef::Shape(ShapeId { index: 99 }),
                "Width",
                CellWrite::new("4", CellValue::Number(4.0)),
            )
        }
        .expect_err("a stale shape is rejected at capture");
        assert!(error.to_string().contains("unknown shape id"), "{error}");

        let error = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            CellEdit::new(
                doc,
                session,
                "Set Width",
                SheetRef::Page(PageId { index: 9 }),
                "Width",
                CellWrite::new("4", CellValue::Number(4.0)),
            )
        }
        .expect_err("a stale page is rejected at capture");
        assert!(error.to_string().contains("unknown page id"), "{error}");

        let mut edit = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            CellEdit::new(
                doc,
                ghost_session(),
                "Set Width",
                SheetRef::Shape(shapes[0]),
                "Width",
                CellWrite::new("4", CellValue::Number(4.0)),
            )
            .expect("capture")
        };
        let error = edit.apply(&mut ctx).expect_err("gone session");
        assert!(error.to_string().contains("session is gone"), "{error}");
        let error = edit.revert(&mut ctx).expect_err("gone session");
        assert!(error.to_string().contains("session is gone"), "{error}");
    }

    #[test]
    fn structural_edits_detach_and_restore() {
        let (mut ctx, session, _page, shapes) = fixture(2);
        let root = shapes[0];
        let before = ctx.active_document().expect("doc").document.clone();
        let mut edit = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            StructuralEdit::removal(doc, session, "Delete shape", root).expect("removal")
        };
        assert_eq!(edit.label(), "Delete shape");
        edit.apply(&mut ctx).expect("apply");
        let after = ctx.active_document().expect("doc").document.clone();
        assert!(after.page_of(root).is_none(), "the root is detached");
        edit.revert(&mut ctx).expect("revert");
        assert_eq!(ctx.active_document().expect("doc").document, before);
        edit.apply(&mut ctx).expect("apply");
        assert_eq!(ctx.active_document().expect("doc").document, after);
    }

    /// The mirror of a delete: an insertion edit removes on revert and
    /// replays on apply (the undo step `insert.*` commands hand out).
    #[test]
    fn insertion_edits_undo_an_insertion() {
        let (mut ctx, session, _page, shapes) = fixture(1);
        let root = shapes[0];
        let mut edit = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            StructuralEdit::insertion(doc, session, "Insert shape", root).expect("insertion")
        };
        let before = ctx.active_document().expect("doc").document.clone();
        edit.revert(&mut ctx).expect("revert removes the subtree");
        let after = ctx.active_document().expect("doc").document.clone();
        assert!(after.page_of(root).is_none());
        edit.apply(&mut ctx).expect("apply replays it");
        assert_eq!(ctx.active_document().expect("doc").document, before);
    }

    /// The master's note (task P5-02): an edit that keeps a `PageId`
    /// re-resolves it when a page delete or move renumbered the list.
    #[test]
    fn structural_edits_re_resolve_their_page() {
        let (mut ctx, session, _page, shapes) = fixture(1);
        ctx.active_document_mut()
            .expect("doc")
            .document
            .add_page("Page-2")
            .expect("page");
        let root = shapes[0];
        let mut edit = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            StructuralEdit::removal(doc, session, "Delete shape", root).expect("removal")
        };
        assert_eq!(edit.label(), "Delete shape");
        edit.apply(&mut ctx).expect("apply");
        // A page edit (P5-08) removes the first page: the recorded
        // PageId 0 no longer names the page the subtree came from.
        ctx.active_document_mut()
            .expect("doc")
            .document
            .delete_page(PageId { index: 0 })
            .expect("delete");
        edit.revert(&mut ctx).expect("revert");
        let doc = &ctx.active_document().expect("doc").document;
        assert_eq!(doc.page_of(root), Some(PageId { index: 0 }));
        assert_eq!(
            doc.pages
                .first()
                .map(|page| page.name.as_str())
                .unwrap_or_default(),
            "Page-2",
            "the page that kept its visio id is the one that got the shape back"
        );
    }

    /// The two empty-input arms of the glue counter (task P5-19
    /// coverage): no session, and no active page.
    #[test]
    fn deleted_glue_warnings_without_a_session_or_page_are_empty() {
        let ctx = AppCtx::new(Settings::default());
        assert!(deleted_glue_warnings(&ctx, Some(PageId { index: 0 }), &[]).is_empty());
        let (ctx, _session, page, shapes) = fixture(1);
        assert!(deleted_glue_warnings(&ctx, None, &shapes).is_empty());
        assert!(deleted_glue_warnings(&ctx, Some(page), &shapes).is_empty());
    }

    #[test]
    fn structural_edits_report_missing_state() {
        let (mut ctx, session, _page, shapes) = fixture(1);
        let mut edit = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            StructuralEdit::insertion(doc, session, "Insert shape", shapes[0]).expect("insertion")
        };
        // A first `apply` has nothing captured yet.
        let error = edit.apply(&mut ctx).expect_err("no snapshot");
        assert!(error.to_string().contains("nothing to restore"), "{error}");

        // A restore whose session is gone names the session.
        let mut edit = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            let mut edit = StructuralEdit::removal(doc, ghost_session(), "Delete shape", shapes[0])
                .expect("removal");
            edit.snapshot = Some(DetachedShapes {
                page: PageId { index: 0 },
                parent: None,
                index: 0,
                shapes: Vec::new(),
                connects: Vec::new(),
            });
            edit
        };
        let error = edit.revert(&mut ctx).expect_err("gone session");
        assert!(error.to_string().contains("session is gone"), "{error}");

        // A shape that is not on a page cannot be keyed.
        let doc = document_mut(&mut ctx, session).expect("doc");
        let error = StructuralEdit::removal(doc, session, "Delete", ShapeId { index: 9 })
            .expect_err("stale shape");
        assert!(error.to_string().contains("not on a page"), "{error}");
    }

    #[test]
    fn geometry_edits_rescale_and_restore() {
        let (mut ctx, session, _page, shapes) = fixture(1);
        let shape = shapes[0];
        let before = ctx.active_document().expect("doc").document.clone();
        let mut edit = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            GeometryEdit::new(
                doc,
                session,
                "Resize shape",
                shape,
                Size::new(2.0, 1.0),
                Size::new(4.0, 1.0),
            )
            .expect("geometry edit")
        };
        assert_eq!(edit.label(), "Resize shape");
        edit.apply(&mut ctx).expect("apply");
        let after = ctx.active_document().expect("doc").document.clone();
        assert_ne!(before, after);
        edit.revert(&mut ctx).expect("revert");
        assert_eq!(ctx.active_document().expect("doc").document, before);
        edit.apply(&mut ctx).expect("apply");
        assert_eq!(ctx.active_document().expect("doc").document, after);
    }

    #[test]
    fn geometry_edits_report_missing_state() {
        let (mut ctx, session, _page, _shapes) = fixture(1);
        let error = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            GeometryEdit::new(
                doc,
                session,
                "Resize shape",
                ShapeId { index: 7 },
                Size::new(2.0, 1.0),
                Size::new(4.0, 1.0),
            )
        }
        .expect_err("stale shape");
        assert!(error.to_string().contains("unknown shape id"), "{error}");

        let mut edit = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            GeometryEdit::new(
                doc,
                ghost_session(),
                "Resize shape",
                ShapeId { index: 0 },
                Size::new(2.0, 1.0),
                Size::new(4.0, 1.0),
            )
            .expect("geometry edit")
        };
        edit.shape = ShapeId { index: 7 };
        let error = edit.revert(&mut ctx).expect_err("nothing captured");
        assert!(error.to_string().contains("nothing to restore"), "{error}");
    }

    /// A page-sheet write round-trips the same way (the `SheetRef::Page`
    /// arms of the sheet lookup), and a vanished page is an error.
    #[test]
    fn page_cell_edits_round_trip_and_report_a_vanished_page() {
        let (mut ctx, session, page, _shapes) = fixture(1);
        let before = ctx.active_document().expect("doc").document.clone();
        let mut edit = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            CellEdit::new(
                doc,
                session,
                "Set Page Width",
                SheetRef::Page(page),
                "PageWidth",
                CellWrite::new("11", CellValue::Number(11.0)),
            )
            .expect("capture")
        };
        edit.apply(&mut ctx).expect("apply");
        assert_ne!(ctx.active_document().expect("doc").document, before);
        edit.revert(&mut ctx).expect("revert");
        assert_eq!(ctx.active_document().expect("doc").document, before);

        // The page goes away between apply and revert (a page edit
        // ran and was not undone): the revert reports it, never panics.
        edit.apply(&mut ctx).expect("apply");
        ctx.active_document_mut()
            .expect("doc")
            .document
            .delete_page(page)
            .expect("delete");
        let error = edit.revert(&mut ctx).expect_err("the page is gone");
        assert!(error.to_string().contains("unknown page id"), "{error}");
    }

    /// `restore_cells` covers all four positions of its match.
    #[test]
    fn restore_cells_handles_every_position() {
        let entry = |name: &str, value: f64| Cell {
            name: CellName::new(name),
            formula: Some(number_literal(value)),
            value: Some(CellValue::Number(value)),
            unit: ferrogram_geom::Unit::Unitsless,
            preserved: Preserved::new(),
        };
        let mut cells = vec![entry("kept", 1.0), entry("gone", 2.0)];
        restore_cells(
            &mut cells,
            &[
                (CellName::new("gone"), None),
                (CellName::new("back"), Some(entry("back", 3.0))),
                (CellName::new("never"), None),
                (CellName::new("kept"), Some(entry("kept", 9.0))),
            ],
        );
        let names: Vec<&str> = cells.iter().map(|cell| cell.name.as_str()).collect();
        assert_eq!(names, ["kept", "back"]);
        assert_eq!(
            cells.first().and_then(|cell| cell.value.clone()),
            Some(CellValue::Number(9.0))
        );
    }

    /// The geometry edit's two stale-shape arms (apply and revert) name
    /// the shape; neither panics.
    #[test]
    fn geometry_edits_report_a_vanished_shape() {
        let (mut ctx, session, _page, shapes) = fixture(1);
        let mut edit = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            GeometryEdit::new(
                doc,
                session,
                "Resize shape",
                shapes[0],
                Size::new(2.0, 1.0),
                Size::new(4.0, 1.0),
            )
            .expect("geometry edit")
        };
        edit.shape = ShapeId { index: 7 };
        let error = edit.apply(&mut ctx).expect_err("apply: stale shape");
        assert!(error.to_string().contains("unknown shape id"), "{error}");

        let mut edit = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            GeometryEdit::new(
                doc,
                session,
                "Resize shape",
                shapes[0],
                Size::new(2.0, 1.0),
                Size::new(4.0, 1.0),
            )
            .expect("geometry edit")
        };
        edit.apply(&mut ctx).expect("apply");
        edit.shape = ShapeId { index: 7 };
        let error = edit.revert(&mut ctx).expect_err("revert: stale shape");
        assert!(error.to_string().contains("unknown shape id"), "{error}");
    }

    /// The model failure mapping (ADR 0028: `commands` maps the domain
    /// error): a non-finite cached number is refused by the model.
    #[test]
    fn a_non_finite_write_is_a_model_failure() {
        let (mut ctx, session, _page, shapes) = fixture(1);
        let mut edit = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            CellEdit::new(
                doc,
                session,
                "Set Width",
                SheetRef::Shape(shapes[0]),
                "Width",
                CellWrite::new("inf", CellValue::Number(f64::INFINITY)),
            )
            .expect("capture")
        };
        let error = edit.apply(&mut ctx).expect_err("the model refuses");
        assert!(error.to_string().contains("non-finite"), "{error}");
    }

    /// Task P5-19 review F1: the ancestor walk is bounded — a cyclic
    /// parent chain stops at the cap, it never loops.
    #[test]
    fn selected_ancestor_walk_stops_on_a_cycle() {
        let (mut ctx, session, _page, shapes) = fixture(3);
        {
            let doc = document_mut(&mut ctx, session).expect("doc");
            if let Some(entry) = doc.shape_mut(shapes[0]) {
                entry.parent = Some(shapes[1]);
            }
            if let Some(entry) = doc.shape_mut(shapes[1]) {
                entry.parent = Some(shapes[0]);
            }
        }
        let doc = document_mut(&mut ctx, session).expect("doc");
        assert!(
            !has_selected_ancestor(doc, shapes[0], &[shapes[2]]),
            "the walk stops at the cap"
        );
        assert!(has_selected_ancestor(doc, shapes[0], &[shapes[1]]));
    }

    /// Criterion 6 (task P6-07): `dirty_cells` covers every edit type
    /// that writes cells — a test per type. `CellEdit` first: the target
    /// cell and the derived cells `Document::write_cell` refreshes.
    #[test]
    fn cell_edits_dirty_their_touched_cells() {
        let (mut ctx, session, _page, shapes) = fixture(1);
        let edit = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            CellEdit::new(
                doc,
                session,
                "Set Width",
                SheetRef::Shape(shapes[0]),
                "Width",
                CellWrite::new("4", CellValue::Number(4.0)),
            )
            .expect("capture")
        };
        let names: Vec<&str> = edit.dirty_cells().iter().map(|a| a.cell.as_str()).collect();
        assert_eq!(
            names,
            ["Width", "LocPinX", "TxtPinX", "TxtLocPinX", "TxtWidth"]
        );
        assert!(
            edit.dirty_cells()
                .iter()
                .all(|address| address.sheet == SheetRef::Shape(shapes[0]))
        );
    }

    /// `StructuralEdit` dirties every cell of the subtree it adds or
    /// removes (ADR 0056).
    #[test]
    fn structural_edits_dirty_their_subtree_cells() {
        let (mut ctx, session, _page, shapes) = fixture(2);
        let edit = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            StructuralEdit::removal(doc, session, "Delete shape", shapes[0]).expect("removal")
        };
        let dirty = edit.dirty_cells();
        // Four transform cells and three rows of two geometry cells.
        assert_eq!(dirty.len(), 10, "{dirty:?}");
        assert!(dirty.iter().all(|a| a.sheet == SheetRef::Shape(shapes[0])));
        assert!(dirty.iter().any(|a| a.cell.as_str() == "PinX"));
        assert!(dirty.iter().any(|a| a.row.is_some()));
    }

    /// `GeometryEdit` dirties the geometry row cells it writes (the
    /// formula rows come back through the recalc pass).
    #[test]
    fn geometry_edits_dirty_their_row_cells() {
        let (mut ctx, session, _page, shapes) = fixture(1);
        let edit = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            GeometryEdit::new(
                doc,
                session,
                "Resize shape",
                shapes[0],
                Size::new(2.0, 1.0),
                Size::new(4.0, 1.0),
            )
            .expect("geometry edit")
        };
        let dirty = edit.dirty_cells();
        assert_eq!(dirty.len(), 6, "three rows of X and Y: {dirty:?}");
        assert!(dirty.iter().all(|a| {
            a.row.is_some()
                && a.section
                    .as_ref()
                    .is_some_and(|name| name.as_str() == "Geometry")
        }));
    }

    /// Criterion 4 (ADR 0056's `GUARD` rule): an indirect write is
    /// skipped for a guarded cell with one `Guarded` warning and the
    /// formula unchanged; a direct user edit replaces it.
    #[test]
    fn a_guarded_cell_survives_an_indirect_write_and_a_direct_edit_replaces_it() {
        let (mut ctx, session, _page, shapes) = fixture(1);
        let shape = shapes[0];
        {
            let doc = document_mut(&mut ctx, session).expect("doc");
            let cells = sheet_cells_mut(doc, SheetRef::Shape(shape)).expect("cells");
            if let Some(entry) = cells.iter_mut().find(|c| c.name.as_str() == "Width") {
                entry.formula = Some(String::from("GUARD(2)"));
                entry.value = Some(CellValue::Number(2.0));
            }
        }
        let before = ctx.active_document().expect("doc").document.clone();
        let mut indirect = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            CellEdit::new(
                doc,
                session,
                "Set Width",
                SheetRef::Shape(shape),
                "Width",
                CellWrite::new("4", CellValue::Number(4.0)),
            )
            .expect("capture")
        };
        indirect.apply(&mut ctx).expect("apply");
        let guarded = ctx.active_document().expect("doc").document.clone();
        assert_eq!(guarded, before, "the indirect write is skipped entirely");
        assert_eq!(ctx.warnings.len(), 1, "{:?}", ctx.warnings);
        assert!(ctx.warnings[0].contains("Guarded"), "{:?}", ctx.warnings);
        indirect.revert(&mut ctx).expect("revert");
        assert_eq!(
            ctx.active_document().expect("doc").document,
            before,
            "the skipped step reverts to the same document"
        );

        let mut direct = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            CellEdit::direct(
                doc,
                session,
                "Set Width",
                SheetRef::Shape(shape),
                "Width",
                CellWrite::new("4", CellValue::Number(4.0)),
            )
            .expect("capture")
        };
        direct.apply(&mut ctx).expect("apply");
        let doc = &ctx.active_document().expect("doc").document;
        let cells = sheet_cells(doc, SheetRef::Shape(shape)).expect("cells");
        let width = cells
            .iter()
            .find(|c| c.name.as_str() == "Width")
            .expect("Width");
        assert_eq!(width.formula.as_deref(), Some("4"), "the direct edit wins");
        assert_eq!(width.value, Some(CellValue::Number(4.0)));
    }

    /// The size checks keep the model's error texts (a degenerate
    /// `from` and a negative `to` are refused before any write).
    #[test]
    fn geometry_edits_reject_degenerate_sizes() {
        let (mut ctx, session, _page, shapes) = fixture(1);
        let mut edit = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            GeometryEdit::new(
                doc,
                session,
                "Resize shape",
                shapes[0],
                Size::new(2.0, 1.0),
                Size::new(4.0, 1.0),
            )
            .expect("geometry edit")
        };
        edit.from = Size::new(0.0, 1.0);
        let error = edit.apply(&mut ctx).expect_err("zero source");
        assert!(
            error.to_string().contains("zero or non-finite source size"),
            "{error}"
        );
        edit.from = Size::new(1.0, 1.0);
        edit.to = Size::new(f64::NAN, 1.0);
        let error = edit.apply(&mut ctx).expect_err("bad target");
        assert!(
            error
                .to_string()
                .contains("negative or non-finite target size"),
            "{error}"
        );
        assert_eq!(
            ctx.active_document().expect("doc").changes,
            0,
            "a refused rescale writes nothing"
        );
    }

    /// The row source of a rescale: the shape's own rows when they carry
    /// any, else the nearest master rung's rows (the ADR 0019 union).
    #[test]
    fn geometry_source_falls_back_to_the_master_rung() {
        use ferrogram_model::{Master, MasterId, SectionName};
        let (mut ctx, session, page, shapes) = fixture(1);
        let doc = document_mut(&mut ctx, session).expect("doc");
        // No own rows and no master: no source at all.
        if let Some(entity) = doc.shape_mut(shapes[0]) {
            entity.sheet.sections.clear();
        }
        assert!(geometry_source(doc, shapes[0]).is_empty());

        // A master whose one shape carries rows.
        let mut rung = Shape::new(ShapeId { index: 0 }, 9);
        let mut geometry = Section::new(SectionName::from_static("Geometry"));
        let mut row = Row::new();
        row.row_type = Some("LineTo".to_owned());
        row.ix = Some(0);
        row.cells = vec![number_cell("X", 1.0), number_cell("Y", 2.0)];
        geometry.rows.push(row);
        rung.sheet.sections.push(geometry);
        let rung = doc.add_shape(rung);
        let mut master = Master::new(MasterId { index: 0 }, 7);
        master.shapes = vec![rung];
        let master = doc.add_master(master).expect("master");
        if let Some(entity) = doc.shape_mut(shapes[0]) {
            entity.master = Some(master);
            entity
                .sheet
                .sections
                .push(Section::new(SectionName::from_static("Geometry")));
        }
        let source = geometry_source(doc, shapes[0]);
        assert_eq!(source.len(), 1);
        assert_eq!(source[0].rows.len(), 1, "the master rung's rows");

        // A master chain that loops stops at the visited set.
        if let Some(entity) = doc.shape_mut(rung) {
            entity.master = Some(master);
        }
        assert_eq!(geometry_source(doc, shapes[0]).len(), 1);
        let _ = page;
    }

    /// The rescale's per-cell rules (the P5-02 scaling map): `X`/`Y` on
    /// a plain row, `A`/`B`/`C`/`D` on an `Ellipse`, `A` on an `ArcTo`
    /// (a bow keeps its size) and nothing on a `Rel*` row.
    #[test]
    fn scale_row_scales_the_mapped_cells_only() {
        let row = |kind: &str, cells: &[(&str, f64)]| {
            let mut row = Row::new();
            row.row_type = Some(kind.to_owned());
            row.cells = cells
                .iter()
                .map(|(name, value)| number_cell(name, *value))
                .collect();
            row
        };
        let mut section = Section::new(SectionName::from_static("Geometry"));
        section.rows = vec![
            row("LineTo", &[("X", 1.0), ("Y", 1.0), ("A", 9.0)]),
            row(
                "Ellipse",
                &[
                    ("X", 1.0),
                    ("Y", 1.0),
                    ("A", 1.0),
                    ("B", 1.0),
                    ("C", 1.0),
                    ("D", 1.0),
                ],
            ),
            row("ArcTo", &[("X", 1.0), ("Y", 1.0), ("A", 1.0)]),
            row("RelLineTo", &[("X", 1.0), ("Y", 1.0)]),
        ];
        let scaled = scale_sections(&[section], Size::new(1.0, 1.0), Size::new(2.0, 3.0));
        assert!(scaled.touched);
        let value = |row: usize, name: &str| -> f64 {
            match &scaled.sections[0].rows[row]
                .cells
                .iter()
                .find(|cell| cell.name.as_str() == name)
                .expect("cell")
                .value
            {
                Some(CellValue::Number(value)) => *value,
                _ => f64::NAN,
            }
        };
        assert_eq!(
            (value(0, "X"), value(0, "Y"), value(0, "A")),
            (2.0, 3.0, 9.0)
        );
        assert_eq!(
            (
                value(1, "X"),
                value(1, "A"),
                value(1, "B"),
                value(1, "C"),
                value(1, "D")
            ),
            (2.0, 2.0, 3.0, 2.0, 3.0)
        );
        assert_eq!((value(2, "X"), value(2, "A")), (2.0, 1.0), "the bow keeps");
        assert_eq!((value(3, "X"), value(3, "Y")), (1.0, 1.0), "Rel rows keep");
    }

    /// A constant cell whose scaled value would not be finite is left
    /// alone, and a shape whose rows all compute needs no write.
    #[test]
    fn scale_row_skips_the_cells_it_cannot_scale() {
        let mut row = Row::new();
        row.row_type = Some("LineTo".to_owned());
        row.cells = vec![
            number_cell("X", f64::MAX),
            number_cell("Y", 1.0),
            Cell {
                name: CellName::new("A"),
                formula: Some(String::from("Width")),
                value: Some(CellValue::Number(1.0)),
                unit: ferrogram_geom::Unit::Inches,
                preserved: Preserved::new(),
            },
        ];
        let mut section = Section::new(SectionName::from_static("Geometry"));
        section.rows.push(row);
        let scaled = scale_sections(&[section], Size::new(1.0, 1.0), Size::new(2.0, 2.0));
        let cells = &scaled.sections[0].rows[0].cells;
        assert_eq!(cells[0].value, Some(CellValue::Number(f64::MAX)), "kept");
        assert_eq!(cells[1].value, Some(CellValue::Number(2.0)), "scaled");
        assert_eq!(
            cells[2].formula.as_deref(),
            Some("Width"),
            "a formula keeps"
        );

        // Every row computes: the pass owns it and nothing is written.
        let mut row = Row::new();
        row.row_type = Some("LineTo".to_owned());
        row.cells = vec![Cell {
            name: CellName::new("X"),
            formula: Some(String::from("Width")),
            value: Some(CellValue::Number(1.0)),
            unit: ferrogram_geom::Unit::Inches,
            preserved: Preserved::new(),
        }];
        let mut section = Section::new(SectionName::from_static("Geometry"));
        section.rows.push(row);
        assert!(!scale_sections(&[section], Size::new(1.0, 1.0), Size::new(2.0, 2.0)).touched);
    }

    /// The section-replacement rule keeps existing sections in place
    /// (preserved content and section cells included), drops trailing
    /// ones and appends the extra new ones.
    #[test]
    fn replace_geometry_sections_keeps_sections_in_place() {
        let section = |name: &'static str| Section::new(SectionName::from_static(name));
        // More old geometry sections than new: the trailing one goes.
        let mut sections = vec![
            section("Geometry"),
            section("Property"),
            section("Geometry"),
        ];
        let mut old_cells = section("Geometry");
        old_cells.cells.push(number_cell("NoFill", 1.0));
        sections[0] = old_cells;
        replace_geometry_sections(&mut sections, vec![section("Geometry")]);
        let names: Vec<&str> = sections.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(names, ["Geometry", "Property"]);
        assert_eq!(
            sections[0].cells.len(),
            1,
            "an existing section keeps its section cells"
        );

        // Fewer old ones than new: the extra lands at the end.
        let mut sections = vec![section("Geometry")];
        let old_preserved = sections[0].preserved.clone();
        replace_geometry_sections(
            &mut sections,
            vec![section("Geometry"), section("Geometry")],
        );
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].preserved, old_preserved, "preserved stays");
    }

    /// Criterion 5 (ADR 0039) at the edit seam: a failing `apply`
    /// changes nothing and records nothing.
    #[test]
    fn a_failing_edit_changes_nothing_and_recalculates_nothing() {
        let (mut ctx, session, _page, shapes) = fixture(1);
        let before = ctx.active_document().expect("doc").document.clone();
        let mut edit = {
            let doc = document_mut(&mut ctx, session).expect("doc");
            CellEdit::new(
                doc,
                session,
                "Set Width",
                SheetRef::Shape(shapes[0]),
                "Width",
                CellWrite::new("inf", CellValue::Number(f64::INFINITY)),
            )
            .expect("capture")
        };
        let error = edit.apply(&mut ctx).expect_err("the model refuses");
        assert!(error.to_string().contains("non-finite"), "{error}");
        assert_eq!(
            ctx.active_document().expect("doc").document,
            before,
            "a failing edit changes nothing"
        );
        assert!(ctx.warnings.is_empty(), "nothing was recalculated");
        assert!(ctx.active_document().expect("doc").undo.is_empty());
    }
}
