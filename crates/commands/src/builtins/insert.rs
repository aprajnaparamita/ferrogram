//! `insert.*` commands (P5-06).
//!
//! Insert commands: new shapes from the drawing tools and from the Shapes
//! window (ADR 0034, ADR 0046).
//! Planned ids (task P5-06 fills [`TABLE`] and the handlers):
//!
//! - `insert.shape` — a master instance at a point (`stencil`, `master`,
//!   `x`, `y`, `width`, `height`, optional glue args, ADR 0046);
//! - `insert.drawing.rectangle`, `insert.drawing.ellipse`,
//!   `insert.drawing.line`, `insert.drawing.text_box`.

use super::CommandTable;

/// The implemented `insert.*` commands (empty until P5-06).
pub(crate) const TABLE: CommandTable = &[];
