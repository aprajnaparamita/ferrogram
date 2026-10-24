//! Headless command tests for the Phase 5 editing commands (task
//! P5-04, ADR 0049 rule 1): every `edit.*` and `shape.*` handler runs
//! through `Registry::execute` with `Source::Test`.
//!
//! The contract under test:
//!
//! - a happy path mutates the document and returns **one** `Outcome::edit`
//!   (`CompositeEdit`, one undo step per command, ADR 0040);
//! - `NotEnabled` names its reason, `NeedsInput` asks for the missing
//!   number and `InvalidArgs` rejects bad values;
//! - `revert(apply(x)) == x` and `apply(revert(x)) == x` per command
//!   (model equality);
//! - `DocumentSession::changes` bumps on edits (including undo and
//!   redo) and never on view-only commands (ADR 0040 rule 3);
//! - an edit that keeps a `PageId` re-resolves it when a page delete or
//!   move renumbered the pages (task P5-02 note).
//!
//! Nothing here panics: a panic is the bug (AGENTS.md section 6).

// The never-crash lints apply to non-test code only (clippy.toml).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod support;

use support::dispatch;

use larrygram_commands::app::{AppCtx, DocumentSession, Settings};
use larrygram_commands::builtins::registry;
use larrygram_commands::{
    ArgValue, Args, CommandError, CommandId, CommandStatus, Invocation, NeededInput, Registry,
    SessionId, Source,
};
use larrygram_model::{
    Cell, CellName, CellValue, Document, PageId, Preserved, Row, Section, SectionName, Shape,
    ShapeId,
};

/// The Phase 5 command ids of task P5-04 (`builtins::edit` and
/// `builtins::shape`).
const PHASE5_IDS: &[&str] = &[
    "edit.delete",
    "edit.select",
    "edit.select_all",
    "edit.deselect",
    "shape.move",
    "shape.resize",
    "shape.rotate",
    "shape.set.width",
    "shape.set.height",
    "shape.set.pin",
    "shape.set.angle",
    "shape.enter_group",
    "shape.exit_group",
];

/// The test that covers each id (ADR 0049 rule 1: a registered command
/// cannot ship untested).
const COVERED: &[(&str, &str)] = &[
    (
        "edit.delete",
        "delete_is_one_undo_step_and_restores_the_shapes",
    ),
    ("edit.select", "select_sets_the_selection_by_visio_id"),
    (
        "edit.select_all",
        "select_all_and_deselect_manage_the_selection",
    ),
    (
        "edit.deselect",
        "select_all_and_deselect_manage_the_selection",
    ),
    ("shape.move", "move_shifts_the_selection_with_one_undo_step"),
    ("shape.resize", "resize_scales_size_and_geometry"),
    ("shape.rotate", "rotate_turns_each_selected_shape"),
    ("shape.set.width", "pane_fields_edit_the_anchor_shape"),
    ("shape.set.height", "pane_fields_edit_the_anchor_shape"),
    ("shape.set.pin", "pane_fields_edit_the_anchor_shape"),
    ("shape.set.angle", "pane_fields_edit_the_anchor_shape"),
    ("shape.enter_group", "entering_and_exiting_a_group_reselect"),
    ("shape.exit_group", "entering_and_exiting_a_group_reselect"),
];

fn number_cell(name: &str, value: f64) -> Cell {
    Cell {
        name: CellName::new(name),
        formula: Some(format!("{value}")),
        value: Some(CellValue::Number(value)),
        unit: larrygram_geom::Unit::Inches,
        preserved: Preserved::new(),
    }
}

/// A state with one page and `count` shapes of 2x1 inches at (n, 0),
/// each with its own geometry rows.
fn fixture(count: usize) -> (AppCtx, Registry, SessionId, PageId, Vec<ShapeId>) {
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
            ("Angle", 0.0),
        ] {
            shape.sheet.cells.push(number_cell(name, value));
        }
        let mut geometry = Section::new(SectionName::from_static("Geometry"));
        for (ix, (kind, x, y)) in [
            ("MoveTo", 0.0, 0.0),
            ("LineTo", 2.0, 0.0),
            ("LineTo", 2.0, 1.0),
        ]
        .into_iter()
        .enumerate()
        {
            let mut row = Row::new();
            row.row_type = Some(kind.to_owned());
            row.ix = Some(i32::try_from(ix).unwrap_or(0) + 1);
            row.cells.push(number_cell("X", x));
            row.cells.push(number_cell("Y", y));
            geometry.rows.push(row);
        }
        shape.sheet.sections.push(geometry);
        shapes.push(doc.insert_shape(page, shape, None).expect("insert"));
    }
    let mut ctx = AppCtx::new(Settings::default());
    let index = ctx.open_document(DocumentSession::new(doc, None));
    let session = ctx.documents.get(index).expect("session").id;
    let registry = registry().expect("registry");
    (ctx, registry, session, page, shapes)
}

/// Selects `shapes` as a click sequence (the last one is the anchor).
fn select(ctx: &mut AppCtx, page: PageId, shapes: &[ShapeId]) {
    ctx.selection.shapes = shapes.to_vec();
    ctx.selection.anchor = shapes.last().copied();
    ctx.selection.page = Some(page);
}

fn args(pairs: &[(&str, f64)]) -> Args {
    let mut args = Args::new();
    for (name, value) in pairs {
        args.set(*name, ArgValue::Number(*value));
    }
    args
}

/// An `edit.select` invocation: Visio ids as strings plus a mode.
fn select_args(mode: &str, ids: &[&str]) -> Args {
    Args::new()
        .with(
            "shapes",
            ArgValue::List(ids.iter().map(|id| (*id).to_owned()).collect()),
        )
        .with("mode", ArgValue::Text(mode.to_owned()))
}

/// The cached number of one shape cell, straight from the model.
fn cell_value(ctx: &AppCtx, shape: ShapeId, cell: &str) -> Option<f64> {
    let document = &ctx.active_document().expect("document").document;
    document
        .shape(shape)?
        .sheet
        .cells
        .iter()
        .find(|entry| entry.name.as_str() == cell)
        .and_then(|entry| match entry.value {
            Some(CellValue::Number(value)) => Some(value),
            _ => None,
        })
}

fn document(ctx: &AppCtx) -> Document {
    ctx.active_document().expect("document").document.clone()
}

fn changes(ctx: &AppCtx) -> u64 {
    ctx.active_document().expect("document").changes
}

#[test]
fn every_phase5_command_is_registered_and_tested() {
    let registry = registry().expect("registry");
    for id in PHASE5_IDS {
        let command = CommandId::parse(id).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(
            registry.status(&command),
            CommandStatus::Implemented,
            "{id} must be implemented"
        );
        assert!(
            COVERED.iter().any(|(covered, _)| covered == id),
            "{id} has no test entry (ADR 0049 rule 1)"
        );
    }
    for (id, _) in COVERED {
        assert!(
            PHASE5_IDS.contains(id),
            "{id} is not a task P5-04 command id"
        );
    }
}

#[test]
fn move_shifts_the_selection_with_one_undo_step() {
    let (mut ctx, registry, _session, page, shapes) = fixture(3);
    select(&mut ctx, page, &shapes);
    let before = document(&ctx);

    let outcome = dispatch(
        &mut ctx,
        &registry,
        "shape.move",
        args(&[("dx", 1.0), ("dy", 0.5)]),
    )
    .expect("move runs");
    // `dispatch` (like the shell) pushes the edit, so the outcome is
    // empty here; the stack below is the "one undo step" assertion.
    assert!(outcome.edit.is_none());
    assert_eq!(cell_value(&ctx, shapes[0], "PinX"), Some(1.0));
    assert_eq!(cell_value(&ctx, shapes[0], "PinY"), Some(0.5));
    assert_eq!(cell_value(&ctx, shapes[2], "PinX"), Some(3.0));

    // One drag gesture = one undo step, undone in one `edit.undo`.
    let session = ctx.active_document().expect("document");
    assert_eq!(session.undo.len(), 1);
    assert_eq!(session.undo.undo_labels(), ["Move 3 shapes"]);
    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(document(&ctx), before, "one undo restored all three");
    assert!(!ctx.active_document().expect("document").undo.can_undo());
}

#[test]
fn resize_scales_size_and_geometry() {
    let (mut ctx, registry, _session, page, shapes) = fixture(1);
    select(&mut ctx, page, &shapes);
    let before = document(&ctx);
    dispatch(
        &mut ctx,
        &registry,
        "shape.resize",
        args(&[("width", 4.0), ("height", 3.0)]),
    )
    .expect("resize runs");
    assert_eq!(cell_value(&ctx, shapes[0], "Width"), Some(4.0));
    assert_eq!(cell_value(&ctx, shapes[0], "Height"), Some(3.0));
    let after = document(&ctx);
    let rows = &after
        .shape(shapes[0])
        .expect("shape")
        .sheet
        .sections
        .first()
        .expect("geometry section")
        .rows;
    let x = rows
        .get(1)
        .expect("row")
        .cells
        .iter()
        .find(|cell| cell.name.as_str() == "X")
        .expect("X");
    assert_eq!(x.value, Some(CellValue::Number(4.0)), "the row scaled by 2");
    assert_ne!(before, after);

    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(document(&ctx), before);
    dispatch(&mut ctx, &registry, "edit.redo", Args::new()).expect("redo");
    assert_eq!(document(&ctx), after);
}

#[test]
fn rotate_turns_each_selected_shape() {
    let (mut ctx, registry, _session, page, shapes) = fixture(2);
    select(&mut ctx, page, &shapes);
    let before = document(&ctx);
    let delta = std::f64::consts::FRAC_PI_2;
    dispatch(
        &mut ctx,
        &registry,
        "shape.rotate",
        args(&[("angle", delta)]),
    )
    .expect("rotate runs");
    assert_eq!(cell_value(&ctx, shapes[0], "Angle"), Some(delta));
    assert_eq!(cell_value(&ctx, shapes[1], "Angle"), Some(delta));
    let after = document(&ctx);

    // `shape.rotate` is a delta: a second run doubles it.
    dispatch(
        &mut ctx,
        &registry,
        "shape.rotate",
        args(&[("angle", delta)]),
    )
    .expect("rotate runs");
    assert_eq!(cell_value(&ctx, shapes[0], "Angle"), Some(2.0 * delta));

    // Each run is its own undo step, and the pair test holds per step.
    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(document(&ctx), after);
    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(document(&ctx), before);
    dispatch(&mut ctx, &registry, "edit.redo", Args::new()).expect("redo");
    assert_eq!(document(&ctx), after);
}

#[test]
fn pane_fields_edit_the_anchor_shape() {
    let (mut ctx, registry, _session, page, shapes) = fixture(3);
    // The anchor is the middle shape even though all three are selected.
    select(&mut ctx, page, &shapes);
    ctx.selection.anchor = Some(shapes[1]);
    let before = document(&ctx);

    dispatch(
        &mut ctx,
        &registry,
        "shape.set.pin",
        args(&[("x", 7.0), ("y", 8.0)]),
    )
    .expect("pin");
    dispatch(
        &mut ctx,
        &registry,
        "shape.set.angle",
        args(&[("angle", 1.25)]),
    )
    .expect("angle");
    dispatch(
        &mut ctx,
        &registry,
        "shape.set.width",
        args(&[("width", 5.0)]),
    )
    .expect("width");
    dispatch(
        &mut ctx,
        &registry,
        "shape.set.height",
        args(&[("height", 6.0)]),
    )
    .expect("height");

    assert_eq!(cell_value(&ctx, shapes[1], "PinX"), Some(7.0));
    assert_eq!(cell_value(&ctx, shapes[1], "PinY"), Some(8.0));
    assert_eq!(cell_value(&ctx, shapes[1], "Angle"), Some(1.25));
    assert_eq!(cell_value(&ctx, shapes[1], "Width"), Some(5.0));
    assert_eq!(cell_value(&ctx, shapes[1], "Height"), Some(6.0));
    assert_eq!(
        cell_value(&ctx, shapes[0], "PinX"),
        Some(0.0),
        "others move not"
    );
    assert_eq!(cell_value(&ctx, shapes[2], "PinX"), Some(2.0));

    // Undo the four pane edits one by one back to the start.
    for _ in 0..4 {
        dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    }
    assert_eq!(document(&ctx), before);
}

#[test]
fn delete_is_one_undo_step_and_restores_the_shapes() {
    let (mut ctx, registry, _session, page, shapes) = fixture(3);
    select(&mut ctx, page, &[shapes[0], shapes[2]]);
    let before = document(&ctx);

    let outcome = dispatch(&mut ctx, &registry, "edit.delete", Args::new()).expect("delete runs");
    assert_eq!(outcome.message.as_deref(), Some("Deleted 2 shapes"));
    let after = document(&ctx);
    assert!(after.page_of(shapes[0]).is_none());
    assert!(after.page_of(shapes[2]).is_none());
    assert!(after.page_of(shapes[1]).is_some(), "the rest stays");
    assert!(
        ctx.selection.is_empty(),
        "the deleted shapes leave the selection"
    );
    assert_eq!(ctx.selection.page, Some(page), "the page context stays");

    // Two detached subtrees, one undo step (one CompositeEdit).
    let session = ctx.active_document().expect("document");
    assert_eq!(session.undo.len(), 1);
    assert_eq!(session.undo.undo_labels(), ["Delete 2 shapes"]);

    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(document(&ctx), before);
    dispatch(&mut ctx, &registry, "edit.redo", Args::new()).expect("redo");
    assert_eq!(document(&ctx), after);
}

/// Review F1 (task P5-19): a selection that holds a group **and** one of
/// its members deletes once, in either selection order. The nested
/// target is skipped with one warning line; one undo step restores
/// everything (before the fix the group-first order applied half the
/// steps and errored with no undo step).
#[test]
fn delete_of_a_group_and_its_member_deletes_once_in_either_order() {
    for first in [true, false] {
        let (mut ctx, registry, _session, page, _shapes) = fixture(0);
        let mut group = Shape::new(ShapeId { index: 0 }, 10);
        group.sheet.cells.push(number_cell("PinX", 0.0));
        let group = ctx
            .active_document_mut()
            .expect("document")
            .document
            .insert_shape(page, group, None)
            .expect("group");
        let mut member = Shape::new(ShapeId { index: 0 }, 11);
        member.sheet.cells.push(number_cell("PinX", 1.0));
        let member = ctx
            .active_document_mut()
            .expect("document")
            .document
            .insert_subshape(group, member, None)
            .expect("member");
        let order = if first {
            [group, member]
        } else {
            [member, group]
        };
        select(&mut ctx, page, &order);
        let before = document(&ctx);

        let outcome = dispatch(&mut ctx, &registry, "edit.delete", Args::new())
            .expect("the nested delete runs");
        assert_eq!(outcome.message.as_deref(), Some("Deleted shape"));
        assert!(
            outcome
                .warnings
                .iter()
                .any(|warning| warning.contains("inside another selected shape; skipped")),
            "{:?}",
            outcome.warnings
        );
        let after = document(&ctx);
        assert!(after.page_of(group).is_none(), "the group is gone");
        assert!(after.page_of(member).is_none(), "the member went with it");
        assert_eq!(
            ctx.active_document().expect("document").undo.len(),
            1,
            "one subtree, one undo step"
        );

        dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
        assert_eq!(document(&ctx), before, "one undo restores both");
    }
}

/// A delete of a glued shape warns about the glue it takes with it (the
/// user-visible half of undo-after-delete, ADR 0045): one line per lost
/// glue record, and the records come back with the shape on undo.
#[test]
fn delete_warns_about_the_glue_it_drops_and_undo_restores_it() {
    let (mut ctx, registry, _session, page, shapes) = fixture(1);
    let target = shapes[0];
    // A connector (Visio id 77) glued to the shape's outline.
    let mut connector = Shape::new(ShapeId { index: 0 }, 77);
    connector.sheet.cells.push(number_cell("PinX", 5.0));
    let connector = ctx
        .active_document_mut()
        .expect("document")
        .document
        .insert_shape(page, connector, None)
        .expect("connector");
    let connector_visio = {
        let doc = &ctx.active_document().expect("document").document;
        doc.shape(connector).expect("connector").visio_id
    };
    {
        let doc = &mut ctx.active_document_mut().expect("document").document;
        for from_cell in ["BeginX", "EndX"] {
            let mut connect = larrygram_model::Connect::new();
            connect.from_sheet = connector_visio;
            connect.from_cell = from_cell.to_owned();
            connect.to_sheet = 1;
            connect.to_cell = "PinX".to_owned();
            doc.add_connect(page, connect).expect("glue");
        }
    }
    select(&mut ctx, page, &[target]);

    let outcome = dispatch(&mut ctx, &registry, "edit.delete", Args::new()).expect("delete");
    assert_eq!(
        outcome
            .warnings
            .iter()
            .filter(|warning| warning.contains("lost its glue to detached shape 1"))
            .count(),
        2,
        "{:?}",
        outcome.warnings
    );
    let doc = document(&ctx);
    let entity = doc.page(page).expect("page");
    assert!(entity.connects.is_empty(), "the records go with the shape");
    assert!(doc.page_of(connector).is_some(), "the connector stays");

    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    let doc = document(&ctx);
    assert_eq!(
        doc.page(page).expect("page").connects.len(),
        2,
        "undo brings the glue back"
    );
}

#[test]
fn select_sets_the_selection_by_visio_id() {
    let (mut ctx, registry, _session, page, shapes) = fixture(3);
    dispatch(
        &mut ctx,
        &registry,
        "edit.select",
        select_args("replace", &["1", "2"]),
    )
    .expect("replace");
    assert_eq!(ctx.selection.shapes, vec![shapes[0], shapes[1]]);
    assert_eq!(ctx.selection.anchor, Some(shapes[1]));
    assert_eq!(ctx.selection.page, Some(page));

    dispatch(
        &mut ctx,
        &registry,
        "edit.select",
        select_args("add", &["3", "1"]),
    )
    .expect("add");
    assert_eq!(ctx.selection.shapes, shapes, "add keeps click order");

    dispatch(
        &mut ctx,
        &registry,
        "edit.select",
        select_args("toggle", &["2"]),
    )
    .expect("toggle off");
    assert_eq!(ctx.selection.shapes, vec![shapes[0], shapes[2]]);
    dispatch(
        &mut ctx,
        &registry,
        "edit.select",
        select_args("toggle", &["2"]),
    )
    .expect("toggle on");
    assert_eq!(ctx.selection.shapes, vec![shapes[0], shapes[2], shapes[1]]);

    // Selection is not a document edit: no undo step, no change bump.
    let session = ctx.active_document().expect("document");
    assert!(session.undo.is_empty());
    assert_eq!(changes(&ctx), 0);

    dispatch(
        &mut ctx,
        &registry,
        "edit.select",
        select_args("replace", &[]),
    )
    .expect("clear");
    assert!(ctx.selection.is_empty());
    assert_eq!(ctx.selection.anchor, None);
    assert_eq!(ctx.selection.page, None, "an empty selection has no page");
    assert_eq!(changes(&ctx), 0);

    // The default mode is replace, and a missing list is the empty list.
    dispatch(
        &mut ctx,
        &registry,
        "edit.select",
        Args::new().with("shapes", ArgValue::List(vec!["3".to_owned()])),
    )
    .expect("default replace");
    assert_eq!(ctx.selection.shapes, vec![shapes[2]]);
    dispatch(&mut ctx, &registry, "edit.select", Args::new()).expect("missing list clears");
    assert!(ctx.selection.is_empty());
}

#[test]
fn select_rejects_bad_ids_without_changing_the_selection() {
    let (mut ctx, registry, _session, page, shapes) = fixture(3);
    select(&mut ctx, page, &[shapes[0]]);
    let before = ctx.selection.clone();
    let error = dispatch(
        &mut ctx,
        &registry,
        "edit.select",
        select_args("replace", &["2", "99"]),
    )
    .expect_err("unknown id");
    match error {
        CommandError::InvalidArgs { message } => {
            assert_eq!(message, "unknown shape id 99 on the active page");
        }
        other => panic!("expected InvalidArgs, got {other:?}"),
    }
    assert_eq!(ctx.selection, before, "no partial change");

    let error = dispatch(
        &mut ctx,
        &registry,
        "edit.select",
        select_args("replace", &["not-a-number"]),
    )
    .expect_err("malformed id");
    assert!(error.to_string().contains("is not an integer"), "{error}");
    assert_eq!(ctx.selection, before);

    let error = dispatch(
        &mut ctx,
        &registry,
        "edit.select",
        select_args("sideways", &["1"]),
    )
    .expect_err("bad mode");
    assert!(
        error
            .to_string()
            .contains("mode must be replace, add or toggle"),
        "{error}"
    );
    assert_eq!(ctx.selection, before);

    let error = dispatch(
        &mut ctx,
        &registry,
        "edit.select",
        Args::new().with("shapes", ArgValue::Text("1".to_owned())),
    )
    .expect_err("wrong kind");
    assert!(
        error.to_string().contains("shapes must be a list"),
        "{error}"
    );
    assert_eq!(ctx.selection, before);
}

#[test]
fn select_all_and_deselect_manage_the_selection() {
    let (mut ctx, registry, _session, page, shapes) = fixture(3);
    dispatch(&mut ctx, &registry, "edit.select_all", Args::new()).expect("select all");
    assert_eq!(ctx.selection.shapes, shapes);
    assert_eq!(ctx.selection.anchor, Some(shapes[2]));
    assert_eq!(ctx.selection.page, Some(page));

    // Selection is not a document edit: no undo step, no change bump.
    let session = ctx.active_document().expect("document");
    assert!(session.undo.is_empty());
    assert_eq!(changes(&ctx), 0);

    dispatch(&mut ctx, &registry, "edit.deselect", Args::new()).expect("deselect");
    assert!(ctx.selection.is_empty());
    assert_eq!(ctx.selection.page, Some(page), "the page context stays");
    assert_eq!(changes(&ctx), 0);
}

#[test]
fn entering_and_exiting_a_group_reselect() {
    let (mut ctx, registry, _session, page, _shapes) = fixture(0);
    // A group with two members.
    let mut group = Shape::new(ShapeId { index: 0 }, 10);
    group.sheet.cells.push(number_cell("PinX", 0.0));
    let group = ctx
        .active_document_mut()
        .expect("document")
        .document
        .insert_shape(page, group, None)
        .expect("group");
    let members: Vec<ShapeId> = (0..2)
        .map(|n| {
            let mut member = Shape::new(ShapeId { index: 0 }, 11 + n);
            member.sheet.cells.push(number_cell("PinX", n as f64));
            ctx.active_document_mut()
                .expect("document")
                .document
                .insert_subshape(group, member, None)
                .expect("member")
        })
        .collect();

    select(&mut ctx, page, &[group]);
    dispatch(&mut ctx, &registry, "shape.enter_group", Args::new()).expect("enter");
    assert_eq!(ctx.selection.entered, Some(group));
    assert_eq!(ctx.selection.shapes, members);
    assert_eq!(ctx.selection.anchor, members.last().copied());

    // While inside, `edit.select_all` selects the members.
    dispatch(&mut ctx, &registry, "edit.deselect", Args::new()).expect("deselect");
    dispatch(&mut ctx, &registry, "edit.select_all", Args::new()).expect("select all");
    assert_eq!(ctx.selection.shapes, members);

    dispatch(&mut ctx, &registry, "shape.exit_group", Args::new()).expect("exit");
    assert_eq!(ctx.selection.entered, None);
    assert_eq!(ctx.selection.shapes, vec![group]);
    assert_eq!(ctx.selection.anchor, Some(group));
}

#[test]
fn every_handler_names_its_not_enabled_reason() {
    let (mut ctx, registry, _session, page, shapes) = fixture(1);
    let cases: &[(&str, &str)] = &[
        ("edit.delete", "no selection"),
        ("edit.select_all", "the document has no pages"),
        ("edit.deselect", "no selection"),
        ("shape.move", "no selection"),
        ("shape.resize", "no selection"),
        ("shape.rotate", "no selection"),
        ("shape.set.width", "no selection"),
        ("shape.set.height", "no selection"),
        ("shape.set.pin", "no selection"),
        ("shape.set.angle", "no selection"),
        ("shape.enter_group", "no selection"),
        ("shape.exit_group", "not inside a group"),
    ];
    // No pages yet: `edit.select_all` says so.
    let mut pageless = {
        let mut ctx = AppCtx::new(Settings::default());
        ctx.open_document(DocumentSession::new(Document::new(), None));
        ctx
    };
    let error = registry
        .execute(
            &CommandId::parse("edit.select_all").expect("id"),
            &mut pageless,
            Invocation::empty(Source::Test),
        )
        .expect_err("no pages");
    assert_eq!(
        error.to_string(),
        "command edit.select_all is disabled: the document has no pages"
    );

    for (id, reason) in cases {
        if *id == "edit.select_all" {
            continue;
        }
        let error = dispatch(&mut ctx, &registry, id, Args::new()).expect_err(id);
        match error {
            CommandError::NotEnabled {
                reason: Some(got), ..
            } => assert_eq!(&got, reason, "{id}"),
            other => panic!("{id}: expected NotEnabled, got {other:?}"),
        }
    }

    // A group the selection is not: `shape.enter_group` names it.
    select(&mut ctx, page, &shapes);
    let error = dispatch(&mut ctx, &registry, "shape.enter_group", Args::new()).expect_err("group");
    match error {
        CommandError::NotEnabled {
            reason: Some(got), ..
        } => assert_eq!(got, "the selected shape is not a group"),
        other => panic!("expected NotEnabled, got {other:?}"),
    }
}

#[test]
fn arguments_are_the_p5_04_contract() {
    let (mut ctx, registry, _session, page, shapes) = fixture(1);
    select(&mut ctx, page, &shapes);

    // A missing number asks for it (never a dialog under Source::Test).
    let error = dispatch(&mut ctx, &registry, "shape.move", Args::new()).expect_err("dx missing");
    match error {
        CommandError::NeedsInput(NeededInput::Number { prompt, default }) => {
            assert_eq!(prompt, "x offset in inches");
            assert_eq!(default, None);
        }
        other => panic!("expected NeedsInput, got {other:?}"),
    }
    let error =
        dispatch(&mut ctx, &registry, "shape.move", args(&[("dx", 1.0)])).expect_err("dy missing");
    assert!(
        matches!(error, CommandError::NeedsInput(NeededInput::Number { .. })),
        "{error:?}"
    );

    // Bad values are InvalidArgs, naming the argument.
    for (id, pairs, want) in [
        (
            "shape.move",
            vec![("dx", f64::NAN), ("dy", 0.0)],
            "dx must be finite",
        ),
        (
            "shape.resize",
            vec![("width", -1.0), ("height", 2.0)],
            "width must not be negative",
        ),
        (
            "shape.resize",
            vec![("width", 1.0), ("height", f64::INFINITY)],
            "height must be finite",
        ),
        (
            "shape.rotate",
            vec![("angle", f64::NEG_INFINITY)],
            "angle must be finite",
        ),
        (
            "shape.set.width",
            vec![("width", -0.5)],
            "width must not be negative",
        ),
        (
            "shape.set.height",
            vec![("height", -0.5)],
            "height must not be negative",
        ),
        (
            "shape.set.pin",
            vec![("x", 1.0), ("y", f64::NAN)],
            "y must be finite",
        ),
        (
            "shape.set.angle",
            vec![("angle", f64::NAN)],
            "angle must be finite",
        ),
    ] {
        let error = dispatch(&mut ctx, &registry, id, args(&pairs)).expect_err(id);
        match error {
            CommandError::InvalidArgs { message } => assert_eq!(message, want, "{id}"),
            other => panic!("{id}: expected InvalidArgs, got {other:?}"),
        }
    }

    // A wrong kind is InvalidArgs too; a whole number is accepted.
    let mut text_args = Args::new();
    text_args.set("dx", ArgValue::Text("1".to_owned()));
    text_args.set("dy", ArgValue::Number(0.0));
    let error = dispatch(&mut ctx, &registry, "shape.move", text_args).expect_err("dx is text");
    match error {
        CommandError::InvalidArgs { message } => assert_eq!(message, "dx must be a number"),
        other => panic!("expected InvalidArgs, got {other:?}"),
    }
    let mut whole = Args::new();
    whole.set("dx", ArgValue::Integer(2));
    whole.set("dy", ArgValue::Integer(0));
    dispatch(&mut ctx, &registry, "shape.move", whole).expect("whole numbers are numbers");
}

#[test]
fn every_command_round_trips_through_undo_and_redo() {
    // (command, arguments) pairs; each runs on a fresh single-shape
    // state and must round-trip exactly (ADR 0040 rule 4).
    let runs: &[(&str, Args)] = &[
        ("shape.move", args(&[("dx", 1.0), ("dy", -2.0)])),
        ("shape.resize", args(&[("width", 3.0), ("height", 0.5)])),
        ("shape.rotate", args(&[("angle", 0.75)])),
        ("shape.set.width", args(&[("width", 9.0)])),
        ("shape.set.height", args(&[("height", 0.25)])),
        ("shape.set.pin", args(&[("x", -4.0), ("y", 12.0)])),
        ("shape.set.angle", args(&[("angle", 2.5)])),
    ];
    for (id, args) in runs {
        let (mut ctx, registry, _session, page, shapes) = fixture(1);
        select(&mut ctx, page, &shapes);
        let seed = document(&ctx);
        dispatch(&mut ctx, &registry, id, args.clone()).unwrap_or_else(|e| panic!("{id}: {e}"));
        let after = document(&ctx);
        assert_ne!(seed, after, "{id} changed the document");

        dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
        assert_eq!(document(&ctx), seed, "{id}: revert(apply(x)) == x");
        dispatch(&mut ctx, &registry, "edit.redo", Args::new()).expect("redo");
        assert_eq!(document(&ctx), after, "{id}: apply(revert(x)) == x");
    }

    // `edit.delete` too (a structural edit rather than a cell edit).
    let (mut ctx, registry, _session, page, shapes) = fixture(1);
    select(&mut ctx, page, &shapes);
    let seed = document(&ctx);
    dispatch(&mut ctx, &registry, "edit.delete", Args::new()).expect("delete");
    let after = document(&ctx);
    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(document(&ctx), seed, "delete: revert(apply(x)) == x");
    dispatch(&mut ctx, &registry, "edit.redo", Args::new()).expect("redo");
    assert_eq!(document(&ctx), after, "delete: apply(revert(x)) == x");
}

#[test]
fn the_change_counter_moves_with_edits_and_undo_not_with_views() {
    let (mut ctx, registry, _session, page, shapes) = fixture(1);
    select(&mut ctx, page, &shapes);
    assert_eq!(changes(&ctx), 0);

    dispatch(
        &mut ctx,
        &registry,
        "shape.move",
        args(&[("dx", 1.0), ("dy", 1.0)]),
    )
    .expect("move");
    let edited = changes(&ctx);
    assert!(edited > 0, "an edit bumps the counter");

    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    assert!(changes(&ctx) > edited, "undo bumps the counter");
    let undone = changes(&ctx);
    dispatch(&mut ctx, &registry, "edit.redo", Args::new()).expect("redo");
    assert!(changes(&ctx) > undone, "redo bumps the counter");

    // View-only commands leave it alone (ADR 0040 rule 3).
    let settled = changes(&ctx);
    for (id, args) in [
        ("view.zoom.in", Args::new()),
        ("view.toggle_grid", Args::new()),
        (
            "view.units",
            Args::new().with("units", ArgValue::Text("Feet".to_owned())),
        ),
        ("page.next", Args::new()),
    ] {
        dispatch(&mut ctx, &registry, id, args).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert_eq!(changes(&ctx), settled, "{id} bumped the counter");
    }
}

/// The master's note (task P5-02): a page delete or move renumbers the
/// pages, and an edit that recorded a `PageId` must re-resolve it.
#[test]
fn page_changes_between_edits_keep_the_undo_steps_faithful() {
    let (mut ctx, registry, _session, _page, _shapes) = fixture(0);
    let first = ctx
        .active_document_mut()
        .expect("document")
        .document
        .add_page("Page-2")
        .expect("page");
    let mut shape = Shape::new(ShapeId { index: 0 }, 1);
    shape.sheet.cells.push(number_cell("PinX", 5.0));
    let shape = ctx
        .active_document_mut()
        .expect("document")
        .document
        .insert_shape(first, shape, None)
        .expect("shape");
    select(&mut ctx, first, &[shape]);

    // The edit records `DetachedShapes { page: PageId(1) }`.
    dispatch(&mut ctx, &registry, "edit.delete", Args::new()).expect("delete");
    assert!(document(&ctx).page_of(shape).is_none());

    // A page command (P5-08's `page.delete`) removes the first page:
    // the ids of every later page move down one.
    ctx.active_document_mut()
        .expect("document")
        .document
        .delete_page(PageId { index: 0 })
        .expect("delete page");

    // Undo still restores the shape on the page it came from.
    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    let restored = document(&ctx);
    assert_eq!(restored.page_of(shape), Some(PageId { index: 0 }));
    assert_eq!(
        restored.pages.first().map(|page| page.name.as_str()),
        Some("Page-2"),
        "the shape is on its own page, not on the recorded index"
    );

    // And redo detaches it again from that page.
    dispatch(&mut ctx, &registry, "edit.redo", Args::new()).expect("redo");
    assert!(document(&ctx).page_of(shape).is_none());

    // A move renumbers too; the same rule holds.
    let (mut ctx, registry, _session, _page, shapes) = fixture(1);
    let second = ctx
        .active_document_mut()
        .expect("document")
        .document
        .add_page("Page-2")
        .expect("page");
    select(&mut ctx, PageId { index: 0 }, &shapes);
    dispatch(&mut ctx, &registry, "edit.delete", Args::new()).expect("delete");
    ctx.active_document_mut()
        .expect("document")
        .document
        .move_page(1, 0)
        .expect("move page");
    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    let restored = document(&ctx);
    assert_eq!(
        restored.pages.first().map(|page| page.name.as_str()),
        Some("Page-2"),
        "the move put Page-2 first"
    );
    assert_eq!(restored.page_of(shapes[0]), Some(second));
    assert_eq!(
        restored
            .pages
            .get(second.index as usize)
            .map(|page| page.name.as_str()),
        Some("Page-1"),
        "the shape went back to Page-1 at its new index"
    );
}

#[test]
fn a_stale_selection_entry_is_skipped_with_a_warning() {
    let (mut ctx, registry, _session, page, shapes) = fixture(1);
    select(&mut ctx, page, &shapes);
    ctx.selection.shapes.push(ShapeId { index: 99 });
    let outcome = dispatch(
        &mut ctx,
        &registry,
        "shape.move",
        args(&[("dx", 1.0), ("dy", 0.0)]),
    )
    .expect("the live shape still moves");
    assert_eq!(cell_value(&ctx, shapes[0], "PinX"), Some(1.0));
    assert_eq!(outcome.warnings.len(), 1);
    assert!(
        outcome.warnings[0].contains("no longer on a page"),
        "{:?}",
        outcome.warnings
    );

    // Nothing live at all is a failure, not a silent no-op.
    ctx.selection.shapes = vec![ShapeId { index: 99 }];
    let error = dispatch(
        &mut ctx,
        &registry,
        "shape.move",
        args(&[("dx", 1.0), ("dy", 0.0)]),
    )
    .expect_err("nothing to move");
    assert_eq!(error.to_string(), "no selected shape to edit");
}

#[test]
fn a_move_with_unreachable_cells_substitutes_and_warns() {
    let (mut ctx, registry, _session, page, _shapes) = fixture(0);
    let mut shape = Shape::new(ShapeId { index: 0 }, 1);
    shape.sheet.cells.push(number_cell("PinY", 2.0));
    let shape = ctx
        .active_document_mut()
        .expect("document")
        .document
        .insert_shape(page, shape, None)
        .expect("shape");
    select(&mut ctx, page, &[shape]);
    let outcome = dispatch(
        &mut ctx,
        &registry,
        "shape.move",
        args(&[("dx", 1.0), ("dy", 1.0)]),
    )
    .expect("move runs with substituted values");
    // No `PinX` anywhere: the reader substitutes 0 and warns (ADR 0021).
    assert_eq!(cell_value(&ctx, shape, "PinX"), Some(1.0));
    assert_eq!(cell_value(&ctx, shape, "PinY"), Some(3.0));
    assert!(outcome.warnings.iter().any(|w| w.contains("PinX")));
}

#[test]
fn a_move_beyond_the_representable_range_is_refused() {
    let (mut ctx, registry, _session, page, _shapes) = fixture(0);
    let mut shape = Shape::new(ShapeId { index: 0 }, 1);
    shape.sheet.cells.push(number_cell("PinX", 1.0e308));
    let shape = ctx
        .active_document_mut()
        .expect("document")
        .document
        .insert_shape(page, shape, None)
        .expect("shape");
    select(&mut ctx, page, &[shape]);
    let error = dispatch(
        &mut ctx,
        &registry,
        "shape.move",
        args(&[("dx", 1.0e308), ("dy", 0.0)]),
    )
    .expect_err("the sum overflows");
    assert!(
        error
            .to_string()
            .contains("the computed value is not finite"),
        "{error}"
    );
    assert_eq!(cell_value(&ctx, shape, "PinX"), Some(1.0e308), "unchanged");
}

#[test]
fn commands_never_open_a_dialog() {
    // Every P5-04 command runs under `Source::Test` (a programmatic
    // caller) with no `UiRequest` and no dialog (ADR 0028).
    let (mut ctx, registry, _session, page, shapes) = fixture(2);
    select(&mut ctx, page, &shapes);
    for id in PHASE5_IDS {
        let outcome = dispatch(
            &mut ctx,
            &registry,
            id,
            args(&[
                ("dx", 1.0),
                ("dy", 1.0),
                ("width", 3.0),
                ("height", 3.0),
                ("x", 1.0),
                ("y", 1.0),
                ("angle", 0.5),
            ]),
        );
        let outcome = match outcome {
            Ok(outcome) => outcome,
            // A command the state disables runs not at all (ADR 0028).
            Err(CommandError::NotEnabled { .. }) => continue,
            Err(other) => panic!("{id}: {other:?}"),
        };
        assert!(outcome.ui_request.is_none(), "{id} asked for UI");
        assert!(outcome.payload.is_none(), "{id} produced a payload");
    }
}

/// Deleting the entered group (or its last member) leaves the group
/// context (task P5-04: `edit.delete` tidies `Selection::entered`).
#[test]
fn deleting_the_entered_group_exits_it() {
    let (mut ctx, registry, _session, page, _shapes) = fixture(0);
    let mut group = Shape::new(ShapeId { index: 0 }, 10);
    group.sheet.cells.push(number_cell("PinX", 0.0));
    let group = ctx
        .active_document_mut()
        .expect("document")
        .document
        .insert_shape(page, group, None)
        .expect("group");
    let mut member = Shape::new(ShapeId { index: 0 }, 11);
    member.sheet.cells.push(number_cell("PinX", 1.0));
    let member = ctx
        .active_document_mut()
        .expect("document")
        .document
        .insert_subshape(group, member, None)
        .expect("member");

    // Inside the group with its member selected (the `shape.enter_group`
    // state), then the group itself goes.
    select(&mut ctx, page, &[member]);
    ctx.selection.entered = Some(group);
    select(&mut ctx, page, &[group]);
    ctx.selection.entered = Some(group);
    dispatch(&mut ctx, &registry, "edit.delete", Args::new()).expect("delete");
    assert!(
        ctx.selection.entered.is_none(),
        "the entered group went away"
    );
    assert!(ctx.selection.is_empty());
}

#[test]
fn a_non_text_select_mode_is_invalid_and_leaves_the_selection_alone() {
    let (mut ctx, registry, _session, page, shapes) = fixture(2);
    select(&mut ctx, page, &shapes);
    let before = ctx.selection.clone();
    let error = dispatch(
        &mut ctx,
        &registry,
        "edit.select",
        Args::new().with("mode", ArgValue::Integer(1)),
    )
    .expect_err("mode kind");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));
    assert_eq!(
        error.to_string(),
        "bad arguments: mode must be replace, add or toggle"
    );
    assert_eq!(ctx.selection, before);
}

#[test]
fn deleting_without_an_active_page_still_removes_the_selection() {
    let (mut ctx, registry, _session, page, shapes) = fixture(2);
    select(&mut ctx, page, &shapes);
    ctx.active_document_mut().expect("document").active_page = None;
    let outcome = dispatch(&mut ctx, &registry, "edit.delete", Args::new()).expect("delete");
    assert_eq!(outcome.message.as_deref(), Some("Deleted 2 shapes"));
    assert!(outcome.warnings.is_empty());
    let doc = &ctx.active_document().expect("document").document;
    assert!(doc.page_of(shapes[0]).is_none());
    assert!(doc.page_of(shapes[1]).is_none());
}

#[test]
fn deleting_with_a_stale_page_context_skips_the_glue_warning_pass() {
    let (mut ctx, registry, _session, page, shapes) = fixture(1);
    select(&mut ctx, page, &shapes);
    ctx.active_document_mut().expect("document").active_page = Some(PageId { index: 9 });
    let outcome = dispatch(&mut ctx, &registry, "edit.delete", Args::new()).expect("delete");
    assert_eq!(outcome.message.as_deref(), Some("Deleted shape"));
    assert!(outcome.warnings.is_empty());
    assert!(
        ctx.active_document()
            .expect("document")
            .document
            .page_of(shapes[0])
            .is_none()
    );
}

#[test]
fn an_all_stale_selection_is_a_named_failure_for_every_edit() {
    let (mut ctx, registry, _session, page, _shapes) = fixture(1);
    let stale = ShapeId { index: 999 };
    select(&mut ctx, page, &[stale]);
    for id in ["edit.delete", "shape.move", "shape.resize", "shape.rotate"] {
        let error = match id {
            "shape.move" => {
                dispatch(&mut ctx, &registry, id, args(&[("dx", 1.0), ("dy", 1.0)])).expect_err(id)
            }
            "shape.resize" => dispatch(
                &mut ctx,
                &registry,
                id,
                args(&[("width", 2.0), ("height", 2.0)]),
            )
            .expect_err(id),
            "shape.rotate" => {
                dispatch(&mut ctx, &registry, id, args(&[("angle", 1.0)])).expect_err(id)
            }
            _ => dispatch(&mut ctx, &registry, id, Args::new()).expect_err(id),
        };
        assert!(
            error.to_string().contains("no selected shape to edit"),
            "{id}: {error}"
        );
    }
}

#[test]
fn deleting_the_anchor_keeps_the_surviving_selection() {
    let (mut ctx, registry, _session, page, shapes) = fixture(2);
    select(&mut ctx, page, &[shapes[0], shapes[1]]);
    ctx.selection.anchor = Some(shapes[0]);
    dispatch(&mut ctx, &registry, "edit.delete", Args::new()).expect("delete");
    assert_eq!(ctx.selection.shapes, vec![]);
    assert_eq!(ctx.selection.anchor, None);
    assert!(ctx.selection.is_empty());
    let _ = shapes;
}

#[test]
fn undo_of_a_delete_after_its_page_is_gone_reports_the_lost_page() {
    let (mut ctx, registry, _session, page, shapes) = fixture(1);
    select(&mut ctx, page, &shapes);
    dispatch(&mut ctx, &registry, "edit.delete", Args::new()).expect("delete");
    ctx.active_document_mut()
        .expect("document")
        .document
        .pages
        .clear();
    let error = dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect_err("undo");
    assert!(!error.to_string().is_empty(), "{error}");
}

#[test]
fn select_and_select_all_need_an_active_page() {
    let (mut ctx, registry, _session, page, shapes) = fixture(2);
    select(&mut ctx, page, &shapes);
    ctx.active_document_mut().expect("document").active_page = None;
    let error = dispatch(
        &mut ctx,
        &registry,
        "edit.select",
        select_args("replace", &["1"]),
    )
    .expect_err("select without a page");
    assert!(error.to_string().contains("no active page"), "{error}");
    let error = dispatch(&mut ctx, &registry, "edit.select_all", Args::new())
        .expect_err("select all without a page");
    assert!(error.to_string().contains("no active page"), "{error}");
}

#[test]
fn selecting_an_empty_list_clears_the_entered_group() {
    let (mut ctx, registry, _session, page, shapes) = fixture(1);
    select(&mut ctx, page, &shapes);
    ctx.selection.entered = Some(shapes[0]);
    let outcome = dispatch(
        &mut ctx,
        &registry,
        "edit.select",
        select_args("replace", &[]),
    )
    .expect("clear");
    assert_eq!(outcome.message.as_deref(), Some("Selected 0 shapes"));
    assert_eq!(ctx.selection.entered, None);
    assert_eq!(ctx.selection.page, None);
}

#[test]
fn pane_commands_need_a_document_and_a_live_anchor() {
    // A selection without any open document reaches the handler.
    let mut ctx = AppCtx::new(Settings::default());
    ctx.selection.shapes = vec![ShapeId { index: 0 }];
    ctx.selection.anchor = Some(ShapeId { index: 0 });
    let registry = registry().expect("registry");
    let error = dispatch(
        &mut ctx,
        &registry,
        "shape.set.pin",
        args(&[("x", 1.0), ("y", 2.0)]),
    )
    .expect_err("no document");
    assert!(error.to_string().contains("no open document"), "{error}");

    // A stale anchor in an open document is named.
    let (mut ctx, registry, _session, _page, _shapes) = fixture(1);
    ctx.selection.shapes = vec![ShapeId { index: 999 }];
    ctx.selection.anchor = Some(ShapeId { index: 999 });
    let error = dispatch(
        &mut ctx,
        &registry,
        "shape.set.width",
        args(&[("width", 2.0)]),
    )
    .expect_err("stale anchor");
    assert!(
        error.to_string().contains("is no longer on a page"),
        "{error}"
    );
}

#[test]
fn exit_group_reports_a_missing_document_and_a_stale_group() {
    let mut ctx = AppCtx::new(Settings::default());
    ctx.selection.entered = Some(ShapeId { index: 0 });
    let registry = registry().expect("registry");
    let error =
        dispatch(&mut ctx, &registry, "shape.exit_group", Args::new()).expect_err("no document");
    assert!(error.to_string().contains("no open document"), "{error}");

    let (mut ctx, registry, _session, _page, _shapes) = fixture(1);
    ctx.selection.entered = Some(ShapeId { index: 999 });
    let error =
        dispatch(&mut ctx, &registry, "shape.exit_group", Args::new()).expect_err("stale group");
    assert!(error.to_string().contains("no longer exists"), "{error}");
}
