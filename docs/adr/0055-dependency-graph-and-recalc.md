# 0055: The dependency graph and recalculation

- Status: accepted
- Date: 2026-10-21
- Task: P6-01

## Context

KICKOFF: "a dependency graph with incremental recalc driven by edits, so
that size and position changes recompute geometry". The graph must sit
inside the layering (`shapesheet` is L2 and runs on `model`, ADR 0002),
must detect the reference cycles the corpus really contains
(`corpus/vsdx/recursion-cycle.vsdx`), must have a predictable cost — the
planning question is a resize on a 1000-shape page — and must keep the
Phase 5 invariants: a failed mutation changes nothing (ADR 0039) and
every change goes through `AppCtx::push_edit` (ADR 0040).

## Decision

### Where the graph lives

`ferrogram-shapesheet::graph::DepGraph`, over `model` addresses. One node
is one cell ([`CellAddress`]: `SheetRef` + `SectionName` + `RowTarget` +
`CellName`, all model types). Nothing below L2 changes; `vsdx`, `render`
and `model` never see the graph.

### Edges

A node's dependencies are:

1. every cell its formula references (the `CellRef`s of ADR 0051,
   resolved per ADR 0054),
2. the cell its **inherited formula** comes from: a cell whose formula is
   `Inh` or absent depends on the rung that defines the formula (the
   master's or style's cell), because editing that one redefines this
   one.

An `Inh` cell that recalc materialises (ADR 0056) drops edge 2 and gains
edge 1 from its own formula.

### Building and invalidating

`DepGraph::build(&Document)` walks every sheet once and parses its
formulas: O(cells with formulas), done once per document on the first
recalc and kept on `DocumentSession` beside the undo stack. An edit
invalidates the nodes it writes and the nodes whose formulas change; the
pass rebuilds only those nodes' edges (the rest of the graph is stable).
A full rebuild is the fallback when an edit adds or removes shapes.

### Evaluation order and cycles

`evaluation_order(&[CellAddress])` returns the dirty cells and their
transitive dependents, dependencies first, **topologically sorted with a
stable tie-break on the address** so the same input always produces the
same result (determinism is what makes the cross-check and undo work).
A back edge means a cycle: the pass **cuts the edge that closes it**,
every cell in the cycle keeps its cached value, and one `Cycle` warning
lists the cells in it. The pass never loops and never recurses
unboundedly: the walk is iterative with an explicit visited set, and the
depth of formula nesting is already capped by `MAX_NESTING` (ADR 0051).

### Budget

One pass has a deterministic budget of `max_evaluations` cell evaluations
(100 000 by default). Not a wall-clock budget: reproducibility beats
latency here, and the number is measured in units the tests can force.
On reaching the budget the pass stops, sets
[`RecalcOutcome::truncated`], leaves the remaining cells stale and
records one `BudgetExceeded` warning. A resize on a 1000-shape page
touches one shape's transform and geometry cells and their dependents —
tens of nodes, not thousands — and the budget exists for the pathological
case (a formula that fans out over a whole page). Task P6-10 measures the
resize case and may tighten the constant.

### What triggers recalc

1. **An edit** — after `Edit::apply` and before the undo step is pushed
   (ADR 0056), and after `Edit::revert` too, so redo/undo leave the same
   consistent state.
2. **A fill-in pass on open** — only for cells that have a formula and no
   cached value (ADR 0057); the rest of the file keeps the values Visio
   wrote.
3. **Never on save.** The writer is a pure function of the model (ADR
   0057); recalc is an editing act with undo.

## Consequences

- A resize recomputes geometry incrementally; the cost is proportional to
  the dirty subgraph, not to the document.
- Deterministic output makes the cross-check (ADR 0059) reproducible and
  a recalc idempotent (recalc twice, same values).
- Cycles are survivable: `recursion-cycle.vsdx` opens, renders and saves
  with one warning instead of hanging (the Phase 5 behaviour of such a
  file is undefined; this defines it).
- Keeping the graph on the session (not in `model`) leaves L1 free of
  engine state and keeps the layering check untouched.

## Alternatives considered

- **Graph in `model` or in `commands`.** Rejected: `model` must stay
  engine-free (it is shared with the reader and writer), and `commands`
  is L4 — the engine belongs with the formulas (L2).
- **Recompute everything on every edit.** Rejected: O(document) per drag
  step, and the Phase 5 drag budget (0.6–1.6 ms per frame) would not
  survive it.
- **Lazy evaluation on read (a value cache invalidated per cell).**
  Rejected: it makes the renderer's cost unpredictable and hides the
  cross-check's disagreements; an explicit pass reports what it changed.
- **A wall-clock time budget.** Rejected: results would depend on machine
  load, breaking determinism, the cross-check and the undo property.
- **Erroring on a cycle.** Rejected: a cyclic file is readable (ADR 0013
  makes the same call for style chains), and failing the document would
  break "never crash, never fail the file".
- **Evaluate in dependency order with memoisation only (no graph).**
  Half-adopted: the evaluator memoises within one pass, but the graph is
  what answers "what is dirty" for an incremental pass, and it is where
  the cycle cut lives.
