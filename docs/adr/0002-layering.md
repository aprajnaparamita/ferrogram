# 0002: Crate layering

- Status: accepted
- Date: 2026-10-12
- Task: P0

## Context

Following the ArtCraft pattern, layering keeps the engine independent of the GUI toolkit and prevents dependency cycles as many agents work in parallel.

## Decision

A workspace crate may depend only on workspace crates in strictly lower layers. Crates below L6 may not depend on egui, eframe, winit, rfd, egui_kittest or egui-wgpu. Checked by `cargo xtask layers` (part of the gate); the table lives in `xtask/src/layers.rs`.

| Layer | Crates | Notes |
|---|---|---|
| L0 | geom, ui-spec | no workspace deps |
| L1 | model, text | model owns the document; text does layout given fonts and a box |
| L2 | vsdx, vsd-binary, shapesheet | readers/writers and the formula engine operate on `model` |
| L3 | render | scene graph from model + text |
| L4 | commands | registry, undo, selection; uses everything below |
| L5 | control | JSON control channel and MCP over the command registry |
| L6 | ui | egui |
| L7 | apps/* | thin shells |

## Consequences

- `shapesheet` cannot call `vsdx` and vice versa; anything both need belongs in `model`.
- `text` cannot see `model`; render passes it plain text runs and formatting.
- Changing a layer needs a new ADR and an update to the table in xtask.

## Alternatives considered

- No enforcement: proven to drift in multi-agent work.
