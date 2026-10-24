# 0058: Typed Shape Data and the panes

- Status: accepted
- Date: 2026-10-21
- Task: P6-01

## Context

Phase 4 built the Shape Data and Size & Position panes (ADR 0035) while
values were strings and cells had no evaluator (P4-06: "Shape Data values
are strings until the formula engine"). Phase 6 gives values a type
(ADR 0052) and gives every cell an evaluated value (ADR 0055), so the
panes can show what the cell means and accept what the user types,
including unit suffixes. The panes are read-only for values they cannot
edit safely today (a formula cell), and every edit still goes through the
command registry (AGENTS.md section 6).

## Decision

### What the panes show

- **Shape Data** shows one row per `Prop` row: the row name, the `Label`
  cell's text and the `Value` cell **typed**. The type is the cached
  value's kind (ADR 0052), displayed in the cell's unit: a number shows
  with its unit and with `Settings::units` as the display system (P5-05
  took inches and degrees; this closes it), a boolean shows as a
  checkbox-like value, a colour as its RGB value, a date or duration as
  its cached text. The row's `Type` cell (`0` string, `2` number, ...)
  is used for the *format* only when it disagrees with the cached kind:
  the cached value wins (it is what the file holds).
- **Size & Position** shows `PinX`, `PinY`, `Width`, `Height`, `Angle`,
  `LocPinX`, `LocPinY`, `FlipX`, `FlipY` as **evaluated** values (the
  recalc result, ADR 0055), in `Settings::units` display units, and
  marks the cells that carry a formula (read-only in this phase).

### What the panes accept

- A number field accepts the same literals the grammar does (ADR 0051):
  `2.5`, `2.5 in`, `2.5 cm`, `37%`, `45 deg`, `5 1/2` (via
  `shapesheet::parse`, so the two never disagree). The parsed value is
  converted to the cell's unit before the write.
- A text field writes a string; a boolean field writes `TRUE`/`FALSE`.
- A field for a **formula** cell is read-only in this phase. Editing a
  formula is the formula editor of a later task; typing a bare number
  into a formula cell would silently replace the formula and break the
  master link, so the pane refuses it and says so in one status line.
- Every accepted edit is one registry command with one undo step
  (`shape.set.text` style, ADR 0040); the panes never touch the model
  directly and never open dialogs programmatically.

### Where the change lives

`crates/ui/src/panes.rs` only (task P6-09). No new public interface:
the panes read through `CellReader` (ADR 0021, amended by ADR 0057) and
write through the registry. The typed value mapping they use is the
engine's (`Value::to_cached`, ADR 0052).

## Consequences

- P4-06's "Shape Data values are strings" closes; the panes show what
  the file holds and what the engine computes.
- `Settings::units` finally reaches the editable fields (P5-05's deferred
  item), through one conversion path (the engine's units, ADR 0052).
- Refusing formula edits is a visible gap, not a silent data loss; the
  formula editor is a Phase 7+ feature.

## Alternatives considered

- **Make formula cells editable as text (write `F`).** Rejected: no
  validation exists yet (the parser does, but the panes have no error
  surface for a rejected formula in this phase), and a bad write would
  break a master-driven shape.
- **Show `Prop.Type`'s type instead of the cached kind.** Rejected: the
  cached value is what renders and what the file holds; the `Type` cell
  is a Shape Data *format* hint (P4-06 note) and often disagrees.
- **A shared "value formatting" module in `crates/ui`.** Rejected for
  this phase: one file, one task; a formatting module can come with the
  formula editor.
- **Convert display units in the panes by hand.** Rejected: P5-05's
  inch/degree fields already show the cost; the engine's unit table
  (ADR 0052) is the one conversion point.
