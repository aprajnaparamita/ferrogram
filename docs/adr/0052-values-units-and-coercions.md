# 0052: Values, units and coercions

- Status: accepted
- Date: 2026-10-21
- Task: P6-01

## Context

A formula evaluates to something; the evaluator needs one value type that
carries numbers with units (so `DEG()` and `1 in + 2 cm` work), strings,
booleans, colours, lists and Visio's error values. The model already
stores a cached value and a unit per cell (ADR 0011) and the renderer
reads canonical units (inches, radians, fractions, ADR 0011 amendment).
P2-14 left one open problem to this phase: a hand-built value does not
always read back the same, because the writer's `V`/`U` spelling has no
typed mapping yet.

Sources (`docs/sources.md` row 19): `about-error-values.md`,
`about-units-of-measure.md`, `about-operators.md` (coercion rule),
`about-currency-constants.md` and `about-date-time-and-duration-values.md`.
Corpus facts: the `U=` spellings in the 15 corpus files (MM 3587, PT 752,
STR 255, BOOL 242, IN 192, DL 153, DA 24, DEG 23, DATE 20, IN_F 9,
COLOR 4, PER 2, DT 2) and the unit suffixes in their formulas (PT, MM, %,
DEG, DA, IN, DT).

## Decision

### The value type

```rust
pub enum Value {
    Number(Number),        // magnitude + unit
    Str(String),
    Bool(bool),
    Color(model::Color),   // RGB as the model stores it
    List(Vec<Value>),
    Err(ErrorCode),
}
pub struct Number { pub magnitude: f64, pub unit: Unit }
pub enum ErrorCode { Div0, Value, Ref, Num, Na, Dim }   // #DIV/0! #VALUE! #REF! #NUM! #N/A! #DIM!
```

The error codes are `about-error-values.md`'s six. An evaluation problem
is a **value**, not a Rust error: `10/0` is `#DIV/0!`, a wrong operand
type is `#VALUE!`, a missing cell is `#REF!`, a non-finite result is
`#NUM!` (the P3-11/P3-16 rule, now enforced at the one place values are
produced). `Error` (the Rust error) is only for parse failures, failed
conversions and a broken context.

### Units

`Unit` is the formula-side unit set of `about-units-of-measure.md`: the
lengths (`in`, `ft`, `yd`, `mi`, `nm`, `mm`, `cm`, `m`, `km`), the
typographic lengths (`pt`, `p`, `c`, `d`), the angles (`deg`, `rad`, `'`,
`"`, `°`), the ratio (`%`), the durations (`ew`, `ed`, `eh`, `em`, `es`,
`date`), the fractional spellings (`IN_F`, `MI_F`), the compound
spellings (`FEET/INCH`, `PICAPOINTS`, `CICERO/DIDOT`) and the implicit
units `DL`, `DP`, `DT`, `DA`, `DE`.

Families and canonical units (ADR 0011 unchanged):

| Family | Canonical | Notes |
|---|---|---|
| Length | inches | includes `IN_F`, `MI_F`, compound spellings |
| Typographic | points for display, inches for computation | `pt`, `p`, `c`, `d` |
| Angle | radians | `deg`, `rad`, `'`, `"`, `°` |
| Ratio | fraction (`0.5` = 50%) | `%` |
| Duration | seconds | `ew`..`es`, `date` |
| Dimensionless | the number itself | no suffix |

The implicit units (`DL`, `DP`, `DT`, `DA`, `DE`) mean "internal units,
displayed in that measurement system": they carry no scale of their own
and convert like a plain number in the family the system implies
(`DA` angle, `DE` duration, the rest length). The `number[unit, flag]`
form keeps its flag for display; the flag does not change the value.

Area spellings (`SQ IN`, `HA`, `ACRES`) are **not** accepted in this
phase: no corpus formula uses one and `Value` carries one unit, not a
dimension vector. A formula with one is a syntax error (`InvalidUnit`).

### Coercion

`coerce(&Value, ValueKind)` is the single rule of `about-operators.md`:
convert when the target kind can express the value (a numeric string to a
number, a number to a string, a number to a boolean where 0 is `FALSE`),
and otherwise return the typed default of the target kind (0, `""`,
`FALSE`, black, empty list) **with a `Coercion` warning**. Comparison
operators convert both sides to numbers, as Visio does; string equality
that must not convert is `STRSAME` (ADR 0053). Coercion never fails and
never panics.

### Mapping to and from the model (P2-14)

`Value::to_cached` writes a `model::CellValue` plus the `larrygram_geom::Unit`
the cell records; `Value::from_cached` reads one. The rules:

- A number converts to the family's canonical unit and stores the unit
  the file used (`U=`) so the writer can restore it (ADR 0011).
- A colour stores `CellValue::Color` (the reader already resolves palette
  indices and `Themed` into colours, ADR 0021).
- A list stores its raw string (`CellValue::List`) as the file writes it;
  the evaluator's list values map element-wise.
- A string or boolean stores `Str`/`Bool`; the writer emits `U="STR"` or
  `U="BOOL"` and the reader's kind table reads them back. The P2-14
  round-trip gap for "a `Bool`/`Str` with a real unit" closes by
  **dropping the unit on write for a non-numeric kind**: a unit on a
  string is meaningless and the reader maps it to the string kind anyway.
- An error value stores `CellValue::Error` with its `#...` text, exactly
  as the reader reads it.

## Consequences

- The evaluator can compute `1 in + 2 cm` and `DEG(PI())` without a
  separate unit layer, and the renderer keeps reading canonical numbers.
- Non-finite values die at the one place values are produced, so no
  caller downstream (render, write, panes) needs its own guard (the
  P3-16 `Ctx::finite` removal is now a rule, not a habit).
- `Unit` duplicates a subset of `larrygram_geom::Unit` (which has no
  durations, didots or implicit units). The two are bridged by
  `Unit::to_geom`/`from_geom`; keeping `geom` unchanged avoids touching
  L0 in this phase (AGENTS.md "stay in your lane").
- Dates and durations have values but no display in this phase: the
  panes show their cached text (ADR 0058).

## Alternatives considered

- **Use `larrygram_geom::Unit` as the value unit.** Rejected: it has no
  didots, durations, implicit units or compound spellings, and adding
  them needs an L0 change this task does not own.
- **Values without units (plain `f64`).** Rejected: unit arithmetic is
  in KICKOFF's core set and the corpus writes unit literals in formulas.
- **A dimensional-analysis type (length^2, length/time).** Rejected: one
  unit per value covers every corpus formula and Visio's own `#DIM!`
  corner, and the type would complicate every function signature.
- **Coercion that raises `#VALUE!` on failure.** Rejected: the documented
  rule supplies the typed default (`5 * "sheep"` is 0), and failing harder
  than Visio would break the cross-check (ADR 0059).
- **Keep the `U` of a string cell on write.** Rejected: P2-14 shows it
  does not read back; the kind spelling is what the reader understands.
