# Layout format v2

Reference for `ui-spec/layouts/*.yaml`. The decision record is ADR
0061, and the machine-checked definition is
`ui-spec/layout-v2.schema.json`. This page explains each field and how
the widgets should read it.

## File

```yaml
schema_version: 2
id: panel-modern                # matches the file name
name: Panel Modern              # shown in the layout picker
kind: panel-modern              # panel-classic | panel-modern
description: ...
theme: panel-modern             # ui-spec/themes/<theme>.yaml
window: {regions: [...]}  # frame regions, outermost first
# kind: panel-classic
menu_bars: [...]
toolbars: [...]
# kind: panel-modern
quick_access_toolbar: [...]
file_button: {label: File, opens: file_view}
tabs: [...]
contextual_tabs: [...]
file_view: {...}
# both
context_menus: [...]
```

### `window.regions`

Each region has an `id`, an `edge` and a nominal `size` in logical
pixels: a height for top and bottom edges, a width for left and right.
They are listed outermost first.

- `edge: fill` is the drawing area.
- `top_left_of_drawing` is the rulers.
- `bottom_of_drawing` is the page-tab strip with the horizontal scroll
  bar.

`notes` says what the region holds. The shell (P7-09) builds its egui
`Panel` areas in this order.

## Items

An item is one control. Items nest through `items`.

| Field | Meaning |
|---|---|
| `id` | Unique dotted id within the layout, for example `tab.home.clipboard.paste` or `drawing.edit.cut`. It identifies the placement. |
| `kind` | See the table below. |
| `label` | Short UI label. In menus, draw it with the `access` letter underlined. |
| `access` | Access-key letter (menus and context menus). Alt plus the letter opens a top-level menu; inside an open menu, the bare letter activates the item. |
| `shortcut` | Shortcut text shown right-aligned in menus and in tooltips. Dispatch still goes through `shortcuts.yaml`. |
| `command` | Registry id dispatched on activation. |
| `new_command` | The id is added by Phase 7 (`ui-spec/commands-phase7.yaml`). |
| `fluent` | Fluent control id, for cross-reference with the 2016/365 data and the parity matrix. |
| `visuicmd` | The documented UI command constant, for reference. |
| `icon` | Key into `ui-spec/icons.yaml`. If absent, the item has no icon (menus keep the gutter empty; toolbars show the label instead). |
| `display` | Toolbar only: `icon` (default), `text` or `icon_text`. |
| `size` | Panel Modern only: `large` (32 px icon over the label), `medium` (16 px icon beside the label) or `small` (16 px icon only). Combos, checks and launchers have no size. |
| `column`, `row` | Panel Modern only: the slot in the group (see Panel Modern groups below). |
| `width` | Width of combos, in logical pixels. |
| `begin_group` | Draw a separator before this item. |
| `hidden` | Part of the layout, but not drawn by default. |
| `dynamic_label` | The app replaces the label at run time, for example "Undo Move" or "Can't Redo". |
| `list` | The app fills this part from live data (see Run-time lists below). |
| `options` | A fixed list of choices (page sizes, orientations, layout directions). |
| `items` | Children: submenu entries, dropdown or split-button contents, toolbar palette entries. |

### Kinds

| `kind` | Panel Classic rendering | Panel Modern rendering |
|---|---|---|
| `button` | Menu item or toolbar button | Button at its `size` |
| `check` | Menu item with a check mark; latched toolbar button | Check box with its label |
| `toggle` | Latched toolbar button | Latched button |
| `split` | Toolbar button with an arrow part that opens `items` | Top part runs `command`; the arrow part opens `items` |
| `split_recent` | Like `split`, but the main part shows the last item chosen | (not used) |
| `dropdown` | Toolbar button that opens `items` | Button with an arrow that opens `items` |
| `gallery` | (not used) | Dropdown whose content is a grid of `list` entries |
| `palette` | Toolbar dropdown showing graphic choices (line weights, patterns, ends) | (not used) |
| `combo` | Editable combo box (font, size, zoom) | Editable combo box |
| `submenu` | Menu item with ▸ that opens `items` | Menu entry with ▸ inside a dropdown |
| `launcher` | (not used) | Small corner button in the group caption (also `group.launcher`) |
| `heading` | Non-clickable section heading inside a palette | Same |
| `list` | Run-time entries inserted at this point | Same |
| `options` | Fixed text choices inserted at this point | Same |

## Panel Classic containers

```yaml
menu_bars:
- id: drawing            # drawing | shapesheet | stencil
  label: Drawing menu bar
  shown_in: shapesheet_window   # absent for the default bar
  menus:
  - {id: drawing.file, label: File, access: f, items: [...]}
toolbars:
- id: toolbar.standard
  label: Standard
  visible: true          # shown by default
  dock: top              # top | bottom | left | right | floating
  row: 0                 # docked row, top to bottom
  shown_in: shapesheet_window   # mode toolbars only
  items: [...]
context_menus:
- {id: context.shape, label: Shape, trigger: right-click a shape, items: [...]}
```

## Panel Modern containers

```yaml
quick_access_toolbar: [items]
tabs:
- id: tab.home
  label: Home
  fluent: TabHome
  groups:
  - id: tab.home.clipboard
    label: Clipboard
    layout: stack        # stack | rows
    rows: 2              # stack only; default 3
    controls: [items with column/row/size]
    launcher: {label: Font, command: format.all.text.props}
contextual_tabs:
- id: tab.picture_tools.format
  tab_set: Picture Tools
  label: Format
  shown_in: picture_selected
  groups: [...]
file_view:
  back_button: true
  default_page: file_view.info
  nav:
  - {id: file_view.save, kind: command, label: Save, command: file.save}
  - {id: file_view.print, kind: page, label: Print, command: file.print}
```

### Panel Modern groups

- **`stack`**:
  - columns run left to right in `column` order;
  - a `large` control fills its column from the top of the group down
    to the caption;
  - other controls stack in rows `0..rows-1`, each 23 px high;
  - with `rows: 2`, the two rows are centred vertically.
- **`rows`**:
  - two rows of small controls and combos, each laid out left to right
    in `column` order;
  - row 0 sits on the upper half and row 1 on the lower half;
  - a gap of `rows.gap` separates clusters, at the places where the
    columns jump in x (see the theme).

Hidden controls have no slot.

## Run-time lists

| `list` | Filled with |
|---|---|
| `recent_files` | `Settings::recent_files`, numbered 1–9 with the digit as the access key |
| `windows` | Open document windows, with a check on the active one |
| `pages` | Pages of the active document |
| `template_categories`, `stencil_categories`, `my_shapes` | Larrygram's own template categories and stencils (ADR 0034) |
| `colors` | Theme colours, then standard colours, then recent colours, then a More Colours command |
| `theme_colors`, `theme_tints`, `standard_colors`, `recent_colors` | The parts of the colour list, for the Panel Classic palettes |
| `line_weights`, `line_patterns`, `line_ends`, `fill_patterns`, `shadow_patterns`, `corner_roundings`, `transparency_levels` | Larrygram's preset values, drawn as small samples |
| `themes`, `theme_variants`, `backgrounds`, `borders_and_titles`, `shape_styles`, `effects`, `*_presets`, `container_styles`, `container_headings`, `callouts`, `containers`, `data_graphics`, `shapes_for_change_shape` | Larrygram's own content (ADR 0023 styles, our stencils). An empty list shows one disabled "(none)" entry. |
| `hyperlinks`, `data_recordset_links` | The selected shape's hyperlinks or data links |
| `add_ons`, `macros` | Always empty (out of scope) |
| `align`, `position`, `diagram_rotations`, `validation_rule_sets`, `open_documents`, `reviewers`, `markup_overlays`, `ink_colors`, `pen_widths` | Command-specific lists, defined with the command when it is implemented |
