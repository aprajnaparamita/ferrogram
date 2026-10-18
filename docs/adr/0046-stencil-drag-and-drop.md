# 0046: Stencil drag and drop

- Status: accepted
- Date: 2026-10-18
- Task: P5-01

## Context

The Shapes window shows the stencils (ADR 0034) but dragging onto the
page is Phase 5 (the P4-06 deferred item). The brief (decision 9) asks
for the drop position (`PinX`/`PinY`), the instance size, and glue on
drop. `instantiate` has no size parameter (P4-06 note) and Phase 5 has
no formula engine (ADR 0038).

## Decision

- **Drop position:** `PinX`/`PinY` are the pointer's page coordinates at
  the drop, snapped per ADR 0043 (`Snapper::snap_point`; connection
  points first). The drag shows the ghost outline on the overlay (ADR
  0042).
- **Instance size:** the master's own size (`MasterDef::width`/
  `height`) unless the drag is a click-drag with a drawn rectangle (the
  `Shift`-drag variant): then the drawn rectangle is the size. The model
  primitive is `Document::instantiate_master(master, page, at, size)`
  (ADR 0039); `size: None` is the master size, closing the P4-06 note.
- **Glue on drop:** when the drop point is within snap distance of an
  existing shape's connection point and the dropped master has
  connection points (flowchart connectors do), the new shape's nearest
  connection point glues to that target (one `Connect` record, ADR 0045)
  and any glued connector endpoints on the page re-route. A drop near a
  shape's outline with no connection point in range only snaps; it does
  not glue (glue is a deliberate act in Phase 5).
- **Command:** the drop dispatches `insert.shape` (`stencil`, `master`,
  `x`, `y`, optional `width`/`height`, optional `glue_to`) with
  `Source::Ui`; the command is undoable (one `CompositeEdit`, ADR 0040)
  and headlessly runnable (the CLI and Phase 8 build shapes the same
  way).

## Consequences

- The exit flowchart can be drawn from the stencil in a couple of
  gestures per shape.
- Drop behaviour is one command with explicit arguments, so the MCP
  agent (Phase 8) can place shapes without a GUI.
- Glue-on-drop is deliberately narrower than Visio's (outline glue needs
  a deliberate modifier); recorded for the screenshot pass.

## Alternatives considered

- **Instance size scaled to a default page fraction.** Rejected: the
  master's own size is predictable and matches the stencil preview.
- **Glue on every drop near a shape.** Rejected: it surprises users who
  want proximity layout; connection-point glue only.
- **A drag-and-drop-only path (no command).** Rejected outright: AGENTS.md
  section 6 — every user action is a command.
