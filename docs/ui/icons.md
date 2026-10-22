# Icons

ADR 0062 records the decision. `ui-spec/icons.yaml` holds the map, and
the SVGs are in `crates/ui/assets/icons/tabler/`.

## The set

The set is Tabler Icons 3.49.0, outline style, under the MIT licence.
Every icon is a 24×24 view box with 2 px round strokes in
`currentColor`. We commit only the icons the map uses. The licence file
sits next to them.

## How a control finds its icon

1. The layout item has `icon: <key>`. The key is usually the command
   id (`edit.cut`); a few are item ids.
2. `icons.yaml` maps the key to `{icon: scissors, tint: icon.blue}`.
3. The widget asks the icon cache for (icon, size, tint, scale) and
   draws the texture it gets back:
   - size is 16 for menus, toolbars and small or medium command bar
     controls, and 32 for large command-bar controls;
   - the tint colour comes from the active theme (`icon.*` tokens);
   - with no tint, the icon uses `icon.neutral`.
4. If the key has no row, the widget draws the Phase 4 placeholder
   mark.

## Choosing an icon

Pick by meaning. Ask what the command does and which everyday symbol
says that best:

- Cut → `scissors`;
- Bring to Front → `stack-front`;
- Union → `layers-union`;
- Group → `box-multiple`;
- Pointer Tool → `pointer`;
- Connector → `route-2`;
- Lasso Select → `lasso`;
- Rulers → `ruler`;
- Shape Data → `table`.

Rules:

- Never look for an icon that resembles another application's artwork,
  and never trace or edit one. A plain, recognisable metaphor is the
  goal.
- The same command gets the same icon everywhere.
- Related commands get related icons: the `layout-align-*` family for
  shape alignment, `align-*` for text alignment, `stack-*` for z-order.
- Tints are a hint, not a code. Blue is the default accent. Use orange
  for clipboard and arrangement, red for delete-like actions, green for
  validation. Leave most icons neutral.

## Adding or changing an icon

1. Find the name in the pinned Tabler release (`@tabler/icons` 3.49.0,
   `icons/outline/<name>.svg`).
2. Copy the SVG into `crates/ui/assets/icons/tabler/` unchanged.
3. Add or edit the row in `ui-spec/icons.yaml`.
4. Run `cargo xtask codegen` to refresh the embedded icon table.
5. Run the ui-spec data tests: each key resolves, and each file exists.

`ATTRIBUTION.md` has one row for the whole directory, so a new file
needs no new row unless it comes from a different set.

## Coverage today

- 394 icon keys across both layouts, using 245 distinct Tabler icons.
- 130 keys carry a tint.
- Items without an `icon` key draw text only (toolbars) or an empty
  gutter (menus).
