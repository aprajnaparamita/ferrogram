# Phase 6 report: ShapeSheet engine (core subset)

Phase 6 makes formulas live. `crates/shapesheet` parses, prints and evaluates ShapeSheet formulas, resolves references across shapes, masters, pages and styles, and recalculates dependents incrementally. The engine runs after every edit (inside the same undo step), fills in missing values when a file opens, and feeds the Shape Data and Size & Position panes. Resizing a master-based shape now recomputes its geometry. Started 2026-10-21, closed 2026-10-24, tag `phase-6`.

## What was built

| Area | Content | Tasks |
|---|---|---|
| Parser and printer | The full formula grammar (ADR 0051): numbers with units, strings, operators including the postfix percent, function calls, and every reference form (`Sheet.N!`, named shapes, `Masters[...]`, `Pages[...]`, `Styles!`, `ThePage!`, `TheDoc!`, quoted names, section and row cells). Spans on every node; parse-print-parse is the identity. Limits on input size, token count and nesting. | P6-02, P6-17 |
| Values and units | Numbers with units, strings, booleans, colours, errors; the unit table and conversions; Visio's type coercions as one `coerce` function returning the value and whether a default was used (ADR 0052). | P6-03 |
| Evaluator | 42 functions: arithmetic and comparison, unit helpers, `IF`/`AND`/`OR`/`NOT`, `MIN`/`MAX`/`ABS`/`INT`/`ROUND` and the rounding family, `GUARD`, `SETF`, trig, string helpers. An unknown function or a formula that does not parse keeps the cell's cached value and raises one structured warning (ADR 0053). | P6-04, P6-11, P6-17 |
| Reference resolution | `DocumentResolver` over the model: local cells, then the master chain, then the effective style link, with the `Inh` snapshot rule (ADRs 0054, 0057). One name rule, `shapesheet::names`, decides column versus named row against the section, for both the resolver and the graph. | P6-05, P6-15, P6-17 |
| Dependency graph and recalc | `DepGraph` with section-level cells and master rungs; deterministic incremental recalc in dependency order, cycle detection that keeps cached values, an evaluation budget that counts nested evaluations (ADR 0055). `SETF` writes are recorded by the resolver and applied by the pass, so they are undone with the edit (ADR 0056). | P6-06, P6-11, P6-17 |
| Edits with formulas | `commands::recalc`: every edit runs a recalc pass inside its undo step (`RecalcEdit`, snapshot undo); geometry routes through formulas when the cell has one. `data.set.property` writes Shape Data values. | P6-07, P6-18 |
| Open, render, save | Missing values are filled in on open (`recalc::fill_missing`); the renderer reads cached values; the writer keeps `U` codes per cell and recovers the contents relationship id. Master style links are followed, so `dwg`, `fdo86664` and `testfile5` draw. | P6-08, P6-15, P6-16, P6-18 |
| Panes | Typed Shape Data and Size & Position panes (ADR 0058). Shape Data lists the master chain's rows too; an edit of an inherited row writes a local row. Formula cells are shown read-only. | P6-09, P6-18 |
| Tests | Parser property tests, the `parse` and `eval_source` fuzz targets with our own seeds, function tables, recalc order and determinism over a 200-cell chain and a 1 000-cell page, the undo property over recalculating commands, malformed-value tests, and the cross-check. | P6-02..P6-11, P6-17, P6-18 |

## Exit criteria

**Resizing master-based shapes from the corpus recomputes geometry correctly for supported functions: met.** `crates/commands/tests/exit_resize.rs` resizes the shapes of `blue-box.vsdx` and `qs-box.vsdx` and the master instance of `testfile5.vsdx`, and checks the recomputed geometry. The master-instance resize takes 22 evaluations against the default budget of 100 000.

**The agreement rate with cached values: 100.0%.** `crates/commands/tests/crosscheck.rs` (ADR 0059, amended) evaluates every formula in the 15 corpus `.vsdx` files and compares the result with the value the file stores.

| | Cells |
|---|---:|
| Formulas seen | 9 242 |
| Counted (every function supported, every reference resolved) | 5 184 |
| Agree | 5 184 (100.0%) |
| Disagree | 0 |
| Skipped: no formula | 2 589 |
| Skipped: does not parse (`FORMATEX` with omitted arguments) | 27 |
| Skipped: unknown function | 629 |
| Skipped: missing reference (mostly the document and master sheets, not modelled) | 750 |
| Skipped: default only (the file holds no value to compare) | 31 |
| Skipped: documented exclusions (ADR 0059) | 32 |

976 of the 994 distinct formulas in the corpus parse; the 18 others are the `FORMATEX` calls with omitted arguments, rejected by decision (ADR 0051).

## Gate results

```
cargo xtask gate (full, 2026-10-24, main after the P6-18 merge, 152fae3)
  PASS  codegen
  PASS  parity
  PASS  fmt
  PASS  clippy
  PASS  test      (doc tests; coverage runs the rest)
  PASS  doc
  PASS  coverage
  PASS  deny
  PASS  layers
  PASS  wasm
  PASS  fuzz      (render, shapesheet, ui-spec, ui, vsdx)
gate: GREEN

FERROGRAM_REQUIRE_CORPUS=1 cargo test --workspace --all-features: OK (3 110 tests, 2 ignored)
cargo xtask extract --check: OK
```

## Coverage

| Crate | Lines | Coverage | Threshold | Phase 5 ratchet | New ratchet |
|---|---|---|---|---|---|
| ferrogram-geom | 2501 | 100.0% | 85 | 100.0 | 100.0 |
| ferrogram-vsd-binary | 281 | 100.0% | 85 | — | 100.0 |
| ferrogram-model | 10786 | 99.7% | 85 | 99.7 | 99.7 |
| ferrogram-ui-spec | 2043 | 99.4% | 85 | 99.3 | 99.4 |
| ferrogram-text | 1768 | 98.9% | 75 | 98.8 | 98.9 |
| ferrogram-render | 7221 | 98.5% | 75 | 98.5 | 98.5 |
| ferrogram-commands | 10563 | 98.4% | 85 | 98.3 | 98.4 |
| ferrogram-vsdx | 7727 | 98.2% | 85 | 98.1 | 98.2 |
| ferrogram-shapesheet | 6744 | 97.0% | 85 | 85.0 (reset) | 97.0 |
| ferrogram-ui | 11896 | 97.0% | 50 | 96.8 | 97.0 |
| ferrogram-cli | 1266 | 96.8% | 50 | 96.7 | 96.8 |
| xtask | 10457 | 93.4% | — | 93.4 | 93.4 |
| ferrogram | 250 | 80.0% | 50 | 80.0 | 80.0 |
| ferrogram-web | 140 | 77.9% | — | 77.8 | 77.9 |

The new ratchet is each crate's value rounded down to one decimal place. The `shapesheet` ratchet was reset to 85.0 at the start of the phase, because its stubs were replaced by about 6 700 lines of real code. `vsd-binary` holds the Phase 8 skeleton (P8-01), all stubs with contract tests. `ferrogram-control` is still empty.

## Reviewer findings and resolution

| Report | Verdict | Blocker / major / minor | Resolution |
|---|---|---|---|
| [Test pass](../reviews/phase-6-tests.md) (P6-11) | — | four product bugs | Fixed in P6-11: section-level cells were never recalculated; one malformed formula failed the whole pass; four warning kinds never reached the report; a master without the section stopped the master chain. |
| [Code review](../reviews/phase-6-review.md) (P6-12) | approve with fixes | 0 / 2 / 15 | F1 (a nested `SETF` bypassed undo) and F2 (two column-versus-row rules) fixed in P6-17, with the engine minors; the commands, model, reader and pane minors fixed in P6-18. |
| [Coverage](../reviews/phase-6-coverage.md) (P6-13) | pass | 0 / 16 / 17 | The major paths (the hard-error abort chain, the redo arm, `SETF` into section, row and page, graph edges by name, `rung_unit`) tested in P6-17; the reader and pane paths in P6-18. |
| [Clean-room](../reviews/phase-6-cleanroom.md) (P6-14) | pass | 0 / 0 / 5 | Two error-value glosses reworded and the reference examples replaced with our own names (P6-17); the sources and attribution rows fixed (P6-18). |

## Fixes made on main by the master

- Refused `SETF` targets return the new `Error::SetfRefused` instead of a conversion error (`fff98f9`).
- 541 libFuzzer-generated inputs committed beside the 14 `eval_source` seeds were removed (`7244bce`).
- The P6-05 task file's reference examples were renamed to our own names, matching P6-17.
- The gate's `test` step runs only the doc tests when coverage also runs, because `cargo llvm-cov` runs every other test (`6f44208`); this cut 6 to 13 minutes from each full gate.
- AGENTS.md section 7: workers run the full hand-off gate once, at the end.
- `cargo xtask extract --check` lists the three project-compiled Phase 7 data files as hand-maintained.

## Decisions made during the phase

- **Keep the cached value when unsure:** an unknown function or a formula that does not parse keeps the stored value and warns; it never fails the document (ADR 0053).
- **One write path for `SETF`:** the resolver records writes, the pass applies them, undo restores them (ADR 0056, amended).
- **One name rule:** a bare name is matched against the section's columns first, then its named rows, in one module for both the resolver and the graph (ADR 0054, amended).
- **Effective style link:** an instance without its own style link uses the first one on its master chain (ADRs 0013, 0057, amended).
- **Omitted arguments stay a syntax error** until a supported function needs them (ADR 0051).
- **Cross-check rules:** booleans compare across their spellings, and a cell whose value is only the default is skipped as `default-only` (ADR 0059, amended).

## ADRs added

- 0051 formula grammar, AST and printer
- 0052 values, units and coercions
- 0053 function table and evaluator
- 0054 reference resolution and inheritance
- 0055 dependency graph and recalc
- 0056 edits with formulas
- 0057 values on open, rendering and saving
- 0058 typed Shape Data and panels
- 0059 cached-value cross-check
- 0060 Phase 6 test strategy

## Known issues

- **Document and master sheets are not modelled.** Most of the 750 missing-reference skips point at them; `TheDoc!` resolves against `Document::properties`.
- **`Prop.<row>` references build no dependency edge**, so a formula that reads a Shape Data value is not re-evaluated when that value changes.
- **The recalc pass re-records the `Value` unit display hint** of Shape Data cells it writes (ADR 0039).
- **Our catalog masters have constant geometry**, so drawn and catalog shapes still scale their rows literally on resize; corpus masters with formula geometry recompute.
- **Page add and delete recalculate nothing**, and style edits do not dirty instances through the graph.
- **CI has still never run** (no remote).
