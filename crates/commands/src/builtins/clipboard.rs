//! `clipboard.*` commands (P5-09).
//!
//! Clipboard commands over the internal clipboard (ADR 0047).
//! Planned ids (task P5-09 fills [`TABLE`] and the handlers):
//!
//! - `edit.copy`, `edit.cut`, `edit.paste`, `edit.duplicate`;
//! - paste takes an optional `x`/`y` (paste at the pointer) else the
//!   recorded offset (ADR 0047).

use super::CommandTable;

/// The implemented `clipboard.*` commands (empty until P5-09).
pub(crate) const TABLE: CommandTable = &[];
