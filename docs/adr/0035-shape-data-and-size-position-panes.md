# 0035: Shape Data and Size & Position panes

- Status: accepted
- Date: 2026-10-16
- Task: P4-01

## Context

KICKOFF Phase 4: "Panes: basic Shape Data and Size & Position panes."
Phase 4 has no selection (that is Phase 5), so the panes must show
something honest with nothing selected and be ready for selection and
editing later. AGENTS.md section 6: every user action is a command, so
edits from a pane must dispatch commands, not mutate the model from
widget code.

## Decision

### Read-only in Phase 4

Both panes are **read-only** in Phase 4. They render from `AppCtx` only.

- **Shape Data** (`crates/ui::panes::show_shape_data`):
  - With no selection: the document's properties (`Document::properties`,
    the `Property` rows written to `DocumentProperties`) and the active
    page's name and size, as a two-column label/value table.
  - With a selection (Phase 5): the selected shape's Shape Data rows.
- **Size & Position** (`crates/ui::panes::show_size_position`):
  - With no selection: the active page's width and height, formatted in
    `Settings::units`, and its name.
  - With a selection (Phase 5): the shape's `Width`, `Height`, `PinX`,
    `PinY`, `Angle`, `LocPinX`, `LocPinY` (from the ADR 0019 cell
    reader), in the same two-column table.

Values are formatted with units at this edge (inches internally,
AGENTS.md section 6). Non-finite or missing values show "—" and one
warning on `AppCtx::warnings` (ADR 0021 style: never a crash).

### Editing arrives with Phase 5 through commands

When Phase 5 adds selection, the pane rows become editable fields that
dispatch reserved command ids (`shape.set.width`, `shape.set.height`,
`shape.set.pin`, `shape.set.angle`, `data.set.property`) with typed `Args`
(ADR 0028); each returns an `Edit` for undo. The Phase 4 pane code keeps
one `Row` abstraction (label, formatted value, optional `CommandId` +
`Args` for the edit) so Phase 5 fills in dispatch without redesigning the
widget. The row ids for the edit commands are Phase 5's to add to the
extractor's "our own commands" list (ADR 0028); no Phase 4 pane code
depends on them.

## Consequences

- Panes are honest in Phase 4: no dead edit fields, no fake values.
- One formatting path for units serves both panes, rulers and the status
  bar (ADR 0033).
- Phase 5 adds editing behind the existing `Row` interface and the
  registry; no egui state leaks into L4.

## Alternatives considered

- **Editable fields now, no-ops on commit.** Rejected: a control that
  looks editable and does nothing violates the "honest" parity goal and
  the never-crash spirit.
- **Show Shape Data for the first shape when nothing is selected.**
  Rejected: silently showing someone else's data is confusing; page data
  is what the user is looking at.
- **Panes read cells directly from the model with their own defaults.**
  Rejected: the ADR 0021 `CellReader` is the one place defaults and
  warnings are defined.

## Amendment 2026-10-23 (task P6-18): `data.set.property` lands

The reserved id `data.set.property` is implemented (the P6-09 note and
the P6-12 review F11): the Shape Data pane's value column is editable
through it, exactly like the Size & Position fields edit through
`shape.set.*` (one registry command per edit, one undo step, no dialogs
from programmatic calls).

- **Arguments** (the command contract, ADR 0028): `row` is the Shape
  Data row's key — its `N` name, or `Row_<n>` for the row at 1-based
  position `n` when it has no name (a row name wins the key on a
  clash) — and `value` is the value's own spelling.
- **What `value` means** follows the kind the row's `Value` cell
  caches (ADR 0058: "the cached value wins"): a **number** is one
  literal of the formula grammar (`2.5`, `2.5 in`, `37%`, `45 deg`,
  `5 1/2`) converted through the engine's unit table, where a literal
  without a unit is a number of the cell's own unit; a **boolean** is
  `TRUE`/`FALSE` (or `1`/`0`); a **colour** is the `#RGB`/`#RRGGBB`
  spelling the pane shows (the file's own text; the formula grammar has
  no colour literal); a **string** is the text as written. A list or
  error value, and a row with no cached value, are not editable (one
  `InvalidArgs`, nothing written).
- **The write** goes through `CellEdit::direct` (a direct user edit,
  ADR 0056: it replaces a `GUARD`-ed formula) with the cell spelled
  `Prop.<row>.Value`, and gets the same undo step and recalc pass as
  every other cell edit (`AppCtx::push_edit`). The `Value` cell keeps
  the unit it records (own or inherited).
- **The pane lists the union** of the shape's own `Property` rows and
  the rows it inherits from its master chain (the P6-09 note): the
  shape's own row wins on a key clash, a deleted own row hides the
  inherited row, and an inherited row says so in its hover note. An
  edit of an inherited row writes a **local row** on the instance (the
  master's row is untouched); undoing the edit removes the local row
  again.
- **Formula values stay refused** (ADR 0058): the row keeps its typed
  field and refuses the edit with one status line, as in the other
  panes.
