//! `arrange.*` commands (P5-09).
//!
//! Arrangement commands: align, distribute, group, z-order (ADR 0047).
//! Planned ids (task P5-09 fills [`TABLE`] and the handlers):
//!
//! - `arrange.align.left|center|right|top|middle|bottom`;
//! - `arrange.distribute.horizontal|vertical`;
//! - `arrange.group`, `arrange.ungroup`;
//! - `arrange.bring_forward`, `arrange.send_backward`,
//!   `arrange.bring_to_front`, `arrange.send_to_back`.

use super::CommandTable;

/// The implemented `arrange.*` commands (empty until P5-09).
pub(crate) const TABLE: CommandTable = &[];
