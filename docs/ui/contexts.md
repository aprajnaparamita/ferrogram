# Selection contexts

ADR 0064 defines the contexts, and `ui-spec/contexts.yaml` holds the
data. This page explains how to use them.

## The ten contexts

| Context | Set up a test document like this |
|---|---|
| `empty_page` | A new drawing, one empty page, nothing selected |
| `shape_selected` | Draw one rectangle and select it *(data pending)* |
| `shapes_selected` | Draw two rectangles and select both |
| `text_editing` | Draw a rectangle with text and open the text editor on it |
| `connector_selected` | Two rectangles glued with a dynamic connector; select the connector *(data pending)* |
| `group_selected` | Two rectangles grouped; select the group |
| `second_page` | Add a second page and make it active; nothing selected |
| `picture_selected` | Insert a picture and select it |
| `container_selected` | Insert a container and select it |
| `shapesheet_window` | Open the ShapeSheet window of a shape and make it active |

P7-08 builds a test document for each context. The same documents
serve the cross-check test and the contextual-tab tests.

## Reading the table

```yaml
commands:
  edit.cut:
    enabled_in: [shapes_selected, text_editing, group_selected, picture_selected, container_selected]
  view.rulers:
    enabled_in: [empty_page, shapes_selected, ...]
    on_by_default: true
```

- **Listed context**: the command is available there.
- **Unlisted context**, other than a pending one: the command is greyed
  out there.
- **`on_by_default`**: the toggle starts switched on in a new document.
  Phase 4 already has defaults for rulers and grid; this table also
  covers guides, connection points, page breaks, AutoConnect and
  Dynamic Grid.

## Rules of thumb for implementers

- **Pending `shape_selected`**: treat it like `group_selected`, minus
  the commands that only make sense for a group (Ungroup, the group
  submenu items).
- **Pending `connector_selected`**: treat it like `shape_selected`,
  minus the text-direction and container commands.
- **`text_editing`**: text formatting and the clipboard are available.
  Selection-wide arrangement commands follow the shape being edited.
- **`shapesheet_window`**: most drawing commands are greyed out. The
  ShapeSheet Tools tab and the `shapesheet` menu bar take over.

## Changing the data

- Edit `contexts.yaml` by hand, keep the context order, and rerun the
  cross-check test.
- When a predicate deliberately differs from the table, record the
  reason in the test's allow-list rather than editing the table to
  match.
