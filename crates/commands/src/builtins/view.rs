//! The `edit`, `view`, `page` and `help` commands (ADR 0028): undo,
//! zoom, the grid and rulers toggles, the ribbon layout switch (ADR
//! 0032), page navigation and the About request.
//!
//! Every view command works on the active document's [`ViewState`]
//! (ADR 0029); without an open document they are disabled with a
//! reason instead of doing nothing.

use ferrogram_model::{CellValue, Page};

use crate::app::AppCtx;
use crate::error::CommandError;
use crate::{Invocation, Outcome, UiRequest};

use super::{no_document, outcome};

/// The zoom change of one `view.zoom.in` / `view.zoom.out` step.
const ZOOM_STEP: f64 = 1.25;

/// The lowest zoom the view commands set (ADR 0029: `zoom > 0`).
const ZOOM_MIN: f64 = 0.05;

/// The highest zoom the view commands set.
const ZOOM_MAX: f64 = 40.0;

/// The page size of a sheet with no positive `PageWidth` or
/// `PageHeight`: US Letter portrait, per axis (the same fallback the
/// renderer applies in `ferrogram_render::build`).
const FALLBACK_PAGE_SIZE: (f64, f64) = (8.5, 11.0);

/// `edit.undo`: reverts the newest edit of the active document.
pub(super) fn run_undo(ctx: &mut AppCtx, _inv: &Invocation) -> Result<Outcome, CommandError> {
    match with_undo(ctx, |stack, ctx| stack.undo(ctx))? {
        Some(label) => Ok(outcome(format!("Undo {label}"), Vec::new())),
        None => Ok(outcome("Nothing to undo", Vec::new())),
    }
}

/// `edit.redo`: re-applies the newest redoable edit.
pub(super) fn run_redo(ctx: &mut AppCtx, _inv: &Invocation) -> Result<Outcome, CommandError> {
    match with_undo(ctx, |stack, ctx| stack.redo(ctx))? {
        Some(label) => Ok(outcome(format!("Redo {label}"), Vec::new())),
        None => Ok(outcome("Nothing to redo", Vec::new())),
    }
}

/// Runs `step` on the active document's undo stack.
///
/// The stack lives inside [`AppCtx`] (ADR 0029) but the step needs
/// `&mut AppCtx` for its [`Edit`](crate::Edit)s, so it is taken out
/// for the call and put back afterwards.
fn with_undo(
    ctx: &mut AppCtx,
    step: fn(&mut crate::undo::UndoStack, &mut AppCtx) -> Result<Option<String>, CommandError>,
) -> Result<Option<String>, CommandError> {
    let Some(index) = ctx.active else {
        return Err(no_document());
    };
    let Some(session) = ctx.documents.get_mut(index) else {
        return Err(no_document());
    };
    let mut stack = std::mem::take(&mut session.undo);
    let result = step(&mut stack, ctx);
    if let Some(session) = ctx.documents.get_mut(index) {
        session.undo = stack;
    }
    result
}

/// `view.zoom.in`: multiplies the zoom by [`ZOOM_STEP`].
pub(super) fn run_zoom_in(ctx: &mut AppCtx, _inv: &Invocation) -> Result<Outcome, CommandError> {
    zoom_by(ctx, ZOOM_STEP)
}

/// `view.zoom.out`: divides the zoom by [`ZOOM_STEP`].
pub(super) fn run_zoom_out(ctx: &mut AppCtx, _inv: &Invocation) -> Result<Outcome, CommandError> {
    zoom_by(ctx, 1.0 / ZOOM_STEP)
}

/// Sets the zoom of the active document, clamped to
/// [`ZOOM_MIN`]..=[`ZOOM_MAX`].
fn zoom_by(ctx: &mut AppCtx, factor: f64) -> Result<Outcome, CommandError> {
    let session = ctx.active_document_mut().ok_or_else(no_document)?;
    let zoom = (session.view.zoom * factor).clamp(ZOOM_MIN, ZOOM_MAX);
    session.view.zoom = zoom;
    Ok(outcome(format!("Zoom {:.0}%", zoom * 100.0), Vec::new()))
}

/// `view.zoom.fit`: zooms so the active page fits the canvas rectangle
/// given by the `width` and `height` arguments, in points (ADR 0028).
pub(super) fn run_zoom_fit(ctx: &mut AppCtx, inv: &Invocation) -> Result<Outcome, CommandError> {
    let width = need_number(&inv.args, "width", "canvas width in points")?;
    let height = need_number(&inv.args, "height", "canvas height in points")?;
    let session = ctx.active_document_mut().ok_or_else(no_document)?;
    let page_id = session
        .active_page
        .ok_or_else(|| CommandError::failed("no active page"))?;
    let page = session
        .document
        .pages
        .iter()
        .find(|page| page.id == page_id)
        .ok_or_else(|| CommandError::failed("no active page"))?;
    let (page_width, page_height) = page_size(page);
    let fit = ((width / (page_width * 72.0)).min(height / (page_height * 72.0)))
        .clamp(ZOOM_MIN, ZOOM_MAX);
    session.view.zoom = fit;
    Ok(outcome(format!("Zoom {:.0}%", fit * 100.0), Vec::new()))
}

/// A positive `width` / `height` argument in points, or
/// [`CommandError::NeedsInput`] when it is missing.
fn need_number(args: &crate::Args, name: &str, prompt: &str) -> Result<f64, CommandError> {
    match args.number(name) {
        Some(value) if value.is_finite() && value > 0.0 => Ok(value),
        Some(_) => Err(CommandError::invalid_args(format!(
            "{name} must be finite and positive"
        ))),
        None => Err(CommandError::NeedsInput(crate::NeededInput::Number {
            prompt: prompt.to_owned(),
            default: None,
        })),
    }
}

/// The page size in inches, mirroring the renderer's per-axis fallback.
fn page_size(page: &Page) -> (f64, f64) {
    (
        positive_cell(page, "PageWidth", FALLBACK_PAGE_SIZE.0),
        positive_cell(page, "PageHeight", FALLBACK_PAGE_SIZE.1),
    )
}

/// The positive cached number of one page-sheet cell, or `fallback`.
fn positive_cell(page: &Page, name: &str, fallback: f64) -> f64 {
    page.sheet
        .cells
        .iter()
        .find(|cell| cell.name.as_str() == name)
        .and_then(|cell| match &cell.value {
            Some(CellValue::Number(value)) if value.is_finite() && *value > 0.0 => Some(*value),
            _ => None,
        })
        .unwrap_or(fallback)
}

/// `view.toggle_grid`: flips the grid of the active document.
pub(super) fn run_toggle_grid(
    ctx: &mut AppCtx,
    _inv: &Invocation,
) -> Result<Outcome, CommandError> {
    let session = ctx.active_document_mut().ok_or_else(no_document)?;
    session.view.show_grid = !session.view.show_grid;
    Ok(outcome(on_off("Grid", session.view.show_grid), Vec::new()))
}

/// `view.rulers`: flips the rulers of the active document.
pub(super) fn run_toggle_rulers(
    ctx: &mut AppCtx,
    _inv: &Invocation,
) -> Result<Outcome, CommandError> {
    let session = ctx.active_document_mut().ok_or_else(no_document)?;
    session.view.show_rulers = !session.view.show_rulers;
    Ok(outcome(
        on_off("Rulers", session.view.show_rulers),
        Vec::new(),
    ))
}

/// The status line of a boolean view toggle.
fn on_off(what: &str, on: bool) -> String {
    format!("{what} {}", if on { "on" } else { "off" })
}

/// `view.ui_layout.visio_2016`: switches the ribbon to the 2016 layout
/// (ADR 0032; always enabled, no restart).
pub(super) fn run_ui_layout_2016(
    ctx: &mut AppCtx,
    _inv: &Invocation,
) -> Result<Outcome, CommandError> {
    ctx.settings.ui_layout = ferrogram_ui_spec::LayoutKind::Visio2016;
    Ok(outcome("Ribbon layout: Visio 2016", Vec::new()))
}

/// `view.ui_layout.visio_365`: switches the ribbon to the Microsoft
/// 365 layout (ADR 0032).
pub(super) fn run_ui_layout_365(
    ctx: &mut AppCtx,
    _inv: &Invocation,
) -> Result<Outcome, CommandError> {
    ctx.settings.ui_layout = ferrogram_ui_spec::LayoutKind::Visio365;
    Ok(outcome("Ribbon layout: Visio 365", Vec::new()))
}

/// `page.next`: moves to the next page, wrapping at the end.
pub(super) fn run_page_next(ctx: &mut AppCtx, _inv: &Invocation) -> Result<Outcome, CommandError> {
    move_page(ctx, 1)
}

/// `page.previous`: moves to the previous page, wrapping at the start.
pub(super) fn run_page_previous(
    ctx: &mut AppCtx,
    _inv: &Invocation,
) -> Result<Outcome, CommandError> {
    move_page(ctx, -1)
}

/// Moves the active page by `delta`, wrapping over the document's
/// pages.
fn move_page(ctx: &mut AppCtx, delta: i64) -> Result<Outcome, CommandError> {
    let session = ctx.active_document_mut().ok_or_else(no_document)?;
    let len = session.document.pages.len();
    if len == 0 {
        return Err(CommandError::failed("the document has no pages"));
    }
    let current = session
        .active_page
        .and_then(|id| session.document.pages.iter().position(|page| page.id == id));
    let index = match current {
        Some(index) => (index as i64 + delta).rem_euclid(len as i64) as usize,
        None if delta >= 0 => 0,
        None => len.saturating_sub(1),
    };
    let Some(page) = session.document.pages.get(index) else {
        return Err(CommandError::failed("the document has no pages"));
    };
    let number = index.saturating_add(1);
    session.active_page = Some(page.id);
    let label = if page.name.is_empty() {
        format!("Page {number}")
    } else {
        page.name.clone()
    };
    Ok(outcome(label, Vec::new()))
}

/// `help.about`: asks the dispatcher to show the About window
/// (never a dialog from inside the command, AGENTS.md section 6).
pub(super) fn run_about(_ctx: &mut AppCtx, _inv: &Invocation) -> Result<Outcome, CommandError> {
    Ok(Outcome {
        message: Some("About Ferrogram".to_owned()),
        ui_request: Some(UiRequest::ShowAbout),
        ..Outcome::default()
    })
}
