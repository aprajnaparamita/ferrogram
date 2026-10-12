# 0008: ShapeSheet table format and location

- Status: accepted
- Date: 2026-10-12
- Task: P1-02

## Context

`crates/model` and `crates/shapesheet` need a catalogue of every ShapeSheet
section, row and cell that appears in Visio: its name, the `VisSectionIndices`
it lives at, the row index inside the section, the cell name and the cell's
type and default unit. The catalogue is consumed:

- by `crates/model` when reading and writing `.vsdx` (cell names → XML
  element names),
- by `crates/shapesheet` when the formula engine references a cell by name
  (`Sheet.N!Cell`),
- by `crates/ui-spec` (and later the UI) to display "PinX (Shape Transform
  Section)" tooltips and to validate the `commands.yaml` mapping.

The P1-01 reference data has this catalogue already: 1082 markdown files
under `reference/ms-vsdx/office-developer-client-docs/docs/visio/`,
including ~60 `<section>-section.md` files, ~700 `<cellname>-cell-<section>-section.md`
files, and section/row/cell indices from the `visio.vissectionindices.md`,
`visio.visrowindices.md` and `visio.viscellindices.md` pages.

KICKOFF.md says: "A ShapeSheet table of sections, rows and cells with types
and default units, used as codegen input for `crates/model`."

## Decision

The catalogue is a single **TOML** file committed at
`ui-spec/shapesheet.toml`. It is the source of truth and is **generated** by
the P1-04 extractor from the markdown files under `reference/`.

### File path and why TOML

- Path: `ui-spec/shapesheet.toml` (committed).
- Format: TOML, parsed by `toml` (MIT/Apache-2.0, already used by xtask and
  in `deny.toml`).
- Reason for TOML: simple key/value, no nesting ambiguity, easy to read in
  diffs, and Rust has a fast zero-dep parser. YAML is overkill for a
  table-of-tables with no lists of mixed records.

### Shape

```toml
# ui-spec/shapesheet.toml
schema_version = 1
product = "visio"
source = "Microsoft Learn ShapeSheet reference (office-developer-client-docs f8c5dfed...)"
extracted = "2026-10-12T00:00:00Z"

# One [[sections]] entry per ShapeSheet section.
[[sections]]
name = "Shape Transform"
section_index = 1                     # visSectionObject
kind = "object"                       # "object" | "collection" — distinguishes
                                      # per-shape fixed sections (one row each)
                                      # from repeated-row sections.
description_source = "shape-transform-section.md"  # file under reference/, not committed
default_unit = "Inches"               # default unit for cells in this section

# Cells live at fixed positions in fixed sections (kind = "object").
# For "collection" sections, the same cell set repeats per row.
[[sections.cells]]
name = "PinX"
cell_index = 0                        # local index within the row
type = "Number"                       # "Number" | "String" | "Bool" | "Color" | "List"
default_unit = "Inches"
default_value = "0 in"                # optional
description_source = "pinx-cell-shape-transform-section.md"

[[sections.cells]]
name = "PinY"
cell_index = 1
type = "Number"
default_unit = "Inches"
default_value = "0 in"
description_source = "piny-cell-shape-transform-section.md"

# Example: a collection section (rows repeat).
[[sections]]
name = "Geometry"
section_index = 10                    # visSectionFirstComponent
kind = "collection"
# rows are defined globally so the same row definitions can be referenced
# by every collection section that has them.
rows = ["visRowComponent", "visRowVertex", "visRowLine", "visRowArc", "visRowEllipse", "visRowNURBS", "visRowSpline", "visRowMoveTo", "visRowPolylineTo"]

# Sections may also reference rows that are "owned" by another section
# (e.g. visRowXFormOut is owned by Shape Transform, but its index is
# defined in visio.visrowindices.md globally). The global row table
# captures the index.
[[rows]]
name = "visRowXFormOut"
row_index = 6
section = "Shape Transform"

[[rows]]
name = "visRowLine"
row_index = 2
section = "Geometry"
```

### How `crates/model` consumes it

- A `build.rs` in `crates/model` reads `ui-spec/shapesheet.toml` at build
  time and writes a `shapesheet_table.rs` next to `lib.rs`.
  `shapesheet_table.rs` is `.gitignore`d and contains a `pub const SECTIONS: &[Section]`
  and `pub const CELLS: &[Cell]` slice. `crates/model` re-exports
  `shapesheet_table` so other crates can read it.
- The `build.rs` path is the only consumer path. Tests that need the table
  use the public re-export, never the TOML.
- If `ui-spec/shapesheet.toml` is missing, `build.rs` writes a stub
  `SECTIONS = &[]` and prints a warning; the model crate still builds.
  This means an empty Phase 1 does not break `cargo check`. The full
  `cargo xtask gate` will not pass until the table is filled in and the
  build.rs uses it.

### Why `crates/model` and not `crates/shapesheet`

`crates/model` is L1 and `crates/shapesheet` is L2 (layering, ADR 0002).
`shapesheet` already depends on `model`. The ShapeSheet table is metadata
about the document, not about the formula engine, so it belongs to
`model`. `shapesheet` consumes the model re-export for cell-by-name
lookups in formulas.

### Extractor rules (P1-04)

The P1-04 extractor walks `reference/ms-vsdx/office-developer-client-docs/docs/visio/`:

- A file ending in `-section.md` whose name has no `cell-` substring is a
  section. Parse the first paragraph after the `# Title` heading for a
  one-line summary. Cells inside are linked by name.
- A file matching `<name>-cell-<section>.md` is a cell. Parse the table that
  has "Section index" and "Row index" rows for those fields. The
  `description_source` is the file's relative path under `reference/`.
- A file matching `<name>-function[-visioshapesheet].md` is a function; the
  functions table is **not** part of `shapesheet.toml` (functions live in
  `crates/shapesheet` and are extracted separately; the function catalogue
  is a `crates/shapesheet/function_table.rs` build output of a different
  file in a later task).
- The three enumeration files (`visio.vissectionindices.md`,
  `visio.visrowindices.md`, `visio.viscellindices.md`) provide the integer
  index for every name.

The extractor outputs sorted sections, sorted cells within a section, and
sorted rows. The output is deterministic: re-running on the same input
produces byte-identical TOML.

## Consequences

- One file, one place to read the ShapeSheet metadata. Easy to diff.
- The TOML is small (estimated ~80 sections, ~700 cells → ~50 KB) and
  parses in <1 ms; build.rs cost is negligible.
- The same `build.rs` pattern works for any future catalogue (e.g. a
  function table).
- If Microsoft renames a cell, only the TOML changes; the generated Rust
  re-emits with the new constant name. Tests against a corpus catch
  rename errors.
- The TOML is a **derived** file. It is committed (so `cargo check`
  without `reference/` works in CI), but the extractor is the only writer.
- The `description_source` field stores the relative path under
  `reference/`, not the content. No Microsoft text is committed.

## Alternatives considered

- **Hand-write `shapesheet_table.rs` as checked-in Rust.** Rejected: 700
  cells to maintain; humans will make typos; the source of truth should
  be the upstream markdown.
- **Put the table in `crates/shapesheet/build.rs` and have model
  depend on shapesheet.** Rejected: violates layering; shapesheet is
  L2 and cannot export to L1.
- **YAML instead of TOML.** Both work; TOML is faster to parse, has a
  stable 1.0 spec, and matches the rest of the workspace (`xtask` already
  uses `toml` for `coverage.toml`).
- **Use a database (SQLite).** Rejected: massive overkill, makes
  read-time joins slower, and the data has no relations beyond section →
  cells.
- **Have the loader parse the TOML at runtime.** Rejected: every
  consumer would need the TOML file at runtime, bloating the binary and
  requiring a lookup path. Build-time codegen is the standard pattern
  and keeps the runtime free of strings.
