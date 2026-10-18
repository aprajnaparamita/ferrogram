//! `connector.*` commands (P5-08).
//!
//! Dynamic connector commands: glue, unglue and re-route (ADR 0045).
//! Planned ids (task P5-08 fills [`TABLE`] and the handlers):
//!
//! - `connector.glue_begin`, `connector.glue_end`, `connector.unglue`;
//! - `connector.reroute_all` — recompute every glued connector's path
//!   (`Document::write_connector_path`) after a move;
//! - `connector.style` — straight or right-angle (ADR 0045).

use super::CommandTable;

/// The implemented `connector.*` commands (empty until P5-08).
pub(crate) const TABLE: CommandTable = &[];
