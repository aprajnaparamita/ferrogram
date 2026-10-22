# 0062: Open icon set and icon map

- Status: accepted (2026-10-23, task P7-01); amended below
- Date: 2026-10-23
- Task: P7-01

## Context

Since Phase 4, every control draws a placeholder letter (ADR 0030,
`crates/ui/src/icons.rs`). AGENTS.md section 4 allows an open-licensed
icon set with its licence recorded in `ATTRIBUTION.md`, and forbids
Office icons and any traced or redrawn copy of them. The Phase 7 layouts
mark 394 icon keys across both layouts. Each needs a picture that says
what the command does, at 16 px (menus, toolbars, small command bar
controls) and 32 px (large command-bar controls).

## Decision

### One set: Tabler Icons, outline style

- Tabler Icons 3.49.0, MIT licence, from the npm package
  `@tabler/icons`. It has about 5,200 outline icons. One consistent
  stroke style covers every concept we need, including drawing tools,
  text formatting, alignment, layers, stacking and data.
- Only the icons the map uses are committed, under
  `crates/ui/assets/icons/tabler/` (245 files), with the upstream
  `LICENSE`. One `ATTRIBUTION.md` row covers the directory.
- Icons are chosen by meaning: scissors for Cut, `stack-front` for Bring
  to Front, `layers-union` for Union. No icon is chosen to resemble
  another product's artwork, and no icon is traced or edited. Recolouring
  is the only change, and it happens at run time.

### The map: `ui-spec/icons.yaml`

- `set`: name, version, licence, style and path.
- `icons`: a map from icon key to `{icon, tint?}`. The icon key is
  usually the item's command id, so every placement of a command shares
  one picture. Where one command needs different pictures in different
  places, the key is the item id instead (for example
  `classic.toolbar.standard.shapes`).
- `tint`: an optional theme token (`icon.blue`, `icon.orange`, ...).
  Without one, the icon is drawn in the theme's `icon.neutral` colour.
- A key without a row draws the placeholder mark. The loader warns once
  per missing key in debug builds.

### Rendering

- `resvg` (MIT OR Apache-2.0; its `tiny-skia` dependency is already in
  the tree) rasterises each SVG once per (icon, size, tint, scale
  factor) into an egui texture.
- Before parsing, `currentColor` in the SVG is replaced with the tint
  colour.
- Stroke width stays at Tabler's 2 px on a 24 px grid. At 16 px this
  reads as about 1.3 px, which matches the weight of menu text.
- The SVG bytes are embedded with `include_bytes!` from a table that
  `cargo xtask codegen` writes from `icons.yaml`
  (`crates/ui/src/icons/table.rs`). `codegen --check` keeps the table
  and the map in sync. This works on wasm, which has no filesystem.
- Disabled controls draw the icon at 40% alpha.

## Consequences

- Real icons in every surface, with one licence row.
- Adding an icon means adding a row to `icons.yaml`, copying the SVG
  from the pinned Tabler release, and running `cargo xtask codegen`.
- `resvg` and `usvg` are new dependencies of `crates/ui` (L6). The wasm
  build covers them.

## Alternatives considered

- **Lucide, Phosphor or Iconoir.** These are also permissive, but each
  has fewer icons for diagramming concepts (arrangement, layers, text
  direction). Mixing sets breaks the visual consistency.
- **Two-tone or filled styles to suggest colour.** Rejected: a single
  stroke icon plus a tint keeps the set consistent and recolourable.
- **Rasterise at build time to PNG.** Rejected: fixed sizes would blur
  at fractional scale factors. Run-time rasterisation is cheap with a
  cache.
- **Paint icons as egui shapes from the path data.** Rejected: it
  rebuilds an SVG renderer for no gain.

## Amendments (task P7-01)

- The icon pipeline module is `crates/ui/src/icons/mod.rs` (the Phase
  4 placeholder module gains `icon_atom` and `draw`, implemented by
  P7-04).
- Licence check (task P7-01, `cargo deny` over the closure in a
  scratch workspace): `resvg` 0.45.1/0.48.1 and `usvg` are
  MIT OR Apache-2.0 (as this ADR says), and so are `tiny-skia`,
  `fontdb` and `svgtypes`. No MPL crate is involved. The repository's
  `deny.toml` allow-list covers the closure; the real check runs when
  P7-04 adds the dependency.
- Icon keys that name a placement (for example
  `classic.toolbar.standard.shapes`) keep the placement's id; those
  ids are renamed only where they carry the old layout term.
