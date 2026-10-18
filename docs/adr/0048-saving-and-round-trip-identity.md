# 0048: Saving and round-trip identity

- Status: accepted
- Date: 2026-10-18
- Task: P5-01

## Context

The Phase 5 exit criterion: "saved .vsdx files re-open identically in
our app" (planning brief decision 11). The writer patches the source
trees (ADR 0018) so unmapped content survives a save; Phase 5 adds
content that did not come from a source tree (new shapes, new pages,
`Connects`) and removes content (deleted pages). The brief asks what
"identically" means and how the round trip is tested.

## Decision

### "Identically" is semantic model equality after one round trip

For a document `d` read from a file: `read(write(read(write(d))))`
compares **equal** to `read(write(d))` as `model::Document` values,
ignoring:

- the volatile document properties (`docProps` modified time, when the
  writer touches it),
- `Preserved` attribute *order* (the raw XML diff is semantic, ADR 0012's
  `diff`),
- the change counter, undo stack and other `AppCtx` state (they are not
  in the file).

In words: saving and re-opening changes nothing the model can see, and a
second save writes the same bytes as the first (the writer is
deterministic, ADR 0025's determinism rule). The exit test also keeps
*editing* after the reopen (the criterion's "keep editing").

### Writer additions (P5-02)

- **New shapes:** patched into the page part's `Shapes` element in
  z-order, with their cells (formula + cached value, ADR 0038) and, for
  master instances, the `Master` attribute. The writer writes both `F`
  and `V`.
- **New pages:** a new page part, its relationship, the content-type
  override and the `pages.xml` entry; ids from `Document::next_visio_id`
  style allocation (ADR 0039).
- **`Connects`:** the page's `Connect` records are written to the
  `Connects` element.
- **Deletes:** a deleted page or master drops its part, its relationships
  and its content-type entry (`Package::remove_part`, the P2-07 deferred
  item that page delete triggers). A deleted shape simply stops being
  written (it is unreachable in the arena, ADR 0039).

### Tests (P5-02 and P5-10)

- `crates/vsdx/tests/roundtrip_edit.rs`: build a document with the
  mutation API (new shape, new page, connects, text), write, read, and
  assert the model equality above.
- The exit-criterion harness (`apps/ferrogram/tests/flowchart.rs`) saves
  the built flowchart, reopens it through the registry and keeps editing.
- The existing corpus round trip (`FERROGRAM_REQUIRE_CORPUS=1`) must stay
  green: the writer only gains *additive* behaviour.

## Consequences

- "Re-open identically" is a test, not a slogan; the equality rule is
  written down so a reviewer can disagree with a specific clause.
- New content goes through the same deterministic writer; byte-equality
  of two saves is a cheap extra assertion.
- Visio's own reading of our written files is *not* asserted (no Visio
  in CI); schema validation (the Phase 2 test) is the guard, plus the
  libvisio oracle where it can compare.

## Alternatives considered

- **Byte equality of the whole file on round trip.** Rejected: the
  patch-based writer can legitimately reorder attributes it did not
  write, and doc properties carry timestamps.
- **Model equality including `Preserved`.** Rejected: `Preserved` holds
  raw XML whose attribute order is not semantic; `raw::diff` is the
  comparison that matters.
- **A schema-version field for future migration.** Rejected: ADR 0038
  shows the formula engine needs no migration, and an unused version
  field is a liability.
- **Writing new shapes as flattened XML mirrors of the model.** Rejected:
  one writer path (the patch/emit split of ADR 0018) stays the single
  source of written XML.
