//! `edit.*` commands (P5-04).
//!
//! Editing state: undo/redo plumbing, the `CellEdit` and `StructuralEdit`
//! undo steps, and the selection commands.
//! Planned ids (task P5-04 fills [`TABLE`] and the handlers):
//!
//! - `edit.delete` — detach the selection (undoable);
//! - `edit.select_all`, `edit.deselect` — selection housekeeping.
//!
//! `edit.undo` and `edit.redo` stay in [`crate::builtins::view`]; the
//! P5-04 task fixes their `with_undo` placeholder stack (ADR 0040).

use super::CommandTable;

/// The implemented `edit.*` commands (empty until P5-04).
pub(crate) const TABLE: CommandTable = &[];
