# Phase 6 coverage audit

- Task: P6-13 (coverage-auditor), branch `agent/P6-13`, 2026-10-23.
- Scope: the whole phase, with the new surface named in the task:
  `crates/shapesheet` (the whole engine), `crates/model/src/values.rs`
  (the `Inh` snapshot policy), `crates/commands/src/recalc.rs`,
  `crates/vsdx` value spelling, `crates/ui/src/panels.rs`.
- Numbers below come from this run in the worktree (quoted as required):

```sh
cargo llvm-cov --workspace --all-features
```

  which `cargo xtask coverage` executes as
  `cargo llvm-cov --workspace --all-features --json --summary-only
  --output-path target/llvm-cov-summary.json`. Per-file detail was taken
  from `cargo llvm-cov report --lcov --output-path target/lcov.info`
  (same profile data, no re-run). Zero-hit lines are exact; llvm-cov
  folds partially covered lines into the covered count, so a file's
  misses understate its dead arms. Inline `#[cfg(test)]` code counts as
  instrumented lines, as in Phases 4 and 5.

## Per-crate results

| Crate | Lines | Covered | Coverage | Threshold | Phase 5 | Phase 5 ratchet | Delta |
|---|---:|---:|---:|---:|---:|---:|---:|
| ferrogram-geom | 2501 | 2501 | 100.0% | 85 | 100.0% | 100.0 | ±0.0 |
| ferrogram-model | 11234 | 11204 | 99.7% | 85 | 99.7% | 99.7 | ±0.0 |
| ferrogram-ui-spec | 2043 | 2031 | 99.4% | 85 | 99.4% | 99.3 | ±0.0 |
| ferrogram-text | 1768 | 1748 | 98.9% | 75 | 98.9% | 98.8 | ±0.0 |
| ferrogram-render | 7221 | 7114 | 98.5% | 75 | 98.5% | 98.5 | ±0.0 |
| ferrogram-commands | 9537 | 9372 | 98.3% | 85 | 98.3% | 98.3 | ±0.0 |
| ferrogram-vsdx | 7722 | 7583 | 98.2% | 85 | 98.1% | 98.1 | +0.1 |
| ferrogram-cli | 1266 | 1225 | 96.8% | 50 | 96.8% | 96.7 | ±0.0 |
| ferrogram-ui | 11671 | 11324 | 97.0% | 50 | 96.8% | 96.8 | +0.2 |
| ferrogram-shapesheet | 6733 | 6433 | 95.5% | 85 | 100.0% (45 stub lines) | 100.0 | n/a, see below |
| xtask | 10443 | 9757 | 93.4% | — | 93.4% | 93.4 | ±0.0 |
| ferrogram | 250 | 200 | 80.0% | 50 | 80.0% | 80.0 | ±0.0 |
| ferrogram-web | 140 | 109 | 77.9% | — | 77.9% | 77.8 | ±0.0 |
| ferrogram-control | 0 | 0 | n/a | 50 | — | — | placeholder |
| ferrogram-vsd-binary | 0 | 0 | n/a | 85 | — | — | placeholder |

Phase 5 numbers are the table in `docs/progress/phase-5.md` (measured at
the phase end, after the P5-19/P5-20 fixes); the ratchet column is
`[ratchet]` in `coverage.toml` as of the end of Phase 5 — the
`ferrogram-shapesheet` row shows 100.0 there because Phase 5 measured 45
all-stub lines; the plan reset that one entry to 85.0 when P6-01
merged. Deltas are against the Phase 5 recorded percentage, one decimal
place. The `ferrogram-shapesheet` delta is meaningless on its face (100%
of 45 stub lines vs 95.5% of 6 733 real lines) and is marked n/a.

The run reproduces `docs/reviews/phase-6-tests.md`'s table exactly
except `ferrogram-render` (7 114 covered here vs 7 113 there, one line;
both round to 98.5%).

## Statement per crate

- **At threshold and at ratchet (exact):** geom (100.00, 2 501/2 501),
  ferrogram (80.00), xtask (93.43 vs 93.4). Zero margin, but not below.
- **Above threshold and above ratchet:** model (99.73 vs 99.7),
  ui-spec (99.41 vs 99.3), text (98.87 vs 98.8), render (98.52 vs 98.5),
  vsdx (98.20 vs 98.1), cli (96.76 vs 96.7), ui (97.03 vs 96.8),
  web (77.86 vs 77.8).
- **Above threshold, below ratchet inside the gate's 0.05 tolerance
  (blocker under the strict rule of the role file):**
  `ferrogram-commands` 9 372/9 537 = **98.27%** against the 98.3
  ratchet, 0.03 pp below it. This is the same crate P5-13 flagged with
  the same 0.03 pp debt; the Phase 5 end-of-phase ratchet update moved
  the number up to the rounded measured value and kept the debt. The
  gate is green only because `cargo xtask coverage` tolerates 0.05 for
  float noise. One uncovered line in `ferrogram-commands` is about
  0.01 pp: the fix tasks below should give this crate its margin first.
- **`ferrogram-shapesheet`:** 6 433/6 733 = 95.54%, far above its 85
  threshold. Its ratchet (85.0) is the Phase 5 all-stub number and is
  stale: it does not describe the engine that exists now. It must move
  (proposal below). The engine's gaps are real but narrow (88.9% in
  `resolve.rs` is the one file to watch).
- **Placeholders:** control and vsd-binary have zero instrumented lines
  and pass by the "empty placeholder" rule (vsd-binary's 85% threshold
  becomes meaningful in Phase 8).

## Ratchet proposal for the master (numbers only)

`cargo xtask coverage --update-ratchet` floors to one decimal place.
Applying that to this run and keeping a ratchet from ever dropping
(AGENTS.md section 7):

| Crate | Current | Proposed |
|---|---:|---:|
| ferrogram | 80.0 | 80.0 |
| ferrogram-cli | 96.7 | 96.7 |
| ferrogram-commands | 98.3 | 98.3 |
| ferrogram-geom | 100.0 | 100.0 |
| ferrogram-model | 99.7 | 99.7 |
| ferrogram-render | 98.5 | 98.5 |
| ferrogram-shapesheet | 85.0 | **95.5** |
| ferrogram-text | 98.8 | 98.8 |
| ferrogram-ui | 96.8 | **97.0** |
| ferrogram-ui-spec | 99.3 | **99.4** |
| ferrogram-vsdx | 98.1 | **98.2** |
| ferrogram-web | 77.8 | 77.8 |
| xtask | 93.4 | 93.4 |

The floor is below the current value only for `ferrogram-commands`
(98.2); keeping 98.3 there refuses to accept the 0.03 pp drop, the same
judgement P5-13 made. The `ferrogram-shapesheet` move is the one the
task names: the ratchet is the Phase 5 all-stub number and must land on
the measured engine number (95.5).

## Critical uncovered paths

Severity follows the role file: blocker (below threshold or ratchet),
major (a critical path no test reaches), minor (defensive or
line-table). Line numbers are from this worktree.

### Recalc failure paths

- **major** `crates/shapesheet/src/recalc.rs:233` — a hard evaluation
  error (not a syntax error) aborts the whole pass with
  `Err(error) => return Err(error)`. The P6-11 pass made a bad formula
  a warning instead of a broken engine, so no test can now make an
  evaluation fail hard; the abort path and what it leaves behind (a
  pass that stopped half-way) are unproven. Combined with the command
  arm below, the "a broken engine is one warning, never a lost step"
  contract is only half tested (the `wrap` side, `recalc.rs:163-164`,
  is covered).
- **major** `crates/shapesheet/src/recalc.rs:558` —
  `Err(BadCell("row without a section"))` when a `CellAddress` names a
  row without its section. Hostile or hand-built input must produce an
  error, never a panic (AGENTS.md section 6); nothing feeds such an
  address through the write path today. One test with a malformed
  address closes it.
- **major** `crates/shapesheet/src/recalc.rs:443` — the `?` on the
  per-cell write of the pass: a write the model rejects aborts the pass
  at the cell. Same chain as 233, and the same absence of a test that
  can make a write fail.
- **minor** `crates/shapesheet/src/recalc.rs:250-253` — the
  `MissingReference` warning with `target: None` (a reference no name
  can describe). The named form is covered by the nine broken-reference
  tests; this form is not.
- **minor** `crates/shapesheet/src/recalc.rs:416` — the
  `done.contains(&address)` skip of the pass loop (dedup arm).
- **minor** `crates/commands/src/recalc.rs:252` — "the document has no
  dependency graph" (the graph is rebuilt two lines above; defensive
  pair of the `?` beside it).
- **minor** `crates/commands/src/recalc.rs:274-276` — `engine_error`'s
  body: reached only when `DocumentEngine` itself fails. With the P6-11
  change nothing breaks an engine on document data any more, so this
  may be permanently dark. Keep (it is the mapping the two arms above
  rely on) and say so in a comment.
- **minor** `crates/commands/src/recalc.rs:431` — the
  `u32::try_from(index)` overflow `continue` in the dirty-address walk
  (checked arithmetic, AGENTS.md section 6; unreachable on 64-bit).
- **minor** `crates/commands/src/recalc.rs:476` — `_ => return true`
  in `reads_or_calls` (the `ExprKind` is `#[non_exhaustive]`): a future
  expression kind silently counts as "reads or calls", so the router
  treats its geometry as formula-driven. Worth one pin test when the
  next kind lands; keep meanwhile.

### Undo of recalculated cells

- **major** `crates/commands/src/recalc.rs:211` — the redo arm of
  `RecalcEdit::apply`: `Err(error) => ctx.record_warning("recalculation
  failed: …")`. Undo restores the snapshot (covered,
  `tests/undo_recalc_prop.rs` and `tests/recalc_edit.rs`) but **redo**,
  which re-runs the step and the pass, has no test whose re-run fails.
  The ADR 0056 contract "one user action is one undo step" includes the
  redo direction; a redone action whose pass now fails must keep the
  step and warn, not lose the edit. One test through a stub step whose
  second apply fails closes it (the inline `StubEdit` in
  `crates/commands/src/recalc.rs:857` is the natural place).
- **major** (with `resolve.rs` below) SETF undo of a section-level,
  row or page cell: `write_slot`'s non-direct arms
  (`crates/shapesheet/src/resolve.rs:1291-1299`) and `sheet_mut`'s page
  arm (`1278-1282`) never run, so the snapshot restore of such a write
  is untested too. The snapshot is per sheet (`SheetSnapshot`), so the
  restore itself is sound; what is unproven is the write-then-restore
  round trip for anything but a direct cell.

### Coercion edges

- **major** `crates/shapesheet/src/lexer.rs:238` — a formula literal
  whose spelling parses to a non-finite float (`=1e999`) is an
  `InvalidNumber` **syntax error**. Hostile input, and the twin of the
  reader path P6-11 pinned (a `V` attribute of `1e999999`, `inf` or
  `nan` reads as the IEEE value). No formula test feeds a huge literal,
  so the two sides' disagreement (reader accepts, lexer rejects) is
  unproven on the formula side. One parser test with `1e999`.
- **major** `crates/shapesheet/src/eval.rs:911-912` — a value with a
  non-finite magnitude (a stored `inf`/`NaN` read from a file, or an
  operand that became one) is turned into `#NUM!` with one `NonFinite`
  warning. This is exactly the path P6-11's "not reached" note names:
  only the computed-result path is tested
  (`eval.rs:946-949` in `coerce_num`, also uncovered here). One test
  with a hand-built cached `1e999` value closes both.
- **major** `crates/shapesheet/src/eval.rs:382-388` — the exponent
  rules of `^`: `x^0` (`384`), `x^1` (`385-386`) and `x^2` and up →
  `#DIM!` (`388`, with `371` the inverse-unit arm of the same match).
  Unit arithmetic on powers is an ADR 0052 table; none of its three
  rows is tested.
- **minor** `crates/shapesheet/src/eval.rs` — the untested function
  arms: `629` (log of a dimensioned number → `#DIM!`), `635` (`LN`;
  `LOG10` is covered), `650-652` (`SINH`, `COSH`, `TANH`),
  `659` (inverse trig of a dimensioned number → `#VALUE!`),
  `666-667` (`ACOS`, `ATAN`; `ASIN` is covered), `675` (`ATAN2` of two
  different families → `#VALUE!`), `690` (`DEG`/`RAD` of a non-angle →
  `#VALUE!`), `744` (`extreme`'s no-argument arm), `770` (`sum`'s empty
  arm), `784` (`rounded`'s truncate path), `807` (`modulus`'s
  family-merge failure). One test each; the ADR 0053 family table is
  otherwise pinned.
- **minor** `crates/shapesheet/src/eval.rs` — the arity edges:
  `471` (`IF()` with no arguments), `485` (`IF(FALSE, 1)` with no else
  branch evaluates to `#VALUE!` — the behaviour is defined, untested),
  `493`/`499` (`INDEX`/`LOOKUP` without the fallback), `509` (`SETF`
  with one argument).
- **minor** `crates/shapesheet/src/eval.rs` — defensive arms: `266`
  (unary over a kept-cached operand → `KeepCached`), `320`
  (`MAX_TEXT_LEN`, the bounded-allocation rule), `394` (concat decided
  earlier), `458` (warn-once dedup), `716` (function kind not yet
  implemented), `965` (non-boolean coerces to `false` in `coerce_bool`),
  `998`/`1011`/`1024` (split with an empty delimiter, a string arm, a
  unit-family lookup failure).
- **minor** `crates/shapesheet/src/coerce.rs:142`, `146`, `199` —
  `FALSE` → `"FALSE"`, an error value → its text (`to_str`), and
  `as_str`'s conversion error when the coercion defaulted. The `TRUE`
  twin of 142 is covered; that is what makes 142 worth one test.
- **minor** `crates/shapesheet/src/value.rs:215` — a cached `"FALSE"`
  spelling reads as `Value::Bool(false)` (the `"TRUE"` twin at 212 is
  covered by the round-trip properties). `182` (`#NUM!` cached
  spelling), `256` (ratio → percent unit), `333`
  (`Conversion("cached value kind")`, `from_cached` on an unknown kind
  — a malformed-input guard that deserves one test), `units.rs:534`
  (inside the test module).

### Resolution fallbacks (`resolve.rs`, 88.9%, the biggest hole)

- **major** SETF into anything but a direct shape cell is untested, as
  one cluster: `resolve.rs:1005-1030` (`section_slot`, entirely dark),
  `621-639` (`set_cell`'s section arm; `612` and `639` are the two
  `SetfRefused` errors — a SETF aimed at `TheDoc!` or a style, and a
  SETF naming a row that does not exist), `1291-1299` (`write_slot`'s
  `SectionLevel`/`Row` arms) and `1278-1282` (`sheet_mut`'s page arm).
  P6-11's SETF tests all write direct cells. `SETF(Geometry1.X1, …)`,
  `SETF` into a `Prop` row and `SETF` into a page cell are the writes a
  real drawing makes; the P6-08 spelling and the undo snapshot both
  hinge on them. Three tests close the whole cluster.
- **major** `resolve.rs:403-423` (`rung_unit`) — both `return found.unit`
  arms (`410`, `420`) are dark: the function only ever falls through to
  `Unit::Unitsless`. The unit the defining rung recorded (a master
  `Width` in millimetres, say) is therefore untested *and* apparently
  unused in anger. One fixture master with a non-inch unit would pin
  the behaviour or expose the bug.
- **major** `crates/shapesheet/src/graph.rs:712` — the match arm of
  `find_shape_by_name` that finds a shape (`Sheet!Foo.Width` naming a
  shape on the page) never runs, so a formula that references another
  shape **by name** builds no dependency edge. The resolver side of
  name references is tested (`tests/resolve.rs`); the graph side is
  not, and the graph is what the recalc order depends on. One test with
  a cross-shape reference by name, asserting the dependent recomputes.
- **major** `crates/shapesheet/src/graph.rs:764-772` — `bare_address`'s
  row arm: a bare name that matches a row cell (`Width`) resolves to a
  row address. Dark, so a formula reading a row cell by bare name has
  no edge either. Same fix shape as above.
- **major** `resolve.rs:578` — a stored formula that does not parse on
  a cell that caches nothing reads as `#REF!` (the resolver's own eval
  fallback). P6-11 pinned the engine rule ("keeps its cached value");
  this is the read-side rule for the no-cached-value case and is
  exactly where a hostile file lands. One test with a malformed
  formula and no cached value.
- **major** `resolve.rs:1162-1169` (`sibling_position`) — the parent
  arm (a shape finding its position among its parent's children) is
  dark; the ADR 0013 sub-shape position rule is untested for a grouped
  instance. `1187-1191` (`master_shape`'s multi-candidate arm: the
  position pick, then the first) is the sibling gap: a master with two
  same-named sub-shapes.
- **minor** `resolve.rs:334-348` — `find_bare_in`'s `Inh`-snapshot
  return (`337-341`) and hard-default fallback (`345-348`): the
  style-sheet counterparts of the sheet paths P6-11 tested. `287` (a
  style's section cell, `Line.Color` on a style ref), `243`
  (`page_context` for a style/document target), `300` (a document
  target with a section, defensive).
- **minor** `resolve.rs:876-882` (`split_names`' "a section the parser
  left in the name list"), `780`/`851` (the never-failing `parse`
  arms of the name splitters), `915-929` (`find_entry`'s section-level
  and row slots; the direct slot is covered), `1235-1238`
  (`shape_by_visio_id`'s `name_u` base-name match, the `Foo.3` form),
  `1261`/`1264` (`walk_shapes` bound and missing-shape guards),
  `1111` (`master_rungs` cycle/bound `break`), `1141`
  (`instance_chain_start`'s visited guard), `1066` (`column_kind`'s
  `None` arm), `757` (brace artefact in `split_row_name`).
- **minor** `crates/shapesheet/src/graph.rs` — `462`/`547` (row without
  a section), `505` (a page has no master), `671`/`677` (the page arms
  of the page-context helpers: references *from* a page-level cell),
  `629-631` (`canonical_row` fallback for a suffixed row name), `641`
  (catch-all), `704`/`707` (walk guards), `409` (topo-sort push),
  `783`/`786` (`is_column`'s missing sheet/section arms), `946-950`
  (`base_name`'s `Row[ix]` spelling), `305`/`401` (brace artefacts).

### Writer (and reader) value spelling

- **major** `crates/vsdx/src/read/value.rs:159` — a `V` attribute
  declared `COLOR` whose text `parse_color` cannot parse keeps the raw
  text as a `Str`. Malformed input silently changes the value's kind;
  one fixture with junk color text pins the fallback (and the warning,
  if the reader records one).
- **minor** `crates/vsdx/src/write/value.rs:136`, `155` — the future
  `CellValue` kind arms of `declares_kind` (no kind declaration) and
  `encode_value` (the empty string, never a guessed spelling). The doc
  comments state the contract; the enum kind can be added only in this
  crate, so the arms wait for that day. Keep.
- The P6-11 open item stands and is cited here so the fix tasks see it
  once: the reader accepts `1e999999`/`inf`/`nan` in a `V` attribute
  (`crates/vsdx/src/read/value.rs`, `f64::parse`) while the writer
  rejects non-finite numbers (`mutate::checked_number`). A hardening
  (reject on read) should change P6-11's two pinned tests deliberately.

### Panels (`crates/ui/src/panels.rs`)

- **major** `panels.rs:1960-1961` — the formula read-only marker
  (`FORMULA_MARKER` label with `FORMULA_HINT` on hover) never renders
  in any `egui_kittest` run. The ADR 0058 rule "formula cells are
  read-only" is tested at the policy layer; the affordance that tells
  the *user* is untested UI. One panel test that opens a formula cell.
- **minor** `panels.rs:795` — the entry error for a non-finite number
  (`"nan" is not a finite number`), the twin of the reader edge above.
  `489` ("not a number" reason for a non-finite row), `640`/`643`
  (a list-valued cell's display; the non-exhaustive `Value` kind's
  "no display" answer), `572` (Shape Data display fallback),
  `991` (`formula_computes`' catch-all: a future `ExprKind` counts as
  computing, keeping the cell read-only — pin it when the kind lands),
  `1036` (a stored formula that does not parse is *not* read-only).
- **minor** `panels.rs:1467`, `1484`, `1554`, `1843` — closing braces of
  the P6-09 panel tests (line-table artefacts, not dead code).

### Test-code noise (minor, same class as P5-15's note)

- `crates/commands/src/recalc.rs:860-862` (`StubEdit::label` never
  called), `627`/`908` (`_ => None` arms of the inline test helpers),
  `769`/`980` (assert closing braces).
- `crates/shapesheet/src/recalc.rs:642-647` (the string-skip loop of
  `#[cfg(test)] testing::scan_refs`; a formula with a quoted `"` would
  reach it), `parser.rs` test-module arms (`803`, `893`, `907`, `917`,
  `944`, `962`, `970`, `978`, `1103`), `eval.rs` test-module arms
  (`1041-1054`), `lexer.rs:400`, `value.rs:435`, `units.rs:534`.
- `crates/shapesheet/src/parser.rs` product-code arms worth one test
  each, listed here rather than above because they are the same shape:
  `146`/`154` (span fallbacks at end of input and with no tokens),
  `179-181` (`error_at` and its two dark callers `528` — an empty
  reference — and `626`), `202` (the alternative-operator probe),
  `385` (`None` token arm), `417` (paren mismatch), `462` (parse probe
  early exit), `505-508` (expression parse failure, `UnexpectedEnd` /
  `UnexpectedToken`), `608-610` (the unit-suffix spelling
  `5 mm` round trip), `668-674` (mixed-fraction continuation breaks),
  `733-739` (`offset`'s saturation arm).
  `crates/shapesheet/src/printer.rs:65-68` (`MAX_SOURCE_LEN` →
  `InputTooLong`, the printer's capped-allocation rule).

## Gate

- `cargo xtask coverage`: green (the per-crate table above; every crate
  at or above its threshold and, within the 0.05 tolerance, its
  ratchet; `ferrogram-commands` is the one crate under its ratchet on
  exact numbers).
- `cargo xtask gate --quick`: green in the worktree (codegen, parity,
  fmt, clippy, test, doc, deny, layers). No code changed in this task,
  so wasm and fuzz were not triggered and coverage is not part of
  `--quick`; it was run separately as quoted above.
