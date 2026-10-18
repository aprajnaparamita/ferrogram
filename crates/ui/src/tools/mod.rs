//! Interaction tools (ADR 0042): the tool state machine and its event
//! seam.
//!
//! One tool is active at a time (pointer, rectangle, ellipse, line,
//! connector, text). Each tool consumes [`ToolEvent`]s and returns a
//! [`ToolEffect`]: [`Intent`]s to dispatch through the command registry
//! (never a direct model mutation, ADR 0028) plus the transient
//! [`Overlay`] marks of this frame (ADR 0042). The state machine never
//! panics on any event order (AGENTS.md section 6).
//!
//! The modules are split one per Phase 5 task so parallel tasks never
//! touch the same file (task P5-01 wires the seams):
//!
//! - [`mod@pointer`] — select, marquee, move, resize, rotate (P5-05);
//! - [`mod@draw`] — rectangle, ellipse and line drawing tools (P5-06);
//! - [`mod@stencil`] — drag from the Shapes window, drop on the page (P5-06);
//! - [`mod@text`] — the text tool and the in-place editor hand-off (P5-07);
//! - [`mod@connector`] — the dynamic connector tool (P5-08).

pub mod connector;
pub mod draw;
pub mod pointer;
pub mod stencil;
pub mod text;

use ferrogram_commands::AppCtx;
use ferrogram_geom::Point;
use ferrogram_render::Overlay;

use crate::{Intent, UiError};

/// The active interaction tool (ADR 0042).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Tool {
    /// Select, move, resize, rotate (the default).
    #[default]
    Pointer,
    /// Draw a rectangle shape.
    Rectangle,
    /// Draw an ellipse shape.
    Ellipse,
    /// Draw a line shape.
    Line,
    /// Draw a dynamic connector (ADR 0045).
    Connector,
    /// Draw a text box (ADR 0044).
    Text,
}

/// The modifier keys held during an event (ADR 0042).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    /// Shift held.
    pub shift: bool,
    /// Ctrl (Cmd on macOS mapping, ADR 0036) held.
    pub ctrl: bool,
    /// Alt/Option held (disables snapping, ADR 0043).
    pub alt: bool,
}

impl Modifiers {
    /// No modifiers held.
    pub const fn none() -> Self {
        Self {
            shift: false,
            ctrl: false,
            alt: false,
        }
    }
}

/// One input event, in page coordinates and inches (the canvas converts
/// from screen points at the edge, AGENTS.md section 6).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ToolEvent {
    /// A pointer button press.
    PointerDown {
        /// The pointer position, in page inches.
        pos: Point,
        /// The held modifiers.
        modifiers: Modifiers,
    },
    /// A pointer move.
    PointerMove {
        /// The pointer position, in page inches.
        pos: Point,
        /// The held modifiers.
        modifiers: Modifiers,
    },
    /// A pointer button release.
    PointerUp {
        /// The pointer position, in page inches.
        pos: Point,
        /// The held modifiers.
        modifiers: Modifiers,
    },
    /// A double click (enter a group, start text edit, ADR 0044).
    DoubleClick {
        /// The pointer position, in page inches.
        pos: Point,
    },
    /// Escape or another tool-cancelling key.
    Cancel,
}

/// What one event produced (ADR 0042).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ToolEffect {
    /// Commands to dispatch through the registry, in order.
    pub intents: Vec<Intent>,
    /// The overlay marks to draw this frame (ADR 0042).
    pub overlay: Overlay,
}

impl ToolEffect {
    /// An effect that does nothing.
    pub fn new() -> Self {
        Self::default()
    }
}

/// The tool state machine (ADR 0042).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ToolMachine {
    /// The active tool.
    pub tool: Tool,
    /// The opaque tool state (drag anchors, draw previews); only the
    /// owning tool module reads it.
    pub(crate) state: Option<pointer::PointerState>,
}

impl ToolMachine {
    /// A machine with the pointer tool active.
    pub fn new() -> Self {
        Self::default()
    }

    /// Handles one event and returns what to do (ADR 0042).
    ///
    /// `ctx` is read-only: the machine computes [`Intent`]s and the
    /// caller dispatches them (ADR 0028). Task P5-05 implements the
    /// pointer tool and dispatches to the other tool modules.
    pub fn handle(&mut self, event: ToolEvent, ctx: &AppCtx) -> Result<ToolEffect, UiError> {
        let _ = (event, ctx);
        Err(UiError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    //! Stub contract (P5-01): every tool reports "not implemented" until
    //! its task lands. Replace these with real tests when implementing.
    use super::*;
    use ferrogram_commands::Settings;

    type Handler = fn(&mut ToolMachine, &ToolEvent, &AppCtx) -> Result<ToolEffect, UiError>;
    type Anchor = fn(&ToolMachine) -> Option<Point>;

    #[test]
    fn every_tool_is_a_not_implemented_stub() {
        let ctx = AppCtx::new(Settings::default());
        let event = ToolEvent::PointerDown {
            pos: Point::new(1.0, 2.0),
            modifiers: Modifiers::none(),
        };
        let mut machine = ToolMachine::new();
        assert_eq!(machine.tool, Tool::Pointer);
        assert!(matches!(
            machine.handle(event.clone(), &ctx),
            Err(UiError::NotImplemented)
        ));
        let tools: [(Handler, Anchor); 5] = [
            (pointer::handle, pointer::anchor),
            (draw::handle, draw::anchor),
            (connector::handle, connector::anchor),
            (stencil::handle, stencil::anchor),
            (text::handle, text::anchor),
        ];
        for (handle, anchor) in tools {
            assert!(matches!(
                handle(&mut machine, &event, &ctx),
                Err(UiError::NotImplemented)
            ));
            assert_eq!(anchor(&machine), None);
        }
        assert_eq!(ToolEffect::new(), ToolEffect::default());
        assert_eq!(Modifiers::none(), Modifiers::default());
    }
}
