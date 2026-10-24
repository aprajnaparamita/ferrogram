# 0061: Panel Classic and Panel Modern layouts, layout format v2

- Status: accepted (2026-10-23, task P7-01); amended below
- Date: 2026-10-23
- Task: P7-01

## Context

Phase 4 renders the command bar from the two Fluent-data layouts
(`panel-2016.yaml`, `panel-365.yaml`; ADRs 0007, 0031). That data lists
tabs, groups and controls. It does not say:

- the order of controls inside a group, or how they stack;
- control sizes;
- what a dropdown or split button contains;
- which controls draw an icon;
- which contexts make a tab appear.

The v1 schema reserved `bounds`, `size_variant` and `icon_ref` for
enrichment, but those fields cannot express menus, toolbars, run-time
lists or contextual tabs.

Phase 7 adds two complete layouts: **Panel Classic** (menu bar, toolbars,
context menus) and **Panel Modern** (tabs with placed controls, contextual
tabs, QAT, File view). They need one format that both widget sets read,
and that the Fluent layouts can be converted into.

## Decision

### Files

- `ui-spec/layouts/panel-classic.yaml` and `ui-spec/layouts/panel-modern.yaml`,
  validated by `ui-spec/layout-v2.schema.json`.
- `ui-spec/contexts.yaml` (ADR 0064), `ui-spec/icons.yaml` (ADR 0062)
  and `ui-spec/themes/*.yaml` (ADR 0063) are shared by all layouts.
- The v1 files stay as they are. `crates/ui-spec` converts a v1
  `Layout` into the v2 model on load (see "Adapter" below). The widgets
  only see v2.

### Shape of a v2 layout

Top level:

- `schema_version: 2`, `id`, `name`, `kind` (`panel-classic` | `panel-modern`),
  `theme`;
- `window` (frame regions in order: title bar, menu bar or tab strip,
  toolbars or command bar, Shapes window, rulers, drawing, page tabs, status
  bar, each with a nominal size);
- `context_menus`.

`kind: panel-classic` adds:

- `menu_bars`: one per window mode (drawing, ShapeSheet window, stencil
  editing). A mode-specific bar has `shown_in`.
- `toolbars`, each with `visible` (shown by default), `dock`, `row` and
  optional `shown_in`.

`kind: panel-modern` adds:

- `quick_access_toolbar` and `file_button`;
- `tabs` and `contextual_tabs` (with `tab_set` and `shown_in`);
- `file_view` (nav entries that are either pages or commands).

Panel Modern groups have `layout: stack | rows` and an optional `launcher`:

- `stack`: large controls take a column; others stack up to `rows`
  (default 3) per column.
- `rows`: two rows that flow left to right.

**Item**: the one recursive control record.

- Required: `id` (unique dotted path in the layout) and `kind`.
- `kind` is one of: `button`, `toggle`, `check`, `split`,
  `split_recent`, `dropdown`, `submenu`, `gallery`, `combo`, `palette`,
  `launcher`, `heading`, `list` or `options`.
- Optional fields:
  - `label`, `access` (access-key letter), `shortcut` (display text);
  - `command` (registry id) and `new_command` (the id is a Phase 7
    proposal);
  - `fluent` and `visuicmd` (cross-reference only);
  - `icon` (key into `icons.yaml`) and `display` (toolbar: icon, text
    or both);
  - `size`, `column`, `row` and `width` (command bar placement);
  - `begin_group` (a separator before the item);
  - `hidden` (part of the layout but not shown by default, for example
    reserved for a context or a later customisation);
  - `dynamic_label` (the label changes at run time, for example
    "Undo <action>");
  - `list` (a run-time list the app fills: `recent_files`, `windows`,
    `pages`, `colors`, `line_weights`, `themes` and so on);
  - `options` (a fixed list of choices);
  - `items` (children).

Labels are short UI labels (AGENTS.md section 4). Tooltips are written
by us, not stored in the layout.

### Identity and commands

- An item's `id` identifies the placement; its `command` identifies the
  action. As in ADR 0031, many placements share one command.
- `command` must exist in `commands.yaml`. The loader cross-checks it,
  as v1 does. Ids that Phase 7 introduces are in
  `ui-spec/commands-phase7.yaml` until task P7-02 merges them.
- Every actionable kind needs a `command`. `submenu`, `heading`, `list`
  and `options` do not.

### LayoutKind and switching

- `LayoutKind` (the v2 one, `ui_spec::v2::LayoutKind`) has the four
  variants `PanelModern`, `PanelClassic`, `Panel2016` and `Panel365`.
  The registry gains `view.ui_layout.panel_classic` and
  `view.ui_layout.panel_modern`.
- `--ui-layout` accepts `panel-modern`, `panel-classic`, `panel-2016`
  and `panel-365`.
- New installs default to `PanelModern`. A persisted pre-Phase 7
  setting is kept (P7-00 renames the stored values).
- Switching rebuilds the widgets on the next frame, as in ADR 0032.

### Adapter for v1 layouts

`ui_spec::v2::from_v1(&Layout) -> LayoutV2` builds a panel-modern
layout:

- Each v1 tab becomes a tab, and each group becomes a `stack` group with
  no slot data, so controls flow in file order.
- Galleries, menus and dropdowns become `dropdown` items whose children
  come from the v1 parent chain.
- The backstage becomes the `file_view`.
- Context menus are carried over.

The output of the adapter is pinned with a snapshot test.

### Embedding

The new YAML files are embedded with `include_str!` (ADR 0031) and
parsed by the same loader path the tests use on disk. Schema validation
runs in tests and at extract time, not on every start.

## Consequences

- One widget set per surface (menus, toolbars, command bar). The Fluent
  layouts keep working through the adapter.
- The layout files are hand-curated data, not extractor output.
  Changes are reviewed like code. `cargo xtask codegen --check` does not
  regenerate them.
- The parity matrix (`docs/parity/controls.md`) gains a column per
  layout. Every placement still lists its command and status.
- The embedded data grows by about 450 KB of YAML. Parsing it once at
  start is negligible.

## Alternatives considered

- **Extend the v1 schema.** Rejected: v1 is keyed by Fluent id and has
  no notion of menus, toolbars, run-time lists or contextual
  visibility. Retrofitting them would make every v1 field optional.
- **Generate Rust from the YAML.** Rejected for the reasons in ADR 0031.
- **Separate formats for Panel Classic and Panel Modern.** Rejected: menus,
  dropdowns, split buttons and context menus are the same item tree in
  both. Only the containers differ, and `kind` selects them.
- **Store tooltips in the layout.** Rejected: tooltip text is ours, and
  it belongs with the command, not with each placement.

## Amendments (task P7-01)

The draft names the two layouts with the Microsoft term for a tabbed
command bar. That term is a Microsoft feature name and is not used in
any doc, UI text, file name or identifier; the user renamed the
layouts. The amended decision:

- **Panel Modern** (`panel-modern`) is the 2013 data (the draft's
  tabbed layout), and **Panel Classic** (`panel-classic`) the 2007
  data. The Phase 4 layouts are **Panel (2016 data)** (`panel-2016`)
  and **Panel (365 data)** (`panel-365`).
- The layout files are `ui-spec/layouts/panel-modern.yaml` and
  `panel-classic.yaml`; the themes are
  `ui-spec/themes/panel-modern.yaml` and `panel-classic.yaml`. The
  `kind` values are `panel-modern` and `panel-classic`, and the
  `token`/`itemId` patterns in `layout-v2.schema.json` allow `-`.
- `LayoutKind` is `ui_spec::v2::LayoutKind` with `PanelModern`,
  `PanelClassic`, `Panel2016` and `Panel365` (ids and `file_name`s
  `pane-*.yaml`). The v1 `ui_spec::LayoutKind` is renamed away in
  task P7-00. `view.ui_layout.panel_classic` and
  `view.ui_layout.panel_modern` are the new registry ids; P7-00
  renames the two Phase 4 ids and the CLI values
  (`--ui-layout panel-modern|panel-classic|panel-2016|panel-365`).
- `ui_spec::v2::from_v1` returns `Result<LayoutV2, V2Error>` (the stub
  signature); `V2Error` is the v2 error type until P7-03 folds the
  loader errors in.
- The Panel Modern command-bar window region is `command_bar`, and the
  theme metrics are `command_bar.height` and `command_bar.padding_top`.
- Access-key letters are always quoted in the YAML (`access: "f"`):
  an unquoted `f` or `n` parses as a boolean. The schema description
  says so.
- Menu entries under `menu_bars[].menus` have no `kind` (they are menu
  titles), and `file_view.nav` entries have `command` optional, as the
  schema already says. The v2 types mirror the schema.
- Two Phase 1 command ids carried the old term (derived from
  `visCmdCloseInkToolsRibbonTab` and `visCmdMinimizeRibbonToggle`);
  the layouts and `contexts.yaml` use `file.close.ink.tools.tab` and
  `app.minimize.command_bar.toggle`, and P7-02 changes the extractor
  rules to match.
- The Panel Classic Help menus carry **Welcome** (`help.welcome`) and
  **About Larrygram** (`help.about`) as visible items (ADR 0066); both
  ids are Phase 7 proposals in `ui-spec/commands-phase7.yaml` until
  P7-02 merges them.
- The Panel Modern layout is the default for new installs.
