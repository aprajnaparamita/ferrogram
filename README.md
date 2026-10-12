# Ferrogram

A clean-room, open-source diagramming app in pure Rust that aims to read and write Microsoft Visio files (.vsdx, with read-only .vsd import).

> **Status: pre-alpha bootstrap.** Nothing works yet. The workspace, quality gate and project process are in place; features land phase by phase (see below). Do not use Ferrogram for real work.

## What works today

- Nothing user-facing. The crates are empty placeholders.

## What does not work yet

Everything else, including: opening files, rendering, editing, saving, the ribbon UI, the ShapeSheet engine, .vsd import, the CLI and the MCP/control channel.

## Plan (alpha MVP)

| Phase | Scope | State |
|---|---|---|
| 0 | Bootstrap: workspace, gate, CI, agent process | in progress |
| 1 | Sources and UI/ShapeSheet definitions | planned |
| 2 | Document model and .vsdx I/O (lossless round-trip) | planned |
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
```

The gate needs `cargo-deny`, `cargo-llvm-cov`, a nightly toolchain with `cargo-fuzz`, and the `wasm32-unknown-unknown` target.

## License

MIT (see [LICENSE](LICENSE)), except `crates/vsd-binary`, which is MPL-2.0 (see [NOTICE](NOTICE)).
