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
    Cell, CellName, CellReader, CellWrite, DetachedShapes, Document, DocumentedDefault, PageId,
    Section, ShapeId, SheetRef, documented_default,
};

use crate::app::{AppCtx, SessionId};
use crate::edit::CompositeEdit;
use crate::error::CommandError;
use crate::undo::Edit;
use crate::{Invocation, Outcome};

use super::connector::reroute_after_edit;
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
pub(crate) fn number_literal(value: f64) -> String {
    if value == 0.0 {
        "0".to_owned()
    } else {
        format!("{value}")
    }
}

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

/// One undoable sheet-cell write (ADR 0038): the exact prior entries of
/// every cell the write touches, and the write to replay.
///
/// `apply` goes through [`Document::write_cell`] (the mutation API, ADR
/// 0039), which also refreshes the derived cells; `revert` restores the
/// captured entries exactly, so `revert(apply(x)) == x` holds for the
/// model, not just for the lookups.
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
}

impl CellEdit {
    /// Stages one write of `cell` on `sheet`, capturing the exact prior
    /// entries it will overwrite (the target and the cells the
    /// documented-defaults table derives from it).
    pub(crate) fn new(
        doc: &Document,
        session: SessionId,
        label: impl Into<String>,
        sheet: SheetRef,
        cell: &str,
        new: CellWrite,
    ) -> Result<Self, CommandError> {
        let cells = sheet_cells(doc, sheet)?;
        let old = touched_cells(cell)
            .into_iter()
            .map(|name| {
                let existing = cells
                    .iter()
                    .find(|entry| entry.name.as_str() == name.as_str())
                    .cloned();
                (name, existing)
            })
            .collect();
        Ok(Self {
            session,
            label: label.into(),
            sheet,
            cell: CellName::new(cell),
            old,
            new,
        })
    }
}

impl Edit for CellEdit {
    fn label(&self) -> &str {
        &self.label
    }

    fn apply(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError> {
        let session = self.session;
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
#[allow(dead_code)] // `Insert` is reached through `StructuralEdit::insertion`.
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
    /// error, never a panic.
    ///
    /// No P5-04 command inserts shapes: the `insert.*`, `edit.paste`
    /// and `edit.duplicate` commands (P5-06..P5-08) use this constructor
    /// (`#[allow(dead_code)]` until they land; the unit tests cover it).
    #[allow(dead_code)]
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
        Ok(Self {
            session,
            label: label.into(),
            root,
            page: page_key(doc, page)?,
            polarity,
            snapshot: None,
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

/// One undoable geometry rescale (ADR 0038 rule 3, the resize without a
/// formula engine): [`Document::scale_geometry`] forward, the exact
/// prior geometry sections back.
///
/// `CellEdit` cannot express this: a rescale rewrites whole geometry
/// sections (it materialises inherited rows), so the inverse is the
/// section list, not a cell write. (Task P5-04 deviation note: a third
/// edit type, because the interface named only `CellEdit` and
/// `StructuralEdit`.)
#[derive(Debug)]
pub(crate) struct GeometryEdit {
    session: SessionId,
    label: String,
    shape: ShapeId,
    from: Size,
    to: Size,
    /// The shape's geometry sections before the first rescale.
    old_sections: Option<Vec<Section>>,
}

impl GeometryEdit {
    /// The edit that rescales `shape` from the `from` box to the `to`
    /// box (inches). The caller checks the sizes; the model rejects a
    /// degenerate `from` and any non-finite value.
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
        Ok(Self {
            session,
            label: label.into(),
            shape,
            from,
            to,
            old_sections: None,
        })
    }
}

impl Edit for GeometryEdit {
    fn label(&self) -> &str {
        &self.label
    }

    fn apply(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError> {
        let session = self.session;
        let doc = document_mut(ctx, session)?;
        if self.old_sections.is_none() {
            let entity = doc.shape_mut(self.shape).ok_or_else(|| {
                CommandError::failed(format!("unknown shape id {}", self.shape.index))
            })?;
            self.old_sections = Some(entity.sheet.sections.clone());
        }
        doc.scale_geometry(self.shape, self.from, self.to)
            .map_err(model_error)?;
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
/// The removed shapes leave the selection; an entered group that went
/// with them is exited.
pub(super) fn run_delete(ctx: &mut AppCtx, _inv: &Invocation) -> Result<Outcome, CommandError> {
    let (session, targets, mut warnings) = selection_targets(ctx)?;
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
    if let Some(page) = page
        && let Err(error) = reroute_after_edit(ctx, page)
    {
        ctx.record_warning(format!("connectors were not re-routed: {error}"));
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
}
