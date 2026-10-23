# 0051: Formula grammar, AST and printer

- Status: accepted
- Date: 2026-10-21
- Task: P6-01

## Context

Phase 6 turns the formula source strings the model stores (ADR 0014) back
into values. The first piece is the grammar: literals with units, strings,
operators and precedence, function calls and every reference form a corpus
file or the ShapeSheet reference shows. The parser must never panic on bad
input (AGENTS.md section 6), must report errors a UI can underline, and
must produce a tree a printer can turn back into source text so a
parse-print-parse property test is possible (ADR 0060).

Sources (`docs/sources.md` rows 4 and 19): the Microsoft Learn ShapeSheet
reference's `about-formulas`, `about-operators`, `about-cell-references`,
`about-strings`, `about-error-values`, `about-units-of-measure` and
`about-functions` pages. Corpus facts come from the 15 `corpus/vsdx` files
(11 903 formulas; ADR 0053).

## Decision

### The grammar

One formula is one line of source text, **without** the leading `=` (the
model stores it that way and Visio inserts the `=` itself). In EBNF, with
the pieces kept as written where noted:

```
formula   = expr ;
expr      = comparison ;
comparison= concat { ( "=" | "<>" | "<" | "<=" | ">" | ">=" | alt_op ) concat } ;
concat    = additive { "&" additive } ;
additive  = multiplicative { ( "+" | "-" ) multiplicative } ;
multiplicative = power { ( "*" | "/" ) power } ;
power     = unary [ "^" power ] ;
unary     = ( "+" | "-" | "%" ) unary | postfix ;
postfix   = primary { "%" } ;        (* "37%" is 0.37 *)
primary   = number | string | bool | call | reference | "(" expr ")" ;
call      = ident "(" [ expr { "," expr } ] ")" ;
bool      = "TRUE" | "FALSE" ;
string    = '"' { char | '"' '"' } '"' ;
number    = digits [ "." digits ] [ exponent ] [ unit ] [ mixed ] ;
unit      = ident | "%" | "'" | '"' | "°" ;
mixed     = digits "/" digits ;            (* "5 1/2" is 5.5 *)
```

- **Precedence** is the `about-operators.md` order: unary `+`, `-`, `%`,
  then `^`, `* /`, `+ -`, `&`, the ordering comparisons (`< <= > >=`), and
  the equality comparisons (`= <>`) last. `^` is right-associative; the
  binary operators are left-associative. Parentheses override the order.
- **Alternative comparison operators** `_GT_`, `_LT_`, `_GE_`, `_LE_`,
  `_EQ_`, `_NE_` parse to the same six comparison operators. They
  compare exactly at the full 15 significant digits Visio stores values
  in, where the symbolic forms are the everyday choice (P6-02 flagged
  this difference for the black-box track).
- **Strings** use `"` delimiters and double themselves as the escape
  (`"say ""hi"""` is `say "hi"`).
- **Numbers** are decimal with an optional exponent, an optional unit
  suffix (ADR 0052) and the mixed-fraction form `5 1/2` of
  `about-units-of-measure.md`. The implicit form `number[unit, flag]`
  keeps `unit` and `flag` (`A`, `D`, `E`, `P`, `T`) on the literal.
- **Function calls** need their parentheses even with no arguments
  (`about-functions.md`); arguments are comma-separated. Names are
  case-insensitive and are kept as written. **No argument may be left
  out**: the grammar has no empty-argument production (`FORMATEX(a,,b)`
  is a syntax error, P6-02), and one is added only when a function in
  the evaluated set needs it (P6-12 review F5 ruling).
- **References** are every form of `about-cell-references.md`: `Width`,
  `Wheel!Angle`, `Frame.2!Height`, `Char.Size[3]`, `Scratch.B3`,
  `Sheet.12!FillForegnd`, `Masters[Bearing]!Hub!Geometry1.X1`,
  `ThePage!User.Anchor_Point`, `Pages[Sheet-3]!Sheet.4!BeginX`,
  `Styles!Ledger!LineColor`, `TheDoc!DocScale`, and a quoted
  nonstandard name (`'2-C'!LineColor`, with `''` as the quote escape).

### Parts are kept as written

A [`CellRef`] holds the sheet parts before the `!` separators, the section
part and the dotted name parts after it, **verbatim**. The parser does not
decide whether `Prop.Cost` names a row (the `Cost` row's `Value` cell) or
`Scratch.B3` a column at row 3: that depends on the section the name
resolves against, so it is a resolution rule (ADR 0054), not a syntax
rule. Splitting at parse time was tried first and rejected: it cannot be
done without the document.

### The AST

`Expr { span, kind }` with `ExprKind::{Literal, Ref, Unary, Binary, Call}`,
`Literal::{Number(NumberLiteral), Str, Bool}`, `NumberLiteral { value,
unit, flag }`, `UnaryOp::{Plus, Minus, Percent}` and `BinaryOp` for the
eleven infix operators. Spans are byte offsets into the source
([`Span`]); they are not part of equality, so a round-trip test compares
trees ignoring spans.

### Printer

`print(&Expr) -> Result<String, Error>` emits source text that parses to
the same tree: it parenthesises by precedence, quotes strings with the
doubled-quote escape and prints numbers in a form that reparses to the
same value (round-trip by value, not by digits). The property test is
parse-print-parse identity on generated trees (ADR 0060).

### Errors and limits

Bad input is `Error::Syntax { span, kind }` with `SyntaxKind` naming the
class (`UnexpectedEnd`, `UnexpectedToken`, `UnterminatedString`,
`InvalidNumber`, `InvalidUnit`, `InvalidReference`, `InputTooLong`,
`TooManyTokens`, `NestingTooDeep`). Three limits bound the work, all
checked before allocation grows: `MAX_SOURCE_LEN` 65 536 bytes,
`MAX_TOKENS` 4 096, `MAX_NESTING` 32. Over a limit is a syntax error of
the matching kind, never a panic and never an unbounded recursion.

## Consequences

- Parse-print-parse is testable without a document; the parser is a
  hand-written lexer plus recursive descent, ~600 lines with tests.
- Keeping reference parts as written makes the parser ignorant of the
  document and pushes the row/column rules into one resolver (ADR 0054).
- The limits make the fuzz target (ADR 0060) cheap: input that would
  explode is rejected by length and depth first.
- Unknown reference forms and function names parse fine (an identifier is
  an identifier); they fail later, at resolution or evaluation, where the
  policy is a warning and a cached value (ADRs 0053, 0054).

## Alternatives considered

- **A parser generator (`chumsky`, `lalrpop`, `pest`).** Rejected: a new
  dependency and a build step for a small grammar, and error messages
  with spans are easier to control by hand.
- **Split references into structured `section`/`row`/`cell` fields at
  parse time.** Rejected: `Prop.Cost`, `Scratch.B3` and `Char.Size` need
  the section's row shape to split correctly (see above).
- **Store the parsed row index in the AST.** Same problem; the resolver
  already has to look the section up.
- **A lossless syntax tree (keep every token, including trivia).** The
  writer never rewrites formulas (ADR 0057 keeps `F` as stored), so
  lossless trivia buys nothing over keeping the source string alongside.
- **Case-fold names at parse time.** Rejected: the printer must reproduce
  what the file wrote (round-trip identity, ADR 0048); folding is a
  lookup rule (ADR 0054).
