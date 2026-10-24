# Phase 6 code review (P6-12)

Task P6-12, reviewer, 2026-10-23, branch `agent/P6-12`. Review of the
Phase 6 diff (`git diff c6bd729..cea619f`, the phase-5 end to the branch
tip; the interleaved P7-01 docs, layout data and stubs are out of scope)
against AGENTS.md section 6, the layering (ADR 0002) and ADRs 0051-0060
plus the ADR 0013 and 0057 amendments. The reviewer role is read-only:
this task changed no code.

Scope covered: `crates/shapesheet` (`lexer`, `parser`, `ast`, `printer`,
`formula`, `value`, `units`, `coerce`, `functions`, `eval`, `resolve`,
`graph`, `recalc`, `error`, the fuzz targets), `crates/model` (`lookup`,
`values`, `geometry`, `mutate`), `crates/commands` (`recalc`, `app`,
`edit`, `undo`, `builtins/{edit,file,insert,shape,arrange,connector}`),
`crates/vsdx` (`read/{mod,sheet,value}`, `write/{mod,value,patch}`),
`crates/ui/src/panes.rs`, the ADRs 0051-0060, `about-operators.md`
against the grammar, and the P6-11 product fixes one by one (see
"Checks that passed").

## Verdict

**Approve with fixes: 0 blocker, 2 major, 15 minor.** The phase's hard
rules hold: the grammar and its precedence table match
`about-operators.md`, the parser is bounded by three tested limits with
`get()` throughout, evaluation follows the one coercion rule with errors
as values and the unknown-function stand-back, resolution splits formula
from value and keeps provenance with the `Inh` snapshot, recalc is
deterministic with a cycle cut and a budget, and one user action is one
undo step through `push_edit`. The two majors are one write path that
escapes the recalc pass (a nested `SETF` can survive undo) and two
different column-versus-row rules (the dependency graph can miss an edge
the resolver uses, leaving stale values after an edit).

## Checks that passed

- **Grammar against ADR 0051 and `about-operators.md`.** The precedence
  chain is the source table's order exactly: unary `+ - %`, then `^`
  (right-associative), `* /`, `+ -`, `&`, the ordering comparisons and
  the equality comparisons last (`parser.rs:207-330`); `^` is the only
  right-associative operator. The six alternative forms (`_GT_` family)
  map to the same six operators (`alt_operator`, `parser.rs:58-75`).
  Every reference form of the ADR 0054 table parses and prints back
  (`tests/parser.rs::every_reference_form_of_the_table_parses_and_prints_back`),
  the parts stay verbatim (`CellRef`), names fold case only at lookup.
  976 of 994 unique corpus formulas parse and round-trip (P6-02).
- **Never-crash in the parser crates.** `MAX_SOURCE_LEN`/`MAX_TOKENS`/
  `MAX_NESTING` are checked before allocation grows and each has a
  dedicated test (`lexer.rs:620-650`); `Parser::enter` bounds parens and
  brackets, `take_bracket_group` and `call_end` are iterative, and the
  evaluator and the resolver each carry their own depth bound
  (`eval.rs:69`, `resolve.rs:42`). `collect_refs`, `find_cycles`,
  `topological_order` and every document walk are iterative with visited
  sets and capped counts. No `unwrap`/`expect`/`panic!` outside test
  modules in the new code (full scan of `crates/*/src`); input-derived
  positions go through `get()`/`split_first()` (`arg_at`, `index`,
  `rgb`), and `Span`/`ByteOffset` saturate instead of wrapping. The gate
  runs `clippy::indexing_slicing` denied and is green.
- **Evaluation semantics (ADR 0052, 0053, 0056).** One coercion rule
  (`coerce.rs`), errors as values with `blocked()` short-circuiting,
  non-finite numbers killed at the one place values are produced
  (`num_value`/`to_cached`), wrong arity checked before any argument is
  evaluated (so no `SETF` side effect fires), `IF` lazy and `IFERROR`
  not catching the stand-back, `GUARD` a value-wise no-op, `SETF`
  refusing a non-reference target with `SetfRefused`. The unknown-function
  rule is as ADR 0053 says: one `UnknownFunction` warning per formula
  naming the first offender (`stand_back`), the whole evaluation stands
  back and the pass keeps the cell's cached value
  (`recalc.rs:444-458`). `RAND` is absent from the table (pinned by
  `functions.rs:364`).
- **Resolution (ADR 0013 + 0054).** The rungs run local, master (with
  the sub-shape and position rules), style chain (with P6-15's effective
  entry link), section default, hard default; `Inh` defines nothing and
  the value side prefers the local snapshot (including an `Inh` one)
  while provenance stays on the formula side. The provenance promise has
  its named test (`values.rs:2572 an_inh_snapshot_is_the_value_and_the_master_stays_the_source`)
  and the row/section snapshots have theirs. `section_rung` and
  `graph::effective` (P6-11) are correct: a rung without the section no
  longer stops the chain, and a section-level cell is addressed as one.
- **Recalc (ADR 0055).** Deterministic: `BTreeSet`/`BTreeMap` through
  the pass, the address order as the stable tie-break, and
  `tests/recalc_corpus.rs` runs every corpus file twice to the same
  report. Cycles are cut (the cells keep their cached values, one
  `Cycle` warning per cycle, the pass never loops), the budget sets
  `truncated` with one `BudgetExceeded` warning, and `formula_text`
  implements the materialisation split of ADR 0056 (own formula or
  materialised rung formula; a cell with no local entry keeps no
  half-written entry). Writes go through `Document::write_cell` /
  `write_section_cell` / `write_row_cell` only. A read prefers the cached
  value (P6-10) and is safe here: the pass writes each computed value
  before dependents evaluate and the order is dependencies-first
  (`recalc.rs:375-465`) — with the caveat of finding F2.
- **Undo (ADR 0056, 0040).** The recalc runs inside `push_edit`
  (`app.rs:517`), one stack entry holds the step plus the pass snapshot
  (`RecalcEdit`), `revert` restores the sheet snapshots and then the
  step (the right order), redo re-runs both halves, and no second undo
  path exists (`fill_missing` pushes nothing; nothing else pushes).
  `SheetSnapshot` is per sheet, so the derived-cell refreshes
  `write_cell` makes beside a write are covered too. The ADR 0040
  property over recalculating commands (`undo_recalc_prop.rs`) passes
  over resize, move, a cell edit and a `SETF`-carrying cell — with the
  exceptions of findings F1 and F13.
- **Writer (ADR 0057).** The writer never recalculates and writes `F`
  as stored, so the ADR 0048 identity is intact (the reader's fill-in
  pass fills only cells that had no `V`, and does it at read, not at
  write). The kind spelling (`BOOL`, `STR`, `COLOR`, `LIST`, the
  unit-drop for non-numeric kinds) has round-trip tests through write
  and read (`value_roundtrip.rs` in both crates). P6-16's per-cell rule
  is well built: the reader asks the writer's own decision function
  (`read/sheet.rs:55-65 writer_spells_code`), so the two cannot drift,
  and the P5-10 artefacts are gone with `reader_artefacts.rs` proving
  the removal. `source_rel_id` recovers the contents relationship id
  from the source element, so patched files do not grow a second `Rel`.
- **P6-11's four product fixes** are correct and narrow: the
  section-aware `graph::effective`/`section_rung`, the keep-cached
  `Syntax` path (a parse failure is document data, not a broken engine),
  the four new `RecalcWarningKind` variants mapped 1:1 in
  `map_warning`, and `Document::write_section_cell` (unit rule matching
  its row sibling).

## Findings

| id | severity | file:line | problem | suggested fix |
|----|----------|-----------|---------|---------------|
| F1 | major | crates/shapesheet/src/resolve.rs:563-653 | a `SETF` in a *referenced* cell's formula writes through `DocumentResolver::set_cell` (`write_slot`) during another cell's evaluation: the write bypasses `Evaluation::writes`, so it lands outside `apply_write` (no `CellUpdate`, no derived refresh, no dirtied dependents) and outside the `SnapshotEngine` snapshot — `RecalcEdit::revert` cannot restore it | make the resolver *record* writes and let the pass apply them (the `RecordingContext` shape, for nested evaluations too), or evaluate referenced formulas with a read-only context; see "F1 reproduces" below |
| F2 | major | crates/shapesheet/src/graph.rs:781 (`is_column`) vs crates/shapesheet/src/resolve.rs:1035 (`column_known`) | two different column-versus-named-row decisions: the resolver consults the ShapeSheet table and the section's level cells, the graph only the cells the file wrote in rows. Where they disagree, `resolve_ref_address` (`graph.rs:610`) answers `None`, the formula gets no dependency edge, and the pass can evaluate it before its dependency (address tie-break) or never re-evaluate it after an edit — silently stale values | one shared "what does this name mean against this section" helper in `shapesheet`, used by both the resolver and `resolve_ref_address` (also the twin name splitters: `resolve.rs` `split_row_name`/`split_section_name`/`canonical_section` vs `graph.rs` `split_row_suffix`/`split_section_number`/`spell_section`) |
| F3 | minor | crates/shapesheet/src/recalc.rs:456-476 | when an evaluation stands back (`UnknownFunction`/`Syntax`) its cell keeps its cached value but the `SETF` writes it recorded are still applied (`for op in evaluation.writes` runs outside the `keep_cached` guard) | decide one rule (ADR 0056 note): either drop the writes of a standing-back evaluation, or document that a completed `SETF` fires even when a later term stands back |
| F4 | minor | crates/shapesheet/src/ast.rs:16-18 | `Expr` derives `PartialEq`, so equality includes `span`, while ADR 0051 says spans are not part of equality; two test helpers exist only to strip spans (`parser.rs:746 without_spans`, `tests/parser.rs:18 strip`) | **ruling:** a hand-written `PartialEq` for `Expr` comparing `kind` only (the derived `ExprKind` eq then recurses through it); no ADR amendment — the ADR already states the contract |
| F5 | minor | crates/shapesheet/src/parser.rs:479-498 | omitted call arguments (`FORMATEX(a,b,,,c)`, 18 of 994 corpus formulas) are a spanned syntax error; the divergence from Visio, which wrote those formulas, is decided but recorded only in the module docs and the P6-02 Result | **ruling:** the decision holds (see "Master notes"); record it in ADR 0051 (the EBNF has no empty-argument production) and add the production when a known function needs one |
| F6 | minor | crates/commands/tests/crosscheck.rs:283-287 and :570-581 | two comparison rules the harness applies without an ADR 0059 text: the boolean spelling bridge and rule 3 read as excluding hard defaults; the second also reports a *resolved* reference as `missing-reference` | **ruling:** both behaviours stand (see "Master notes"); one ADR 0059 amendment adds the bridge row to the table and spells rule 3 as "a value the document holds", and the hard-default skip gets its own reason (`default-only`) instead of `missing-reference` |
| F7 | minor | crates/shapesheet/src/recalc.rs:430 | the budget counts only the pass's own `engine.evaluate` calls; a referenced cell without a cached value is evaluated *inside* one call (`resolve.rs:563`), so those evaluations are unbudgeted and the "deterministic budget" undercounts work in the fill-in case | count nested evaluations (a counter on the resolver) or document that the budget bounds pass-level evaluations only |
| F8 | minor | crates/commands/src/recalc.rs:279-306 | `warning_line` prints the P6-11 kinds (`WrongArgumentCount`, `SetfRefused`, `Coercion`, `NonFinite`, `Syntax`) through the fallback `"recalc: warning of X"` | **confirmed, keep rank** (queued for the fix task): add one line per kind; status-bar text only |
| F9 | minor | crates/vsdx/src/read/value.rs:164 | a non-finite numeric literal (`1e999999`, `inf`) reads as `CellValue::Number(inf)` while `Document::write_cell` refuses non-finite writes and `Value::from_cached` maps it to `#NUM!` | **confirmed, keep rank** (queued hardening): reject or normalise at read with a documented rule. No crash path today: `render/src/png.rs:659 fits_device` skips non-finite device bounds and `to_cached` guards |
| F10 | minor | crates/commands/src/builtins/file.rs:226-276 | `fill_in_values`/`missing_value_cells` re-implement the fill-in pass `commands::recalc::fill_missing` already provides (its own doc names it "the future home"), and `sheet_refs` is a third sheet walker beside `graph::sheet_refs` and `commands::recalc::document_addresses` | **confirmed, keep rank** (queued for the fix task): call `fill_missing` (or `fill_missing_with`) from the open path and delete the copies |
| F11 | minor | crates/commands/src/builtins/edit.rs:272 | `CellEdit::direct` is `#[allow(dead_code)]` with the comment "Task P6-09's pane edits are the first non-test callers", but P6-09's panes dispatch `shape.set.*` and never call it (P6-09's own open issues say so) — a stub seam that outlived its task | give it its caller (`data.set.property`) or drop the `#[allow(dead_code)]` and the comment until then; same sweep for the retired `Document::scale_geometry` (model/mutate.rs:1118), still `pub` with no product caller |
| F12 | minor | crates/commands/src/edit.rs:29 | `CompositeEdit::edits` is `pub` while the dirty union is maintained by `push`; a step pushed straight into the list silently loses its dirty cells (the doc warns about it, the type does not prevent it) | make `edits` private, or compute `dirty_cells()` from `edits` on demand so the union cannot drift |
| F13 | minor | crates/commands/src/recalc.rs:152-170 | when `run_pass` returns `Err` mid-pass (a broken engine), the writes the partial pass already made are dropped with the error: `wrap` records a warning and pushes the step with `before: Vec::new()`, so a later undo cannot remove them | return the collected snapshots alongside the error (or run the pass in two phases) so `wrap` can keep them; `Err` is near-unreachable today (`to_cached` cannot fail), so this is hardening |
| F14 | minor | crates/commands/src/builtins/insert.rs:138-147 | if the `GeometryEdit` of a sized insert fails after `instantiate_master`, the shape stays in the document with no undo entry (the glue path rolls back with `detach_shape`, the resize path does not) — the shape of phase-5 review F1 | roll the insertion back on any later staging/apply failure, or stage the `GeometryEdit` before mutating |
| F15 | minor | crates/model/src/mutate.rs:959-1105 | `write_cell`, `write_row_cell` and `write_section_cell` repeat the same sheet lookup, find-or-insert and unit rule three times | factor the shared tail (or pin the three with one shared test table) |
| F16 | minor | crates/commands/src/recalc.rs:450-455 | a warning raised while a *referenced* cell's formula is evaluated is reported against the outer cell (`warning.target` is set to the cell being evaluated), so "unknown function THEMEVAL of PinX" can name the wrong cell | carry the originating cell on `EvalWarning` (or record nested warnings against their own sheet/cell) |
| F17 | minor | crates/shapesheet/src/eval.rs:42-47; docs/adr/0051-formula-grammar-ast-and-printer.md:39 | docs nits: the evaluator's module list has a stranded sentence ("...its own warning text (ADR 0053). the caller keeps the cell's cached value."); ADR 0051's EBNF writes `%` as a prefix operator only while the parser and printer also take the postfix form (`37%`); the `_GT_` family's "full 15 digits" exactness is noted nowhere | rewrite the sentence in order; add the postfix production to the EBNF (and one line on the 15-digit comparison) |

### F1 reproduces

1. Open a file where cell B carries a `SETF`-with-a-real-target formula
   and caches nothing (the fill-in pass writes no value for it when the
   formula also calls an unknown function, e.g.
   `SETF(Width,5)+THEMEVAL(x)`: ADR 0053's stand-back keeps the cell
   empty, P6-04's open issue).
2. Give another cell A a formula that reads B (`B+1`) and edit anything
   that dirties A (or its dependencies).
3. The pass evaluates A; A's read of B evaluates B inline through
   `DocumentResolver` (`resolve.rs:563`), whose `set_cell` writes
   `Width = 5` straight into the sheet.
4. Undo the edit: `RecalcEdit` restores only the sheets
   `SnapshotEngine` saw (`commands/recalc.rs:311-345` — the outer cell's
   sheet and the *recorded* write targets), so `Width = 5` stays.

The same hole reaches the cross-check harness
(`crosscheck.rs:595-600` evaluates counted cells through a bare
`DocumentResolver`, so a counted `SETF` with a real target mutates the
document under test), which is why the harness should get the read-only
variant of the fix.

### F2 reproduces

1. A shape S whose master defines `Character.1!Size`; S has no own
   `Character` rows (the common instance shape: the corpus writes the
   rows on the master).
2. S's `PinX` carries `Char.Size*2`.
3. The resolver answers the reference (`find_section_in` walks the
   master rungs, `column_known` knows `Size` from the ShapeSheet table);
   `graph::resolve_ref_address` does not (`is_column` sees no rows on S,
   `named_row` finds no row named `Size`, `graph.rs:610` returns `None`),
   so `PinX` has no edge to the master's cell.
4. Edit the master's `Size`: the graph's dependent set omits `PinX`, the
   recalc pass never evaluates it, and it keeps its stale value even
   though its formula computes a new one.

## Master notes: rulings

- **`Expr` equality includes spans (P6-02).** Rule: **a hand-written
  `PartialEq` for `Expr` that compares `kind` only** (F4). The ADR text
  already says spans are not part of equality, so no amendment is
  needed; the derived equality is what contradicts it. The two
  span-stripping test helpers can then go.
- **Omitted call arguments (P6-02).** Rule: **the decision holds.** The
  18 corpus formulas all sit in `FORMATEX`, which is unknown and keeps
  its cached value anyway (ADR 0053), the parser failure is safe since
  P6-11's `Syntax` path keeps the cached value with one warning, and `F`
  is written back as stored (ADR 0057), so nothing is lost. Keep the
  rejection as the grammar's rule and record it in ADR 0051 (F5); add
  the empty-argument production when a function in the evaluated set
  needs one.
- **P6-10's cached-value reads during recalc.** **Safe as implemented.**
  The pass writes each computed value immediately after its evaluation
  (`recalc.rs:456-462`) and evaluates dependencies first
  (`order_and_cycles`, stable), so a later read sees the fresh value,
  and the fill-in case (no cached value) still evaluates the referenced
  formula inline. The one caveat is F2: the order is only as good as the
  graph's edges, and a missed edge makes a read see a stale value that
  nothing revisits.
- **P6-10's two comparison rules.** Both behaviours are right and both
  need an **ADR 0059 amendment** (F6): (a) the boolean bridge reflects
  how the file spells a `BOOL` cell (`V='0'`/`'1'`, no `U`), so an
  evaluated `Bool` agreeing with the cached `0`/`1` is correct — add it
  as a row of the comparison table rather than a harness-only rule;
  (b) rule 3's exclusion of hard defaults is the conservative reading
  (a table default is not a cell to agree with anything, and our
  defaults are not all verified), and it keeps the gate honest — spell
  it out in rule 3 and give the skip its own reason so the report stops
  calling a resolved reference a `missing-reference`.
- **`testfile5.vsdx` for the exit test.** Accepted as noted: the
  master-based resize needs a master instance and only `testfile5` has
  one; the file's 22 evaluations sit far inside the budget.
- **P6-11's open issues.** Both **confirmed at their rank**: the
  `warning_line` fallback text is F8 (minor, status text) and the
  non-finite `vsdx::read::value` literals are F9 (minor, hardening —
  the renderer already skips non-finite device bounds and the engine
  maps them to `#NUM!`, so there is no crash path). Re-rank neither.

## Confirmed open items (no new action)

P6-03's `Coerced`-instead-of-`Result` and the area-unit syntax error;
P6-04's black-box list (`MODULUS` sign, `CEILING`/`FLOOR` on negatives,
unit-ed number to string, string to boolean — the evaluator's choices
are all flagged in its docs); P6-06's master-rung-as-first-shape
approximation (`graph::rung_address`, harmless while masters have one
top-level shape) and the unmeasured `DEFAULT_MAX_EVALUATIONS`; P6-07's
"page add/delete recalculates nothing" and the un-materialisable
instance (needs a row-insertion mutation); P6-09's display-only Shape
Data values and the formula-field read-only rule (the pane refuses with
one status line, as ADR 0058 says).
