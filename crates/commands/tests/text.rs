//! Headless text command tests (task P5-07, ADR 0044).
//!
//! Every text command runs through `Registry::execute` with
//! `Source::Test`: `shape.set.text` writes one run and is one undo step
//! whose revert restores the exact prior `TextContent`; the editor
//! commands validate their target and need no GUI. A stale or missing
//! shape is an error, never a panic.

// The never-crash lints apply to non-test code only (clippy.toml).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod support;

use support::dispatch;

use ferrogram_commands::app::{AppCtx, DocumentSession, Settings};
use ferrogram_commands::builtins::registry;
use ferrogram_commands::{
    ArgValue, Args, CommandError, CommandId, NeededInput, Registry, SessionId,
};
use ferrogram_model::{
    Document, PageId, Row, Section, SectionName, Shape, ShapeId, TextContent, TextPart,
};

/// A page and one shape, optionally with text and the three run sections.
fn fixture(old: Option<TextContent>, rows: bool) -> (AppCtx, Registry, SessionId, PageId, ShapeId) {
    let mut doc = Document::new();
    let page = doc.add_page("Page-1").expect("page");
    let mut shape = Shape::new(ShapeId { index: 0 }, 1);
    if rows {
        for name in ["Character", "Paragraph", "Tabs"] {
            let mut section = Section::new(SectionName::new(name.to_owned()));
            section.rows.push(Row::new());
            shape.sheet.sections.push(section);
        }
    }
    let shape = doc.insert_shape(page, shape, None).expect("shape");
    if let Some(text) = old.clone() {
        doc.set_shape_text(shape, text).expect("text");
    }
    let mut ctx = AppCtx::new(Settings::default());
    let index = ctx.open_document(DocumentSession::new(doc, None));
    let session = ctx.documents.get(index).expect("session").id;
    ctx.selection.shapes = vec![shape];
    ctx.selection.anchor = Some(shape);
    ctx.selection.page = Some(page);
    (ctx, registry().expect("registry"), session, page, shape)
}

/// Dispatches one command and pushes its edit like the shell does.
fn document(ctx: &AppCtx) -> Document {
    ctx.active_document().expect("session").document.clone()
}

fn text_args(text: &str) -> Args {
    Args::new().with("text", ArgValue::Text(text.to_owned()))
}

fn shape_text(ctx: &AppCtx, shape: ShapeId) -> Option<TextContent> {
    ctx.active_document()
        .and_then(|session| session.document.shape(shape))
        .and_then(|shape| shape.text.clone())
}

#[test]
fn set_text_writes_one_run_and_undo_restores_the_exact_old_text() {
    let old = TextContent {
        parts: vec![
            TextPart::CharRun { ix: 2 },
            TextPart::Text("old".to_owned()),
            TextPart::ParaRun { ix: 4 },
            TextPart::Text(" text".to_owned()),
        ],
    };
    let (mut ctx, registry, _session, _page, shape) = fixture(Some(old.clone()), true);
    let before = document(&ctx);
    dispatch(
        &mut ctx,
        &registry,
        "shape.set.text",
        text_args("new\nline"),
    )
    .expect("set");
    let after = document(&ctx);
    assert_eq!(
        shape_text(&ctx, shape),
        Some(TextContent {
            parts: vec![
                TextPart::CharRun { ix: 0 },
                TextPart::ParaRun { ix: 0 },
                TextPart::TabsRun { ix: 0 },
                TextPart::Text("new\nline".to_owned()),
            ],
        })
    );
    {
        let session = ctx.active_document().expect("session");
        assert_eq!(session.undo.undo_labels(), ["Write shape text"]);
        assert_eq!(session.changes, 1, "one edit bumps the change counter");
    }

    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(document(&ctx), before, "undo is byte-for-byte");
    assert_eq!(shape_text(&ctx, shape), Some(old));
    dispatch(&mut ctx, &registry, "edit.redo", Args::new()).expect("redo");
    assert_eq!(document(&ctx), after);
}

#[test]
fn a_shape_without_text_undoes_back_to_no_text() {
    let (mut ctx, registry, _session, _page, shape) = fixture(None, false);
    let before = document(&ctx);
    assert_eq!(shape_text(&ctx, shape), None);
    dispatch(&mut ctx, &registry, "shape.set.text", text_args("hello")).expect("set");
    assert_eq!(
        shape_text(&ctx, shape),
        Some(TextContent {
            parts: vec![TextPart::Text("hello".to_owned())],
        }),
        "no run sections means no markers"
    );
    dispatch(&mut ctx, &registry, "edit.undo", Args::new()).expect("undo");
    assert_eq!(document(&ctx), before);
    assert_eq!(shape_text(&ctx, shape), None);
}

#[test]
fn a_missing_or_mistyped_text_argument_is_rejected() {
    let (mut ctx, registry, _session, _page, _shape) = fixture(None, false);
    let error = dispatch(&mut ctx, &registry, "shape.set.text", Args::new()).expect_err("missing");
    match error {
        CommandError::NeedsInput(NeededInput::Text { prompt, default }) => {
            assert_eq!(prompt, "shape text");
            assert_eq!(default, None);
        }
        other => panic!("expected NeedsInput(Text), got {other}"),
    }
    let error = dispatch(
        &mut ctx,
        &registry,
        "shape.set.text",
        Args::new().with("text", ArgValue::Number(3.0)),
    )
    .expect_err("wrong type");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));
    let error = dispatch(
        &mut ctx,
        &registry,
        "text.commit",
        Args::new().with("text", ArgValue::Flag(true)),
    )
    .expect_err("wrong type through commit");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));
}

#[test]
fn no_selection_or_a_stale_selection_disables_the_commands() {
    let (mut ctx, registry, _session, _page, shape) = fixture(None, false);
    ctx.selection.shapes.clear();
    ctx.selection.anchor = None;
    let error = dispatch(&mut ctx, &registry, "shape.set.text", text_args("x")).expect_err("none");
    assert!(matches!(error, CommandError::NotEnabled { .. }));

    ctx.selection.shapes = vec![ShapeId { index: 99 }];
    ctx.selection.anchor = Some(ShapeId { index: 99 });
    let error = dispatch(&mut ctx, &registry, "text.begin", Args::new()).expect_err("stale");
    match error {
        CommandError::NotEnabled { reason, .. } => {
            assert_eq!(reason.as_deref(), Some("the selected shape is stale"));
        }
        other => panic!("expected NotEnabled, got {other}"),
    }

    ctx.selection.shapes = vec![shape];
    ctx.selection.anchor = Some(shape);
    let error = dispatch(
        &mut ctx,
        &registry,
        "shape.set.text",
        Args::new().with("text", ArgValue::Number(1.0)),
    )
    .expect_err("wrong type");
    assert!(matches!(error, CommandError::InvalidArgs { .. }));
}

#[test]
fn begin_commit_and_cancel_run_headlessly() {
    let (mut ctx, registry, _session, _page, _shape) = fixture(None, false);
    let begin = dispatch(&mut ctx, &registry, "text.begin", Args::new()).expect("begin");
    assert!(
        begin
            .message
            .as_deref()
            .is_some_and(|message| message.contains("begins on shape 1"))
    );
    assert!(begin.edit.is_none());

    let commit = dispatch(&mut ctx, &registry, "text.commit", Args::new()).expect("commit");
    assert!(
        commit
            .message
            .as_deref()
            .is_some_and(|message| message.contains("committed on shape 1"))
    );
    let cancel = dispatch(&mut ctx, &registry, "text.cancel", Args::new()).expect("cancel");
    assert!(
        cancel
            .message
            .as_deref()
            .is_some_and(|message| message.contains("cancelled on shape 1"))
    );

    // `text.commit` with a text argument commits through the same write.
    dispatch(&mut ctx, &registry, "text.commit", text_args("written")).expect("commit writes");
    assert_eq!(
        ctx.active_document()
            .expect("session")
            .undo
            .undo_labels()
            .last()
            .copied(),
        Some("Write shape text")
    );
    assert_eq!(
        shape_text(&ctx, _shape),
        Some(TextContent {
            parts: vec![TextPart::Text("written".to_owned())],
        })
    );
}

#[test]
fn every_text_id_is_registered_as_implemented() {
    let (ctx, registry, _session, _page, _shape) = fixture(None, false);
    for id in ["text.begin", "text.commit", "text.cancel", "shape.set.text"] {
        let id = CommandId::intern(id).expect("id");
        assert_eq!(
            registry.status(&id),
            ferrogram_commands::CommandStatus::Implemented,
            "{id}"
        );
        assert!(registry.enabled(&id, &ctx).is_enabled(), "{id}");
    }
}

#[test]
fn a_shape_with_rows_keeps_its_first_row_markers() {
    let (mut ctx, registry, _session, _page, shape) = fixture(None, true);
    dispatch(&mut ctx, &registry, "shape.set.text", text_args("x")).expect("set");
    assert_eq!(
        shape_text(&ctx, shape),
        Some(TextContent {
            parts: vec![
                TextPart::CharRun { ix: 0 },
                TextPart::ParaRun { ix: 0 },
                TextPart::TabsRun { ix: 0 },
                TextPart::Text("x".to_owned()),
            ],
        })
    );
}

#[test]
fn a_deleted_run_row_is_not_kept_as_a_marker() {
    let (mut ctx, registry, _session, _page, shape) = fixture(None, true);
    {
        let session = ctx.active_document_mut().expect("session");
        let entry = session.document.shape_mut(shape).expect("shape");
        if let Some(section) = entry.sheet.sections.first_mut()
            && let Some(row) = section.rows.first_mut()
        {
            row.preserved
                .attributes
                .push(ferrogram_model::XmlAttribute {
                    name: ferrogram_model::QName::local("Del"),
                    value: "1".to_owned(),
                });
        }
    }
    dispatch(&mut ctx, &registry, "shape.set.text", text_args("x")).expect("set");
    assert_eq!(
        shape_text(&ctx, shape),
        Some(TextContent {
            parts: vec![
                TextPart::ParaRun { ix: 0 },
                TextPart::TabsRun { ix: 0 },
                TextPart::Text("x".to_owned()),
            ],
        })
    );
}
