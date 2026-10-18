# 0039: The model mutation API

- Status: accepted
- Date: 2026-10-18
- Task: P5-01

## Context

Phase 5 edits documents from many commands (move, resize, delete, group,
pages, connectors, text). The commands crate (L4) must not reach into
`Sheet` internals or duplicate bookkeeping: one mutation API in
`crates/model` (L1) owns the invariants. The planning brief (decision 2)
asks for creating, deleting and re-parenting shapes, writing cells,
adding and re-ordering pages, new shape and page ids, master instances
and `Connects`, with invariants stated and errors returned, never a
panic.

One constraint comes from undo (ADR 0040): an edit holds ids across
`revert` and `apply`. `model::id` promises arena ids stay valid for the
document's lifetime ("entries are never removed or moved while any id is
live").

## Decision

### Operations are methods on `Document` (`model::mutate`)

The full stubs are committed in `crates/model/src/mutate.rs`; the shape
is: `insert_shape`, `insert_subshape`, `detach_shape`, `restore_shapes`,
`reparent_shape`, `set_z_order`, `set_shape_text`, `write_cell`,
`write_row_cell`, `scale_geometry`, `add_page`, `delete_page`,
`rename_page`, `move_page`, `add_master`, `instantiate_master`,
`add_connect`, `remove_connects`, `connects_of`, `group_shapes`,
`ungroup_shape`, `write_connector_path`, `next_visio_id`.

All return `Result<_, ferrogram_model::Error>`; new variants go through
the crate's `error.rs` (task P5-02 may add a bounded set of them).

### Invariants

1. **Ids are stable.** A mutation never removes or shifts entries of
   `Document::shapes`. Delete is **detach**: the subtree leaves its page
   or parent list and comes back through `restore_shapes` (the undo step
   holds the `DetachedShapes` value). The arena may hold detached shapes
   no page reaches; the renderer and the writer walk pages, so they never
   see them. (The alternative — compacting the arena — breaks the id
   promise and every id an undo step holds.)
2. **Lists hold live shapes in order.** `Page::shapes` is z-order;
   `Shape::children` is child order. Every index an operation takes is
   clamped or rejected (`Err`), never a panic (AGENTS.md section 6).
3. **No cycles.** `reparent_shape` refuses to make a shape its own
   ancestor; group/ungroup keep the tree a forest.
4. **Ids are allocated, not guessed.** `Document::add_shape` assigns the
   arena id; `next_visio_id` assigns Visio's `ID` (max on the page plus
   one); `NameU` values are made unique by the caller-visible helper rule
   (append ` (n)`), and page/master `ID`s follow the same max-plus-one
   rule. Arithmetic is checked and input-derived counts are capped
   (AGENTS.md section 6).
5. **Connects are owned by the page.** `add_connect` de-duplicates by
   (connector endpoint, target); `detach_shape` and `delete_page` remove
   the records naming the removed shapes and return them for undo.

### Master instances

`instantiate_master(master, page, at, size)` adds one instance shape with
the `Master` link and the literal cells of ADR 0038 (`PinX`/`PinY`,
`Width`/`Height`, `LocPinX`/`LocPinY` as `Width*0.5` etc.), `size: None`
meaning the master's own size (the P4-06 deferred item). Geometry stays
the master's rows (the ADR 0019 row union) until `scale_geometry`.

### Errors

Every failure is a typed `Error` value (`UnknownShape`, `UnknownPage`,
`NotImplemented` today; `WouldCycle`, `BadCell`, `TooManyShapes` as
needed). The model never panics on input, uses `get()` over indexing
(`clippy::indexing_slicing` is denied in the crate), and bounds every
allocation derived from input.

## Consequences

- Commands build undo steps from `DetachedShapes` snapshots and
  `CellWrite` old values; no command writes `Sheet` directly.
- The writer (P5-02) sees one shape of data: pages, shapes and connects
  the API produced are consistent by construction.
- Detached shapes linger in the arena until the document is dropped or
  re-read; a save does not write them (they are unreachable). A
  compaction pass would need a new ADR.
- Phase 8 (control channel) gets editing over the same API with no GUI.

## Alternatives considered

- **A builder/transaction object per edit.** Rejected for Phase 5: the
  undo `Edit` *is* the transaction (ADR 0040), and a second wrapper would
  obscure which layer keeps which invariant.
- **Free functions over `&mut Document`.** Rejected: inherent methods
  discover better and the crate already grows `impl Document` blocks
  (`lookup`, `document`).
- **Deleting from the arena (compaction).** Rejected: breaks the id
  stability promise ids in undo steps rely on.
- **A `ShapeRef` guard type that panics on stale ids.** Rejected:
  "never crash" means stale ids are `Err`, not panics.
