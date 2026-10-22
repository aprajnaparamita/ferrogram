# 0064: Selection contexts and availability

- Status: accepted (2026-10-23, task P7-01); amended below
- Date: 2026-10-23
- Task: P7-01

## Context

Three parts of the UI depend on what the user is doing:

1. which commands are enabled;
2. which contextual tabs appear (Picture Tools, Container Tools,
   ShapeSheet Tools);
3. which context menu opens on right-click.

The registry already gives each implemented command an enabled
predicate (ADR 0028). The UI also needs a named, testable notion of
"context", plus a target table for the hundreds of stub commands, so
that they behave as expected once implemented.

## Decision

### Contexts

`ui-spec/contexts.yaml` names ten contexts:

| Context | Meaning |
|---|---|
| `empty_page` | A page with no shapes, nothing selected |
| `shape_selected` | One ordinary 2-D shape selected |
| `shapes_selected` | Two or more shapes selected |
| `text_editing` | A shape's text is open for editing |
| `connector_selected` | One connector selected |
| `group_selected` | One group selected |
| `second_page` | Another, empty page is active |
| `picture_selected` | One picture selected |
| `container_selected` | One container selected |
| `shapesheet_window` | A ShapeSheet window is active |

Panel Classic adds two window modes, `shapesheet_window` and `stencil_edit`,
which choose the menu bar and the mode toolbars.

`crates/ui` computes the current context from `AppCtx` (selection,
shape kinds, text editor state, active window) as one value per frame.
The value is a small enum in `crates/ui/src/contexts.rs`, so nothing
below L6 learns about the UI. The enum has one variant per context
above, plus `PageSelected` (the page has shapes but nothing is
selected, which counts as `empty_page` for availability) and
`StencilEdit`.

### Availability table

`contexts.yaml` has a `commands` map from command id to `enabled_in`, a
list of contexts, plus `on_by_default` for toggles such as Rulers and
Guides.

- **Implemented commands**: the registry predicate decides. A test
  (P7-08) checks every implemented command against `enabled_in` in each
  context that has data, and lists the differences. A difference is
  either a predicate bug to fix or a deliberate deviation recorded in
  the test with a one-line reason.
- **Stub commands**: these are disabled anyway (ADR 0031). When one is
  implemented, its predicate starts from its row.
- Contexts with `status: pending` have no data yet. The test skips
  them.

### Contextual tabs and context menus

- A contextual tab is shown when the current context equals its
  `shown_in`. It appears to the right of the core tabs under its tab-set
  caption. It is not selected automatically. If the active tab
  disappears, the command bar falls back to the last core tab used.
- A right-click picks the context menu whose trigger matches the hit
  target: a shape, the empty page, a page tab, text being edited, a
  connection point, and so on. P7-08 owns the mapping from hit-test
  result to menu id.

## Consequences

- Enablement is testable per context, without the GUI.
- The two `pending` contexts need data before the cross-check covers
  them. Adding it is a data-only change.

## Alternatives considered

- **Encode availability in each layout item.** Rejected: availability
  belongs to the command, and per-placement copies would drift apart.
- **Derive availability only from predicates.** Rejected: stub commands
  have no predicate yet, and the UI still has to choose contextual tabs.

## Amendments (task P7-01)

- The data-side name is `ui_spec::v2::ContextId`: the ten contexts
  plus the `stencil_edit` window mode that `shown_in` accepts. The UI
  enum is `crates/ui/src/contexts.rs::SelectionContext` with twelve
  variants (the ten plus `PageSelected` and `StencilEdit`) and an
  `id()` mapping (`PageSelected` counts as `empty_page`). Both are
  defined as stubs by P7-01; `contexts::current` returns
  `Result<SelectionContext, UiError>` until P7-08 implements it (it
  may drop the `Result`).
- A contextual tab appears in the Panel Modern command bar; the term
  "command bar" replaces the draft's Microsoft term throughout.
