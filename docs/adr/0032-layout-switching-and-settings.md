# 0032: Layout switching and settings persistence

- Status: accepted
- Date: 2026-10-16
- Task: P4-01

## Context

KICKOFF Phase 4: layout switching (2016 <-> 365) is available three ways
and both layouts bind to the same command registry:

1. from View/Options at runtime, with no restart;
2. from the CLI flag `--ui-layout`;
3. as a persisted setting.

The exit criterion is "both layouts load and switch without a restart".
`ui_spec::LayoutKind` and `LayoutSets::get` are the existing seam
(ADR 0007); `Settings` and `SettingsStore` live in `crates/commands`
(ADR 0029).

## Decision

### The setting

`Settings::ui_layout: ui_spec::LayoutKind` (`Visio2016` | `Visio365`).
The default is **`Visio2016`**: the classic ribbon arrangement most Visio
users know, and the smaller layout (7,511 vs 9,985 YAML lines).

### Switching at runtime

Two registry commands dispatch the switch like any other command:

- `view.ui_layout.visio_2016`
- `view.ui_layout.visio_365`

(Ids are Ferrogram-only; ADR 0028's extractor rule adds them to
`commands.yaml` with `mapping_confidence: unknown`.) Each is always
enabled; executing one sets `Settings::ui_layout` and returns
`Outcome { message: Some("Ribbon layout: ..."), .. }`. The shell rebuilds
its ribbon widget from `LayoutSets::get(kind)` on the next frame. No
restart, no widget state beyond the active tab index (which resets to the
first tab when the id set changes). The same two commands appear in the
View/Options dialog (`ui::settings::show_options`) as a radio choice; the
dialog edits `Settings` and marks the store dirty.

**Amendment (P4-11 review, fix task P4-16).** The dialog does not
mutate `Settings` itself. `show_options` returns the `Intent`s for the
rows the user changed and the shell dispatches them through the
registry like any other control: the layout radios dispatch
`view.ui_layout.visio_2016` / `view.ui_layout.visio_365`, the grid and
rulers checkboxes dispatch `view.toggle_grid` / `view.rulers`, and the
units combo dispatches the Ferrogram-only `view.units` with a `units`
argument. The dialog itself opens through `app.office.center.options`,
whose handler returns `UiRequest::ShowOptions` (no shell intercept).
`Settings::show_grid` and `Settings::show_rulers` are the defaults for
new document sessions: `file.new`/`file.open` seed `ViewState` from
them, and the two toggle commands update both the active `ViewState`
and the `Settings` default.

`--ui-layout 2016|365` on the CLI sets `Settings::ui_layout` before the
first frame and overrides the persisted value (CLI wins over the store;
the store wins only when the flag is absent).

### Persistence

`SettingsStore` (ADR 0029) is implemented by the apps over `eframe::Storage`
(eframe's `persistence` feature): `eframe::App::save` writes
`settings.json` (serialized `Settings`, stable key order) into eframe's
storage slot, and startup loads it. eframe maps the slot to the platform
config directory on desktop and to `localStorage` on wasm, so one
implementation covers both targets and no new dependency is needed.

Load failures are not fatal: a corrupt or missing slot yields
`Settings::default()` with one warning on `AppCtx::warnings` (ADR 0021
rule: bad data substitutes a default and warns, never crashes).

## Consequences

- One switch path (registry) serves the dialog, the CLI, and later the
  JSON control channel and MCP (`view.ui_layout.*` is dispatchable there
  for automated screenshots of both layouts).
- eframe's storage keeps settings out of our own config-file plumbing.
- The persisted setting survives restarts on desktop and in the browser.

## Alternatives considered

- **Two layouts in two windows / a restart flag.** Rejected: KICKOFF
  requires switching without a restart.
- **A config file path of our own (directories crate + JSON).** Rejected
  for Phase 4: eframe already persists per platform, including wasm
  (`localStorage`), where a file path is meaningless.
- **One `view.ui_layout` command with a `kind` argument.** Rejected: two
  ids are discoverable in the registry listing and map 1:1 to UI radio
  buttons and CLI values; an argument adds validation surface for no gain.
  (Phase 8 can still dispatch both.)
- **Default 365.** Rejected: 2016 is the classic arrangement and the
  simpler starting point; the default is one field either way.
- **Store the layout kind in the document.** Rejected: it is a user
  preference, not document data.
