# 0029: Application state

- Status: accepted
- Date: 2026-10-16
- Task: P4-01

## Context

Phase 4 needs shared application state: open documents, the active page,
the selection (empty until Phase 5), view state (pan, zoom) and settings
(the Panel layout kind, grid and rulers toggles, display units). The
layering (ADR 0002) and AGENTS.md section 6 require that headless callers
(the CLI today, the JSON control channel and MCP in Phase 8) can run the
same commands over the same state, so the state cannot live inside the
egui app. The question is what lives in `crates/commands` (L4) and what
stays in `crates/ui` (L6).

## Decision

### In `crates/commands` (L4): everything a command mutates

```rust
pub struct AppCtx {
    pub documents: Vec<DocumentSession>,   // open documents, in open order
    pub active: Option<usize>,             // index into `documents`
    pub selection: Selection,              // empty in Phase 4
    pub settings: Settings,
    pub warnings: Vec<String>,             // recent warnings, capped ring
}

pub struct DocumentSession {
    pub document: larrygram_model::Document,
    pub path: Option<PathBuf>,             // None for never-saved documents
    pub dirty: bool,
    pub active_page: Option<PageId>,       // the page shown on the canvas
    pub view: ViewState,
    pub undo: UndoStack,                   // per document (ADR 0028)
}

pub struct ViewState {
    pub pan: geom::Point,                  // page-space offset, inches
    pub zoom: f64,                         // view scale multiplier (1.0 = 100%), > 0
    pub show_grid: bool,
    pub show_rulers: bool,
}

pub struct Selection;                      // Phase 5 adds shape ids

pub struct Settings {
    pub ui_layout: ui_spec::LayoutKind,    // 2016 or 365 (ADR 0032)
    pub show_grid: bool,
    pub show_rulers: bool,
    pub units: DisplayUnits,
    pub recent_files: Vec<PathBuf>,
}

pub enum DisplayUnits { Inches, Millimeters, Centimeters, Meters, Feet }
```

`AppCtx` is plain data with trivial constructors and accessors
(`active_document`, `active_document_mut`, `open_document`,
`close_document`, `record_warning`). Commands receive `&AppCtx` for
`enabled` and `&mut AppCtx` for `execute`; nothing else mutates the state.

Units rule (AGENTS.md section 6): `Document` values stay in inches.
`ViewState::pan` is in inches. `DisplayUnits` affects only presentation:
the Size & Position pane and the status bar format numbers, the ruler
labels convert at the edge.

`Selection` is an opaque empty type in Phase 4 (it exists so Phase 5 can
add fields without touching any signature). `Selection::len` is 0 and
`is_empty` is true until then.

### Persistence seam

```rust
pub trait SettingsStore: Send {
    fn load(&mut self) -> Result<Settings, SettingsError>;
    fn save(&mut self, settings: &Settings) -> Result<(), SettingsError>;
}
```

`Settings` derives `serde::Serialize`/`Deserialize` (the only serde use in
`commands`). The desktop and web apps implement `SettingsStore` over
`eframe::Storage` (ADR 0032); the CLI never persists settings.

### In `crates/ui` (L6): everything egui

Widget-local state stays in `crates/ui`: the command bar's active tab and
backstage flag, the canvas texture cache, pane collapse flags, the
shortcut chord-in-progress buffer, and the egui `Context` itself. UI code
translates between egui input and `Intent` values (ADR 0028) and never
mutates `AppCtx` directly; the shell calls `Registry::execute`.

## Consequences

- The Phase 8 control channel constructs an `AppCtx`, runs commands and
  reads the state; no GUI types appear in L4.
- Per-document undo keeps a command on one document local; cross-document
  undo is out of scope.
- `commands` depends on `serde` and `ui-spec` (`LayoutKind`), `geom`
  (`Point`), `model` (`Document`, `PageId`).
- Warning history is capped (last 200) so a malformed file cannot grow
  memory without bound.

## Alternatives considered

- **State in `crates/ui` with commands taking explicit parameters.**
  Rejected: every caller would reconstruct state plumbing, and the Phase 8
  channel would need a parallel state type.
- **A global `static` state singleton.** Rejected: hidden mutable state
  makes tests order-dependent and Phase 8 concurrency harder.
- **One `UndoStack` for the whole app.** Rejected: closing one document
  would keep edits to another undoable.
- **`Settings` serialized by the apps only.** Rejected: `SettingsStore` is
  the seam Phase 8 needs for "get/set option" commands; the format belongs
  with the data.
- **Millimeters in the model, inches at the edge.** Rejected: AGENTS.md
  section 6 fixes inches as internal units.
