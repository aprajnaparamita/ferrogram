//! The registry itself (ADR 0028): one entry per command id.
//!
//! The registry is built from `ui-spec/commands.yaml` (ADR 0006) and
//! then has its implemented commands registered over the stubs. The UI
//! reads [`CommandStatus`] and [`Registry::enabled`] to decide whether a
//! control renders enabled, greyed with a reason, or disabled with the
//! "Not yet implemented" tooltip (ADR 0031).
//!
//! Alpha status (task P4-01): [`Registry::new`] and the accessors are
//! defined; `register`, `from_mappings`, `enabled` and `execute` are
//! "not implemented" stubs until P4-02.

use ferrogram_ui_spec::{CommandMapping, FluentId, MappingConfidence};

use crate::app::AppCtx;
use crate::error::{CommandError, RegistryError};
use crate::{Command, CommandId, Enabled, Invocation, Outcome};

/// Whether a command id has a real handler (ADR 0028).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CommandStatus {
    /// A real handler is registered.
    Implemented,
    /// The id exists (usually from `commands.yaml`) but has no handler
    /// yet: the UI renders it disabled with "Not yet implemented".
    Stub,
}

/// The metadata of one registered command (ADR 0028).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandMeta {
    /// The command id (ADR 0006).
    pub id: CommandId,
    /// Short UI label (from `commands.yaml`).
    pub label: String,
    /// Whether a real handler backs this id.
    pub status: CommandStatus,
    /// Fluent control names that surface this command (ADR 0006).
    pub fluent_ids: Vec<FluentId>,
    /// The VisUICmds constant, when one maps to this command.
    pub visuicmd: Option<String>,
    /// How confident the mapping is (ADR 0006).
    pub mapping_confidence: MappingConfidence,
}

impl CommandMeta {
    /// Metadata for a registered command id and label.
    pub fn new(id: CommandId, label: impl Into<String>, status: CommandStatus) -> Self {
        Self {
            id,
            label: label.into(),
            status,
            fluent_ids: Vec::new(),
            visuicmd: None,
            mapping_confidence: MappingConfidence::Unknown,
        }
    }

    /// Builds metadata from one `commands.yaml` entry (ADR 0006).
    pub fn from_mapping(mapping: &CommandMapping) -> Result<CommandMeta, RegistryError> {
        let id = CommandId::try_from(&mapping.id).map_err(|error| RegistryError::BadId {
            id: mapping.id.as_str().to_owned(),
            reason: error.reason,
        })?;
        Ok(CommandMeta {
            id,
            label: mapping.label.clone(),
            status: CommandStatus::Stub,
            fluent_ids: mapping.fluent_ids.clone(),
            visuicmd: mapping.visuicmd.clone(),
            mapping_confidence: mapping.mapping_confidence,
        })
    }
}

/// A command that reports "not implemented" (ADR 0006, ADR 0031).
///
/// [`Registry::from_mappings`] creates one per `commands.yaml` entry;
/// registering a real command over it flips the id to
/// [`CommandStatus::Implemented`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StubCommand {
    meta: CommandMeta,
}

impl StubCommand {
    /// A stub for the given metadata.
    pub fn new(meta: CommandMeta) -> Self {
        Self { meta }
    }
}

impl Command for StubCommand {
    fn meta(&self) -> &CommandMeta {
        &self.meta
    }

    fn enabled(&self, _ctx: &AppCtx) -> Enabled {
        Enabled::no("not yet implemented")
    }

    fn execute(
        &self,
        _ctx: &mut AppCtx,
        _invocation: &Invocation,
    ) -> Result<Outcome, CommandError> {
        Err(CommandError::Unsupported {
            id: self.meta.id.to_string(),
        })
    }
}

/// The command registry (ADR 0028): `CommandId` to [`Command`].
///
/// One dispatch table for the menus, the ribbon, shortcuts, the CLI, the
/// JSON control channel and MCP (AGENTS.md section 6).
#[derive(Default)]
pub struct Registry {
    commands: Vec<Box<dyn Command>>,
}

impl Registry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of registered commands.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// True when nothing is registered.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// The command under `id`, if one is registered.
    pub fn get(&self, id: &CommandId) -> Option<&dyn Command> {
        self.commands
            .iter()
            .find(|command| command.meta().id == *id)
            .map(|command| command.as_ref())
    }

    /// The metadata under `id`, if one is registered.
    pub fn meta(&self, id: &CommandId) -> Option<&CommandMeta> {
        self.get(id).map(Command::meta)
    }

    /// The status under `id`; unknown ids are [`CommandStatus::Stub`].
    pub fn status(&self, id: &CommandId) -> CommandStatus {
        self.meta(id)
            .map(|meta| meta.status)
            .unwrap_or(CommandStatus::Stub)
    }

    /// Iterates the registered ids in registration order.
    pub fn ids(&self) -> impl Iterator<Item = CommandId> + '_ {
        self.commands.iter().map(|command| command.meta().id)
    }

    /// Registers a command.
    ///
    /// Not implemented yet (P4-02).
    pub fn register(&mut self, command: Box<dyn Command>) -> Result<(), RegistryError> {
        let _ = command;
        Err(RegistryError::NotImplemented {
            what: "Registry::register",
        })
    }

    /// Builds a registry of stubs from `ui-spec/commands.yaml` (ADR 0006).
    ///
    /// Not implemented yet (P4-02).
    pub fn from_mappings(
        mappings: &ferrogram_ui_spec::Commands,
    ) -> Result<Registry, RegistryError> {
        let _ = mappings;
        Err(RegistryError::NotImplemented {
            what: "Registry::from_mappings",
        })
    }

    /// Whether the command can run in the current state (ADR 0028).
    ///
    /// Unknown ids report disabled with the "not yet implemented"
    /// reason, the same tooltip the UI shows for stubs (ADR 0031).
    pub fn enabled(&self, id: &CommandId, ctx: &AppCtx) -> Enabled {
        match self.get(id) {
            Some(command) => command.enabled(ctx),
            None => Enabled::no("not yet implemented"),
        }
    }

    /// Dispatches one invocation (ADR 0028).
    ///
    /// Not implemented yet (P4-02).
    pub fn execute(
        &self,
        id: &CommandId,
        ctx: &mut AppCtx,
        invocation: Invocation,
    ) -> Result<Outcome, CommandError> {
        let _ = (id, ctx, invocation);
        Err(CommandError::NotImplemented {
            what: "Registry::execute",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Settings;
    use crate::{Args, Source};

    fn meta(id: &'static str) -> CommandMeta {
        CommandMeta::new(
            CommandId::parse(id).expect("id"),
            "Test",
            CommandStatus::Stub,
        )
    }

    #[test]
    fn meta_builders() {
        let meta = meta("file.save");
        assert_eq!(meta.id.as_str(), "file.save");
        assert_eq!(meta.label, "Test");
        assert_eq!(meta.status, CommandStatus::Stub);
        assert!(meta.fluent_ids.is_empty());
        assert!(meta.visuicmd.is_none());
        assert_eq!(meta.mapping_confidence, MappingConfidence::Unknown);

        let mapping = CommandMapping {
            id: ferrogram_ui_spec::CommandId::new("file.save"),
            label: "Save".to_owned(),
            visuicmd: Some("visCmdFileSave".to_owned()),
            fluent_ids: vec![FluentId::new("FileSave")],
            mapping_confidence: MappingConfidence::Exact,
        };
        let meta = CommandMeta::from_mapping(&mapping).expect("mapping");
        assert_eq!(meta.id.as_str(), "file.save");
        assert_eq!(meta.label, "Save");
        assert_eq!(meta.status, CommandStatus::Stub);
        assert_eq!(meta.visuicmd.as_deref(), Some("visCmdFileSave"));
        assert_eq!(meta.mapping_confidence, MappingConfidence::Exact);
    }

    #[test]
    fn a_stub_command_is_disabled_and_unsupported() {
        let stub = StubCommand::new(meta("file.save"));
        let mut ctx = AppCtx::new(Settings::default());
        let enabled = stub.enabled(&ctx);
        assert!(!enabled.is_enabled());
        assert_eq!(enabled.reason.as_deref(), Some("not yet implemented"));
        let error = stub
            .execute(&mut ctx, &Invocation::empty(Source::Test))
            .expect_err("stub");
        assert_eq!(
            error.to_string(),
            "command file.save is not implemented yet"
        );
    }

    #[test]
    fn an_empty_registry_reports_nothing_registered() {
        let registry = Registry::new();
        let id = CommandId::parse("file.save").expect("id");
        let ctx = AppCtx::new(Settings::default());
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.get(&id).is_none());
        assert!(registry.meta(&id).is_none());
        assert_eq!(registry.status(&id), CommandStatus::Stub);
        assert_eq!(registry.ids().count(), 0);
        let enabled = registry.enabled(&id, &ctx);
        assert!(!enabled.is_enabled());
    }

    #[test]
    fn registry_operations_are_stubs() {
        let mut registry = Registry::new();
        assert!(
            registry
                .register(Box::new(StubCommand::new(meta("file.save"))))
                .is_err()
        );
        assert!(Registry::from_mappings(&ferrogram_ui_spec::Commands::default()).is_err());
        let id = CommandId::parse("file.save").expect("id");
        let mut ctx = AppCtx::new(Settings::default());
        let outcome = registry.execute(&id, &mut ctx, Invocation::new(Args::new(), Source::Test));
        assert!(outcome.is_err());
    }
}
