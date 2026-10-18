//! `page.*` commands (P5-09).
//!
//! Page commands: add, rename, delete and reorder (ADR 0047).
//! Planned ids (task P5-09 fills [`TABLE`] and the handlers):
//!
//! - `page.add`, `page.delete`, `page.rename` (`name` arg);
//! - `page.reorder` (`from`, `to` args) with `page.move_left` /
//!   `page.move_right` wrappers for the tabs.

use super::CommandTable;

/// The implemented `page.*` commands (empty until P5-09).
pub(crate) const TABLE: CommandTable = &[];
