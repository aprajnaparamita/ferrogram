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

Task P6-10's run over the 15 corpus files counted 5 184 cells and found
32 accepted divergences (six causes), each with its line below. The
harness (`crates/commands/tests/crosscheck.rs`, its `EXCLUSIONS` table)
matches them by file and cell address and reports them as
`documented-exclusion`; every other counted cell agrees within the
tolerance above.

- `bgcolor.vsdx` `shape3!TextBkgnd` (`F=RGB(157,187,97)+1`): colour arithmetic — Visio caches the RGB unchanged where our evaluator coerces the colour to a number first (ADR 0052 has no colour arithmetic).
- `tdf154379-QuickStyleFillMatrix.vsdx` `page0!Actions1.Action1` (`F=SETF("User.ConnectorType",NOT(User.ConnectorType))`): `SETF` takes a `GetRef(...)` target (ADR 0053's basic SETF); the string-literal target form is refused, and an action cell caches 0 whatever it evaluates to.
- `tdf154379-QuickStyleFillMatrix.vsdx` `shape84!Actions1.Action1` (`F=SETF("User.ShapeAppearance",1)`): same, `SETF`'s string-literal target.
- `tdf154379-QuickStyleFillMatrix.vsdx` `shape84!Actions1.Action2` (`F=SETF("User.ShapeAppearance",2)`): same, `SETF`'s string-literal target.
- `tdf154379-QuickStyleFillMatrix.vsdx` `shape84!Actions1.Action3` (`F=SETF("User.ShapeAppearance",3)`): same, `SETF`'s string-literal target.
- `tdf154379-QuickStyleFillMatrix.vsdx` `shape84!Actions1.Action4` (`F=SETF("User.ShapeAppearance",4)`): same, `SETF`'s string-literal target.
- `tdf154379-QuickStyleFillMatrix.vsdx` `shape17!Property1.LangID1` (`F=User.LCID`): Visio maps an LCID number to a locale tag in a `LangID` cell (1033 caches as `de-DE`); ADR 0052 has no such coercion, so we keep the number.
- `tdf154379-QuickStyleFillMatrix.vsdx` `shape17!Property1.LangID2` (`F=User.LCID`): same, the LCID-to-locale-tag mapping.
- `tdf154379-QuickStyleFillMatrix.vsdx` `shape19!Character1.LangID0` (`F=User.LCID`): same, the LCID-to-locale-tag mapping (1033 caches as `en-US`).
- `tdf154379-QuickStyleFillMatrix.vsdx` `shape20!Character1.LangID0` (`F=User.LCID`): same, the LCID-to-locale-tag mapping.
- `tdf154379-QuickStyleFillMatrix.vsdx` `shape21!Character1.LangID0` (`F=User.LCID`): same, the LCID-to-locale-tag mapping.
- `tdf154379-QuickStyleFillMatrix.vsdx` `shape22!Character1.LangID0` (`F=User.LCID`): same, the LCID-to-locale-tag mapping.
- `tdf154379-QuickStyleFillMatrix.vsdx` `shape23!Character1.LangID0` (`F=User.LCID`): same, the LCID-to-locale-tag mapping.
- `tdf154379-QuickStyleFillMatrix.vsdx` `shape24!Character1.LangID0` (`F=User.LCID`): same, the LCID-to-locale-tag mapping.
- `tdf154379-QuickStyleFillMatrix.vsdx` `shape25!Character1.LangID0` (`F=User.LCID`): same, the LCID-to-locale-tag mapping.
- `tdf154379-QuickStyleFillMatrix.vsdx` `shape67!EventDblClick` (`F=NA()`): `NA()` is `#N/A!` by definition; Visio caches 0 on an event cell whatever its formula evaluates to.
- `tdf154379-QuickStyleFillMatrix.vsdx` `shape69!EventDblClick` (`F=NA()`): same, the event-cell cache.
- `tdf154379-QuickStyleFillMatrix.vsdx` `shape70!EventDblClick` (`F=NA()`): same, the event-cell cache.
- `tdf154379-QuickStyleFillMatrix.vsdx` `shape71!EventDblClick` (`F=NA()`): same, the event-cell cache.
- `tdf154379-QuickStyleFillMatrix.vsdx` `shape73!EventDblClick` (`F=NA()`): same, the event-cell cache.
- `testfile1.vsdx` `shape2!FillBkgndTrans` (`F=GUARD(IF(Actions.End.Checked,LineColorTrans,0))`): the theme-driven `LineColorTrans` caches the `Themed` marker the model keeps (ADR 0021 resolves theme values in the renderer), so the branch evaluates to the marker where Visio caches the theme's value.
- `testfile1.vsdx` `shape2!FillForegndTrans` (`F=GUARD(IF(Actions.End.Checked,LineColorTrans,0))`): same, the `Themed` marker.
- `testfile3.vsdx` `shape2!FillBkgndTrans` (`F=GUARD(IF(Actions.End.Checked,LineColorTrans,0))`): same, the `Themed` marker.
- `testfile3.vsdx` `shape2!FillForegndTrans` (`F=GUARD(IF(Actions.End.Checked,LineColorTrans,0))`): same, the `Themed` marker.
- `testfile4.vsdx` `shape0!FillBkgndTrans` (`F=GUARD(IF(Actions.End.Checked,LineColorTrans,0))`): same, the `Themed` marker.
- `testfile4.vsdx` `shape0!FillForegndTrans` (`F=GUARD(IF(Actions.End.Checked,LineColorTrans,0))`): same, the `Themed` marker.
- `testfile6.vsdx` `shape1!FillBkgndTrans` (`F=GUARD(IF(Actions.End.Checked,LineColorTrans,0))`): same, the `Themed` marker.
- `testfile6.vsdx` `shape1!FillForegndTrans` (`F=GUARD(IF(Actions.End.Checked,LineColorTrans,0))`): same, the `Themed` marker.
- `testfile1.vsdx` `shape4!Character1.Size0` (`F=GUARD(0)`): Visio clamps a character size to 1 pt (the file caches 0.0139 in) where the formula computes 0.
- `testfile3.vsdx` `shape4!Character1.Size0` (`F=GUARD(0)`): same, the 1 pt clamp.
- `testfile4.vsdx` `shape2!Character1.Size0` (`F=GUARD(0)`): same, the 1 pt clamp.
- `testfile6.vsdx` `shape3!Character1.Size0` (`F=GUARD(0)`): same, the 1 pt clamp.

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

## Amendment 2026-10-23 (task P6-18): rule 3, the boolean bridge and the `default-only` skip

The P6-12 review (F6) found two comparison rules the harness applied
without a text here. Both behaviours stand (the master's ruling); this
amendment writes them down and gives the hard-default skip its own
reason.

**Rule 3 reads as "a value the document holds".** The rule was "every
reference it makes resolves to a cell that has a value", which read as
excluding hard defaults and reported a *resolved* reference as
`missing-reference`. The conservative reading stands: a ShapeSheet
table default (`DropOnPageScale`, `LockTextEdit`) is not a cell of the
document to agree with anything, and our defaults are not all verified.
So rule 3 is: *every reference it makes resolves to a value the
document holds*. A reference that resolves only to the table's hard
default is skipped as **`default-only`**, never as `missing-reference`;
a reference that resolves to nothing (or to `#REF!`) stays
`missing-reference`. The skip reasons are now the full set the harness
reports: `no-formula`, `does-not-parse`, `unknown-function`,
`missing-reference`, `default-only`, `no-cached-value`, `cached-error`
and `documented-exclusion`.

**The boolean spelling bridge is a row of the comparison table.** The
file spells a `BOOL` cell's cached value as the number `0`/`1` (the
`V=` of a cell without `U="BOOL"`), so the table compares a boolean
across that spelling:

| Kind | Rule |
|---|---|
| Bool ↔ Number | an evaluated `Bool` agrees with a cached `Number(0)`/`Number(1)` and the other way round (the file's `0`/`1` spelling of a `BOOL` cell without `U="BOOL"`) |

The P6-18 run over the 15 corpus files, with the reason split: 9 242
non-`Inh` formulas, **5 184 counted, 5 184 agree, 0 disagree (100.0 %)**
and 4 058 skipped — `no-formula` 2 589, `does-not-parse` 27,
`unknown-function` 629, `missing-reference` 750, `default-only` 31,
`no-cached-value` 0, `cached-error` 0, `documented-exclusion` 32. The
32 documented exclusions are the same cells as before; 31 cells moved
from `missing-reference` to `default-only`.
