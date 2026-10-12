# 0009: xtask extraction architecture

- Status: accepted
- Date: 2026-10-12
- Task: P1-02

## Context

Phase 1 must produce every committed data file (`ui-spec/*.yaml`,
`ui-spec/*.toml`, `ui-spec/diff-*.md`, `docs/vsdx-inventory.md`,
`docs/parity/controls.md`, `ui-spec/shapesheet.toml`) from the raw
downloads under `reference/` with a single command: `cargo xtask extract`.

The raw data sources are independent, in different formats (`.xlsx`,
`.md`, `.html`, XSD embedded in markdown), and have different licenses and
provenance (P1-01). The extractors must:

- run in any order without coupling (so P1-03, P1-04, P1-05 can run in
  parallel),
- be deterministic (same input → byte-identical output, so a diff is a
  faithful change review),
- never copy Microsoft-owned prose (clean-room rule 4),
- fail clearly when `reference/` is missing or a single source is
  malformed (no panics; AGENTS.md section 6),
- be testable in isolation (each module has a public `run(input, output)
  -> Result<()>` and unit tests with a fixture).

## Decision

`xtask/src/extract/` has one module per source plus a top-level
`mod.rs` dispatcher. The xtask binary exposes `cargo xtask extract`
which:

1. Resolves the input directory (`reference/`, from the workspace root;
   override with `--reference <path>`).
2. Resolves the output directory (`ui-spec/` and `docs/`, in the
   workspace root; override with `--out <path>`).
3. Runs each extractor in order:
   - `fluent` → `panel-2016.yaml`, `panel-365.yaml`,
     `parity/controls.md` (P1-03).
   - `visuicmds` → `commands.yaml` (VisUICmds half) (P1-04).
   - `shapesheet` → `shapesheet.toml` (P1-04).
   - `shortcuts` → `shortcuts.yaml` (P1-04).
   - `vsdx_schema` → `docs/vsdx-inventory.md` (P1-05).
   - `diff` → `ui-spec/diff-2016-vs-365.md` (P1-03, depends on the
     fluent extractor having written both layouts).
4. Validates every output against `ui-spec/schema.json` (via the same
   `jsonschema` instance the loader uses; loaded once, reused).
5. Prints a one-line summary per extractor (`OK`, `SKIPPED (not
   implemented: P1-NN)`, or `FAIL: <message>`).
6. Returns a non-zero exit code if any extractor fails or if any
   produced file fails validation.

### Module layout

```
xtask/src/extract/
  mod.rs           # pub fn run(args: ExtractArgs) -> Result<()>
                   # struct ExtractArgs { reference, out, skip, only, allow_unimplemented }
  fluent.rs        # Office Fluent UI control identifiers -> visio-*.yaml
                   #   pub fn run(in_dir, out_dir) -> Result<()>
                   #   fn parse_xlsx(path: &Path) -> Result<Vec<FluentRow>>
                   #   fn rows_to_layout(rows, source_meta) -> Result<Layout>
                   #   fn write_layout(layout, path) -> Result<()>
  visuicmds.rs     # VisUICmds markdown -> commands.yaml (VisUICmds half)
  shapesheet.rs    # ShapeSheet reference markdown -> shapesheet.toml
  shortcuts.rs     # keyboard-shortcuts-visio.html -> shortcuts.yaml
  vsdx_schema.rs   # MS-VSDX embedded XSD -> docs/vsdx-inventory.md
  diff.rs          # panel-2016.yaml vs panel-365.yaml -> diff-2016-vs-365.md
  parity.rs        # both layouts + commands.yaml -> docs/parity/controls.md
  schema.rs        # JSON Schema loader and validator; shared by mod.rs and by
                   # the per-extractor validation step.
  util.rs          # deterministic writers (sorted keys, sorted rows), path
                   #   validation, sha256, fixture helpers.
```

### Each `run` signature

```rust
pub fn run(in_dir: &Path, out_dir: &Path) -> Result<(), ExtractError>;
```

- `in_dir` is the workspace `reference/` symlink target.
- `out_dir` is the workspace root (so extractors can write to
  `ui-spec/`, `docs/`, and `docs/parity/`).
- Errors are an `enum ExtractError { Missing { path }, Parse { source, message }, Io { path, source }, Schema { path, message } }` and implement `std::error::Error` + `Display`.
- The top-level dispatcher collects extractor errors and reports them
  grouped by extractor. A panic in an extractor is caught at the
  dispatcher level (a `std::panic::catch_unwind`) so a buggy extractor
  does not kill the rest.

### Stub phase (this task)

For P1-02 the per-source modules are stubs: each declares the
`pub fn run` signature, a doc comment listing the input path(s) under
`reference/`, the output path(s) under the workspace, and returns
`Err(ExtractError::Unimplemented("P1-NN".into()))`. The dispatcher
treats `Unimplemented` as a soft warning (printed, not fatal) **only
when** `--allow-unimplemented` is passed. Without the flag, an
unimplemented extractor is an error.

The P1-02 stubs are deliberately cheap: 5 lines of code, 30 lines of
doc comment, and a unit test that asserts
`extract::fluent::run(...)` returns the expected error message.

### Determinism

- Every output writer uses `BTreeMap` for ordering of maps.
- Lists are sorted by a stable key (the id).
- YAML is emitted with `serde_yaml::to_string` (canonical form: keys
  sorted alphabetically because the input `BTreeMap` is sorted).
- Markdown and the parity matrix are templated with sorted rows.
- Every extractor has a unit test that runs the same input twice and
  asserts byte-equality of the output (where P1-NN fills them in).

### Validation

- The dispatcher loads `ui-spec/schema.json` once and holds the
  compiled schema in a `Arc<JSONSchema>`. Each extractor that produces
  a layout, commands, or shortcuts file calls
  `schema::validate(path)?` on the file it just wrote.
- A failed validation aborts that extractor and marks the run as failed.
- A `cargo xtask extract --schema-check` flag re-validates every
  committed file under `ui-spec/` without re-running the extractors.
  CI uses this to catch hand-edits.

### Behaviour when `reference/` is missing

- The top-level `run` checks `in_dir.is_dir()` once and returns
  `Err(ExtractError::Missing { path: in_dir.to_path_buf() })`
  with the message "reference/ not found: <abs path>. Run
  `cargo xtask corpus` first, or pass --reference <path>.".
- No extractor is invoked, no panic, no stack trace.

### Labels only, never descriptions

- The extractors copy **labels** (control name, group name, tab name,
  cell name) and structural fields (ids, types, parents, indices).
- **Descriptions** in the upstream markdown are written only to
  `description_source` (a relative path under `reference/`) and never
  embedded. P1-04 will not commit any description text. The P1-02
  schema enforces this: there is no `description` field on any
  generated entity.
- The exception is `description_source` itself, which is a path string,
  not content.

### Concurrency

The dispatcher runs extractors sequentially. A typical run takes a few
seconds. Parallelism would only complicate the failure-reporting story
and add a `tokio` dependency for no real gain. If a future phase needs
parallel extractors, the dispatcher can be wrapped in
`rayon::iter::into_par_iter` without changing the per-extractor API.

## Consequences

- P1-03..P1-05 can each take one module file, write it, and the rest
  of the workspace is unchanged. P1-12's `corpus` arm uses the same
  dispatcher pattern.
- `cargo xtask extract --allow-unimplemented` is the green state
  during Phase 1: every extractor reports SKIPPED with a task id, and
  the dispatcher exits 0. After P1-03..P1-05 merge, the flag is dropped
  from CI.
- The `description_source` field means a future tool that wants the
  full description can resolve it from `reference/`, without us
  committing the prose.
- A new extractor is a new file under `xtask/src/extract/`, one line
  in `mod.rs`, and a list entry in the dispatcher's run order. No
  other wiring.
- The dispatcher and the per-extractor modules are MIT-licensed, like
  the rest of xtask. The same deny.toml check that passes today
  continues to pass.

## Alternatives considered

- **A single mega-file `xtask/src/extract.rs` with one function per
  source.** Rejected: makes parallel work impossible and grows to
  thousands of lines.
- **Run each extractor as a sub-`cargo xtask` invocation.** Rejected:
  the compile cost of N binaries is large and the data is too small to
  justify it.
- **Use `tokio` and run extractors in parallel.** Rejected: 10× the
  complexity, 0.5 s saved.
- **Have the extractors write to `target/` and copy on success.**
  Rejected: makes `git diff` harder; the files are committed.
- **Use a build script in `xtask/build.rs` to discover extractors.**
  Rejected: the list of extractors is small and stable; a `match` in
  `mod.rs` is the simplest and the easiest to read.
