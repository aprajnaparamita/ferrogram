# Phase 6 test pass

Task P6-11 (test-engineer), branch `agent/P6-11`, 2026-10-22/23.

The pass fills the Phase 6 test gaps of ADR 0060: property and fuzz
coverage for the parser and the evaluator, recalc order and budget tests
over bigger documents, undo property tests with recalculated cells and
malformed-input tests for the new code paths. It found and fixed four
product bugs (below) and closed the P5-20 `sync_uint` open item.

## What was added

| Commit | What |
|---|---|
| `fix(shapesheet): map every evaluator warning kind to a recalc warning` | `recalc::map_warning` kept four `EvalWarningKind`s out of `RecalcOutcome` (`WrongArgumentCount`, `SetfRefused`, `Coercion`, `NonFinite`): the values were right but the warnings never surfaced. New `RecalcWarningKind` variants, mapping arms and one test per kind (the master note). |
| `fix: a recalc pass evaluates and writes section-level cells` | `graph::effective` resolved a section-level cell (a section, no row: the `NoFill` family of `Geometry`) as a direct cell, so `recalc` never evaluated one and the fill-in pass left its hole (the P6-08 open issue). The resolve is section-aware now (the section's own cell, then the master chain with `rung_address`'s rule), and the write goes through the new `Document::write_section_cell` (the narrowest layer that spells the cell's unit). The P6-08 fill-in test that pinned the old behaviour now pins the filled value. |
| `fix(shapesheet): a formula that does not parse keeps its cached value` | `DocumentEngine` propagated a parse failure as a broken engine, so one malformed formula string failed the whole pass. The ADR 0053 rule applies: the cell keeps its cached value and records one `RecalcWarningKind::Syntax` warning. |
| `fix(shapesheet): a rung without the section does not stop the master chain` | `section_rung` returned early when a rung had no such section, hiding the next rung's formula. |
| `test(shapesheet)`: `tests/recalc_engine.rs` | 18 tests over the production engine: section-level cells and their master chains (local, master, chained master, cyclic master cut, rung without the section), P6-15's style-link rule through the engine, every evaluator warning kind reaching `RecalcOutcome`, a malformed formula keeping its cached value, order/determinism/idempotence over a 200-cell chain, budget truncation over the chain and a 1 000-cell page, broken references (nine forms) and cyclic `SETF` (ping-pong cut, self-write as a cycle of one). |
| `test(vsdx)`: `tests/value_malformed.rs` | The malformed-input suite for P6-08's value reader (8 tests): a literal that does not parse names the part and the cell in one `InvalidPackage`, huge and non-finite numeric spellings keep the parsed float (pinned, see "not reached"), error cells keep their `E` text and never parse their value, a `LIST` keeps raw text, a nameless `Cell` is dropped. |
| `test(commands)`: `tests/undo_recalc_prop.rs` | The ADR 0040 property over the recalculating commands: resize, move, a cell edit (`shape.set.width`) and a `SETF`-carrying cell each undo and redo to the same documents (recalculated values, a `GUARD`-ed cell and an unknown function's cached value included), and a proptest drives random sequences of them. |
| `test(commands)`: `tests/recalc_corpus.rs` | Every `corpus/vsdx` file recalculates twice to the same report (determinism, ADR 0055) and terminates inside the budget. `recursion-cycle.vsdx` — ADR 0060's named cycle file — holds no formulas (P6-06's Result: its cycle is in the package relationships), so the file pins that a pass over it changes nothing; the formula-cycle rule is pinned on hand-built cycles. |
| `test(vsdx)`: `sync_uint` unit test | Closes the P5-20 open item (`write/patch.rs` 138-140): the overwrite and set-new arms. |
| `test(shapesheet)`: `eval_source` fuzz target + seeds | The master note: `parse` output fed to `eval` against a document-free context (every reference `#REF!`, every `SETF` dropped). 14 seeds, one per evaluator rule; `ATTRIBUTION.md` row in the same commit. 30 000 smoke runs clean. |
| `test(shapesheet)`: values properties on `proptest` | The master note: P6-03's seeded xorshift loops became `proptest` properties (they shrink now). The unit conversion keeps its every-unit table test beside the random half; the P2-14 value round-trip generates every kind. |

## Bugs the pass found and fixed

1. **Section-level cells were never recalculated** (`graph::effective`,
   `Document::write_section_cell`, `recalc::apply_write`) — the P6-08
   open issue, master note. Regression tests:
   `a_section_level_cell_fills_its_hole`,
   `a_section_level_cell_inherits_its_master_formula`,
   `a_section_level_cell_walks_a_master_chain`,
   `a_rung_without_the_section_does_not_stop_the_chain`,
   `a_section_level_cell_is_evaluated_and_written_in_place` (the unit
   test beside the pass),
   `a_formula_without_a_cached_value_gains_one_on_read` (the P6-08 test
   updated) and the two `write_section_cell` model tests.
2. **One malformed formula failed the whole recalc pass**
   (`DocumentEngine::evaluate`): a parse error was a broken engine. Now
   the cell keeps its cached value with one `Syntax` warning (the ADR
   0053 rule). Regression test:
   `a_formula_that_does_not_parse_keeps_the_cached_value`.
3. **Four evaluator warnings never reached `RecalcOutcome`**
   (`recalc::map_warning`) — the master note. Tests: one per kind in
   `src/recalc.rs` and the four `*_reaches_the_outcome` engine tests.
4. **A master rung without the section stopped the chain**
   (`graph::section_rung`). Regression test:
   `a_rung_without_the_section_does_not_stop_the_chain`.

## The ADR 0051-0059 promises, one named test each

### ADR 0051 (grammar, AST, printer)

| Promise | Test |
|---|---|
| parse-print-parse is the identity on generated trees | `tests/parser.rs::parse_print_parse_is_the_identity` (proptest) |
| a parsed corpus formula prints and reparses to an equal tree | `tests/parser.rs::a_formula_written_the_way_the_corpus_writes_one_round_trips`, `a_number_literal_round_trips_by_value` (proptest) |
| every reference form of the table | `tests/parser.rs::every_reference_form_of_the_table_parses_and_prints_back` |
| parts kept as written | `tests/parser.rs::a_reference_keeps_every_part_as_written`, `tests/formula_types.rs::a_cell_ref_keeps_every_part_as_written` |
| operator precedence and the alternative comparison operators | `tests/parser.rs::operator_precedence_matches_the_order_of_evaluation` |
| unit suffixes and mixed fractions | `tests/parser.rs::every_unit_spelling_parses_and_prints_back`, `a_formula_written_the_way_the_corpus_writes_one_round_trips` |
| strings, calls with no arguments, unknown names | `tests/parser.rs::a_function_call_needs_its_parentheses`, `unknown_names_parse_they_fail_later_if_at_all` |
| the three limits are syntax errors, never a panic | `tests/parser.rs::the_input_length_limit_is_a_syntax_error`, `the_token_limit_is_a_syntax_error`, `the_nesting_limit_is_a_syntax_error`, `a_malformed_formula_is_a_spanned_syntax_error` |
| any bytes, never a panic | fuzz targets `parse` (P6-02) and `eval_source` (P6-11) |

### ADR 0052 (values, units, coercions)

| Promise | Test |
|---|---|
| `to_cached`/`from_cached` round-trip every kind (P2-14) | `tests/values.rs::every_value_kind_round_trips_through_the_cached_form` (proptest), `every_error_value_round_trips_its_cached_text`, `a_boolean_or_string_with_a_unit_drops_the_unit_on_write`, `a_number_reads_back_in_the_unit_the_file_recorded` |
| the same round trip through the file | `crates/vsdx/tests/value_roundtrip.rs`, `crates/commands/tests/value_roundtrip.rs` (P6-08), `crates/vsdx/tests/value_malformed.rs` (P6-11, malformed) |
| unit families and canonical conversions | `tests/values.rs::families_follow_the_adr_table`, `conversions_round_trip_within_1e_12_for_every_unit`, `a_random_conversion_round_trips_within_1e_12` (proptest), `exact_factors_are_exact`, `the_geom_bridge_keeps_the_shared_set`, `units_the_model_lacks_are_conversion_errors` |
| area spellings are rejected | `tests/values.rs::area_spellings_are_out_of_scope_and_rejected_as_invalid_units` |
| coercion is the one documented rule (`5 * "sheep"` is 0) | `tests/values.rs::sheep_is_zero_with_a_coercion_warning`, `a_numeric_string_converts_to_its_number`, `a_number_to_a_string_and_a_boolean_to_a_number_follow_the_docs`, `the_typed_views_read_what_coercion_expresses` |
| a computed non-finite number is `#NUM!` | `tests/eval.rs::a_non_finite_result_is_num_with_one_warning`, `tests/recalc_engine.rs::a_non_finite_result_reaches_the_outcome`, `tests/values.rs::a_non_finite_magnitude_is_num_in_both_directions` |

### ADR 0053 (function table and evaluator)

| Promise | Test |
|---|---|
| every name spelled as the reference spells it, sane arities, `is_supported` agrees | `tests/functions.rs` (all four tests) |
| one test per evaluated family | `tests/eval.rs` (arithmetic on mixed units, comparison coercions, `IF` laziness, `GUARD`, `SETF`, `INDEX`/`LOOKUP` bounds, `RGB`, `STRSAME`, trig, `DEG`/`RAD`, text) |
| a bad index is `#VALUE!`, never a panic | `tests/eval.rs::a_bad_index_is_value_error_not_a_panic` |
| wrong argument count is `#VALUE!` with one warning | `tests/eval.rs::a_wrong_argument_count_is_value_error_with_one_warning`, `tests/recalc_engine.rs::a_wrong_argument_count_warning_reaches_the_outcome` |
| an unknown function keeps the cached value with one warning | `tests/eval.rs::an_unknown_function_keeps_the_cached_value_path_and_warns_once_per_formula`, `a_reserved_internal_name_keeps_the_cached_value_path_with_its_own_text` |
| every evaluator warning reaches `RecalcOutcome` (P6-11 mapping) | `tests/recalc_engine.rs::{a_wrong_argument_count, a_refused_setf, a_coercion, a_non_finite}_warning_reaches_the_outcome`, `src/recalc.rs::map_warning_keeps_*` |
| `SETF` refuses a non-reference target | `tests/eval.rs::a_refused_setf_target_is_ref_error_with_one_warning`, `a_setf_target_that_is_not_a_reference_is_ref_error`, `tests/recalc_engine.rs::a_refused_setf_warning_reaches_the_outcome` |

### ADR 0054 (reference resolution and inheritance)

| Promise | Test |
|---|---|
| per-form resolution of the reference table | `tests/resolve.rs` (the form-by-form suite) |
| an inherited formula evaluates in the target sheet's context | `crates/commands/tests/recalc_edit.rs::resize_recomputes_a_master_instance_geometry_through_recalc`, `tests/recalc_engine.rs::a_section_level_cell_inherits_its_master_formula` |
| the `Inh` snapshot is the preferred value, provenance from the formula rung | `crates/model/tests/resolve_provenance.rs`, `tests/resolve.rs` (the snapshot rule) |
| a missing reference is `#REF!` with one warning | `tests/eval.rs::references_read_the_context_and_a_missing_cell_is_ref_error`, `tests/recalc_engine.rs::broken_references_are_warnings_not_failures` |
| names match case-insensitively | `tests/functions.rs::lookup_is_case_insensitive`, `tests/resolve.rs` |
| the master's style link reaches the instance (P6-15, ADR 0013 amendment) | `tests/recalc_engine.rs::a_style_formula_reaches_the_instance_through_the_masters_style_link` (through the engine) and `crates/model/src/lookup.rs::style_link_tests` (the lookup itself) |

### ADR 0055 (dependency graph and recalculation)

| Promise | Test |
|---|---|
| a diamond evaluates the shared dependency once, in order | `src/recalc.rs::a_diamond_evaluates_the_shared_cell_once` |
| dirtying one cell recomputes exactly its dependents | `src/recalc.rs::dirtying_one_cell_recomputes_exactly_its_dependents` |
| cycles keep cached values and warn once | `src/recalc.rs::{a_cycle_keeps_cached_values_and_warns_once, a_self_reference_is_a_cycle_of_one}`, `tests/recalc_engine.rs::{a_formula_cycle_keeps_the_cached_values, a_cyclic_setf_is_cut_without_a_panic, a_self_referencing_setf_is_a_cycle_of_one}` |
| the budget truncates and reports | `src/recalc.rs::a_tiny_budget_truncates_and_leaves_the_rest_stale`, `tests/recalc_engine.rs::{a_tiny_budget_truncates_a_long_chain, a_thousand_cell_page_serves_the_budget_rule}` |
| recalc is idempotent | `src/recalc.rs::recalc_is_idempotent`, `tests/recalc_engine.rs::a_long_chain_recomputes_in_order_and_is_idempotent` |
| deterministic order and output | `tests/recalc_engine.rs::a_long_chain_recomputes_in_order_and_is_idempotent` (same outcome twice), `crates/commands/tests/recalc_corpus.rs::every_corpus_file_recalculates_deterministically` |
| the order over bigger documents (the task goal) | `tests/recalc_engine.rs` (200-cell chain, 1 000-cell page) |

### ADR 0056 (edits with formulas)

| Promise | Test |
|---|---|
| one user action is one undo step including the recalculated cells, restored by snapshot | `crates/commands/tests/recalc_edit.rs::one_user_action_is_one_undo_step_including_the_recalculated_cells` |
| the undo property over recalc-affecting commands (resize, move, cell edit, `SETF` cells) | `crates/commands/tests/undo_recalc_prop.rs` (four named tests + `random_sequences_undo_to_the_same_documents`) |
| `GUARD` survives an indirect write with one warning | `recalc_edit.rs::a_guarded_cell_survives_an_indirect_write_with_one_warning` |
| `SETF` writes its target and dirties it | `src/recalc.rs::setf_writes_its_target_and_dirties_it_in_the_same_pass`, `undo_recalc_prop.rs::a_setf_write_is_undone_with_the_command_that_triggered_it` |
| resize never calls `scale_geometry` | `recalc_edit.rs::resize_recomputes_a_master_instance_geometry_through_recalc`, `a_resize_scales_constant_rows_and_recomputes_formula_rows` |
| a failed edit changes nothing | `recalc_edit.rs::a_failing_edit_changes_nothing_and_recalculates_nothing` |
| recalc owns formula geometry, the router owns literal geometry | `recalc_edit.rs::reroute_skips_a_connector_whose_geometry_rows_carry_formulas` |

### ADR 0057 (values on open, in rendering, on save)

| Promise | Test |
|---|---|
| the fill-in pass fills only what is missing | `src/recalc.rs::fill_missing_only_recomputes_cells_without_a_cached_value`, `crates/commands/src/builtins/file.rs::{a_formula_without_a_cached_value_gains_one_on_read, a_cached_value_is_unchanged_by_the_fill_in_pass}`, `tests/recalc_engine.rs::a_section_level_cell_fills_its_hole` |
| the fill-in pass is not an edit | `file.rs::the_fill_in_pass_takes_no_undo_entry`, `recalc_edit.rs::fill_missing_fills_the_holes_without_an_undo_entry` |
| the local snapshot wins for an inherited cell at render time | `apps/ferrogram/tests/open_corpus.rs` (P6-08's resized-instance raster assertions, P6-15's blank-file rule) |
| the writer's kind spelling round-trips | `crates/vsdx/tests/value_roundtrip.rs`, `crates/commands/tests/value_roundtrip.rs` (P6-08) |

### ADR 0058 (typed Shape Data and panels)

| Promise | Test |
|---|---|
| typed display, unit conversion on entry, formula cells read-only | `crates/ui/tests/panels_typed.rs` (P6-09); nothing in this pass changed the panels |

### ADR 0059 (the cached-value cross-check)

| Promise | Test |
|---|---|
| the harness, the counted-cell rule, the tolerance, the exclusions | `crates/commands/tests/crosscheck.rs` (P6-10) |
| the exit criterion in code | `crates/commands/tests/exit_resize.rs` (P6-10) |

## What the pass did not reach

- **Huge and non-finite numeric literals in a file** read as the IEEE
  infinity or NaN (`crates/vsdx/src/read/value.rs` parses `V` with
  `f64::parse`, which accepts `1e999999`, `inf` and `nan`).
  `tests/value_malformed.rs` pins that outcome. The writer rejects
  non-finite numbers (`mutate::checked_number`), so the two sides
  disagree; a future hardening (reject on read, as the write does)
  should change those two tests deliberately. Not a crash: the engine
  turns a non-finite magnitude into `#NUM!` on the way to a value.
- **`commands::recalc::warning_line`** maps the four new warning kinds
  through its `#[non_exhaustive]` fallback line (`recalc: warning of
  X`). A friendly line per kind is a one-screen change in
  `crates/commands/src/recalc.rs`, outside this task's lane.
- **P5-20 leftovers other than `sync_uint`**: the connector tool's
  missing-master arm needs an injectable lookup, and the render `_`
  arms for future `#[non_exhaustive]` variants stay as P5-20 listed
  them. Out of scope here.
- **Coverage ratchets** are unchanged (`coverage.toml` is the
  master's). The numbers below are at or above every ratchet.
- **No golden images or `insta` snapshots** were added: nothing in the
  pass changes rendering.

## Coverage (after the pass, `cargo xtask coverage`)

| crate | lines | covered | pct | threshold | ratchet |
|---|---|---|---|---|---|
| ferrogram | 250 | 200 | 80.0 | 50.0 | 80.0 |
| ferrogram-cli | 1266 | 1225 | 96.8 | 50.0 | 96.7 |
| ferrogram-commands | 9537 | 9372 | 98.3 | 85.0 | 98.3 |
| ferrogram-geom | 2501 | 2501 | 100.0 | 85.0 | 100.0 |
| ferrogram-model | 11234 | 11204 | 99.7 | 85.0 | 99.7 |
| ferrogram-render | 7221 | 7113 | 98.5 | 75.0 | 98.5 |
| ferrogram-shapesheet | 6733 | 6433 | 95.5 | 85.0 | 85.0 |
| ferrogram-text | 1768 | 1748 | 98.9 | 75.0 | 98.8 |
| ferrogram-ui | 11671 | 11324 | 97.0 | 50.0 | 96.8 |
| ferrogram-ui-spec | 2043 | 2031 | 99.4 | 85.0 | 99.3 |
| ferrogram-vsd-binary | — | — | n/a | 85.0 | — |
| ferrogram-vsdx | 7722 | 7583 | 98.2 | 85.0 | 98.1 |
| ferrogram-web | 140 | 109 | 77.9 | — | 77.8 |
| xtask | 10443 | 9757 | 93.4 | — | 93.4 |

`ferrogram-shapesheet` grew from the ratchet (85.0) to 95.5 with the
Phase 6 implementation and this pass's tests; `ferrogram-model` dipped
to 99.6 when `write_section_cell` landed and is back at its 99.7
ratchet with the two model tests. The pass adds no new public API
surface beyond `Document::write_section_cell` and the
`RecalcWarningKind` variants.

## Gate

`cargo xtask gate` at hand-off: codegen, parity, fmt, clippy, test, doc,
coverage, deny, layers, wasm and fuzz all green (the run summary is in
the task's Result section).
