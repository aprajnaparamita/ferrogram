//! `shape.*` commands (P5-04).
//!
//! Shape geometry commands: move, resize, rotate and the Size & Position
//! panel edits (ADR 0035).
//! Planned ids (task P5-04 fills [`TABLE`] and the handlers):
//!
//! - `shape.move`, `shape.resize`, `shape.rotate` — the drag commands;
//! - `shape.set.width`, `shape.set.height`, `shape.set.pin`,
//!   `shape.set.angle` — the panel fields;
//! - `shape.enter_group`, `shape.exit_group`.
//!
//! Every command returns one [`crate::Edit`] (one drag = one undo step,
//! ADR 0040) and bumps `DocumentSession::changes` (ADR 0040).

use super::CommandTable;

/// The implemented `shape.*` commands (empty until P5-04).
pub(crate) const TABLE: CommandTable = &[];
