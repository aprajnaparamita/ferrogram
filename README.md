# Ferrogram

A clean-room, open-source diagramming app in pure Rust that aims to read and write Microsoft Visio files (.vsdx, with read-only .vsd import).

> **Status: pre-alpha, Phase 5 of 8 complete, Phase 6 in progress.** The app opens, renders, edits and saves `.vsdx` files: you can draw, move, connect and type into shapes across pages, undo it, save, reopen and keep editing. The ShapeSheet formula engine exists as a library but is not yet wired into editing, opening or saving. It is not ready for real work: `.vsd` import and the control channel are not built, and visual fidelity is approximate.

## Build status

Last full gate on `main`: **green, 2026-10-22** (Phase 6 engine merged, commit `fff98f9`, 2835 tests). There is no public CI yet; the gate runs locally.

| Step | Result |
|---|---|
| codegen, parity, fmt, clippy, doc, deny, layers | pass |
| test (`cargo test --workspace --all-features`) | pass |
| coverage (per-crate thresholds and ratchet) | pass |
| wasm (`ferrogram-web`) | pass |
| fuzz (`cargo +nightly fuzz build`) | pass |

Line coverage per crate (`cargo xtask coverage`):

| Crate | Coverage | Crate | Coverage |
|---|---:|---|---:|
| geom | 100.0% | text | 98.9% |
| shapesheet | 92.9% | commands | 98.3% |
| model | 99.7% | vsdx | 98.1% |
| ui-spec | 99.4% | ferrogram-cli | 96.8% |
| render | 98.5% | ui | 96.8% |
| ferrogram (desktop) | 80.0% | ferrogram-web | 77.9% |

`control` and `vsd-binary` have no code yet (Phases 8 and 7). `shapesheet` dropped from 100% because Phase 6 replaced its stubs with about 6500 lines of real code; its threshold is 85%.

## What works today

- **UI and ShapeSheet definitions** (Phase 1), generated from Microsoft's published data by `cargo xtask extract`: Visio 2016 and Microsoft 365 ribbon layouts, 977 command ids, the keyboard shortcut table and the ShapeSheet table.
- **Document model and `.vsdx` I/O** (Phase 2): pages, masters, shapes, styles and ShapeSheet cells; lossless read and write, with content the model does not cover kept in place. Every corpus file round-trips.
- **Rendering and CLI** (Phase 3): pages render from cached cell values to PNG (`tiny-skia`) and SVG, with text shaping and bundled open fonts (Carlito, Selawik). `ferrogram-cli inspect`, `render` and `export`.
- **App shell** (Phase 4): desktop (eframe) and web (wasm) apps with a ribbon generated from the 2016 or 365 layout, switchable without a restart; one command registry behind the ribbon, menus, shortcuts and CLI; canvas with zoom, pan, grid and rulers; panels; original stencils; file open, save and export.
- **Editing core** (Phase 5):
  - select, marquee, move, resize and rotate with handles and snapping;
  - pointer, rectangle, ellipse, line, text and connector tools, chosen through registry commands;
  - drag masters from the Shapes window onto the page;
  - dynamic connectors that glue to connection points or outlines and re-route when shapes move;
  - in-place text editing;
  - cut, copy, paste, duplicate, group, align, distribute, z-order, and page add, delete, rename and reorder;
  - undo and redo for every edit.

  The Phase 5 exit test builds a 10-shape flowchart with 5 glued connectors on two pages, saves it, reopens it and keeps editing (`apps/ferrogram/tests/flowchart.rs`). Phase report: [docs/progress/phase-5.md](docs/progress/phase-5.md).
- **ShapeSheet engine** (Phase 6, in progress), in `crates/shapesheet`:
  - a formula parser and printer: 976 of the 994 distinct formulas in the test corpus parse, and every one of them prints back unchanged;
  - values with units and Visio's type conversions;
  - an evaluator with 42 functions (arithmetic, logic, `IF`, `GUARD`, `SETF`, trig, rounding, string and unit helpers);
  - reference resolution across shapes, masters, pages and styles, with inheritance;
  - a dependency graph with incremental recalculation and cycle detection.

  Instances of a master now draw at their own size instead of the master's.

## Next

Phase 6 continues: wire the engine into editing and undo, into opening, rendering and saving, and into the Shape Data and Size & Position panels. Then compare its results with the values Visio saved in the corpus files, and prove that resizing a master-based shape recomputes its geometry. Group A (the engine library) is merged; the integration tasks are running.

The live task board is [docs/STATUS.md](docs/STATUS.md). Phase reports are in [docs/progress/](docs/progress/). Command coverage against the ribbon is in [docs/parity/controls.md](docs/parity/controls.md).

## What does not work yet

Formula evaluation in the app (edits still write fixed values, and shapes whose cells have no cached values render blank until Phase 6 wires the engine in), `.vsd` import, the MCP/control channel, and most ribbon commands (they are listed but stubbed).

## Plan (alpha MVP)

| Phase | Scope | State |
|---|---|---|
| 0 | Bootstrap: workspace, gate, CI, agent process | done |
| 1 | Sources and UI/ShapeSheet definitions | done |
| 2 | Document model and .vsdx I/O (lossless round-trip) | done |
| 3 | Rendering (PNG/SVG) and CLI | done |
| 4 | App shell, switchable 2016/365 ribbon | done |
| 5 | Editing core | done |
| 6 | ShapeSheet engine (core subset) | in progress |
| 7 | Binary .vsd import (read-only) | planned |
| 8 | Control channel and MCP | planned |

Out of scope for the alpha: visual fidelity, data linking, containers/lists beyond preservation, VBA/macros, coauthoring, printing, writing .vsd, large stencil libraries.

## Clean room

Ferrogram is built only from public specifications ([MS-VSDX]), Microsoft Learn reference documentation, Microsoft's published Office Fluent UI control identifiers, public support articles, observed behavior, and libvisio (MPL-2.0, confined to `crates/vsd-binary`). No Microsoft code, binaries, icons, fonts, stencils or templates are used or shipped. See [AGENTS.md](AGENTS.md).

Visio and Microsoft 365 are trademarks of Microsoft Corporation. Ferrogram is not affiliated with or endorsed by Microsoft.

## Building

```sh
cargo build --workspace
cargo xtask gate          # full quality gate
cargo xtask gate --quick  # skip coverage, wasm and fuzz
cargo xtask codegen       # regenerate the ShapeSheet table
cargo xtask corpus        # fetch the test corpus (needs network)
```

The gate needs `cargo-deny`, `cargo-llvm-cov`, a nightly toolchain with `cargo-fuzz`, and the `wasm32-unknown-unknown` target.

## License

MIT (see [LICENSE](LICENSE)), except `crates/vsd-binary`, which is MPL-2.0 (see [NOTICE](NOTICE)).
