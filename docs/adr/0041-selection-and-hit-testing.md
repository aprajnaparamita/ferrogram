# 0041: Selection and hit-testing

- Status: accepted
- Date: 2026-10-18
- Task: P5-01

## Context

Phase 5 selects shapes to edit them. ADR 0029 parked `Selection` in
`AppCtx` as an empty type; ADR 0033 records the Phase 5 hit-test upgrade
path (`render::hit_test`). The planning brief (decision 4) asks where the
selection lives, how hit-testing works against the scene (filled and
unfilled shapes, lines, text, groups, tolerance in screen pixels), how
the marquee and multi-select work, and fixes the P4-14 item that
keyboard-only focus moves do not change the shortcut scope.

The scene (ADR 0020) is page inches with node-level transforms; hit
results must map back to model shapes, so the scene must carry identity.

## Decision

### 1. The selection is `AppCtx::selection`

`Selection { shapes: Vec<ShapeId>, anchor: Option<ShapeId>, page:
Option<PageId> }`: selected shapes of the active page in click order,
the **anchor** (last clicked, the panes' subject) and the page they
belong to. It is per application (as ADR 0029 placed it); the commands
that switch document or page clear it. Commands read it for their
subject; `enabled` predicates gate on `!selection.is_empty()`.

### 2. Hit-testing is in `larrygram-render::hit` (L3)

The scene nodes gain `shape_id: Option<ShapeId>` (an ADR 0020 amendment:
a synthesised node like a shadow body has `None`). The API:
`hit_test(scene, point, HitOptions { tolerance }) -> Vec<Hit>` sorted
topmost-first with `Hit { node: NodePath, shape_id, kind, distance }`,
plus `hit_marquee`, `node_bounds` and `scene_bounds`.

Rules (our own, to be compared with Visio in the screenshot pass):

- **Filled shape**: hit inside the fill (`HitKind::Fill`) or within
  tolerance of the outline (`Stroke`).
- **Unfilled shape or line**: outline and text only.
- **Text**: the text block bounds count as hit.
- **Groups**: not hit directly; a hit on a member maps to the top-level
  ancestor shape (`Document` parent chain). Double-click enters a group
  (`shape.enter_group`/`shape.exit_group`, P5-04) and then members select
  individually.
- **Z-order**: the first result is the topmost (page shape order, ADR
  0039).

The tolerance is expressed in **screen pixels** by the caller
(`tolerance = px / (zoom * 96)`), so hit feel does not change with zoom.
The default is 4 px; handles use their own 6 px (`ui::handles`).

### 3. Marquee and multi-select

`hit_marquee(scene, rect)` returns shapes whose **transformed bounds
intersect** the rectangle (intersection, not containment; a black-box
check is on the screenshot-pass list). Click toggles with Shift (add or
remove), a plain click replaces, Ctrl-click adds. The marquee replaces
the selection when it starts on empty space and adds to it with Shift.

### 4. Focus scope follows real focus (P4-14 fix)

`ActiveScopes` gains `Selection` when the selection is non-empty, and
`crates/ui::shell` reads keyboard focus from egui's focus state (the
focused widget id), not from the previous frame's pointer press. A
keyboard-only focus move (Tab, the command bar, the canvas) therefore changes
the active shortcut scopes (ADR 0036). Owner: P5-05.

## Consequences

- One hit-test implementation serves selection, handles, glue candidates
  and the marquee; PNG goldens cover its geometry indirectly.
- The scene carries model identity, so the UI never re-implements the
  shape walk; `NodePath` also names overlay targets.
- Groups select as units in Phase 5; sub-shape selection needs the
  enter-group commands and is in scope for P5-04.
- The marquee rule (intersection) may differ from Visio; recorded as a
  black-box check, not a guess we hide.

## Alternatives considered

- **Hit-test in `crates/ui` over the raster.** Rejected: pixel readback
  is slow and wrong for unfilled shapes; the scene is the tested
  geometry source.
- **Hit-test in `crates/geom` over model shapes.** Rejected: geom is L0
  and must not see the model; the scene already resolved transforms and
  inheritance.
- **Selection per document.** Rejected by ADR 0029's placement; the
  commands that switch documents clear it, which is what a per-document
  selection would do anyway.
- **Bounding-box-only hit testing.** Rejected: an unfilled shape would be
  hit through its hole; users notice this immediately.
