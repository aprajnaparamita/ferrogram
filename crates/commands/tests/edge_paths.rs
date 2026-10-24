//! Error paths and boundary values of the registered commands (task
//! P5-11, ADR 0049 rule 1): `InvalidArgs` and `failed` name their
//! reason, boundaries clamp or fail cleanly, and nothing panics.
//!
//! These are the thin branches the happy-path suites do not walk: the
//! argument validators (`need_finite`, `need_size`, `need_index`) and
//! the page and selection edges behind them.

// The never-crash lints apply to non-test code only (clippy.toml).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod support;

use support::dispatch;

use larrygram_commands::app::{AppCtx, DocumentSession, Settings};
use larrygram_commands::builtins::registry;
use larrygram_commands::{ArgValue, Args, CommandError, Registry};
use larrygram_model::{Document, PageId, Shape, ShapeId};

/// A document with two pages and one shape on the first.
fn fixture() -> (AppCtx, Registry) {
    let mut doc = Document::new();
    let first = doc.add_page("Page-1").expect("page one");
    doc.add_page("Page-2").expect("page two");
    doc.insert_shape(first, Shape::new(ShapeId { index: 0 }, 1), None)
        .expect("shape");
    let mut ctx = AppCtx::new(Settings::default());
    ctx.open_document(DocumentSession::new(doc, None));
    (ctx, registry().expect("registry"))
}

fn numbers(pairs: &[(&str, f64)]) -> Args {
    let mut args = Args::new();
    for (name, value) in pairs {
        args.set(*name, ArgValue::Number(*value));
    }
    args
}

/// The first shape's Visio `ID` (the fixture has one).
fn selected(ctx: &mut AppCtx) {
    let shape = ctx.active_document().expect("document").document.pages[0].shapes[0];
    ctx.selection.shapes = vec![shape];
    ctx.selection.anchor = Some(shape);
    ctx.selection.page = Some(PageId { index: 0 });
}

/// `need_finite` and `need_size` name the bad value's contract.
#[test]
fn a_non_finite_or_negative_number_is_an_invalid_argument() {
    let (mut ctx, registry) = fixture();
    selected(&mut ctx);
    let cases: &[(&str, Args, &str)] = &[
        (
            "shape.move",
            numbers(&[("dx", f64::NAN), ("dy", 0.0)]),
            "dx must be finite",
        ),
        (
            "shape.rotate",
            numbers(&[("angle", f64::INFINITY)]),
            "angle must be finite",
        ),
        (
            "shape.set.width",
            numbers(&[("width", -1.0)]),
            "width must not be negative",
        ),
        (
            "shape.resize",
            numbers(&[("width", 1.0), ("height", -0.5)]),
            "height must not be negative",
        ),
        (
            "shape.set.pin",
            numbers(&[("x", 1.0), ("y", f64::NAN)]),
            "y must be finite",
        ),
    ];
    for (id, args, reason) in cases {
        let error = dispatch(&mut ctx, &registry, id, args.clone()).expect_err(id);
        assert!(
            error.to_string().contains(reason),
            "{id}: {error} should name {reason:?}"
        );
    }
}

/// `need_index` names the page index's contract.
#[test]
fn a_page_index_out_of_range_is_an_invalid_argument() {
    let (mut ctx, registry) = fixture();
    let cases: &[Args] = &[
        Args::new()
            .with("from", ArgValue::Integer(-1))
            .with("to", ArgValue::Integer(1)),
        Args::new()
            .with("from", ArgValue::Number(0.5))
            .with("to", ArgValue::Integer(1)),
        Args::new()
            .with("from", ArgValue::Integer(0))
            .with("to", ArgValue::Text("one".to_owned())),
    ];
    for args in cases {
        let error = dispatch(&mut ctx, &registry, "page.reorder", args.clone())
            .expect_err("a bad page index");
        assert!(
            matches!(error, CommandError::InvalidArgs { .. }),
            "{error:?}"
        );
        assert!(!error.to_string().is_empty(), "the error names its reason");
    }
    // A missing index asks for the value instead (ADR 0028).
    let missing = dispatch(
        &mut ctx,
        &registry,
        "page.reorder",
        Args::new().with("to", ArgValue::Integer(1)),
    )
    .expect_err("missing from");
    assert!(
        matches!(missing, CommandError::NeedsInput(_)),
        "{missing:?}"
    );
}

/// The selection and page edges behind the editing commands.
#[test]
fn the_selection_and_page_edges_name_their_reason() {
    // `view.zoom.fit` on a document with no pages.
    let mut empty = AppCtx::new(Settings::default());
    empty.open_document(DocumentSession::new(Document::new(), None));
    let registry = registry().expect("registry");
    let error = dispatch(
        &mut empty,
        &registry,
        "view.zoom.fit",
        numbers(&[("width", 800.0), ("height", 600.0)]),
    )
    .expect_err("pages");
    assert!(
        error.to_string().contains("the document has no pages"),
        "{error}"
    );

    // `edit.select` with an unknown mode.
    let (mut ctx, registry) = fixture();
    selected(&mut ctx);
    let mut args = Args::new();
    args.set("mode", ArgValue::Text("bogus".to_owned()));
    let error = dispatch(&mut ctx, &registry, "edit.select", args).expect_err("mode");
    assert!(error.to_string().contains("mode"), "{error}");
}

/// The last single-line arms behind the page and text commands (task
/// P5-11): a page with no name, a document with no active page and a
/// text argument of the wrong type.
#[test]
fn the_last_single_line_arms_name_their_reason() {
    // `page.delete` of a page with an empty name uses the label
    // fallback.
    let mut doc = Document::new();
    doc.add_page("").expect("page one");
    doc.add_page("Page-2").expect("page two");
    let mut ctx = AppCtx::new(Settings::default());
    ctx.open_document(DocumentSession::new(doc, None));
    let registry = registry().expect("registry");
    let outcome = dispatch(&mut ctx, &registry, "page.delete", Args::new()).expect("delete");
    let message = outcome.message.unwrap_or_default();
    assert!(!message.is_empty(), "the delete reports the page");

    // `view.zoom.fit` on a document whose active page is gone.
    let (mut ctx, registry) = fixture();
    ctx.active_document_mut().expect("session").active_page = None;
    let error = dispatch(
        &mut ctx,
        &registry,
        "view.zoom.fit",
        numbers(&[("width", 800.0), ("height", 600.0)]),
    )
    .expect_err("no active page");
    assert!(error.to_string().contains("no active page"), "{error}");

    // `shape.set.text` with a non-text argument.
    let (mut ctx, registry) = fixture();
    selected(&mut ctx);
    let mut args = Args::new();
    args.set("text", ArgValue::Integer(3));
    let error = dispatch(&mut ctx, &registry, "shape.set.text", args).expect_err("text type");
    assert!(error.to_string().contains("text"), "{error}");
}
