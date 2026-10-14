# 0022: Geometry semantics and curve flattening

- Status: accepted
- Date: 2026-10-14
- Task: P3-01

## Context

KICKOFF's Phase 3 covers "all Geometry row types". [MS-VSDX] and the
ShapeSheet reference define each row's cells but not the curve
mathematics; the renderer must convert every row to path primitives
exactly once, in `ferrogram-geom` (L0), from sources we are allowed to
use. NURBS and splines are not native path primitives, so flattening needs
a tolerance.

## Decision

### Curve types (`ferrogram_geom::curve`)

| Type | From row(s) | Meaning (sources: [MS-VSDX] and the ShapeSheet reference) |
|---|---|---|
| `Arc` | `ArcTo` | Circular arc from the current point to `(X, Y)` whose **bow** (sagitta) is `A`: the distance from the arc's midpoint to the chord's midpoint. Radius `= bow/2 + chord²/(8·bow)`; `bow = 0` is a straight line; a negative bow curves to the other side of the chord. |
| `EllipticalArc` | `EllipticalArcTo`, `RelEllipticalArcTo` | Arc from the current point to `(X, Y)` through the control point `(A, B)` (a point **on** the arc, best near the middle), with major-axis angle `C` from the parent's x axis and major/minor ratio `D` (the ratio need not exceed 1; `D ≤ 0` is a warning and degrades to a straight line). `C` and `D` are often absent (the corpus omits them); the defaults are `C = 0`, `D = 1`. |
| `Nurbs` | `NURBSTo` | The E-cell `NURBS(knotLast, degree, xType, yType, x1, y1, knot1, weight1, ...)` formula: a nonuniform rational B-spline of `degree`, control points `(xi, yi)` with knots and weights; the row's `(X, Y)` is the last control point and `A`/`B`/`C`/`D` hold the second-to-last knot, last weight, first knot and first weight. `xType`/`yType` `0` means the coordinates are fractions of `Width`/`Height`, `1` means local coordinates. The formula is authoritative when it parses; otherwise the row's `X, Y, A, B, C, D` define a degree-3 B-spline with the current point as first control point (weights `1`). |
| `Spline` | `SplineStart` + `SplineKnot` | A B-spline of degree `D` (integer 1..=25): the first control point is the current point (the preceding row's end), `SplineStart`'s `(X, Y)` the second, each `SplineKnot`'s `(X, Y)` another; knots are `SplineStart`'s `B` (first), `A` (second) and `C` (last) plus each `SplineKnot`'s `A`. |
| `Polyline` | `PolylineTo` | The A-cell point list, ending at `(X, Y)`; equivalent to a `LineTo` sequence. Unparsable list degrades to a single `LineTo`. |
| `Quad` | `RelQuadBezTo` | Quadratic Bézier: endpoint `(X, Y)`, control point `(A, B)`. |
| `Cubic` | `RelCubBezTo` | Cubic Bézier: endpoint `(X, Y)`, control points `(A, B)` and `(C, D)`. |

Whole-section rows:

| Row | Meaning |
|---|---|
| `Ellipse` | An ellipse by centre `(X, Y)` and two conjugate semi-diameter endpoints `(A, B)` and `(C, D)`: `{ centre + u·cos t + v·sin t }` with `u = (A,B) - centre`, `v = (C,D) - centre`. With perpendicular `u, v` (the corpus case) this is the usual ellipse. |
| `InfiniteLine` | The line through `(X, Y)` and `(A, B)`. Rendering clips it to the page rectangle. |

`MoveTo`/`RelMoveTo` start a subpath and also start a **new** subpath in
the middle of a section (a stroking gap); a fill treats the gap as a
straight closing segment (the reference's rule). `LineTo`/`RelLineTo`
append a segment; the `Rel*` values are fractions of the shape's
`Width`/`Height` (ADR 0019). A row type we do not know renders as nothing
and warns (ADR 0021).

### Conversion and flattening

`ferrogram_geom` exposes:

```rust
pub enum Curve {
    Arc(Arc), EllipticalArc(EllipticalArc), Quad { .. }, Cubic(Cubic),
    Nurbs(Nurbs), Spline(Spline), Polyline(Polyline),
}
impl Curve {
    /// Cubic Bézier approximation within `tolerance` inches.
    pub fn to_cubics(&self, tolerance: f64) -> Result<Vec<Cubic>, Error>;
}
```

- `Quad`, `Cubic` and `Polyline` convert exactly (quads to one cubic,
  polyline to lines).
- `Arc` and `EllipticalArc` convert by arc splitting into at most 90°
  pieces, each to one cubic, stopping when the deviation from the true arc
  is below the tolerance.
- `Nurbs` and `Spline` convert by de Boor evaluation with adaptive
  subdivision of the knot spans until the chord deviation is below the
  tolerance.

**Tolerance**: the renderer flattens to at most **0.25 device pixels** of
deviation at the target scale: `tolerance = 0.25 / scale` inches (the
canonical unit is the inch, ADR 0011). The SVG backend writes native
elliptical-arc commands where the curve is an `Arc`/`EllipticalArc` and
cubics otherwise, with the same tolerance.

Degenerate inputs (zero chord, `bow = 0`, `D ≤ 0`, degree 0 or > 25, knot
or weight lists of the wrong length, non-finite values) degrade to the
straight chord or line and warn; they never error the render and never
panic (AGENTS.md section 6).

## Consequences

- Every row type has one implementation point (`to_cubics`) and one test
  point (P3-03 tests the math, P3-05 tests row-to-path mapping).
- PNG and SVG agree on curve shape by construction (same cubics, same
  tolerance).
- The tolerance is resolution-dependent for PNG, so goldens compare at a
  fixed scale (ADR 0025).

## Alternatives considered

- **Flatten in the model when typing rows.** Rejected: the model keeps
  meaning (`GeometryRow`), geometry keeps math (`Curve`), the renderer
  keeps policy (tolerance). Layering (ADR 0002) puts the math at L0.
- **A fixed segment count per curve.** Rejected: visible faceting on large
  arcs and wasted segments on small ones.
- **Evaluate NURBS with a generic B-spline crate.** Rejected: `spline`-type
  crates are few and unmaintained; the de Boor algorithm is small, and our
  inputs are the unusual Visio encodings anyway. Revisit if the alpha needs
  rational weights beyond unit.
- **Emit SVG `A` for NURBS too.** Rejected: SVG arcs are elliptical only.

## Amendments

### Amended 2026-10-14, P3-05: the `NURBSTo` fallback and the knot lists

The fallback sentence above ("otherwise the row's `X, Y, A, B, C, D`
define a degree-3 B-spline with the current point as first control point")
is replaced by a simpler rule, decided by the master when the corpus
showed that no file exercises the encoded path: **when the `NURBS(...)`
formula does not parse (or is absent), the builder draws a straight
segment to the row's `X`/`Y` and records one
`WarningKind::ErrorValue` warning on the `E` cell.** A curve fitted from
`A`..`D` alone would invent shape the file never asked for; the straight
chord is the honest degradation (the same rule libvisio's comment
suggests for empty NURBS data).

The knot lists are assembled in the form `ferrogram-geom` documents
("Knot vectors" in `curve.rs`): one stored knot per control point plus a
final knot. For `NurbsTo` that is the current point (knot `C`, weight
`D`) followed by the formula's control points and the row's `(X, Y)`
(knot `A`, weight `B`), with the formula's `knotLast` as the final knot;
the formula's `xType`/`yType` scaling (fractions of `Width`/`Height`)
applies to the formula's points only. For `SplineStart` + `SplineKnot`
it is `B`, `A`, each `SplineKnot`'s `A`, and `C` as the final knot over
the control points current point, `SplineStart`'s `(X, Y)` and each
`SplineKnot`'s `(X, Y)`.
