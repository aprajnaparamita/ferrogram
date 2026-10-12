# P1-02 review (master)

- Branch reviewed: `agent/P1-02` at `a1caf2a`
- Verdict: **request changes**. Good work overall: the extract skeleton, error handling, ownership matrix and test coverage are solid. The issues below are design decisions that later tasks would build on, so they must be fixed before P1-03..P1-07 start.
- Fix on the same branch (`agent/P1-02`): rebase on `main` first (`git rebase main`), fix, re-run the gate, append a "Review fixes" subsection to the Result in `docs/tasks/P1-02.md`.

## Blocker

**B1. Layout control ids must be the Fluent control ids, not our command ids.**

Fix:
- Tabs, groups, controls, backstage and context-menu entries use the Fluent control name verbatim as `id` (`FluentId`, schema pattern `^[A-Za-z][A-Za-z0-9_]*$`). Tabs and groups also come from the Fluent data (e.g. `TabHome`, `GroupClipboard`), not invented ids.
- Add an optional `command: CommandId | null` on controls, filled from `commands.yaml` when a mapping exists. `parent` is a `FluentId`.
- `docs/parity/controls.md` and `diff-2016-vs-365.md` are keyed by Fluent id.
- Update ADR 0007, the schema, `crates/ui-spec` types and tests, and P1-03/P1-04/P1-06.

## Major

**M1. No build.rs that writes into `src/`; no silent empty fallback.**
ADR 0008 has `crates/model/build.rs` write a gitignored `src/shapesheet_table.rs`, and fall back to an empty table when the TOML is missing. Build scripts must write only to `OUT_DIR`, and a silently empty table hides real failures. Also, ADR 0008 lists `crates/ui-spec` as a consumer of the `model` table, but ui-spec is L0 and model is L1 (ADR 0002): ui-spec cannot depend on model.

Fix: in Phase 1, P1-04 only produces the committed `ui-spec/shapesheet.toml`. Phase 2 adds `cargo xtask codegen`, which turns the committed TOML into a committed `crates/model/src/generated/shapesheet.rs` (header "generated, do not edit"); the gate checks that the generated file is up to date (CI can do this because the TOML is committed). Remove `crates/model/build.rs` from P1-04 ownership. Remove ui-spec as a consumer.

**M2. Command id scopes must be functional, not ribbon tabs.**
ADR 0006 allows `home`, `design` etc. as scopes (`home.align.left`, `home.insert.shape`). The 2016 and 365 layouts place commands on different tabs, and the same command appears on the ribbon, context menus, shortcuts and MCP. An id tied to a tab is wrong in one layout.

Fix: scopes describe what the command does, e.g. `app`, `file`, `edit`, `view`, `insert`, `format`, `text`, `arrange`, `shape`, `connector`, `page`, `layer`, `tools`, `data`, `review`, `developer`, `help`. Examples: `arrange.align.left`, `tools.pointer`, `tools.connector`, `file.save`. Update ADR 0006, the schema pattern if needed, and the examples in all ADRs and task files.

**M3. libvisio oracle: no git submodule, no wrapper crate.**
ADR 0010 adds libvisio as a submodule under `tools/libvisio-oracle/libvisio/` and a wrapper crate outside the workspace that `crates/vsd-binary` tests would depend on. A submodule puts libvisio's source tree into our working tree (and every worktree) on the small drive, and a path dependency on an excluded crate is fragile.

Fix: `build.sh` clones libvisio at the commit recorded in `NOTICE` into `/Volumes/Dara/ferrogram/build/libvisio-oracle/` (overridable with `--build-dir` / an env var for CI) and installs the CLI tools there. Tests find the tools via `FERROGRAM_LIBVISIO_ORACLE` (a directory) and skip with a clear message when it is unset or empty, unless `FERROGRAM_REQUIRE_ORACLE=1`. A small helper (e.g. `cargo xtask oracle <tool> <file>`) is fine. Oracle tests may live in any crate's `tests/` because they only run the binary and compare output; they contain no libvisio code.

**M4. Reserved fields: pick one rule.**
ADR 0007 says the five reserved fields are not required by the schema and a custom lint enforces them; the Result says required-but-nullable. Make them `required` with type `[..., "null"]` in the schema and drop the custom lint.

**M5. YAML crate.**
`serde_yaml` is archived upstream and published as `0.9.34+deprecated`. Choose a maintained YAML crate with a permissive license, confirm `cargo deny check` passes with it (advisories include unmaintained crates), and record the choice in ADR 0007. Use a current `jsonschema` version, not 0.18.

## Minor

- **m1.** `commands.yaml` `visuicmd` pattern accepts the string `"null"` and `""`. Use `"type": ["string", "null"]` with pattern `^visCmd[A-Za-z0-9]+$`.
- **m2.** Shortcut `keys` "normalised to a sorted list" loses key sequences (e.g. Alt, H, ... keytips; Visio has multi-step shortcuts). Represent a shortcut as an ordered list of chords; each chord is a set of modifiers plus one key.
- **m3.** The missing-`reference/` error says "Run `cargo xtask corpus` first". The corpus fetcher does not populate `reference/`. Point to `docs/sources.md` / task P1-01 instead.
- **m4.** The `extracted:` timestamp in generated files makes `cargo xtask extract` output differ on every run (ADR 0009 requires deterministic output). Drop it; `source_revision` is enough.
- **m5.** Microsoft 365 Current Channel as canonical: accepted. Record it in ADR 0007 (not only in the YAML).
