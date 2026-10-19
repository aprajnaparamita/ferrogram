//! The text tool and the in-place editor hand-off (ADR 0044).
//!
//! A click with the `Text` tool asks `insert.drawing.text_box` to create
//! a text-only shape and then `text.begin` to start editing the new
//! selection. The drawing command owns the model insertion and selects
//! the new shape (P5-06); the tool only emits registry intents (ADR
//! 0042). The editor state itself lives in
//! [`crate::text_edit::TextEditor`]; the shell draws it over the shape's
//! text block.
//!
//! The pointer tool's double click is owned by P5-05; its text-editing
//! branch is requested in the P5-07 Result (Open issues) because
//! `tools/pointer.rs` is outside this task's files.

use ferrogram_commands::{ArgValue, Args, CommandError, CommandId, Invocation, Source};
use ferrogram_geom::Point;

use crate::{Intent, UiError};

use super::{ToolEffect, ToolEvent};

/// The default width of a new text box, in inches (our own choice).
pub const TEXT_BOX_WIDTH_IN: f64 = 2.0;

/// The default height of a new text box, in inches (our own choice).
pub const TEXT_BOX_HEIGHT_IN: f64 = 0.5;

/// Handles one event for this tool (owner: P5-07, ADR 0042).
///
/// A pointer release creates one text box at the release point and opens
/// the editor on the selection the insert command will make. The tool
/// never mutates the model or the selection itself.
pub fn handle(
    _state: &mut super::ToolMachine,
    event: &super::ToolEvent,
    ctx: &ferrogram_commands::AppCtx,
) -> Result<super::ToolEffect, UiError> {
    let mut effect = ToolEffect::new();
    if ctx.active_document().is_none() {
        return Ok(effect);
    }
    match event {
        ToolEvent::PointerUp { pos, .. } => {
            effect.intents.push(insert_text_box(*pos)?);
            effect.intents.push(text_begin()?);
        }
        ToolEvent::DoubleClick { .. } => {
            // The clicked shape is the selection made by the pointer
            // tool's press; the command validates it headlessly.
            effect.intents.push(text_begin()?);
        }
        _ => {}
    }
    Ok(effect)
}

/// The drag anchor, in page inches (the preview's start).
///
/// The text tool creates a fixed-size box on release and has no drag
/// preview, so it reports no anchor.
pub fn anchor(_state: &super::ToolMachine) -> Option<Point> {
    None
}

/// The `insert.drawing.text_box` intent for a release at `pos`.
fn insert_text_box(pos: Point) -> Result<Intent, UiError> {
    let args = Args::new()
        .with("x", ArgValue::Number(pos.x))
        .with("y", ArgValue::Number(pos.y))
        .with("width", ArgValue::Number(TEXT_BOX_WIDTH_IN))
        .with("height", ArgValue::Number(TEXT_BOX_HEIGHT_IN));
    intent("insert.drawing.text_box", args)
}

/// The `text.begin` intent for the new selection.
fn text_begin() -> Result<Intent, UiError> {
    intent("text.begin", Args::new())
}

/// Builds one UI-source intent, mapping a bad static id to a command
/// failure (the ids are compile-time constants; this cannot happen with
/// a valid build, and it must never panic).
fn intent(id: &'static str, args: Args) -> Result<Intent, UiError> {
    let command = CommandId::parse(id)
        .map_err(|error| UiError::Command(CommandError::failed(error.to_string())))?;
    Ok(Intent::new(command, Invocation::new(args, Source::Ui)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrogram_commands::Settings;
    use ferrogram_model::Document;

    fn app_with_page() -> ferrogram_commands::AppCtx {
        let mut doc = Document::new();
        doc.add_page("Page-1").expect("page");
        let mut app = ferrogram_commands::AppCtx::new(Settings::default());
        app.open_document(ferrogram_commands::DocumentSession::new(doc, None));
        app
    }

    #[test]
    fn a_release_emits_the_insert_and_begin_intents() {
        let ctx = app_with_page();
        let mut machine = super::super::ToolMachine::new();
        let effect = handle(
            &mut machine,
            &ToolEvent::PointerUp {
                pos: Point::new(1.5, 2.5),
                modifiers: super::super::Modifiers::none(),
            },
            &ctx,
        )
        .expect("handle");
        assert_eq!(effect.intents.len(), 2);
        assert_eq!(
            effect.intents[0].command.as_str(),
            "insert.drawing.text_box"
        );
        assert_eq!(effect.intents[0].invocation.args.number("x"), Some(1.5));
        assert_eq!(
            effect.intents[0].invocation.args.number("width"),
            Some(TEXT_BOX_WIDTH_IN)
        );
        assert_eq!(effect.intents[1].command.as_str(), "text.begin");
        assert_eq!(effect.overlay, Default::default());
    }

    #[test]
    fn no_document_or_other_events_do_nothing() {
        let ctx = ferrogram_commands::AppCtx::new(Settings::default());
        let mut machine = super::super::ToolMachine::new();
        let effect = handle(
            &mut machine,
            &ToolEvent::PointerUp {
                pos: Point::new(0.0, 0.0),
                modifiers: super::super::Modifiers::none(),
            },
            &ctx,
        )
        .expect("handle");
        assert!(effect.intents.is_empty());

        let ctx = app_with_page();
        for event in [
            ToolEvent::PointerDown {
                pos: Point::new(0.0, 0.0),
                modifiers: super::super::Modifiers::none(),
            },
            ToolEvent::PointerMove {
                pos: Point::new(1.0, 1.0),
                modifiers: super::super::Modifiers::none(),
            },
            ToolEvent::Cancel,
        ] {
            let effect = handle(&mut machine, &event, &ctx).expect("handle");
            assert!(effect.intents.is_empty());
        }
        assert_eq!(anchor(&machine), None);
    }

    #[test]
    fn a_double_click_emits_begin() {
        let ctx = app_with_page();
        let mut machine = super::super::ToolMachine::new();
        let effect = handle(
            &mut machine,
            &ToolEvent::DoubleClick {
                pos: Point::new(1.0, 1.0),
            },
            &ctx,
        )
        .expect("handle");
        assert_eq!(effect.intents.len(), 1);
        assert_eq!(effect.intents[0].command.as_str(), "text.begin");
    }

    #[test]
    fn the_static_ids_are_valid() {
        assert!(intent("text.begin", Args::new()).is_ok());
        assert!(insert_text_box(Point::new(0.0, 0.0)).is_ok());
    }
}
