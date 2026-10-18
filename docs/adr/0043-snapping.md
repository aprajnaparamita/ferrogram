# 0043: Snapping

- Status: accepted
- Date: 2026-10-18
- Task: P5-01

## Context

Phase 5 drags need snapping "to the grid and to connection points"
(KICKOFF). The planning brief (decision 6) asks for the snap targets, the
snap distance in screen pixels, and the priority over other targets.
Snapping serves three callers: the pointer tool's move/resize, the
drawing tools, and connector endpoints (glue, ADR 0045).

## Decision

### Targets and priority

Snap candidates, highest priority first (ADR 0043):

1. **Connection points** (`SnapKind::ConnectionPoint`): the shape's
   `Connection` section rows and the stencil masters'
   `MasterDef::connection_points` (ADR 0034). These also drive glue
   (ADR 0045).
2. **Alignment** (`SnapKind::Alignment`): other shapes' left/centre/right
   and top/middle/bottom edges (of the shapes *not* being dragged).
3. **Grid** (`SnapKind::Grid`): the canvas grid (1/8 in default, ADR
   0033's own choice, revisited in the parity pass).

Within one kind, the nearest candidate wins; across kinds the higher kind
wins even when it is slightly farther (still within the snap distance).
A candidate out of range leaves the position unchanged.

### Distance

The snap distance is **8 screen pixels** (`Snapper::settings::
distance_px`), converted to inches with the zoom at the call. It is a
setting (`SnapSettings`) so the screenshot pass can tune it against
observed Visio behaviour.

### Rules per interaction

- **Move:** the drag *delta* is snapped, not each shape's absolute
  position, so a multi-shape drag keeps its relative offsets. Both axes
  snap independently (edge and centre candidates are per axis).
- **Resize:** the moving edge snaps on its axis (`snap_axis`).
- **Drawing tools and drops:** the pointer point snaps (`snap_point`).
- **Connector endpoints:** snap to connection points and shape outlines
  (glue candidates, ADR 0045) before alignment targets.
- **Alt (Option) held:** snapping is off for that gesture (the standard
  modifier; a black-box check of Visio's own modifier is on the
  screenshot-pass list).

The math lives in `ferrogram_geom::snap` (`snap_point`, `snap_axis`,
`snap_grid`); the policy (which candidates, priority, pixel distance)
lives in `crates/ui::snap` and is unit-tested against a fake `AppCtx`.

## Consequences

- One snap implementation serves every drag; the overlay draws snap
  guides (`OverlayMark::SnapGuide`) so the user sees why a shape jumped.
- The priority rule is explicit and testable; a black-box check may
  reorder it later without API changes (it is data order).
- Grid snapping a delta keeps multi-shape drags coherent, which absolute
  snapping would not.

## Alternatives considered

- **Snap only to the grid.** Rejected: alignment snapping is what makes
  diagramming fast; KICKOFF names connection points explicitly.
- **Absolute-position snapping.** Rejected: a multi-shape drag would
  deform the selection's relative layout.
- **A uniform priority (nearest wins regardless of kind).** Rejected: a
  grid line 1 px nearer than a connection point would beat glue, which is
  exactly the case users complain about in other editors.
- **Screen-pixel thresholds inside geom.** Rejected: geom is unit-pure
  (inches); the UI converts at the edge (AGENTS.md section 6).
