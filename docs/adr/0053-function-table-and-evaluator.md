# 0053: The function table and the evaluator

- Status: accepted
- Date: 2026-10-21
- Task: P6-01

## Context

KICKOFF names the core function set (arithmetic and comparison, unit
conversion, `IF`, `AND`, `OR`, `NOT`, `MIN`, `MAX`, `ABS`, `INT`,
`ROUND`, `GUARD`, basic `SETF`, trig) "plus any helper functions that
common masters need", and asks for that need to be **measured**. An
unknown function must keep the cell's cached value and log a structured
warning; it never fails the document. The evaluator must also decide how
the table is kept: hand-written or generated from committed derived data.

## The corpus function census

Method: every `F='...'` / `<F>` formula in the 15 `corpus/vsdx` files,
unescape the XML entities, count `NAME(` calls case-folded. Result: 11 903
formulas carrying a formula other than `Inh`, 10 608 `Inh` markers, 55
distinct function names:

| Function | Calls | | Function | Calls |
|---|---|---|---|---|
| THEMEVAL | 1968 | | DOCMD | 18 |
| GUARD | 1228 | | LUM | 17 |
| IF | 829 | | SHAPETEXT | 16 |
| INDEX | 472 | | THEME | 16 |
| OR | 247 | | SHADE | 14 |
| STRSAME | 207 | | LISTSHEETREF | 14 |
| NOT | 160 | | RUNADDON | 14 |
| SETF | 125 | | ISTHEMED | 13 |
| GETREF | 116 | | _MOD | 13 |
| THEMEGUARD | 111 | | PAR | 12 |
| RGB | 93 | | PNT | 12 |
| LOOKUP | 84 | | TINT | 11 |
| SETATREFEXPR | 80 | | _XFTRIGGER | 6 |
| AND | 79 | | LUMDIFF | 5 |
| RUNADDONWARGS | 71 | | NA | 5 |
| OPENTEXTWIN | 56 | | MSOTINT | 4 |
| TEXTHEIGHT | 54 | | DEPENDSON | 4 |
| SETATREFEVAL | 48 | | NOW | 4 |
| MAX | 43 | | BITXOR | 4 |
| SETATREF | 38 | | SUM | 3 |
| CEILING | 35 | | INT | 3 |
| MIN | 34 | | SHEETREF | 3 |
| DEFAULTEVENT | 30 | | TONE | 2 |
| IFERROR | 30 | | _ELLIPSE_THETA | 2 |
| FIELDPICTURE | 30 | | _ELLIPSE_ECC | 2 |
| FORMATEX | 29 | | DATEVALUE | 2 |
| CONTAINERSHEETREF | 27 | | LISTMEMBERCOUNT | 1 |
| TEXTWIDTH | 20 | | LISTORDER | 1 |

By formula, with the evaluated set below: 7 636 formulas (64.2 %) have no
function call at all (literals, arithmetic, references), 1 735 (14.6 %)
call only evaluated functions, and 2 532 (21.3 %) call at least one
function this phase does not evaluate (the `THEMEVAL` family dominates).
So **78.7 % of the corpus formulas are within the evaluated set**, and
the geometry formulas of masters are almost all in the 64.2 %.

## Decision

### The evaluated set (v1)

Operators of ADR 0051, plus:

- **Logic and control:** `IF`, `AND`, `OR`, `NOT`, `IFERROR`.
- **Math:** `MIN`, `MAX`, `ABS`, `INT`, `ROUND`, `CEILING`, `FLOOR`,
  `MOD` (alias `MODULUS`), `SUM`, `POW`, `SQRT`, `EXP`, `LN`, `LOG10`,
  `SIGN`, `TRUNC`, `PI`.
- **Trigonometry:** `SIN`, `COS`, `TAN`, `ASIN`, `ACOS`, `ATAN`, `ATAN2`,
  `SINH`, `COSH`, `TANH`.
- **Unit conversion (KICKOFF):** `DEG`, `RAD`.
- **Guard and assignment (KICKOFF):** `GUARD`, `SETF` (basic, ADR 0056).
- **Text:** `STRSAME`, and the `&` operator.
- **Colour:** `RGB`.
- **Reference and lookup:** `INDEX`, `LOOKUP`, `NA`.

`RAND` is deliberately **not** evaluated: recalc must be deterministic
(ADR 0055) or the cross-check and undo are meaningless. Every other
corpus name (the `THEMEVAL`/`THEMEGUARD`/`ISTHEMED` theme family, the
`SETATREF`/`GETREF` edit-at-reference family, `RUNADDON*`, `DOCMD`,
`OPENTEXTWIN`, `DEFAULTEVENT`, `FIELDPICTURE`, `FORMATEX`,
`SHAPETEXT`, `TEXTWIDTH`, `TEXTHEIGHT`, `CONTAINERSHEETREF`,
`LISTSHEETREF`, `SHEETREF`, `PAR`, `PNT`, `_`-prefixed internals) is
**unknown** and follows the KICKOFF rule below.

### Unknown functions

An unknown function in a formula makes the whole cell keep its **cached
value** and records one `UnknownFunction` warning naming the function.
Not a partial evaluation, not an error, not a document failure: a file
full of `THEMEVAL()` cells opens, renders and saves unchanged. Reserved
internal names (`_MOD`, `_ELLIPSE_THETA`, ...; `about-functions.md`) get
the same treatment, with their own warning text.

### The table is hand-written

`crates/shapesheet/src/functions.rs` holds one `FunctionDef` per row:
name, argument count bounds and a `FunctionKind` tag. It is **not**
generated from committed derived data:

- the rows are ~45 of the 209 documented functions, so a generated table
  would mostly hold entries nothing evaluates;
- each evaluated function needs an evaluator implementation anyway, so
  the table and the code land together in one task;
- the *names and arities* are still checked against a source: a test
  asserts every name is spelled the way the ShapeSheet reference lists it
  (`functions-visio-shapesheet-reference.md`, `docs/sources.md` rows 4
  and 19). A later extraction task may generate the full 209-name
  catalogue for a formula-completion UI; nothing here needs it.

`xtask` codegen is unchanged (ADR 0015).

### The evaluator

`eval(&Expr, SheetRef, &mut dyn EvalContext) -> Result<Value, Error>`
computes one expression: operators coerce operands (ADR 0052), functions
take their arguments in order, and a reference asks the context for a
value (ADR 0054). Evaluation errors are `Value::Err` values (ADR 0052);
`Err` is reserved for a broken context. Argument counts are checked
against the table before the call, and a wrong count is `#VALUE!` with a
warning, never a panic.

## Consequences

- Geometry recalculation is fully covered: master geometry formulas are
  arithmetic and references (the 64.2 %).
- The cross-check (ADR 0059) counts a cell only when every function it
  calls is in the set, so the agreement rate is honest about the 78.7 %.
- A hand-written table of ~45 rows is reviewable in one screen and its
  arities are pinned by tests.
- `THEMEVAL` cells keep their cached colours: the renderer already
  resolves them through the theme model (ADR 0021), so nothing gets
  darker or invisible because the evaluator stood back.

## Alternatives considered

- **Generate the table from `reference/` (like `shapesheet.toml`).**
  Rejected for now (see above); the extractor for function pages is a
  cheap later task if a UI needs the full catalogue.
- **Evaluate the theme family (`THEMEVAL`, `THEMEGUARD`, `ISTHEMED`).**
  Rejected: they read the theme's style matrices (ADR 0021), the renderer
  already resolves the same cells, and a wrong theme value would *change*
  documents instead of preserving them.
- **Evaluate `SETATREF`/`GETREF`.** Rejected: they are edit-at-reference
  plumbing for interactive edits, not values a recalc needs; basic `SETF`
  covers what KICKOFF asks for (ADR 0056).
- **Unknown function = `#NAME?` error value.** Rejected by KICKOFF: the
  cached value is kept, so a file that Visio computes correctly keeps
  rendering and saving correctly.
- **Evaluate `RAND` with a fixed seed.** Rejected: any seed differs from
  Visio's cached value, so the cell would always disagree in the
  cross-check; keeping it unknown keeps the cached value.
