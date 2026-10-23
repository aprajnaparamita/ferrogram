# Phase 1 review — reviewer (P1-09)

Scope: `git diff phase-0..main` (89 files). Focus per the task: panics and
unbounded work on hostile input (`crates/ui-spec`, xtask parsers), error
handling, determinism of `cargo xtask extract`, duplication, the
`crates/ui-spec` public API for Phase 4, and happy-path-only tests.

Method: full read of `crates/ui-spec` and the xtask extractors; grep sweeps for
panics, indexing, `HashMap` iteration and lossy error mapping; `cargo test
--workspace --all-features` (all green, ~500 tests); hostile-input probes of the
runtime loader (deep nesting, YAML alias bombs) from a scratch crate outside the
repository. The deferred findings already recorded in `docs/STATUS.md` are not
re-raised; I do not rate any of them higher than the master did. The P1-04
`ACTION_TO_COMMAND` phrase question is P1-11's call and is not reviewed here.

Positive findings, verified rather than assumed:

- No `unwrap`/`expect`/`panic!` outside `#[cfg(test)]` in either the runtime
  crate or the extractors; workspace lints enforce this.
- Hostile input in `crates/ui-spec`: deeply nested YAML is rejected with
  `recursion limit exceeded`, and a YAML alias bomb (billion laughs) is cut off
  at serde-saphyr's node budget (`budget breached: Nodes { nodes: 250001 }`,
  ~50 MB resident at the cap). Malformed input is an error, never a crash.
  `serde_json` JSON paths are bounded by its own 128-level recursion limit.
- Determinism of `cargo xtask extract` is real, not incidental: every
  collection is `BTreeMap`/`BTreeSet` or explicitly sorted (directory listings
  included: `shapesheet.rs:154-160`, `vsdx_schema.rs:623-637`), the tie-breaks in
  `commands.rs` match picking are total, and the only timestamp in any output is
  the pinned source date constant (`vsdx_schema.rs:56`). Determinism unit tests
  exist for fluent, shortcuts and vsdx_schema.
- Test quality is well above happy-path: malformed-input suites, `proptest`
  property tests (including "the sequence parser never panics"), fixture
  cross-reference workspaces, and the committed-data loader checks.

## Findings

### 1. major — `xtask/src/extract/schema.rs:77`, `:120`, `:140`: errors name the schema file, not the failing data file

`validate_file(path)` drops `path` before delegating to `validate_str` /
`validate_json`. Every failure is reported with `path: self.source.clone()`,
which is `ui-spec/schema.json`. A YAML syntax error in `ui-spec/commands.yaml`
therefore prints `parse error in ui-spec/schema.json: YAML parse error: ...`,
and a schema violation prints `schema validation failed for
ui-spec/schema.json: ...`. With four data files and two rewritten layouts going
through this validator, the message actively misdirects debugging. (The runtime
loader does this correctly: `crates/ui-spec/src/loader.rs:236-253` threads the
data path through.)

Fix: thread the data path into `validate_str`/`validate_json` (an explicit
`path: &Path` parameter or a `validate_file`-only public surface), keeping
`self.source` only for the two genuine schema-file failures at `:93` and `:98`.
Add a regression test asserting the error path equals the data file.

### 2. major — `xtask/src/extract/shortcuts.rs:248`, `xtask/src/extract/visuicmds.rs:45`: every read failure becomes `ExtractError::Missing`

Both entry points use `read_to_string(...).map_err(|_| ExtractError::Missing { .. })`,
discarding the `io::Error`. Permission denied, a directory in the way, or a
non-UTF-8 byte in the source (plausible in a Wayback HTML snapshot) all report
`missing input: ...html`, which sends the reader to `docs/sources.md` to
re-download a file that is present and unreadable. Everywhere else (e.g.
`shapesheet.rs:167`, `vsdx_schema.rs:640`) the error is preserved as
`ExtractError::Io`.

Fix: map to `ExtractError::Io { path, source: e.to_string() }` and reserve
`Missing` for `ErrorKind::NotFound` (or pre-check `is_file()` as
`fluent.rs:210-216` does). Regression tests for a permission-denied and a
non-UTF-8 file.

### 3. minor — placeholder error paths: `xtask/src/extract/fluent.rs:347`, `xtask/src/extract/commands.rs:483`, `xtask/src/extract/commands.rs:711`

Errors carry `PathBuf::from("<in-memory>")` or `PathBuf::from("<layout>")`
although the real source path is in scope one or two calls up
(`build_layout`'s `xlsx_path`, `run`'s `path_2016`/`path_365`). With two
near-identical xlsx inputs, a bad row cannot be attributed to 2016 or 365.

Fix: pass the source path into `assemble_layout` / `fill_layout_commands` and
use it in the error.

### 4. minor — `xtask/src/extract/fluent.rs:267`: unparsable `Policy ID` silently becomes `i64::MAX`

`get(cols.policy_id).parse::<i64>().unwrap_or(i64::MAX)` is lossy in two ways:
an unparsable id silently sorts to the end (it decides control order inside
groups, `fluent.rs:470-476`), and a genuine `i64::MAX` sentinel collides with a
parse failure. The extractor's own contract for the other columns is "a
deviation means the source changed under us" (`visuicmds.rs:56-60`).

Fix: treat a non-empty, unparsable cell as `ExtractError::Parse` with the row
number; keep `i64::MAX` (or `0`) only for genuinely empty cells, and document
the choice.

### 5. minor — `xtask/src/extract/mod.rs:274-304`: `extract --check` cannot detect stale outputs

`check` compares exactly the files the extractors return. A committed generated
file that an extractor no longer produces (renamed output, dropped module) is
never compared and so never flagged; the stale file survives every check.

Fix: also flag committed files under the known output paths (`ui-spec/*.yaml`
generated set, `docs/parity/controls.md`, `docs/vsdx-inventory.md`,
`ui-spec/diff-2016-vs-365.md`, `ui-spec/shapesheet.toml`) that the run did not
produce, minus the hand-maintained inputs (`ui-spec/out-of-scope.txt`,
`ui-spec/schema.json`, `ui-spec/*.txt`).

### 6. minor — platform/CI hazards in the byte comparison and test temp dirs

Two related flakiness sources:

- `xtask/src/extract/mod.rs:306` `same_bytes` compares bytes, but on a Windows
  checkout with `core.autocrlf=true` the committed files have CRLF while the
  regenerated output has LF (all emitters write `\n` literals). `--check` would
  report every file as differing. Not visible in CI today (CI has no
  `reference/`), but it will bite the planned Windows run.
- Test temp directories are mostly keyed by `std::process::id()` but several are
  fixed names shared by every concurrent process:
  `xtask/src/extract/diff.rs:369` (`larrygram-diff-test`),
  `xtask/src/extract/parity.rs:270`, `:306`, `xtask/src/extract/util.rs:134`,
  `:147`, `:158`. Two agents running `cargo test` at once (e.g. P1-09/P1-10/P1-11
  worktrees) can delete each other's directories mid-test.
  `xtask/src/extract/fixtures.rs:207` already has a per-run helper that is not
  used outside the fixtures module.

Fix: normalise line endings when comparing under `--check` (or force `git
checkout` LF for generated files via `.gitattributes`), and route every test
temp dir through the `fixtures.rs` helper with a pid + counter name.

### 7. minor — duplication to absorb before Phase 2 adds more extractors

- YAML emitter options (`compact_list_indent = false; prefer_block_scalars =
  false`) are hand-set in three places: `fluent.rs:185-190`, `commands.rs:416-422`,
  `shortcuts.rs:812-814`. Any future tweak must be made three times or the
  committed style drifts.
- The "render header comment + text, `write_atomic`, `schema.validate_file`"
  pipeline is copy-pasted in `fluent.rs:59-67`, `commands.rs:397-413`,
  `shortcuts.rs:292-297`.
- `write_atomic` exists twice with different temp-name strategies:
  `xtask/src/extract/util.rs:52` (pid + nanos, text) and `xtask/src/corpus.rs:282`
  (fixed `<name>.tmp`, bytes). The fixed name collides between concurrent
  `cargo xtask corpus` runs (`corpus.rs:312`).
- `unescape_entities` / `collapse` exist in three flavours:
  `shortcuts.rs:756`/`:806`, `shapesheet.rs:662`, and inline in
  `markdown.rs:206-250` (`cell_text`).
- The control-type-word stripping loop is duplicated inside one file:
  `commands.rs:877-893` (`fluent_words`) and `commands.rs:1038-1050`
  (`fluent_command_id`).

Fix: one shared `emit_yaml`, one `write_and_validate`, one `write_atomic`, one
entity decoder in `extract/util.rs` / `extract/markdown.rs`, and have
`fluent_command_id` call `fluent_words`.

### 8. minor — duplicate ids are silently accepted (`crates/ui-spec`)

ADR 0007 makes the Fluent name the primary id, and ADR 0006 makes the command
id unique, but nothing enforces cross-item uniqueness: `ui-spec/schema.json`
only has `uniqueItems` inside `aliases`/`fluent_ids` arrays, and the loader has
no duplicate check. Two controls with the same `id` silently collapse in
`CommandLookup::build` (`crates/ui-spec/src/lookup.rs:55-73`: last write wins
for labels, first-write-wins for commands depending on insert order), and two
`commands.yaml` entries with the same `id` merge silently in
`check_references` (`crates/ui-spec/src/loader.rs:432-448`, `BTreeSet<&CommandId>`).
The extractor side does hard-error on collisions (`commands.rs:523-535`), so
this only bites hand-edited or third-party data — which is exactly what a
runtime loader must survive without silently misbehaving.

Fix: reject duplicate control/group/tab/menu ids per layout and duplicate
command ids in `commands.yaml` as `LoadError::CrossReference` (or a new
`LoadError::Duplicate`), with tests. While in the loader, add an end-to-end test
for the `.json` fallback of `data_file` (`loader.rs:201-216`); today only
`parse_document`'s JSON branch is tested.

### 9. minor — `crates/ui-spec` public API: rough edges before Phase 4

Nothing blocking, but each item below is a likely breaking change once
`crates/commands` and the Phase 4 command bar build on these types:

- No `serde::Serialize` on any model type. The screenshot pass (ADR 0007) must
  write `bounds`/`keytip`/`size_variant` back; with today's API the app would
  have to hand-roll YAML emission (as xtask does) or we add `Serialize` later and
  churn. Deriving `Serialize` alongside `Deserialize` is cheap now.
- `CommandId::new` wraps without validation and there is no `CommandId::parse`
  on the runtime side; the ADR 0006 grammar lives only in `ui-spec/schema.json`.
  `crates/commands` will construct ids in code, where the schema never runs.
  A `parse`/`validate` (shared regex or hand-rolled) would close the gap.
- Two layout switches coexist: `LayoutSets::get` (`lib.rs:707`) and
  `LayoutSet::select` (`lookup.rs:97-110`). Keep one; `get` is the better shape.
- Stale doc: `CommandLookup::empty` says "P1-06 will provide a real constructor"
  (`lib.rs:735`) although `CommandLookup::build` shipped in P1-06; the
  `CommandLookup` struct docs (`lib.rs:715-726`) still speak of P1-06 in the
  future tense too.
- `LoadError::Unknown` (`error.rs:72-83`) is the only variant without a `path`
  field; its message embeds the path as prose, so callers that match on paths
  cannot see it.

## Verdict

**Approve with fixes.**

The Phase 1 code is careful, deterministic and unusually well tested for a
first phase; the two majors are error-reporting defects (wrong file named,
wrong error class), not wrong extraction results — the committed data is
correct and regeneration is byte-stable. Findings 1 and 2 should be fixed
before Phase 2 copies the extractors' error-handling patterns again; the
minors can ride along with the Phase 2 (codegen) work.

Counts: **0 blocker, 2 major, 8 minor**.
