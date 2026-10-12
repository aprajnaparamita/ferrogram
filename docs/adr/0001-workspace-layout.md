# 0001: Workspace layout and crate naming

- Status: accepted
- Date: 2026-10-12
- Task: P0

## Context

KICKOFF.md proposes a layout of library crates under `crates/`, apps under `apps/`, plus `xtask/`, `tools/`, `ui-spec/`, `reference/`, `corpus/` and `docs/`. The ArtCraft apps (gridcraft, wordcraft) keep egui code in a dedicated `ui-egui` crate so the toolkit stays swappable and UI tests run without the binary.

## Decision

- Library crates live in `crates/<short-name>` with package name `ferrogram-<short-name>`: geom, model, vsdx, vsd-binary, shapesheet, text, render, commands, ui-spec, control, and **ui** (added).
- `crates/ui` holds all egui code (ribbon, canvas, panels, dialogs). `apps/ferrogram` and `apps/ferrogram-web` are thin shells around it. This lets egui_kittest tests live in a library crate and keeps the desktop and web apps identical.
- Apps: `apps/ferrogram` (desktop, eframe), `apps/ferrogram-web` (wasm, trunk), `apps/ferrogram-cli`.
- Fuzz targets live in `crates/<name>/fuzz/` in the cargo-fuzz layout. They are separate workspaces and are built by `cargo xtask gate`.
- `reference/` is a gitignored symlink to `/Volumes/Dara/ferrogram/reference` (local disk is small).
- Workspace version 0.0.1, edition 2024, MSRV and pinned toolchain 1.95.0.

## Consequences

- Package names are unambiguous on crates.io should we publish later.
- The UI crate gets a 50% coverage threshold like the apps.

## Alternatives considered

- UI code inside `apps/ferrogram`: duplicates for web, harder to test.
