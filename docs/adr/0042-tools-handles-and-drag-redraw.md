# 0042: Tools, handles and drag redraw

- Status: accepted
- Date: 2026-10-18
- Task: P5-01

## Context

Phase 5 is interactive: a tool state machine, move/resize/rotate
handles, drag previews, and redraw speed while dragging (planning brief
decision 5). ADR 0033 already fixes the canvas as a cached `tiny-skia`
raster uploaded as an egui texture, and warns that "a full raster per
frame while dragging may be too slow". The overlay escape hatch is named
there ("drags draw transient egui shapes over the cached raster") but
not designed.

## Decision

### 1. One tool state machine, events in, intents and overlay out

`crates/ui::tools::ToolMachine` owns the active `Tool`
(`Pointer`, `Rectangle`, `Ellipse`, `Line`, `Connector`, `Text`) and
consumes `ToolEvent`s (`PointerDown/Move/Up`, `DoubleClick`, `Cancel`),
in page inches. It returns a `ToolEffect { intents, overlay }`:

- **intents** are `crate::Intent`s dispatched through the registry
  (ADR 0028): the tool never mutates `AppCtx` (ADR 0029);
- **overlay** marks are `ferrogram_render::OverlayMark`s (selection
  outlines, handles, rubber band, snap guides, connector previews).

`ToolMachine::handle` takes `&AppCtx` read-only. The tool modules are
split one per Phase 5 task (`pointer`, `draw`, `stencil`, `text`,
`connector`) so parallel tasks never touch the same file.

### 2. Handles

`crates/ui::handles`: `HandleKind::{Move, Resize(ResizeEdge), Rotate}`
over the selection bounds (single shape: its rotated bounds; several: the
axis-aligned union). Hit tolerance is **6 screen pixels**
(`TOLERANCE_PX`), converted with the zoom. The rotate handle sits above
the bounds' top edge at a fixed screen offset. Dragging a handle produces
`shape.move` / `shape.resize` / `shape.rotate` intents with the delta in
page inches (snapping per ADR 0043) and one `CompositeEdit` on release
(ADR 0040).

### 3. Drag redraw: overlay over the cached raster, no partial raster

While a drag runs, the page raster is **not** rebuilt. Each frame draws
the cached texture plus the overlay as egui vector art. The raster is
re-rasterized only when the document change counter moves (commit,
ADR 0040) or the zoom settles (ADR 0033's 10% rule). This keeps the
fidelity reference (the PNG goldens) single-sourced and puts the drag
cost in one small scene.

**Frame budget:** the per-frame UI work during a drag (event handling,
snap computation, overlay build and egui paint) must stay under **8 ms**
for a 1000-shape page on the reference machine; the commit
re-rasterization is allowed to take as long as a normal raster. P5-11
measures it (a timing test with a generous CI allowance) and P5-10's
exit-criterion harness exercises the drag path. If profiling cannot meet
the budget, the fallback is a partial raster of the drag bounds in one
raster pass — a new ADR then, because it forks the pixel path.

### 4. Cursor and preview

Each tool and handle sets the cursor through the same effect seam
(`ToolEffect` carries the marks; the cursor is derived from the hovered
handle/tool by the canvas). The drag preview shows the moved or resized
shape as an outline; the committed shape appears after the edit lands.

## Consequences

- Drag cost is independent of page complexity except for hit-testing and
  snapping (both bounded by shape count, with early-outs).
- The overlay is drawn in L6 from L3 plain data; no egui below L6
  (layers stay green).
- Tests can drive `ToolMachine` headlessly (events in, effects out) with
  no egui at all; `egui_kittest` covers the wiring (P5-11).
- Redraw after commit is a full raster (the ADR 0033 cache); large pages
  commit slowly. Accepted until the partial-raster ADR if the budget is
  missed.

## Alternatives considered

- **Full raster per drag frame.** Rejected: the raster is the slowest
  path in the app; a 1000-shape page re-rasterized at pointer rate will
  not hit 8 ms.
- **A scene-to-egui vector backend for everything (ADR 0033's option).**
  Rejected for Phase 5: gradients, patterns, line ends and glyph
  outlines all need conversion code, and the pixel path would fork from
  the PNG goldens. The overlay is the small version of this idea.
- **WGPU retained rendering.** Rejected in ADR 0030 and not needed here.
- **Transient commands per drag frame (one undo step per pixel).**
  Rejected: the drag commits once (ADR 0040).
