# 0065: Phase 7 test strategy

- Status: accepted (2026-10-23, task P7-01); amended below
- Date: 2026-10-23
- Task: P7-01

## Context

Phase 7 is almost all UI and data. The risks are:

- data that does not load, or that references a missing command or
  icon;
- widgets that drop items or place them wrongly;
- clicks that dispatch the wrong command;
- contexts that show the wrong tabs;
- an icon pipeline that fails on wasm.

## Decision

1. **Data tests** (`crates/ui-spec`):
   - every layout validates against `layout-v2.schema.json`;
   - item ids are unique per layout;
   - every `command` exists in `commands.yaml`;
   - every `icon` key exists in `icons.yaml`, and every icon file
     exists;
   - every `contexts.yaml` command exists;
   - every theme token a widget reads exists in both themes;
   - the v1-to-v2 adapter output is snapshotted for 2016 and 365.
2. **Count pins**: an `insta` snapshot of items per surface and kind for
   each layout. Changing the data changes the snapshot in the same
   commit.
3. **Widget tests** (`egui_kittest`), for each layout:
   - open every menu, submenu, dropdown and context menu;
   - click every enabled item and assert the dispatched command;
   - assert that stubs are disabled with the Phase 4 tooltip;
   - access keys (Alt+letter);
   - View > Toolbars toggles;
   - for each context, the contextual tab set;
   - command-bar collapse;
   - File view navigation.
4. **Placement tests**: for the Panel Modern layout, the rendered rectangles
   of a group's controls respect `column`, `row` and `size` (same column
   gives the same x; rows increase downwards; large controls span the
   group height).
5. **Golden PNGs**: Larrygram renders each layout's window with a fixed
   document, using the egui_kittest wgpu snapshot and the same tolerance
   rule as ADR 0027. There are four images: Panel Classic with a menu open,
   Panel Modern on the Home tab, Panel Modern with Picture Tools, and the File view.
6. **Context cross-check** (ADR 0064): implemented predicates against
   `contexts.yaml`. Differences fail unless they are listed with a
   reason.
7. **Wasm**: `cargo build -p ferrogram-web --target
   wasm32-unknown-unknown` with the icon table embedded, plus one
   headless icon rasterisation test that also runs on wasm through
   `wasm-bindgen-test` if available. Otherwise the native test covers
   the same code.

## Consequences

- Most regressions show up as a failing count snapshot or a wrong
  dispatched command, without pixels.
- The golden PNGs change whenever the theme is tuned. Regenerate them
  with `FERROGRAM_UPDATE_GOLDEN=1`, as for the render goldens.

## Alternatives considered

- **Pixel tests only.** Rejected: they are brittle and do not explain a
  failure.
- **No golden images.** Rejected: the exit criteria include how the
  layouts look, and only images pin that.

## Amendments (task P7-01)

- The golden images also cover the welcome wizard's two pages and the
  About window (P7-10), as the Phase 7 plan requires.
- The first data tests are `crates/ui-spec/tests/panel_layouts.rs`
  (schema validation of both layouts, the v2 decode pin and the
  command cross-check that skips the `ui-spec/commands-phase7.yaml`
  ids until P7-02). The remaining data checks move to the loader in
  P7-03.
- The four golden images are named after the layouts (Panel Modern,
  Panel Classic); the phrase "command bar" replaces the draft's
  Microsoft term throughout.
