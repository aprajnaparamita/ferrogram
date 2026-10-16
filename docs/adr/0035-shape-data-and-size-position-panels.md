# 0035: Shape Data and Size & Position panels

- Status: accepted
- Date: 2026-10-16
- Task: P4-01

## Context

KICKOFF Phase 4: "Panels: basic Shape Data and Size & Position panels."
Phase 4 has no selection (that is Phase 5), so the panels must show
something honest with nothing selected and be ready for selection and
editing later. AGENTS.md section 6: every user action is a command, so
edits from a panel must dispatch commands, not mutate the model from
widget code.

## Decision

### Read-only in Phase 4

Both panels are **read-only** in Phase 4. They render from `AppCtx` only.

- **Shape Data** (`crates/ui::panels::show_shape_data`):
  - With no selection: the document's properties (`Document::properties`,
    the `Property` rows written to `DocumentProperties`) and the active
    page's name and size, as a two-column label/value table.
  - With a selection (Phase 5): the selected shape's Shape Data rows.
- **Size & Position** (`crates/ui::panels::show_size_position`):
  - With no selection: the active page's width and height, formatted in
    `Settings::units`, and its name.
  - With a selection (Phase 5): the shape's `Width`, `Height`, `PinX`,
    `PinY`, `Angle`, `LocPinX`, `LocPinY` (from the ADR 0019 cell
    reader), in the same two-column table.

Values are formatted with units at this edge (inches internally,
AGENTS.md section 6). Non-finite or missing values show "—" and one
warning on `AppCtx::warnings` (ADR 0021 style: never a crash).

### Editing arrives with Phase 5 through commands

When Phase 5 adds selection, the panel rows become editable fields that
dispatch reserved command ids (`shape.set.width`, `shape.set.height`,
`shape.set.pin`, `shape.set.angle`, `data.set.property`) with typed `Args`
(ADR 0028); each returns an `Edit` for undo. The Phase 4 panel code keeps
one `Row` abstraction (label, formatted value, optional `CommandId` +
`Args` for the edit) so Phase 5 fills in dispatch without redesigning the
widget. The row ids for the edit commands are Phase 5's to add to the
extractor's "our own commands" list (ADR 0028); no Phase 4 panel code
depends on them.

## Consequences

- Panels are honest in Phase 4: no dead edit fields, no fake values.
- One formatting path for units serves both panels, rulers and the status
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
- **Panels read cells directly from the model with their own defaults.**
  Rejected: the ADR 0021 `CellReader` is the one place defaults and
  warnings are defined.
