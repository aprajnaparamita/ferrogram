# Phase 7: Panel Modern and Panel Classic layouts, the Larrygram rename and the welcome wizard

- Status: planned (runs beside the Phase 6 tail; the rename boundary waits for the `phase-6` tag)
- Plan date: 2026-10-23
- ADRs: 0061 (layouts and format v2), 0062 (icons), 0063 (theme
  tokens), 0064 (selection contexts), 0065 (test strategy), 0066
  (welcome wizard and About window), 0067 (brand assets)
- Guides: `docs/ui/` (how each layout looks and behaves)

## Goal

Give Larrygram two complete, designed layouts on top of the existing
command registry:

- **Panel Classic**: a menu bar, docked toolbars and context menus.
- **Panel Modern**: tabs with sized and placed controls, contextual
  tabs that follow the selection, a Quick Access Toolbar and a File
  view.

Both draw open-licensed icons, use their own theme tokens, and grey
out commands according to the selection context. The two Fluent-data
layouts from Phase 4 become **Panel (2016 data)** and **Panel (365
data)** and stay available. All four switch at run time without a
restart.

Phase 7 also renames the app to **Larrygram** (P7-00, at the rename
boundary), ships the welcome wizard and the About window (P7-15,
P7-16) and the brand assets (ADR 0067).

## Why now

Phase 4 built a working command bar straight from the published
Fluent control lists. Those lists say which controls exist, but not:

- their order;
- their size;
- how they stack inside a group;
- what their dropdowns contain;
- when they are available.

Every control also draws a placeholder letter. Phase 7 replaces that
with complete layout data, a real icon set and a theme, so that the
app looks and behaves like a finished diagramming tool. The registry,
commands and model do not change, apart from new command ids.

## What the package brought in (task P7-01)

| Path | What it is |
|---|---|
| `ui-spec/layouts/panel-classic.yaml` | Panel Classic layout: 3 menu bars (drawing, ShapeSheet window, stencil editing), 21 toolbars, 30 context menus; 1,077 items |
| `ui-spec/layouts/panel-modern.yaml` | Panel Modern layout: QAT, 7 tabs, 3 contextual tabs, File view, 13 context menus; 463 items |
| `ui-spec/layout-v2.schema.json` | JSON Schema for layout format v2 (ADR 0061) |
| `ui-spec/contexts.yaml` | 10 selection contexts and, for 347 commands, the contexts where each is available (ADR 0064) |
| `ui-spec/icons.yaml` | Icon map: 394 icon keys to Tabler outline icons, with optional tints (ADR 0062) |
| `ui-spec/themes/panel-classic.yaml`, `ui-spec/themes/panel-modern.yaml` | Theme tokens: colours, fonts, metrics (ADR 0063) |
| `ui-spec/commands-phase7.yaml` | The command ids the layouts use that `commands.yaml` lacks (176 rows: 170 control ids, the 2 layout-switch ids, 2 renamed ids and the 2 Help ids; task P7-02 moves them into `OUR_COMMANDS` and deletes the file) |
| `crates/ui/assets/icons/tabler/*.svg` | The 245 Tabler icons the map uses, plus `LICENSE` (MIT) |
| `crates/ui/assets/brand/*` | The brand assets (ADR 0067): `about_screen.svg`, `panel-layout.svg`, `classic-layout.svg`, `icon.svg`, `larry.png` |
| `docs/adr/0061`–`0067` | The accepted decision records |
| `docs/ui/*.md` | Guides: layout format, Panel Classic, Panel Modern, contexts, icons, themes |
| `docs/tasks/P7-00.md`–`P7-16.md` | The task files |

The layout and theme data is "Larrygram layout and theme data,
compiled by the project" (`ATTRIBUTION.md`); nothing records how it
was gathered.

## Scope

In:

- Layout format v2: types, loader, schema validation and cross-checks
  in `crates/ui-spec`.
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
  - contextual tabs, the QAT, a collapsible command bar and the File
    view.
- Run-time lists: recent files, windows, pages, colours, line weights,
  patterns and ends, and Larrygram's own themes and styles.
- Icons: the Tabler outline SVGs rasterised at 16 and 32 px with theme
  tints. Keys without a row fall back to the placeholder mark.
- Theme tokens applied per layout, with Selawik as the UI font.
- Selection contexts computed from the app state. Contextual tabs and
  context menus follow them.
- New command ids for the controls, all starting as stubs, plus the
  two layout-switch commands. Fix eleven `commands.yaml` rows whose
  Fluent link is wrong (see Findings below).
- Layout switching between four layouts:
  - the registry commands `view.ui_layout.panel_classic` and
    `view.ui_layout.panel_modern`;
  - the CLI flag
    `--ui-layout panel-modern|panel-classic|panel-2016|panel-365`;
  - the Options dialog;
  - the persisted setting.
- The welcome wizard and the About window (ADR 0066), the brand
  assets (ADR 0067) and the app icon.
- The Larrygram rename at the boundary (P7-00): crates, binaries,
  environment variables, docs, the settings storage key, the web page
  title and the version (`1.0.0-alpha.1`, "Larrygram v1.0 alpha").

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
   Panel Modern, Panel Classic, Panel (2016 data) and Panel (365
   data). The default for new installs is Panel Modern.
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
5. Every actionable item dispatches its `command` through the
   registry:
   - implemented commands follow their enabled predicate;
   - stub commands render disabled with the Phase 4 tooltip.
6. For implemented commands, the enabled predicates agree with
   `ui-spec/contexts.yaml` in the reliable contexts. Each difference
   is either fixed or listed in the phase report.
7. A fresh install opens the wizard. Start applies the chosen layout
   without a restart, and the next launch skips the wizard.
8. **Why Larrygram?** and **Help > About Larrygram** open the same
   About window, and its button opens the video link.
9. No doc, UI text, file name or identifier says "Ferrogram" or the
   old layout term, except verbatim third-party identifiers. A gate
   check enforces this from P7-00 on.
10. The brand assets have their `ATTRIBUTION.md` rows and pass the
    clean-room audit.
11. The full gate is green on `main`, including wasm. The phase report
    includes PNG renders of both new layouts, the two wizard pages and
    the About window, made by Larrygram itself (golden tests).

## Order of work

```
now ─┬─ Phase 6 tail: P6-09, P6-10, P6-15, P6-16 ─ P6-11 ─ P6-12/13/14 ─ tag phase-6
     └─ P7-01 architect (docs, data, stubs) ─────────── merge before the boundary
boundary (nothing else running):
     R1 master: remove the earlier layout records from the tree
     P7-00 worker: rename to Larrygram, the old layout term to Panel, side panels to panes
     R4 master: history rewrite (after "go"), move the folders, re-link
Phase 7 groups:
     A: P7-02  P7-03  P7-04                      (3 in parallel)
     B: P7-05  P7-06  P7-07  P7-08  P7-15  P7-16  (6 in parallel)
     C: P7-09 shell integration
     D: P7-10 tests and golden images
     E: P7-11  P7-12  P7-13                      (3 in parallel)
     F: P7-14 fixes, phase report, tag phase-7
```

## Task plan

| Task | Role | Depends on | Group | Summary |
|---|---|---|---|---|
| P7-01 | architect | none (runs beside the Phase 6 tail) | 0 | Accept ADRs 0061–0067, import the package and the brand assets, write the stubs, finalise the task files |
| P7-00 | implementer | P7-01, `phase-6` tag | boundary | The rename: Ferrogram to Larrygram, the old layout term to Panel, side panels to panes, the 2016/365 layouts to Panel (2016 data) and Panel (365 data) |
| P7-02 | implementer | P7-01 | A | Commands: add the ids to `OUR_COMMANDS`; fix the 11 Fluent rows; regenerate `commands.yaml` and parity |
| P7-03 | implementer | P7-01 | A | `crates/ui-spec`: v2 types, loader, schema check, cross-checks, embedding, `LayoutKind`, v1→v2 adapter; types for themes, icons and contexts |
| P7-04 | implementer | P7-01 | A | Icons: resvg rasteriser, icon cache with tints, embedded SVG table (codegen), placeholder fallback, image loaders for the brand assets |
| P7-05 | implementer | P7-03 | B | Theme: tokens to egui `Style`/`Visuals`, Selawik UI font, per-layout theme switch |
| P7-06 | implementer | P7-03, P7-04 | B | Panel Classic widgets: menu bar, menus, toolbars, context menus, run-time lists |
| P7-07 | implementer | P7-03, P7-04 | B | Panel Modern widgets: tabs, groups, sizes, launchers, galleries, contextual tabs, QAT, File view; v1 layouts through the adapter |
| P7-08 | implementer | P7-02, P7-03 | B | Selection contexts: compute from `AppCtx`; contextual tab and context-menu selection; predicate cross-check test |
| P7-15 | implementer | P7-03 | B | The welcome wizard (both pages, the layout buttons, the settings writes) |
| P7-16 | implementer | P7-03, P7-04 | B | The About window and the app icon |
| P7-09 | implementer | P7-05..P7-08, P7-15, P7-16 | C | Shell: four-way layout switch, CLI, settings default and first-run flag, window regions, Help menu wiring |
| P7-10 | test-engineer | P7-09 | D | egui_kittest coverage of both layouts, the wizard and the About window, golden PNGs, wasm |
| P7-11 | reviewer | P7-10 | E | Code review |
| P7-12 | coverage-auditor | P7-10 | E | Coverage audit, ratchet proposal |
| P7-13 | cleanroom-auditor | P7-10 | E | Assets, licences, attribution, labels |
| P7-14 | implementer | P7-11..P7-13 | F | Fixes from the audits, the phase report, the `phase-7` tag |

Tasks in one group run in parallel on disjoint files. Shared files:

- `crates/ui/src/lib.rs` and `crates/ui/Cargo.toml` belong to P7-04.
  Other tasks request lines in their Results.
- `crates/ui-spec/src/lib.rs` belongs to P7-03.
- `xtask/src/extract/commands.rs` belongs to P7-02.
- `crates/ui/src/settings.rs` and `crates/ui/src/shell.rs` belong to
  P7-09; P7-15 and P7-16 use their APIs only.
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
- **For the user to confirm** (the plan's section 3): the 360 x 360
  pixel resolution of `larry.png`, and the "Watch Larry's story"
  button label with the short wizard description.
