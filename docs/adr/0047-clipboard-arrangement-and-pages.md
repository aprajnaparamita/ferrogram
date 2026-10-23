# 0047: Clipboard, arrangement and pages

- Status: accepted
- Date: 2026-10-18
- Task: P5-01

## Context

KICKOFF Phase 5: copy, cut, paste, duplicate, delete; align, distribute,
group/ungroup, bring forward/send backward; add, rename, delete and
reorder pages. The planning brief (decision 10) asks for the clipboard
format (internal, and whether there is a text or image export to the
system clipboard, including on wasm), and which of these are command bar
controls with `commands.yaml` ids so the parity matrix moves.

AGENTS.md section 6: every user action is a command; the clipboard must
work headlessly (the CLI and Phase 8) as well as from the UI.

## Decision

### Internal clipboard

`commands::clipboard::Clipboard` in `AppCtx` (one slot, ADR 0029):
`ClipboardContent { kind, shapes, connects, source_page }`, a deep clone
of the copied subtree plus the `Connects` records between the copied
shapes. Paste re-inserts through the ADR 0039 primitives with fresh
arena and Visio ids, at the recorded position offset by (0.1 in, -0.1 in)
per consecutive paste, or at `x`/`y` when the paste command carries them
(the pointer tool's paste-at-pointer). Cut = copy + detach (one undoable
edit). Duplicate = paste at the offset, in one edit.

### No system clipboard formats in Phase 5

There is **no** text or image export to the system clipboard in Phase 5,
on desktop or wasm. Reasons: (1) a system format needs a documented
payload (a metafile or SVG flavor) that we would have to invent and then
keep stable; (2) the wasm clipboard API is async and permission-gated,
which would fork the command flow; (3) nothing in the exit criterion
needs it. The seam is the `Clipboard` data: a later task adds
`SystemClipboard` (L6) translating to and from `ClipboardContent`, and
the ADR 0036 `FilePicker` pattern (a trait behind the shell) applies.

### Arrangement and pages are commands

Arrangement (`arrange.align.{left,center_x,right,top,middle,bottom}`,
`arrange.distribute.{horizontal,vertical}`, `arrange.group`,
`arrange.ungroup`, `arrange.bring_forward`, `arrange.send_backward`,
`arrange.bring_to_front`, `arrange.send_to_back`) and pages
(`page.add`, `page.rename`, `page.delete`, `page.reorder` and the
`page.move_left`/`page.move_right` wrappers) are registered commands:

- **Align/distribute** move shapes by delta (one `CompositeEdit`, ADR
  0040); align uses the selection's bounding box edges.
- **Group/ungroup** use `Document::group_shapes` / `ungroup_shape`
  (ADR 0039); groups are ordinary shapes with children (ADR 0019).
- **Z-order** uses `Document::set_z_order`.
- **Pages** use the page primitives (ADR 0039); deleting a page removes
  its part in the writer (ADR 0048).

All of these get rows in the extractor's "our own commands" list where
`commands.yaml` has no VisUICmds row (ADR 0028), so every id appears in
the registry and in the parity matrix (ADR 0037). The parity generator
gains a `| command | label | status |` table for command-only ids (the
P4-15 deferred item; owner P5-10).

### UI affordances

Command bar groups (Home: paste, arrange; Insert: pages) are generated from
the layouts as today; the page-tab strip gains a context menu for
rename/delete/reorder (P5-09 may add the menu lines to
`crates/ui/src/canvas.rs`). Shortcuts come from `shortcuts.yaml` scopes.

## Consequences

- Clipboard works headlessly; the wasm build gets copy/paste for free
  (no system formats to implement).
- Every action is a registry id, so the parity matrix grows honestly and
  Phase 8 exposes them all.
- Paste of master instances keeps the `Master` link (the model clone
  keeps it), so pasted flowchart shapes stay instances.

## Alternatives considered

- **System clipboard as SVG/PNG now.** Rejected: unneeded by the exit
  criterion, and a format commitment we would rather make once
  deliberately.
- **Serialize the clipboard to JSON on `AppCtx`.** Rejected: `Shape` is
  rich (preserved XML); a clone is exact and free of schema churn.
- **Arrangement as UI-only gestures.** Rejected: AGENTS.md section 6.
- **Page reorder by rewriting page ids.** Rejected: ids are identity
  (ADR 0039); reorder moves list entries only.
