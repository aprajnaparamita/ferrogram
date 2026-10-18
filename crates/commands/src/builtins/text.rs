//! `text.*` commands (P5-07).
//!
//! Shape text commands (ADR 0044).
//! Planned ids (task P5-07 fills [`TABLE`] and the handlers):
//!
//! - `text.begin` — open the in-place editor for the selection;
//! - `text.commit`, `text.cancel` — end the edit;
//! - `shape.set.text` — write the text (`text` arg) with one run
//!   (ADR 0044).

use super::CommandTable;

/// The implemented `text.*` commands (empty until P5-07).
pub(crate) const TABLE: CommandTable = &[];
