# Phase 7: Panel Classic and Panel Modern layouts

- Status: planned (follows Phase 6)
- Plan date: 2026-10-23
- ADRs: 0061 (layouts and format v2), 0062 (icons), 0063 (theme tokens), 0064 (selection contexts), 0065 (test strategy)
- Guides: `docs/ui/` (how each layout looks and behaves)

## Goal

Give Larrygram two complete, designed layouts on top of the existing
command registry:

- **Panel Classic**: a menu bar, docked toolbars and context menus.
- **Panel Modern**: tabs with sized and placed controls, contextual tabs that
  follow the selection, a Quick Access Toolbar and a File view.

Both draw open-licensed icons, use their own theme tokens, and grey out
commands according to the selection context. The two Fluent-data
layouts from Phase 4 (2016, 365) stay available. All four switch at run
time without a restart.

## Why now

Phase 4 built a working command bar straight from the published Fluent
control lists. Those lists say which controls exist, but not:

- their order;
- their size;
- how they stack inside a group;
- what their dropdowns contain;
- when they are available.

Every control also draws a placeholder letter. Phase 7 replaces that
with complete layout data, a real icon set and a theme, so that the app
looks and behaves like a finished diagramming tool. The registry,
commands and model do not change, apart from new command ids.

## What is in this package

Copy these into the repository at the same paths.

| Path | What it is |
|---|---|
| `ui-spec/layouts/panel-classic.yaml` | Panel Classic layout: 3 menu bars (drawing, ShapeSheet window, stencil editing), 21 toolbars, 30 context menus; 1,077 items |
| `ui-spec/layouts/panel-modern.yaml` | Panel Modern layout: QAT, 7 tabs, 3 contextual tabs, File view, 13 context menus; 463 items |
| `ui-spec/layout-v2.schema.json` | JSON Schema for layout format v2 (ADR 0061) |
| `ui-spec/contexts.yaml` | 10 selection contexts and, for 347 commands, the contexts where each is available (ADR 0064) |
| `ui-spec/icons.yaml` | Icon map: 394 icon keys to Tabler outline icons, with optional tints (ADR 0062) |
| `ui-spec/themes/panel-classic.yaml`, `ui-spec/themes/panel-modern.yaml` | Theme tokens: colours, fonts, metrics (ADR 0063) |
| `ui-spec/commands-phase7.yaml` | 170 command ids the layouts use that `commands.yaml` lacks, plus the 2 layout-switch ids (task P7-02 adds them) |
| `crates/ui/assets/icons/tabler/*.svg` | The 245 Tabler icons the map uses, plus `LICENSE` (MIT) |
| `docs/adr/0061`–`0065` | Draft ADRs, status `proposed` |
| `docs/ui/*.md` | Guides: layout format, Panel Classic, Panel Modern, contexts, icons, themes |
| `docs/tasks/P7-01.md`–`P7-14.md` | Draft task files |
| `docs/phase-7/attribution-and-sources.md` | Rows to add to `ATTRIBUTION.md` and `docs/sources.md` |
| `docs/phase-7/kickoff-section.md` | The new Phase 7 section for `KICKOFF.md`, and the renumbering |

## Scope

In:

- Layout format v2: types, loader, schema validation and cross-checks in
  `crates/ui-spec`.
- Converting the 2016 and 365 Fluent layouts to the v2 model on load,
  so one set of widgets renders every layout.
- Panel Classic widgets:
  - a menu bar with access keys;
  - menus with an icon gutter, shortcuts, separators, check marks and
    submenus;
  - docked toolbars with View > Toolbars;
  - toolbar combos, split buttons and palettes;
  - context menus.
- Panel Modern widgets:
  - tabs, plus groups laid out as `stack` or `rows`;
  - large, medium and small controls;
  - group launchers, split buttons, dropdowns, galleries and options;
  - contextual tabs, the QAT, a collapsible command bar and the File view.
- Run-time lists: recent files, windows, pages, colours, line weights,
  patterns and ends, and Larrygram's own themes and styles.
- Icons: the Tabler outline SVGs rasterised at 16 and 32 px with theme
  tints. Keys without a row fall back to the placeholder mark.
- Theme tokens applied per layout, with Selawik as the UI font.
- Selection contexts computed from the app state. Contextual tabs and
  context menus follow them.
- 170 new command ids for the controls, all starting as stubs, plus the two layout-switch commands. Fix eleven
  `commands.yaml` rows whose Fluent link is wrong (see Findings below).
- Layout switching between four layouts:
  - the registry commands `view.ui_layout.panel_classic` and
    `view.ui_layout.panel_modern`;
  - the CLI flag `--ui-layout panel-modern|panel-classic|panel-2016|panel-365`;
  - the Options dialog;
  - the persisted setting.

Out (later phases or never):

- Implementing the stub commands behind the new controls. They render
  disabled with "Not yet implemented", as in Phase 4.
- Toolbar and QAT customisation, toolbar drag-to-float, KeyTips.
- Live preview on gallery hover.
- Dialogs that the new launchers name but that do not exist yet.
- Everything already in `ui-spec/out-of-scope.txt`, including macros,
  add-ons and co-authoring.

## Exit criteria

1. All four layouts load and switch at run time without a restart:
   Panel Classic, Panel Modern, 2016 and 365. The default for new installs is
   Panel Modern.
2. **Panel Classic**:
   - the drawing menu bar shows its 10 menus, with access keys (Alt+F
     opens File);
   - Standard and Formatting are docked on rows 0 and 1;
   - View > Toolbars shows and hides each of the 16 toolbars;
   - the shape, page-tab and text-editing context menus open on
     right-click.
3. **Panel Modern**:
   - the 7 tabs render with every group's controls in their
     column/row slots;
   - the Picture Tools, Container Tools and ShapeSheet Tools tabs
     appear exactly in their contexts;
   - the QAT and the File view work.
4. Every item with an `icon` key draws its Tabler icon, never the
   placeholder. The icon count per layout is pinned by a test.
5. Every actionable item dispatches its `command` through the registry:
   - implemented commands follow their enabled predicate;
   - stub commands render disabled with the Phase 4 tooltip.
6. For implemented commands, the enabled predicates agree with
   `ui-spec/contexts.yaml` in the reliable contexts. Each difference is
   either fixed or listed in the phase report.
7. The full gate is green on `main`, including wasm. The phase report
   includes PNG renders of both new layouts made by Larrygram itself
   (golden tests).

## Task plan

| Task | Role | Depends on | Group | Summary |
|---|---|---|---|---|
| P7-01 | architect | Phase 6 | 0 | Accept ADRs 0061–0065, commit the data files, write stubs, finalise task files |
| P7-02 | implementer | P7-01 | A | Commands: add the 170 ids and the 2 layout-switch ids to `OUR_COMMANDS`; fix the 11 Fluent rows; regenerate `commands.yaml` and parity |
| P7-03 | implementer | P7-01 | A | `crates/ui-spec`: v2 types, loader, schema check, cross-checks, embedding, `LayoutKind::{PanelClassic, PanelModern}`, v1→v2 adapter; types for themes, icons and contexts |
| P7-04 | implementer | P7-01 | A | Icons: resvg rasteriser, icon cache with tints, embedded SVG table (codegen), placeholder fallback |
| P7-05 | implementer | P7-03 | B | Theme: tokens to egui `Style`/`Visuals`, Selawik UI font, per-layout theme switch |
| P7-06 | implementer | P7-03, P7-04 | B | Panel Classic widgets: menu bar, menus, toolbars, context menus, run-time lists |
| P7-07 | implementer | P7-03, P7-04 | B | Panel Modern widgets: tabs, groups, sizes, launchers, galleries, contextual tabs, QAT, File view; v1 layouts through the adapter |
| P7-08 | implementer | P7-02, P7-03 | B | Selection contexts: compute from `AppCtx`; contextual tab and context-menu selection; predicate cross-check test |
| P7-09 | implementer | P7-05..P7-08 | C | Shell: four-way layout switch, CLI, settings default, window regions, status bar and page tabs per layout |
| P7-10 | test-engineer | P7-09 | D | egui_kittest coverage of both layouts, golden PNGs, wasm |
| P7-11 | reviewer | P7-10 | E | Code review |
| P7-12 | coverage-auditor | P7-10 | E | Coverage audit, ratchet proposal |
| P7-13 | cleanroom-auditor | P7-10 | E | Assets, licences, attribution, labels |
| P7-14 | implementer | P7-11..P7-13 | F | Fixes from the audits |

Tasks in one group run in parallel on disjoint files. Shared files:

- `crates/ui/src/lib.rs` and `crates/ui/Cargo.toml` belong to P7-04.
  Other tasks request lines in their Results.
- `crates/ui-spec/src/lib.rs` belongs to P7-03.
- `xtask/src/extract/commands.rs` belongs to P7-02.
- The master resolves `Cargo.lock`.

## Findings to fix (P7-02)

These `commands.yaml` rows join a Fluent id to the wrong command. The
layouts already use the right command. The extractor rules must agree,
or the next `cargo xtask extract` brings the wrong joins back.

| Fluent id | Now joined to | Should be |
|---|---|---|
| `AlignLeft` | `arrange.align.left` | `format.text.align_left` (paragraph alignment) |
| `AlignCenter` | `arrange.align.center` | `format.text.align_center` |
| `AlignRight` | `arrange.align.right` | `format.text.align_right` |
| `ObjectsGroup`, `ObjectsGroupMenu` | `arrange.size.objects` | `arrange.group` |
| `ObjectsUngroup` | `tools.objects.ungroup` | `arrange.ungroup` |
| `ContainerGallery` | `arrange.disband.container` | `insert.add.to.new.container` |
| `ShapeSheetShowValues`, `ShapeSheetShowFormulas`, `ShapeSheetShowSections`, `ShowShapeSheet` | `view.show.shape.sheet.shape` | `edit.sw.show.values`, `edit.sw.show.formulas`, `edit.sw.show.sections.dlg`, `view.window.show.shape.sheet` |
| `TextBoxInsertVertical` | `insert.text.box` | `insert.vert.text.box` |
| `ResearchPane` | `help.research.thesaurus` | `view.task.pane.research` |
| `ConnectorsGallery` | `edit.delete.connectors` | `connector.style` |
| `ShapeData` | `data.unlink.shape` | `format.cust.prop.edit` |

The pattern behind most of these is one command row holding several
Fluent ids, or a label-inferred `visuicmd`. P7-02 should check the
other multi-id rows for the same problem.

## Known gaps

- **Two contexts are `pending`** in `contexts.yaml`: `shape_selected`
  and `connector_selected` have no availability data yet. Until they
  do, follow the rule in the file's notes. The data can be added later
  without changing the format.
- **Theme tokens marked `provisional`**: the hover colours and tooltip
  borders are design choices still to confirm.
- **Run-time lists with Larrygram content**: `themes`,
  `theme_variants`, `backgrounds`, `borders_and_titles`,
  `shape_styles`, `effects` and the preset lists. Phase 7 lists what
  Larrygram has, which may be nothing yet. An empty list shows a
  disabled "(none)" row.
- **Launchers and items whose dialog does not exist** stay stubs.
