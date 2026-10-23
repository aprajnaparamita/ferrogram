# 0028: Command registry and invocation

- Status: accepted
- Date: 2026-10-16
- Task: P4-01

## Context

AGENTS.md section 6: "One command registry (`crates/commands`). Every user
action is a command with an id, a label, an enabled-state predicate and undo
support. Menus, command bar, shortcuts, CLI, the JSON control channel and MCP all
dispatch by command id through the same registry. Programmatic calls never
open dialogs."

ADR 0006 defines the id space (`scope.verb[.qualifier]`, ids live in
`ui-spec/commands.yaml`) and names `crates/commands` as the registry home.
ADR 0026 makes the Phase 3 CLI's `Command` enum the seam: `inspect`,
`render` and `export` become registry commands `file.inspect`,
`file.render` and `file.export` in Phase 4. Phase 5 (editing core, undo)
and Phase 8 (JSON control channel, MCP) build on the same registry, so the
interfaces must cover them now without implementing them.

Two hard constraints shape the design: programmatic callers must be able to
run every command without a GUI (headless CLI today, MCP in Phase 8), and a
command that needs input (a file path) must be able to ask for it without
opening a dialog itself.

## Decision

### Types (`crates/commands`, L4)

Plain data and one trait. The full stubs are committed in
`crates/commands/src/`; the shape is:

```rust
pub struct CommandId(pub &'static str);           // ADR 0006 grammar, checked

pub enum ArgValue {
    Text(String),
    Path(PathBuf),
    Integer(i64),
    Number(f64),
    Flag(bool),
    List(Vec<String>),
}
pub struct Args { /* name -> ArgValue, BTreeMap-backed */ }

pub enum Source { Ui, CommandBar, Menu, Shortcut, Cli, Control, Mcp, Test }
pub struct Invocation { pub args: Args, pub source: Source }

pub struct Enabled { pub value: bool, pub reason: Option<String> }

pub struct Outcome {
    pub message: Option<String>,
    pub edit: Option<Box<dyn Edit>>,       // undo support; None = not undoable
    pub ui_request: Option<UiRequest>,     // a request the UI may act on
}

pub enum UiRequest {
    ShowAbout,
    ShowOptions,
    ConfirmClose { name: String },
    // extended as the UI grows; a programmatic caller just records it
}

pub trait Command: Send + Sync {
    fn meta(&self) -> &CommandMeta;
    fn enabled(&self, ctx: &AppCtx) -> Enabled;
    fn execute(&self, ctx: &mut AppCtx, invocation: &Invocation)
        -> Result<Outcome, CommandError>;
}

pub struct CommandMeta {
    pub id: CommandId,
    pub label: String,
    pub status: CommandStatus,             // Implemented | Stub
    pub fluent_ids: Vec<FluentId>,
    pub visuicmd: Option<String>,
    pub mapping_confidence: MappingConfidence,
}

pub struct Registry { /* CommandId -> Box<dyn Command>, BTreeMap-backed */ }
```

`Registry` API: `new`, `register`, `from_mappings(&ui_spec::Commands)`,
`get`, `meta`, `status`, `ids`, `enabled`, `execute`, `len`, `is_empty`.
`from_mappings` registers one `StubCommand` per `commands.yaml` entry: its
`enabled` is `Enabled::no("not yet implemented")` and its `execute` returns
`Err(CommandError::Unsupported)`. A `StubCommand` is therefore the "renders
disabled with a not-yet-implemented tooltip" state of ADR 0006; the UI reads
`CommandStatus::Stub` or the `Enabled` reason for the tooltip text.

### Ids beyond `commands.yaml`

`commands.yaml` is generated from Microsoft data (ADR 0006), so it has no
entry for Larrygram-only affordances: the CLI triad `file.inspect`,
`file.render`, `file.export` (ADR 0026), `app.quit`, `page.next`,
`page.previous`, `view.ui_layout.panel_2016`, `view.ui_layout.panel_365`,
`view.zoom.fit`, `help.about`. Rule (amending ADR 0006): such ids are added
to `commands.yaml` by the extractor from a static "our own commands" list in
`xtask/src/extract/commands.rs`, with `fluent_ids: []` (or the Fluent ids of
controls we bind ourselves) and `mapping_confidence: unknown`. Every
registry id is therefore in `commands.yaml`, and the parity matrix sees
every id. Implementers never hand-edit `commands.yaml` (ADR 0006). The
Phase 4 extractor edits are owned by P4-02 alone.

### `CommandId` storage

`CommandId` wraps `&'static str` (ADR 0006). Ids coming from YAML are
interned once per process by `CommandId::intern`, a bounded `Box::leak`
interner (the same pattern as the model's table defaults, P2-04). The bound
is 4096 ids; past it `intern` returns `CommandIdError`. Static ids use
`CommandId::parse`, which validates the ADR 0006 grammar.

### Invocation without dialogs

A command never opens a dialog and never blocks on the user. When required
input is missing, `execute` returns
`Err(CommandError::NeedsInput(NeededInput::Path { purpose, suggested }))`
(or `Text`/`Number`). The dispatcher decides what to do:

- **Interactive** (`Source::Ui`, `CommandBar`, `Menu`, `Shortcut`): the UI shows
  the matching dialog (native on desktop, upload on wasm, ADR 0036),
  re-dispatches the same command id with the value filled into `Args`.
- **Programmatic** (`Source::Cli`, `Control`, `Mcp`, `Test`): the caller
  receives the error and supplies the argument on the next call. No dialog
  can appear.

Argument names are part of the contract: `file.open` takes `path`,
`file.save_as` takes `path`, `file.render` takes `path`, `out`, `page`,
`format`, `scale`, `background`, `strict` (exactly ADR 0026's flags).

### Results and errors

```rust
#[non_exhaustive]
pub enum CommandError {
    UnknownCommand(String),     // id not in the registry
    NotEnabled { id: String, reason: Option<String> },
    NeedsInput(NeededInput),
    InvalidArgs(String),
    Unsupported { id: String }, // registered as Stub
    Failed(String),             // domain failure (read, render, io)
}
```

`Failed` carries the domain error's message; the domain crates keep their
own typed errors (`larrygram_vsdx::Error`, `larrygram_render::Error`) and
`commands` maps them at the boundary. No error string is ever a panic path.
Errors and warnings print as one line (ADR 0021/0026).

### Undo

An `UndoStack` per open document (in `DocumentSession`, ADR 0029).
`Edit: Send` has `label`, `apply`, `revert`, both over `&mut AppCtx`.
`Outcome::edit` carries the edit of a completed command; the dispatcher
pushes it after `execute` returns `Ok`. `push` truncates the redo tail and
clamps to a capacity (default 100). `undo`/`redo` call `revert`/`apply` and
return the edit label. Phase 4 commands are mostly file and view commands
and produce no edits; Phase 5 fills the stack. Undo of a command is its
`revert` (the Phase 5 property tests check `revert(apply(x)) = x`).

### The CLI move (ADR 0026)

`larrygram-cli`'s `Command` enum keeps its parser, exit codes and output.
`run` dispatches through `Registry::execute` with `Source::Cli` and the
argument names above; `file.inspect`/`file.render`/`file.export` are real
`Command` impls in `crates/commands`, so the CLI, the future control
channel and MCP share one implementation. `larrygram_commands::builtins::registry()`
is the one function that registers every implemented command; the app, the
CLI and the parity generator (ADR 0037) all use it.

## Consequences

- One dispatch table; adding a user action is one handler plus (when it has
  a Microsoft counterpart) extractor mapping rules.
- `commands` depends on `geom`, `model`, `render`, `vsdx`, `ui-spec` and
  `serde` (for `Settings`, ADR 0029). It stays below L5/L6: no egui.
- The JSON control channel (Phase 8) can run every command headlessly,
  including file commands, because input is expressed as `Args`.
- `Box::leak` interning is bounded and documented; the leak is per process
  and tiny (a few hundred short strings).

## Alternatives considered

- **Function-pointer table without a trait object.** Rejected: the trait
  carries the enabled-state predicate over `&AppCtx`, which a bare
  `fn` pointer cannot close over; stateful commands (Phase 5 tools) need
  the object anyway.
- **Commands return `Result` and take closures for input.** Rejected: a
  callback for "get me a path" would be a dialog opening in disguise and
  would not serialize over the Phase 8 JSON channel.
- **Undo as a command-internal concern.** Rejected: one shared stack with
  a uniform `Edit` interface is what Phase 5's "undo is the inverse"
  property tests need.
- **`String`-backed `CommandId`.** Rejected: ADR 0006 fixes `&'static str`;
  the interner is bounded and keeps comparisons cheap.
- **Hand-editing `commands.yaml` for our own ids.** Rejected by ADR 0006:
  the file is generated; extractor rules are the single source.
- **`clap`-style derive for CLI args staying out of the registry.**
  Rejected: ADR 0026 fixes the move onto the registry in Phase 4.
