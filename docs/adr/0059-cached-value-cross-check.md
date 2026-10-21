# 0059: The cached-value cross-check

- Status: accepted
- Date: 2026-10-21
- Task: P6-01

## Context

The phase exit criterion asks for "the agreement rate with cached values"
in the phase report, and the KICKOFF test list names the rule: "a
cross-check against cached values in corpus files: where every function in
a formula is supported, our result must match the stored value." So the
harness must say which cells count, how close is close enough per unit
family, and what a disagreement is logged as.

## Decision

### The harness

`crates/commands/tests/crosscheck.rs` runs over the 15 `corpus/vsdx`
files (read through `ferrogram-vsdx` into a `Document`, like every other
corpus test). With `FERROGRAM_REQUIRE_CORPUS=1` it is mandatory; without
it, it skips when `corpus/vsdx` is absent. Determinism (ADR 0055) makes
the run reproducible; a second run over the same files must report the
same numbers.

### Which cells count

A cell is **counted** when all of these hold:

1. it has a formula that is not `Inh` (nothing to evaluate),
2. every function the formula calls is in the evaluated set (ADR 0053),
3. every reference it makes resolves to a cell that has a value (ADR 0054),
4. it caches a value that is not an error value.

A cell that fails any test is **skipped, by reason**: `unknown-function`,
`no-cached-value`, `missing-reference`, `cached-error`, `no-formula`.
Skipped cells are counted per reason and per file; they are not failures.
Cells whose formula is a bare literal are counted too (they pin the
literal/unit path, which is 64 % of the corpus).

### How values are compared

| Kind | Rule |
|---|---|
| Number (every family) | relative difference ≤ 1e-9, with an absolute floor of 1e-9 for near-zero values |
| String | exact |
| Bool | exact |
| Color | exact RGB |
| List | element-wise, same rules |
| Error value | same `ErrorCode` |

The number tolerance is loose enough for a different but equivalent
evaluation order (our arithmetic is IEEE `f64`; Visio rounds to 15
significant digits) and tight enough to catch a wrong unit, a wrong
factor or a wrong row. A disagreement outside tolerance is a **failure of
the test** for that file, not a warning: KICKOFF says our result *must*
match where the functions are supported. A real divergence we decide to
accept goes in a list of documented exclusions in this ADR (below), one
line each, with the reason — never in the code as a silent skip.

### What is reported

The harness prints one machine-readable block per file plus a total:

```
file                  formulas counted agree disagree skipped rate
blue-box.vsdx             876      412    412        0     464  100.0%
...
total                  11903     5312   5312        0    6591  100.0%
```

plus one line per disagreement (cell address, formula, cached value,
evaluated value, difference) capped at 50 lines, and the skip counts per
reason. The phase report copies the `total` line and the per-file table;
that is the "agreement rate with cached values" of the exit criterion,
next to the share of corpus formulas the evaluated set covers (78.7 %,
ADR 0053).

### Documented exclusions

None yet. A cell that disagrees because of a decision we accept (a
coercion rule, a rounding rule) gets a line here with its formula and the
reason, and the harness matches it by cell address.

## Consequences

- The phase report's number is reproducible from one command
  (`cargo test -p ferrogram-shapesheet --test crosscheck -- --nocapture`).
- "Every counted cell agrees" is a hard assertion, so any evaluator change
  that moves a value fails the gate rather than drifting.
- The skip breakdown is the honest picture of what the engine covers:
  21 % of formulas call an unknown function (ADR 0053) and the report
  says so.

## Alternatives considered

- **A soft rate with a floor (e.g. ≥ 95 %).** Rejected: it would let real
  regressions through; the counted-cell rule is already the scope
  limiter.
- **Compare with a percentage tolerance (1 %).** Rejected: too loose for
  geometry (a 1 % error on a 0.1 in shape is visible) and unnecessary
  once the evaluation order is deterministic.
- **Run the cross-check in `xtask` instead of a test.** Rejected: tests
  run in the gate already, and the corpus helpers (manifest, provenance)
  are test-side conventions.
- **Count only geometry cells.** Rejected: transform and shape-data cells
  are where the coercion rules (ADR 0052) get tested; counting them costs
  nothing.
- **Include cells with unknown functions by evaluating them as cached.**
  Rejected: that would trivially "agree" and inflate the rate.
