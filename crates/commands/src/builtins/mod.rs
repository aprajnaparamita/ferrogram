//! The built-in registry: one function that registers every implemented
//! command (ADR 0028).
//!
//! The desktop app, the web app, the CLI and the parity generator
//! (ADR 0037) all build their registry here, so "which commands are
//! implemented" has exactly one answer. Implementers add their handlers
//! to the `BUILTINS` table when their command lands.
//!
//! [`registry`] starts from the stubs of `ui-spec/commands.yaml`
//! ([`Registry::from_mappings`], ADR 0006) and registers the real
//! builtins over them, so [`CommandStatus`] is accurate for every id
//! the data file knows.

mod file;
mod view;

use ferrogram_ui_spec::Commands;

use crate::app::AppCtx;
use crate::error::{CommandError, RegistryError};
use crate::registry::{CommandMeta, CommandStatus, Registry};
use crate::{Command, CommandId, Enabled, Invocation, Outcome};

pub use file::{
    PageSummary, Summary, kind_text, shape_count, sheet_text, value_text, warning_line,
};

/// The `ui-spec/commands.yaml` mapping table, compiled in (ADR 0007
/// pattern: the loader embeds its schema the same way). The CLI and
/// the parity generator need the stub rows without a data directory.
const COMMANDS_YAML: &str = include_str!("../../../../ui-spec/commands.yaml");

/// Whether a command can run: its enabled-state predicate (ADR 0028).
type EnabledFn = fn(&AppCtx) -> Enabled;

/// What a command does when dispatched (ADR 0028).
type RunFn = fn(&mut AppCtx, &Invocation) -> Result<Outcome, CommandError>;

/// One registered command: metadata plus its two function pointers.
///
/// The builtins are stateless; all mutable state lives in [`AppCtx`]
/// (ADR 0029).
struct Builtin {
    meta: CommandMeta,
    enabled_fn: EnabledFn,
    run: RunFn,
}

impl Command for Builtin {
    fn meta(&self) -> &CommandMeta {
        &self.meta
    }

    fn enabled(&self, ctx: &AppCtx) -> Enabled {
        (self.enabled_fn)(ctx)
    }

    fn execute(&self, ctx: &mut AppCtx, invocation: &Invocation) -> Result<Outcome, CommandError> {
        (self.run)(ctx, invocation)
    }
}

/// Every implemented command: `(id, fallback label, enabled, run)`.
///
/// The label is only used when `commands.yaml` has no row for the id;
/// the row's label wins otherwise. Argument names are the command
/// contract (ADR 0028): `file.open` takes `path` (or `name` + `data`),
/// `file.save_as` takes `path`, `file.render` takes `path`, `out`,
/// `page`, `format`, `scale`, `background` and `strict`, `file.export`
/// takes `path`, `out`, `format`, `scale` and `strict`, `file.inspect`
/// takes `path`, `page` and `json`, `view.zoom.fit` takes `width` and
/// `height` (points), and `file.close` takes an optional `confirm`
/// flag (the answer to its `ConfirmClose` request).
const BUILTINS: &[(&str, &str, EnabledFn, RunFn)] = &[
    ("file.new", "New", always, file::run_new),
    ("file.open", "Open", always, file::run_open),
    ("file.save", "Save", has_document, file::run_save),
    ("file.save_as", "Save As", has_document, file::run_save_as),
    ("file.close", "Close", has_document, file::run_close),
    ("file.inspect", "Inspect", always, file::run_inspect),
    ("file.render", "Render", always, file::run_render),
    ("file.export", "Export", always, file::run_export),
    ("edit.undo", "Undo", can_undo, view::run_undo),
    ("edit.redo", "Redo", can_redo, view::run_redo),
    ("view.zoom.in", "Zoom In", has_document, view::run_zoom_in),
    (
        "view.zoom.out",
        "Zoom Out",
        has_document,
        view::run_zoom_out,
    ),
    (
        "view.zoom.fit",
        "Fit to Window",
        has_pages,
        view::run_zoom_fit,
    ),
    (
        "view.toggle_grid",
        "Toggle Grid",
        has_document,
        view::run_toggle_grid,
    ),
    (
        "view.rulers",
        "Rulers",
        has_document,
        view::run_toggle_rulers,
    ),
    (
        "view.ui_layout.visio_2016",
        "Visio 2016 Ribbon",
        always,
        view::run_ui_layout_2016,
    ),
    (
        "view.ui_layout.visio_365",
        "Microsoft 365 Ribbon",
        always,
        view::run_ui_layout_365,
    ),
    ("page.next", "Next Page", has_pages, view::run_page_next),
    (
        "page.previous",
        "Previous Page",
        has_pages,
        view::run_page_previous,
    ),
    ("help.about", "About", always, view::run_about),
];

/// Builds the registry with every implemented command registered over
/// its `commands.yaml` stub (ADR 0028).
///
/// Returns [`RegistryError::Duplicate`] when two handlers claim one id;
/// that is a build bug, not input-dependent behaviour.
pub fn registry() -> Result<Registry, RegistryError> {
    let mappings = parse_mappings(COMMANDS_YAML)?;
    let mut registry = Registry::from_mappings(&mappings)?;
    for (id, label, enabled_fn, run) in BUILTINS {
        let meta = meta_for(&mappings, id, label)?;
        registry.register(Box::new(Builtin {
            meta,
            enabled_fn: *enabled_fn,
            run: *run,
        }))?;
    }
    Ok(registry)
}

/// Parses a `commands.yaml` document into its typed form.
///
/// The real table is the embedded [`COMMANDS_YAML`]; the tests parse
/// broken text through the same path.
fn parse_mappings(text: &str) -> Result<Commands, RegistryError> {
    let value: serde_json::Value =
        serde_saphyr::from_str(text).map_err(|error| RegistryError::BadMapping {
            message: error.to_string(),
        })?;
    serde_json::from_value(value).map_err(|error| RegistryError::BadMapping {
        message: error.to_string(),
    })
}

/// The metadata of one builtin: the `commands.yaml` row (label, Fluent
/// ids, VisUICmds constant, confidence) with the status flipped to
/// [`CommandStatus::Implemented`], or a minimal fallback when the
/// table has no row (a build gap: every registry id belongs in
/// `commands.yaml`, ADR 0028).
fn meta_for(
    mappings: &Commands,
    id: &'static str,
    label: &str,
) -> Result<CommandMeta, RegistryError> {
    if let Some(mapping) = mappings.entries.iter().find(|m| m.id.as_str() == id) {
        let mut meta = CommandMeta::from_mapping(mapping)?;
        meta.status = CommandStatus::Implemented;
        return Ok(meta);
    }
    let command_id = CommandId::parse(id).map_err(|error| RegistryError::BadId {
        id: id.to_owned(),
        reason: error.reason,
    })?;
    Ok(CommandMeta::new(
        command_id,
        label,
        CommandStatus::Implemented,
    ))
}

/// Always enabled.
pub(crate) fn always(_ctx: &AppCtx) -> Enabled {
    Enabled::yes()
}

/// Enabled when a document is open (ADR 0029 state).
pub(crate) fn has_document(ctx: &AppCtx) -> Enabled {
    if ctx.active_document().is_some() {
        Enabled::yes()
    } else {
        Enabled::no("no open document")
    }
}

/// Enabled when the open document has pages.
pub(crate) fn has_pages(ctx: &AppCtx) -> Enabled {
    match ctx.active_document() {
        None => Enabled::no("no open document"),
        Some(session) if session.document.pages.is_empty() => {
            Enabled::no("the document has no pages")
        }
        Some(_) => Enabled::yes(),
    }
}

/// Enabled when the active document has an undoable edit (ADR 0028).
pub(crate) fn can_undo(ctx: &AppCtx) -> Enabled {
    if ctx.active_document().is_some_and(|s| s.undo.can_undo()) {
        Enabled::yes()
    } else {
        Enabled::no("nothing to undo")
    }
}

/// Enabled when the active document has a redoable edit (ADR 0028).
pub(crate) fn can_redo(ctx: &AppCtx) -> Enabled {
    if ctx.active_document().is_some_and(|s| s.undo.can_redo()) {
        Enabled::yes()
    } else {
        Enabled::no("nothing to redo")
    }
}

/// The "no open document" failure of a command whose enabled-state
/// predicate says so (kept for direct calls that skip [`Registry::execute`]).
pub(crate) fn no_document() -> CommandError {
    CommandError::failed("no open document")
}

/// An outcome with a status-bar message and the run's warnings.
pub(crate) fn outcome(message: impl Into<String>, warnings: Vec<String>) -> Outcome {
    Outcome {
        message: Some(message.into()),
        warnings,
        ..Outcome::default()
    }
}

/// Records warning lines on the state (the status-bar ring, ADR 0021).
pub(crate) fn record_warnings(ctx: &mut AppCtx, warnings: &[String]) {
    for line in warnings {
        ctx.record_warning(line.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_builtin_registry_covers_every_builtins_row() {
        let registry = registry().expect("registry");
        assert!(registry.len() >= BUILTINS.len());
        for (id, _, _, _) in BUILTINS {
            let id = CommandId::parse(id).expect("builtin id");
            assert_eq!(
                registry.status(&id),
                CommandStatus::Implemented,
                "{id} must be implemented"
            );
        }
    }

    #[test]
    fn the_builtin_registry_has_the_stubs_of_the_data_file() {
        let registry = registry().expect("registry");
        let mappings = parse_mappings(COMMANDS_YAML).expect("mappings");
        assert_eq!(registry.len(), mappings.entries.len());
        // A row nothing implements is a stub with the tooltip.
        let id = CommandId::parse("file.print").expect("id");
        assert_eq!(registry.status(&id), CommandStatus::Stub);
    }

    #[test]
    fn the_embedded_table_parses_and_every_row_is_valid() {
        let mappings = parse_mappings(COMMANDS_YAML).expect("mappings");
        assert!(!mappings.entries.is_empty());
        for entry in &mappings.entries {
            CommandId::try_from(&entry.id).expect("row id");
        }
    }

    #[test]
    fn a_broken_mapping_table_is_a_bad_mapping() {
        let error = parse_mappings("{oops").expect_err("broken yaml");
        assert!(matches!(error, RegistryError::BadMapping { .. }), "{error}");
        let error = parse_mappings("a: 1").expect_err("not a list");
        assert!(matches!(error, RegistryError::BadMapping { .. }), "{error}");
    }

    #[test]
    fn meta_for_falls_back_when_the_table_has_no_row() {
        let mappings = Commands::default();
        let meta = meta_for(&mappings, "file.save", "Save").expect("meta");
        assert_eq!(meta.id.as_str(), "file.save");
        assert_eq!(meta.label, "Save");
        assert_eq!(meta.status, CommandStatus::Implemented);

        let mappings = parse_mappings(COMMANDS_YAML).expect("mappings");
        let meta = meta_for(&mappings, "file.save", "Fallback").expect("meta");
        assert_eq!(meta.label, "Save", "the table row wins");
        assert_eq!(meta.status, CommandStatus::Implemented);
        assert!(meta.visuicmd.is_some());
    }

    #[test]
    fn enabled_predicates_name_their_reasons() {
        let ctx = AppCtx::new(crate::Settings::default());
        assert!(always(&ctx).is_enabled());
        let disabled = has_document(&ctx);
        assert_eq!(disabled.reason.as_deref(), Some("no open document"));
        assert_eq!(has_pages(&ctx).reason.as_deref(), Some("no open document"));
        assert_eq!(can_undo(&ctx).reason.as_deref(), Some("nothing to undo"));
        assert_eq!(can_redo(&ctx).reason.as_deref(), Some("nothing to redo"));

        let mut ctx = AppCtx::new(crate::Settings::default());
        ctx.open_document(crate::DocumentSession::new(
            ferrogram_model::Document::new(),
            None,
        ));
        assert!(has_document(&ctx).is_enabled());
        assert_eq!(
            has_pages(&ctx).reason.as_deref(),
            Some("the document has no pages")
        );
    }
}
