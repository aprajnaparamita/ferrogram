# Ferrogram

A clean-room, open-source diagramming app in pure Rust that aims to read and write Microsoft Visio files (.vsdx, with read-only .vsd import).

> **Status: pre-alpha, Phase 2 of 8 complete.** Nothing is usable as an app yet. The libraries read `.vsdx` files into a document model and write them back without loss, but nothing renders or edits them. Do not use Ferrogram for real work.

## What works today

Library code and developer tools only. There is no app, CLI feature or UI yet.

- **UI and ShapeSheet definitions** (Phase 1), generated from Microsoft's published data by `cargo xtask extract`:
  - Visio 2016 and Microsoft 365 ribbon layouts (`ui-spec/`);
  - 935 command ids, 90 keyboard shortcuts and the ShapeSheet table;
  - loaded and checked at runtime by `crates/ui-spec`.
- **Geometry** (`crates/geom`): units and conversions, points, rectangles, angles and affine transforms.
- **Document model** (`crates/model`): pages, masters, shapes, styles and ShapeSheet cells, each cell keeping its formula, cached value and unit. Master and style inheritance is resolved by a lookup, and XML can be compared semantically. The ShapeSheet table is generated from `ui-spec/shapesheet.toml` by `cargo xtask codegen`.
- **`.vsdx` reading and writing** (`crates/vsdx`):
  - The zip package layer reads and writes packages, with caps against zip bombs and oversized input.
  - Pages, masters, shapes, styles and document properties are read into the model, and content the model does not cover is kept.
  - The writer updates each part from the model and keeps everything else in place. All 15 corpus files round-trip without loss.
  - Tested with malformed input, a fuzz target, property tests, the libvisio oracle and schema checks.
- **Test tooling:**
  - `cargo xtask corpus` fetches third-party test files (they are not committed);
  - `cargo xtask oracle` builds libvisio's tools as a test oracle;
  - hand-written test files are in `corpus/fixtures/`.

## Next (Phase 3)

Rendering to PNG and SVG, and the first CLI commands.

The live task board is [docs/STATUS.md](docs/STATUS.md). Phase reports are in [docs/progress/](docs/progress/).

## What does not work yet

Rendering, editing, the ribbon UI, the ShapeSheet formula engine, `.vsd` import, the CLI and the MCP/control channel.

## Plan (alpha MVP)

| Phase | Scope | State |
|---|---|---|
| 0 | Bootstrap: workspace, gate, CI, agent process | done |
| 1 | Sources and UI/ShapeSheet definitions | done |
| 2 | Document model and .vsdx I/O (lossless round-trip) | done |
| 3 | Rendering (PNG/SVG) and CLI | planned |
| 4 | App shell, switchable 2016/365 ribbon | planned |
| 5 | Editing core | planned |
| 6 | ShapeSheet engine (core subset) | planned |
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
