//! `page.*` commands (P5-09).
//!
//! Page commands: add, rename, delete and reorder (ADR 0047), plus the
//! `page.move_left` / `page.move_right` tab wrappers. Every command uses
//! the ADR 0039 page primitives and returns one undoable [`PagesEdit`]
//! snapshot of the page list. `PageId` is a list position, so an edit
//! that moves or deletes a page swaps the whole list back on undo; the
//! active page is part of the snapshot and a page switch clears the
//! selection (ADR 0041).

use ferrogram_model::{Document, Page, PageId};

use crate::app::{AppCtx, SessionId};
use crate::error::CommandError;
use crate::undo::Edit;
use crate::{ArgValue, Invocation, Outcome};

use super::edit::document_mut;
use super::{CommandTable, edit_outcome, has_document, has_pages, model_error};

/// The implemented page commands (`page.next` / `page.previous` stay in
/// `view.rs`, where they started).
pub(crate) const TABLE: CommandTable = &[
    ("page.add", "New Page", has_document, run_add),
    ("page.delete", "Delete Page", can_delete, run_delete),
    ("page.rename", "Rename Page", has_pages, run_rename),
    ("page.reorder", "Reorder", can_move, run_reorder),
    ("page.move_left", "Move Page Left", can_move, run_move_left),
    (
        "page.move_right",
        "Move Page Right",
        can_move,
        run_move_right,
    ),
];

/// Enabled when there is more than one page to delete from.
fn can_delete(ctx: &AppCtx) -> crate::Enabled {
    match ctx.active_document() {
        None => crate::Enabled::no("no open document"),
        Some(session) if session.document.pages.len() < 2 => {
            crate::Enabled::no("the document has only one page")
        }
        Some(_) => crate::Enabled::yes(),
    }
}

/// Enabled when the document has at least two pages.
fn can_move(ctx: &AppCtx) -> crate::Enabled {
    match ctx.active_document() {
        None => crate::Enabled::no("no open document"),
        Some(session) if session.document.pages.len() < 2 => {
            crate::Enabled::no("the document has only one page")
        }
        Some(_) => crate::Enabled::yes(),
    }
}

/// The active session and its active page (the first page as a fallback).
fn active_page(ctx: &AppCtx) -> Result<(SessionId, PageId), CommandError> {
    let session = ctx.active_document().ok_or_else(super::no_document)?;
    let page = session
        .active_page
        .or_else(|| session.document.pages.first().map(|page| page.id))
        .ok_or_else(|| CommandError::failed("the document has no pages"))?;
    Ok((session.id, page))
}

/// The page snapshot an edit restores in either direction.
struct PagesEdit {
    session: SessionId,
    label: String,
    old: Vec<Page>,
    old_active: Option<PageId>,
    new: Vec<Page>,
    new_active: Option<PageId>,
}

impl PagesEdit {
    fn new(
        session: SessionId,
        label: impl Into<String>,
        old: Vec<Page>,
        old_active: Option<PageId>,
        new: Vec<Page>,
        new_active: Option<PageId>,
    ) -> Self {
        Self {
            session,
            label: label.into(),
            old,
            old_active,
            new,
            new_active,
        }
    }

    /// Replaces the page list and active page, clears the selection when
    /// the active page changed, and bumps the change counter (ADR 0040).
    ///
    /// The pages' names and membership come from the snapshot, but a
    /// page that is still open keeps its live `shapes` and `Connects`
    /// lists (task P5-11): those belong to the edits that moved shapes
    /// or changed glue, and a snapshot copy can name arena ids that a
    /// re-done group or insert has replaced.
    fn restore(
        &self,
        ctx: &mut AppCtx,
        pages: &[Page],
        active: Option<PageId>,
    ) -> Result<(), CommandError> {
        let session = self.session;
        let doc = document_mut(ctx, session)?;
        let live = doc.pages.clone();
        let mut restored: Vec<Page> = Vec::with_capacity(pages.len());
        for wanted in pages {
            let mut page = wanted.clone();
            if let Some(current) = live.iter().find(|page| page.visio_id == wanted.visio_id) {
                page.shapes = current.shapes.clone();
                page.connects = current.connects.clone();
            }
            restored.push(page);
        }
        doc.pages = restored;
        let active_changed = {
            let slot = ctx
                .documents
                .iter_mut()
                .find(|slot| slot.id == session)
                .ok_or_else(|| CommandError::failed("the document session is gone"))?;
            let previous = slot.active_page;
            slot.active_page = active;
            previous != active
        };
        if active_changed && ctx.active_document().map(|slot| slot.id) == Some(session) {
            ctx.selection.clear();
        }
        ctx.touch_session(session);
        Ok(())
    }
}

impl Edit for PagesEdit {
    fn label(&self) -> &str {
        &self.label
    }

    fn apply(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError> {
        self.restore(ctx, &self.new, self.new_active)
    }

    fn revert(&mut self, ctx: &mut AppCtx) -> Result<(), CommandError> {
        self.restore(ctx, &self.old, self.old_active)
    }
}

/// `page.add`: adds a new page above the current last page and makes it
/// active. The optional `name` names it; without one the name is
/// `Page N`.
fn run_add(ctx: &mut AppCtx, inv: &Invocation) -> Result<Outcome, CommandError> {
    let session = ctx.active_document().ok_or_else(super::no_document)?;
    let session_id = session.id;
    let old = session.document.pages.clone();
    let old_active = session.active_page;
    let default = format!("Page {}", old.len().saturating_add(1));
    let name = match inv.args.get("name") {
        None => default,
        Some(ArgValue::Text(name)) if !name.trim().is_empty() => name.clone(),
        Some(ArgValue::Text(_)) => default,
        Some(_) => return Err(CommandError::invalid_args("name must be text")),
    };
    let page = {
        let doc = document_mut(ctx, session_id)?;
        doc.add_page(&name).map_err(model_error)?
    };
    let new = {
        let doc = document_mut(ctx, session_id)?;
        doc.pages.clone()
    };
    if let Some(session) = ctx.documents.iter_mut().find(|slot| slot.id == session_id) {
        session.active_page = Some(page);
    }
    ctx.selection.clear();
    ctx.touch_session(session_id);
    let edit = PagesEdit::new(session_id, "Add page", old, old_active, new, Some(page));
    Ok(edit_outcome(
        format!("Added {name}"),
        Vec::new(),
        Box::new(edit),
    ))
}

/// `page.delete`: removes the active page and activates its neighbour.
fn run_delete(ctx: &mut AppCtx, _inv: &Invocation) -> Result<Outcome, CommandError> {
    let (session_id, page) = active_page(ctx)?;
    let session = ctx.active_document().ok_or_else(super::no_document)?;
    let old = session.document.pages.clone();
    let old_active = session.active_page;
    if old.len() < 2 {
        return Err(CommandError::failed("cannot delete the only page"));
    }
    let index = old
        .iter()
        .position(|entry| entry.id == page)
        .ok_or_else(|| CommandError::failed("the active page is gone"))?;
    let removed = {
        let doc = document_mut(ctx, session_id)?;
        doc.delete_page(page).map_err(model_error)?
    };
    let (new, next) = {
        let doc = document_mut(ctx, session_id)?;
        let next = if index < doc.pages.len() {
            doc.pages.get(index).map(|entry| entry.id)
        } else {
            index
                .checked_sub(1)
                .and_then(|previous| doc.pages.get(previous).map(|entry| entry.id))
        };
        (doc.pages.clone(), next)
    };
    if let Some(session) = ctx.documents.iter_mut().find(|slot| slot.id == session_id) {
        session.active_page = next;
    }
    ctx.selection.clear();
    ctx.touch_session(session_id);
    let edit = PagesEdit::new(session_id, "Delete page", old, old_active, new, next);
    Ok(edit_outcome(
        format!("Deleted {}", page_label(&removed, index)),
        Vec::new(),
        Box::new(edit),
    ))
}

/// `page.rename`: renames the active page from the required `name`
/// argument.
fn run_rename(ctx: &mut AppCtx, inv: &Invocation) -> Result<Outcome, CommandError> {
    let name = match inv.args.get("name") {
        None => {
            return Err(CommandError::NeedsInput(crate::NeededInput::Text {
                prompt: "page name".to_owned(),
                default: None,
            }));
        }
        Some(ArgValue::Text(name)) if !name.trim().is_empty() => name.clone(),
        Some(ArgValue::Text(_)) => {
            return Err(CommandError::invalid_args(
                "the page name must not be empty",
            ));
        }
        Some(_) => return Err(CommandError::invalid_args("name must be text")),
    };
    let (session_id, page) = active_page(ctx)?;
    let session = ctx.active_document().ok_or_else(super::no_document)?;
    let old = session.document.pages.clone();
    let old_active = session.active_page;
    {
        let doc = document_mut(ctx, session_id)?;
        doc.rename_page(page, &name).map_err(model_error)?;
    }
    let new = {
        let doc = document_mut(ctx, session_id)?;
        doc.pages.clone()
    };
    ctx.touch_session(session_id);
    let edit = PagesEdit::new(session_id, "Rename page", old, old_active, new, old_active);
    Ok(edit_outcome(
        format!("Renamed page to {name}"),
        Vec::new(),
        Box::new(edit),
    ))
}

/// `page.reorder`: moves the page at zero-based `from` to `to`.
fn run_reorder(ctx: &mut AppCtx, inv: &Invocation) -> Result<Outcome, CommandError> {
    let from = need_index(&inv.args, "from")?;
    let to = need_index(&inv.args, "to")?;
    reorder(ctx, from, to)
}

/// `page.move_left`: moves the active page one position towards the
/// start.
fn run_move_left(ctx: &mut AppCtx, _inv: &Invocation) -> Result<Outcome, CommandError> {
    move_active(ctx, -1)
}

/// `page.move_right`: moves the active page one position towards the
/// end.
fn run_move_right(ctx: &mut AppCtx, _inv: &Invocation) -> Result<Outcome, CommandError> {
    move_active(ctx, 1)
}

/// A required non-negative zero-based index argument.
fn need_index(args: &crate::Args, name: &str) -> Result<usize, CommandError> {
    match args.get(name) {
        None => Err(CommandError::NeedsInput(crate::NeededInput::Number {
            prompt: format!("{name} page index (from 0)"),
            default: None,
        })),
        Some(ArgValue::Integer(value)) if *value >= 0 => usize::try_from(*value)
            .map_err(|_| CommandError::invalid_args(format!("{name} is too large"))),
        Some(ArgValue::Number(value))
            if value.is_finite() && *value >= 0.0 && value.fract() == 0.0 =>
        {
            usize::try_from(*value as i64)
                .map_err(|_| CommandError::invalid_args(format!("{name} is too large")))
        }
        Some(_) => Err(CommandError::invalid_args(format!(
            "{name} must be a whole number of pages from 0"
        ))),
    }
}

/// Moves the active page by `delta`, staying inside the list.
fn move_active(ctx: &mut AppCtx, delta: i64) -> Result<Outcome, CommandError> {
    let (_, page) = active_page(ctx)?;
    let session = ctx.active_document().ok_or_else(super::no_document)?;
    let index = session
        .document
        .pages
        .iter()
        .position(|entry| entry.id == page)
        .ok_or_else(|| CommandError::failed("the active page is gone"))?;
    let len = session.document.pages.len();
    let target = if delta < 0 {
        index.saturating_sub(delta.unsigned_abs() as usize)
    } else {
        index
            .saturating_add(usize::try_from(delta).unwrap_or(usize::MAX))
            .min(len.saturating_sub(1))
    };
    reorder(ctx, index, target)
}

/// Moves the page at list index `from` to index `to` (clamped by the
/// model) and keeps the active page on the same page.
fn reorder(ctx: &mut AppCtx, from: usize, to: usize) -> Result<Outcome, CommandError> {
    let (session_id, page) = active_page(ctx)?;
    let session = ctx.active_document().ok_or_else(super::no_document)?;
    let old = session.document.pages.clone();
    let old_active = session.active_page;
    let active_visio = old
        .iter()
        .find(|entry| entry.id == page)
        .map(|entry| entry.visio_id)
        .ok_or_else(|| CommandError::failed("the active page is gone"))?;
    {
        let doc = document_mut(ctx, session_id)?;
        doc.move_page(from, to).map_err(model_error)?;
    }
    let (new, next) = {
        let doc = document_mut(ctx, session_id)?;
        let next = doc
            .pages
            .iter()
            .find(|entry| entry.visio_id == active_visio)
            .map(|entry| entry.id);
        (doc.pages.clone(), next)
    };
    if let Some(session) = ctx.documents.iter_mut().find(|slot| slot.id == session_id) {
        session.active_page = next;
    }
    let refresh = ctx
        .active_document()
        .filter(|session| session.id == session_id)
        .map(|session| session.document.clone());
    if let Some(document) = refresh {
        ctx.selection.refresh_page(&document);
    }
    ctx.touch_session(session_id);
    let name = new
        .iter()
        .find(|entry| Some(entry.id) == next)
        .map(|entry| page_label(entry, next.map(|id| id.index as usize).unwrap_or(0)))
        .unwrap_or_else(|| "page".to_owned());
    let edit = PagesEdit::new(session_id, "Reorder page", old, old_active, new, next);
    Ok(edit_outcome(
        format!("Moved {name}"),
        Vec::new(),
        Box::new(edit),
    ))
}

/// The display name of one page: its `Name`, else its number.
fn page_label(page: &Page, index: usize) -> String {
    if page.name.is_empty() {
        format!("Page {}", index.saturating_add(1))
    } else {
        page.name.clone()
    }
}

/// A document with no pages cannot be rendered; the check is exposed for
/// the writer's rule (ADR 0048) and keeps the handler's error path
/// explicit.
#[allow(dead_code)]
fn document_pages(doc: &Document) -> usize {
    doc.pages.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_label_falls_back_to_the_number() {
        let mut page = Page::new(PageId { index: 0 }, 1);
        assert_eq!(page_label(&page, 0), "Page 1");
        page.name = "Flow".to_owned();
        assert_eq!(page_label(&page, 4), "Flow");
        assert_eq!(document_pages(&Document::new()), 0);
    }

    #[test]
    fn need_index_accepts_both_number_forms_and_rejects_bad_ones() {
        let mut args = crate::Args::new();
        args.set("from", ArgValue::Integer(2));
        assert_eq!(need_index(&args, "from").expect("integer"), 2);
        args.set("from", ArgValue::Number(3.0));
        assert_eq!(need_index(&args, "from").expect("number"), 3);
        args.set("from", ArgValue::Number(3.5));
        assert!(matches!(
            need_index(&args, "from"),
            Err(CommandError::InvalidArgs { .. })
        ));
        args.set("from", ArgValue::Integer(-1));
        assert!(matches!(
            need_index(&args, "from"),
            Err(CommandError::InvalidArgs { .. })
        ));
        args.set("from", ArgValue::Text("two".to_owned()));
        assert!(matches!(
            need_index(&args, "from"),
            Err(CommandError::InvalidArgs { .. })
        ));
        args = crate::Args::new();
        let error = need_index(&args, "to").expect_err("missing");
        assert!(matches!(error, CommandError::NeedsInput(_)));
    }

    #[test]
    fn add_and_rename_names_are_validated() {
        let mut ctx = AppCtx::new(crate::Settings::default());
        let index = ctx.open_document(crate::DocumentSession::new(Document::new(), None));
        let session = ctx.documents.get(index).expect("session").id;
        let mut invocation = Invocation::empty(crate::Source::Test);
        invocation.args.set("name", ArgValue::Integer(1));
        assert!(matches!(
            run_add(&mut ctx, &invocation),
            Err(CommandError::InvalidArgs { .. })
        ));
        let _ = session;

        let mut invocation = Invocation::empty(crate::Source::Test);
        invocation.args.set("name", ArgValue::Text("  ".to_owned()));
        let outcome = run_add(&mut ctx, &invocation).expect("default name");
        assert_eq!(outcome.message.as_deref(), Some("Added Page 1"));
        let session = ctx.active_document().expect("session");
        assert_eq!(session.document.pages.len(), 1);
        assert_eq!(session.document.pages[0].name, "Page 1");

        let mut invocation = Invocation::empty(crate::Source::Test);
        invocation
            .args
            .set("name", ArgValue::Text("Flow".to_owned()));
        let outcome = run_rename(&mut ctx, &invocation).expect("rename");
        assert_eq!(outcome.message.as_deref(), Some("Renamed page to Flow"));

        let mut invocation = Invocation::empty(crate::Source::Test);
        invocation.args.set("name", ArgValue::Text(String::new()));
        assert!(matches!(
            run_rename(&mut ctx, &invocation),
            Err(CommandError::InvalidArgs { .. })
        ));
        let invocation = Invocation::empty(crate::Source::Test);
        let error = run_rename(&mut ctx, &invocation).expect_err("missing name");
        assert!(matches!(error, CommandError::NeedsInput(_)));
        let mut invocation = Invocation::empty(crate::Source::Test);
        invocation.args.set("name", ArgValue::Integer(1));
        assert!(matches!(
            run_rename(&mut ctx, &invocation),
            Err(CommandError::InvalidArgs { .. })
        ));
    }

    #[test]
    fn draw_without_pages_reports_it() {
        let mut ctx = AppCtx::new(crate::Settings::default());
        assert!(active_page(&ctx).is_err());
        ctx.open_document(crate::DocumentSession::new(Document::new(), None));
        assert!(active_page(&ctx).is_err());
        assert!(run_add(&mut ctx, &Invocation::empty(crate::Source::Test)).is_ok());
        assert!(active_page(&ctx).is_ok());
        assert!(matches!(
            run_delete(&mut ctx, &Invocation::empty(crate::Source::Test)),
            Err(CommandError::Failed { .. })
        ));
    }
}
