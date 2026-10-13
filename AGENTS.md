# AGENTS.md — rules for every agent working on Ferrogram

Ferrogram is a clean-room, pure-Rust, Visio-compatible diagramming app (alpha). These rules apply to every agent and every harness (Claude Code, pi, others). If a task prompt conflicts with this file, this file wins; stop and report the conflict.

## 1. Session start

1. Read this file in full.
2. Read `docs/STATUS.md` (current phase, task board, who owns what).
3. Read your task file `docs/tasks/<task-id>.md`. It names your role, crate ownership, inputs, outputs and acceptance criteria.
4. Read your role file `.claude/agents/<role>.md`. Agents in other harnesses read it as plain Markdown; ignore the frontmatter.
5. Read the ADRs your task references in `docs/adr/`.

## 2. Shared local repository and git workflow

The **main repo** is `/Users/dara/ferrogram`, branch `main`. It is shared by every agent on this machine. Only the master agent commits to `main`.

Each task works in its own git worktree on the external drive:

```sh
TASK=P1-03                                   # your task id
WT=/Volumes/Dara/ferrogram/worktrees/$TASK
git -C /Users/dara/ferrogram worktree add -b agent/$TASK "$WT" main
cd "$WT"
export CARGO_TARGET_DIR=/Volumes/Dara/ferrogram/target/$TASK
```

- Commit only on `agent/<task-id>`. Never commit to `main`, never rewrite `main`, never touch another agent's branch or worktree.
- Before handing off, rebase on the latest `main`: `git rebase main` (worktrees share refs with the main repo, so `main` is always current).
- Hand-off: fill in the `## Result` section of your task file (what changed, gate output summary, open issues) and commit it as the last commit on your branch. Then stop and tell the user the branch is ready.
- The master agent reviews and merges into `main`. If review finds problems, you get a follow-up task.
- Leave the worktree in place; the master removes it after merge. Delete your `CARGO_TARGET_DIR` when the task is merged or abandoned.

## 3. Disk usage

The main drive is small. Put everything large on `/Volumes/Dara/ferrogram/`:

- Worktrees: `/Volumes/Dara/ferrogram/worktrees/`
- Cargo target dirs: `/Volumes/Dara/ferrogram/target/<task-id>` (always set `CARGO_TARGET_DIR`)
- Raw downloads: `/Volumes/Dara/ferrogram/reference/`. The repo's `reference/` directory is a symlink to it (gitignored). In a worktree, create the same symlink: `ln -s /Volumes/Dara/ferrogram/reference reference`.
- Large builds (libvisio, corpora): `/Volumes/Dara/ferrogram/`.

## 4. Clean-room rules (non-negotiable)

Allowed sources only:
- Microsoft Open Specifications ([MS-VSDX]).
- Microsoft Learn: the ShapeSheet reference and the Visio VBA/automation reference, including enumerations.
- Microsoft's published Office Fluent UI control identifiers.
- Microsoft support articles (e.g. keyboard shortcuts).
- libvisio, under section 5.
- Observed behavior of Visio as a black box.

Never:
- Disassemble, decompile or debug any Microsoft binary or installer, or extract resources from one (ribbon XML, strings, icons, images, fonts).
- Commit Microsoft-owned content: spec documents, icons, images, stencils (.vss/.vssx), templates, sample drawings, fonts (Calibri, Segoe UI).
- Copy long-form text (super-tooltips, help descriptions). Short command labels ("Align", "Connector", "Pointer Tool") are fine as UI labels. Write our own descriptions.
- Copy GPL/LGPL/AGPL code.

Always:
- Raw downloads stay in `reference/` (gitignored). Commit only derived facts (IDs, names, structure, enumerations) and our own summaries.
- Record every source in `reference/SOURCES.md` (local) and `docs/sources.md` (committed).
- Fonts: Carlito (for Calibri) and Selawik (for Segoe UI), bundled with their OFL licenses.
- Icons: an open-licensed set with its license recorded in `ATTRIBUTION.md`, or simple placeholders. No Office icons.
- Every stencil master we ship is original work.
- Every committed asset file has a row in `ATTRIBUTION.md`, added in the same commit.

## 5. libvisio rules

libvisio is MPL-2.0 (verify `COPYING` when you fetch it; record the commit hash).

- **Test oracle:** its CLI tools (vsd2raw, vsd2xhtml, vss2raw — whichever exist) are built in a dev-only harness under `tools/libvisio-oracle/`. Never a runtime dependency.
- **Binary .vsd reference:** code ported or derived from libvisio lives **only** in `crates/vsd-binary`, with MPL-2.0 file headers and attribution in `NOTICE`. No libvisio-derived code anywhere else, including tests of other crates.
- For .vsdx, MS-VSDX is the source of truth, not libvisio.
- `cargo deny` enforces that `ferrogram-vsd-binary` is the only MPL crate.

## 6. Engineering rules

- **Never crash.** Malformed input produces errors, never panics. Non-test code may not use `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!` or `unimplemented!` (workspace clippy lints). Parser crates (vsdx, vsd-binary, shapesheet) also deny `clippy::indexing_slicing`: use `get()`. Use checked arithmetic and capped allocations on input-derived values. Bound recursion. Tests may use these (clippy.toml). Every crash fix gets a regression test.
- **No unsafe.** Every crate root has `#![forbid(unsafe_code)]` unless an ADR justifies an exception.
- **One command registry** (`crates/commands`). Every user action is a command with an id, label, enabled-state predicate and undo support. Menus, ribbon, shortcuts, CLI, the JSON control channel and MCP all dispatch by command id through the same registry. Programmatic calls never open dialogs.
- **Units.** Internal units are Visio's (inches). Convert only at the edges (UI pixels, file formats that differ).
- **Layering** (ADR 0002, checked by `cargo xtask layers`). A crate depends only on lower layers. Nothing below L6 depends on egui/eframe/winit/rfd.
  - L0 `geom`, `ui-spec` · L1 `model`, `text` · L2 `vsdx`, `vsd-binary`, `shapesheet` · L3 `render` · L4 `commands` · L5 `control` · L6 `ui` · L7 `apps/*`
- **Docs.** `missing_docs` is on for library crates; document public items.
- **Decisions** go in `docs/adr/NNNN-title.md` (copy `docs/adr/0000-template.md`).
- **Stay in your lane.** Edit only the crates/files your task owns. If you need a change elsewhere, write it under "Open issues" in your Result section.
- **Dependencies.** New crates must have permissive licenses (see `deny.toml`). Prefer well-maintained crates. Mention new dependencies in your Result section.

## 7. Quality gate

`cargo xtask gate` is the single gate. It must be green before hand-off.

| Step | Command |
|---|---|
| codegen | `cargo xtask codegen --check` (generated files are fresh; never skipped) |
| fmt | `cargo fmt --all -- --check` |
| clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| test | `cargo test --workspace --all-features` |
| doc | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` |
| coverage | `cargo llvm-cov`, per-crate thresholds and ratchet from `coverage.toml` |
| deny | `cargo deny check` |
| layers | `cargo xtask layers` |
| wasm | `cargo build -p ferrogram-web --target wasm32-unknown-unknown` |
| fuzz | `cargo +nightly fuzz build` in every `crates/*/fuzz` |

While iterating, use `cargo xtask gate --quick` (skips coverage, wasm, fuzz). Run the full gate before hand-off.

Coverage thresholds: geom, model, vsdx, vsd-binary, shapesheet, commands, ui-spec ≥ 85%; render, text ≥ 75%; control, ui, apps ≥ 50% (UI via egui_kittest). No crate may drop below its previous-phase number (`[ratchet]` in `coverage.toml`; only the master updates it).

## 8. Commits

- Conventional commits: `feat(vsdx): read OPC content types`, `test(geom): ...`, `fix`, `docs`, `chore`, `refactor`, `ci`, `build`.
- Scope = crate or area. One logical change per commit. Small commits.
- Include the task id in the body: `Task: P1-03`.

## 9. Tests

- Unit tests live next to the code. Integration tests in `crates/<name>/tests/`.
- Property tests: `proptest`. Snapshots: `insta`. UI: `egui_kittest`. Fuzz: `cargo fuzz` in `crates/<name>/fuzz/`.
- Test files from the corpus are read from `corpus/` and listed in `corpus/PROVENANCE.md`. Never commit a file whose license is unclear.

## 10. Shell tips (macOS)

- `cp`/`mv` may be aliased interactively: use `/bin/cp -f`, `/bin/mv -f`.
- No `timeout` binary: `perl -e 'alarm 600; exec @ARGV' <cmd>`.
- Never run `cargo clean` on another task's target dir.
