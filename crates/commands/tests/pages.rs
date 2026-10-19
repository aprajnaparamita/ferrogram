//! Headless page command tests (task P5-09, ADR 0047).
//!
//! Cover: add, delete (with a neighbour taking over), rename, reorder
//! and the move-left/right wrappers through the registry; the undo
//! step of each; selection clearing on page switches; and the argument
//! errors.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use ferrogram_commands::app::{AppCtx, DocumentSession, Settings};
use ferrogram_commands::builtins::registry;
use ferrogram_commands::{
    ArgValue, Args, CommandError, CommandId, Invocation, Outcome, Registry, SessionId, Source,
};
use ferrogram_geom::Unit;
use ferrogram_model::{Cell, CellName, CellValue, Document, PageId, Preserved, Shape, ShapeId};
use std::path::PathBuf;

fn fixture(names: &[&str]) -> (AppCtx, Registry, SessionId, Vec<PageId>) {
    let mut doc = Document::new();
    let mut pages = Vec::new();
    for name in names {
        pages.push(doc.add_page(name).expect("page"));
    }
    let mut ctx = AppCtx::new(Settings::default());
    let index = ctx.open_document(DocumentSession::new(doc, None));
    let session = ctx.documents.get(index).expect("session").id;
    (ctx, registry().expect("registry"), session, pages)
}

fn dispatch(
    ctx: &mut AppCtx,
    registry: &Registry,
    id: &str,
    args: Args,
) -> Result<Outcome, CommandError> {
    let ran_against = ctx.active_document().map(|session| session.id);
    let mut outcome = registry.execute(
        &CommandId::intern(id).expect("id"),
        ctx,
        Invocation::new(args, Source::Test),
    )?;
    if let Some(edit) = outcome.edit.take() {
        let target = ran_against.or_else(|| ctx.active_document().map(|session| session.id));
        match target {
            Some(session) => ctx.push_edit(session, edit)?,
            None => ctx.record_warning("an undoable edit with no open document was dropped"),
        }
    }
    Ok(outcome)
}

fn names(ctx: &AppCtx) -> Vec<String> {
    ctx.active_document()
        .expect("document")
        .document
        .pages
        .iter()
        .map(|page| page.name.clone())
        .collect()
}

fn active_name(ctx: &AppCtx) -> Option<String> {
    let session = ctx.active_document()?;
    let page = session.active_page?;
    session
        .document
        .pages
        .iter()
        .find(|entry| entry.id == page)
        .map(|entry| entry.name.clone())
}

fn active_page(ctx: &AppCtx) -> PageId {
    ctx.active_document()
        .expect("document")
        .active_page
        .expect("active page")
}

fn select(ctx: &mut AppCtx, page: PageId, shape: ShapeId) {
    ctx.selection.shapes = vec![shape];
    ctx.selection.anchor = Some(shape);
    ctx.selection.page = Some(page);
}

fn add_shape(ctx: &mut AppCtx, page: PageId) -> ShapeId {
    let mut shape = Shape::new(ShapeId { index: 0 }, 0);
    shape.sheet.cells.push(Cell {
        name: CellName::from_static("PinX"),
        formula: Some("0".to_owned()),
        value: Some(CellValue::Number(0.0)),
        unit: Unit::Inches,
        preserved: Preserved::new(),
    });
    ctx.active_document_mut()
        .expect("session")
        .document
        .insert_shape(page, shape, None)
        .expect("shape")
}

fn text(name: &str) -> Args {
    Args::new().with("name", ArgValue::Text(name.to_owned()))
}

#[test]
fn add_names_and_undoes_a_new_page() {
    let (mut ctx, registry, _session, _pages) = fixture(&["One", "Two"]);
    dispatch(&mut ctx, &registry, "page.add", text("Flow")).expect("add");
    assert_eq!(names(&ctx), vec!["One", "Two", "Flow"]);
    assert_eq!(active_name(&ctx).as_deref(), Some("Flow"));
    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(names(&ctx), vec!["One", "Two"]);
    assert_eq!(active_name(&ctx).as_deref(), Some("One"));
    dispatch(&mut ctx, &registry, "edit.redo", Args::new()).expect("redo");
    assert_eq!(names(&ctx), vec!["One", "Two", "Flow"]);

    let (mut ctx, registry, _session, _pages) = fixture(&["One", "Two"]);
    dispatch(&mut ctx, &registry, "page.add", Args::new()).expect("default name");
    assert_eq!(names(&ctx), vec!["One", "Two", "Page 3"]);

    let mut bad = Args::new();
    bad.set("name", ArgValue::Integer(1));
    let error = dispatch(&mut ctx, &registry, "page.add", bad).expect_err("bad name");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));

    // A blank name falls back to the default rather than an empty page
    // name.
    dispatch(&mut ctx, &registry, "page.add", text("  ")).expect("blank name");
    assert_eq!(names(&ctx).last().map(String::as_str), Some("Page 4"));
}

#[test]
fn delete_activates_a_neighbour_clears_the_selection_and_undoes() {
    let (mut ctx, registry, _session, pages) = fixture(&["One", "Two", "Three"]);
    let shape = add_shape(&mut ctx, pages[1]);
    select(&mut ctx, pages[1], shape);
    ctx.active_document_mut().expect("session").active_page = Some(pages[1]);
    assert_eq!(active_name(&ctx).as_deref(), Some("Two"));

    dispatch(&mut ctx, &registry, "page.delete", Args::new()).expect("delete");
    assert_eq!(names(&ctx), vec!["One", "Three"]);
    assert_eq!(
        active_name(&ctx).as_deref(),
        Some("Three"),
        "next neighbour"
    );
    assert!(ctx.selection.is_empty(), "the selection cleared");
    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(names(&ctx), vec!["One", "Two", "Three"]);
    assert_eq!(active_name(&ctx).as_deref(), Some("Two"));
    dispatch(&mut ctx, &registry, "edit.redo", Args::new()).expect("redo");
    assert_eq!(names(&ctx), vec!["One", "Three"]);
}

#[test]
fn deleting_the_last_page_takes_the_previous_neighbour() {
    let (mut ctx, registry, _session, pages) = fixture(&["One", "Two", "Three"]);
    ctx.active_document_mut().expect("session").active_page = Some(pages[2]);
    dispatch(&mut ctx, &registry, "page.delete", Args::new()).expect("delete");
    assert_eq!(names(&ctx), vec!["One", "Two"]);
    assert_eq!(active_name(&ctx).as_deref(), Some("Two"));
}

#[test]
fn rename_round_trips_through_undo_and_redo() {
    let (mut ctx, registry, _session, _pages) = fixture(&["One", "Two"]);
    dispatch(&mut ctx, &registry, "page.rename", text("Flow")).expect("rename");
    assert_eq!(names(&ctx), vec!["Flow", "Two"]);
    let page = ctx.active_document().expect("session").document.pages[0].clone();
    assert_eq!(page.name_u, "Flow");
    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(names(&ctx), vec!["One", "Two"]);
    dispatch(&mut ctx, &registry, "edit.redo", Args::new()).expect("redo");
    assert_eq!(names(&ctx), vec!["Flow", "Two"]);
}

#[test]
fn reorder_and_move_keep_the_active_page_and_undo() {
    let (mut ctx, registry, _session, _pages) = fixture(&["One", "Two", "Three"]);
    let mut args = Args::new();
    args.set("from", ArgValue::Integer(0));
    args.set("to", ArgValue::Integer(2));
    dispatch(&mut ctx, &registry, "page.reorder", args).expect("reorder");
    assert_eq!(names(&ctx), vec!["Two", "Three", "One"]);
    assert_eq!(
        active_name(&ctx).as_deref(),
        Some("One"),
        "active page follows"
    );
    assert_eq!(active_page(&ctx), PageId { index: 2 });
    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(names(&ctx), vec!["One", "Two", "Three"]);
    assert_eq!(active_name(&ctx).as_deref(), Some("One"));
    dispatch(&mut ctx, &registry, "edit.redo", Args::new()).expect("redo");
    assert_eq!(names(&ctx), vec!["Two", "Three", "One"]);

    dispatch(&mut ctx, &registry, "page.move_left", Args::new()).expect("left");
    assert_eq!(names(&ctx), vec!["Two", "One", "Three"]);
    assert_eq!(active_name(&ctx).as_deref(), Some("One"));
    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo left");
    assert_eq!(names(&ctx), vec!["Two", "Three", "One"]);

    ctx.active_document_mut().expect("session").active_page = Some(PageId { index: 0 });
    dispatch(&mut ctx, &registry, "page.move_right", Args::new()).expect("right");
    assert_eq!(names(&ctx), vec!["Three", "Two", "One"]);
    assert_eq!(active_name(&ctx).as_deref(), Some("Two"));
    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo right");
    assert_eq!(names(&ctx), vec!["Two", "Three", "One"]);
}

#[test]
fn page_switch_clears_the_selection() {
    let (mut ctx, registry, _session, pages) = fixture(&["One", "Two"]);
    let shape = add_shape(&mut ctx, pages[0]);
    select(&mut ctx, pages[0], shape);
    dispatch(&mut ctx, &registry, "page.next", Args::new()).expect("next");
    assert_eq!(active_name(&ctx).as_deref(), Some("Two"));
    assert!(ctx.selection.is_empty());
    select(&mut ctx, pages[0], shape);
    dispatch(&mut ctx, &registry, "page.previous", Args::new()).expect("previous");
    assert_eq!(active_name(&ctx).as_deref(), Some("One"));
    assert!(ctx.selection.is_empty());
}

#[test]
fn page_argument_and_state_failures() {
    let (mut ctx, registry, _session, _pages) = fixture(&["Only"]);
    for id in [
        "page.delete",
        "page.reorder",
        "page.move_left",
        "page.move_right",
    ] {
        let command = CommandId::intern(id).expect("id");
        let error = registry
            .execute(&command, &mut ctx, Invocation::empty(Source::Test))
            .expect_err(id);
        assert!(
            matches!(error, CommandError::NotEnabled { .. }),
            "{id}: {error:?}"
        );
    }

    let error = dispatch(&mut ctx, &registry, "page.rename", Args::new()).expect_err("no name");
    assert!(matches!(error, CommandError::NeedsInput(_)));
    let mut bad = Args::new();
    bad.set("name", ArgValue::Integer(1));
    let error = dispatch(&mut ctx, &registry, "page.rename", bad).expect_err("bad name");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));
    let error = dispatch(&mut ctx, &registry, "page.rename", text("")).expect_err("empty name");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));

    let (mut ctx, registry, _session, _pages) = fixture(&["One", "Two"]);
    let error = dispatch(&mut ctx, &registry, "page.reorder", Args::new()).expect_err("no from");
    assert!(matches!(error, CommandError::NeedsInput(_)));
    let mut args = Args::new();
    args.set("from", ArgValue::Integer(9));
    args.set("to", ArgValue::Integer(0));
    let error = dispatch(&mut ctx, &registry, "page.reorder", args).expect_err("stale from");
    assert!(matches!(error, CommandError::Failed { .. }));
    let mut args = Args::new();
    args.set("from", ArgValue::Integer(0));
    args.set("to", ArgValue::Integer(9));
    dispatch(&mut ctx, &registry, "page.reorder", args).expect("to clamps");
    assert_eq!(names(&ctx), vec!["Two", "One"]);
    let mut args = Args::new();
    args.set("from", ArgValue::Integer(0));
    args.set("to", ArgValue::Number(0.5));
    let error = dispatch(&mut ctx, &registry, "page.reorder", args).expect_err("fractional to");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));
}

#[test]
fn page_add_without_a_document_is_disabled() {
    let mut ctx = AppCtx::new(Settings::default());
    let registry = registry().expect("registry");
    let error = registry
        .execute(
            &CommandId::intern("page.add").expect("id"),
            &mut ctx,
            Invocation::empty(Source::Test),
        )
        .expect_err("no document");
    assert!(matches!(error, CommandError::NotEnabled { .. }));
}

/// The P4-02 item: `file.open` records the path in
/// `Settings::recent_files`, most recent first and de-duplicated.
#[test]
fn file_open_records_recent_files() {
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "ferrogram-pages-recent-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let one = dir.join("one.vsdx");
    let two = dir.join("two.vsdx");
    let mut doc = Document::new();
    doc.add_page("Page-1").expect("page");
    let package = ferrogram_vsdx::from_model(&doc).expect("package");
    let bytes = ferrogram_vsdx::write_package(&package).expect("bytes");
    std::fs::write(&one, &bytes).expect("one");
    std::fs::write(&two, &bytes).expect("two");

    let (mut ctx, registry, _session, _pages) = fixture(&["Page-1"]);
    for path in [&one, &two, &one] {
        let args = Args::new().with("path", ArgValue::Path(PathBuf::from(path)));
        dispatch(&mut ctx, &registry, "file.open", args).expect("open");
    }
    assert_eq!(ctx.settings.recent_files, vec![one, two]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn page_navigation_without_a_recorded_active_page_uses_the_ends() {
    let (mut ctx, registry, _session, pages) = fixture(&["One", "Two", "Three"]);
    ctx.active_document_mut().expect("session").active_page = None;
    dispatch(&mut ctx, &registry, "page.next", Args::new()).expect("next");
    assert_eq!(active_page(&ctx), pages[0]);
    ctx.active_document_mut().expect("session").active_page = None;
    dispatch(&mut ctx, &registry, "page.previous", Args::new()).expect("previous");
    assert_eq!(active_page(&ctx), pages[2]);
}

#[test]
fn select_rejects_a_non_text_mode() {
    let (mut ctx, registry, _session, _pages) = fixture(&["One"]);
    let args = Args::new().with("mode", ArgValue::Integer(1));
    let error = dispatch(&mut ctx, &registry, "edit.select", args).expect_err("bad mode");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));
}
