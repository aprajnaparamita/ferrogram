# 0025: PNG and SVG backends and determinism

- Status: accepted
- Date: 2026-10-14
- Task: P3-01

## Context

Phase 3 outputs PNG and SVG (KICKOFF). The backends live in
`ferrogram-render` (L3) behind the scene graph (ADR 0020). Golden image
tests (ADR 0027) need defined output: a DPI/scale option, a background, and
determinism (same input, same bytes) so a diff means a real change.

## Decision

### PNG: `tiny-skia`

`tiny-skia` (BSD-3-Clause) rasterizes paths, gradients and dashing. It is pure Rust
(no C), builds for `wasm32-unknown-unknown` (the web app renders too), and
is the standard choice in the Rust graphics stack. Anti-aliasing is on by
default and can be turned off in `PngOptions` for debugging.

```rust
pub struct PngOptions { pub scale: f64, pub antialias: bool }
/// Rasterizes a scene to PNG bytes at `scale` pixels per inch.
pub fn render_png(scene: &Scene, opts: &PngOptions) -> Result<Vec<u8>, Error>;
```

Default `scale` is 96 pixels per inch (screen scale 1:1 at 96 DPI);
`scale` is a plain multiplier in pixels per inch, so `--scale 300` is a
print-scale render. The raster is `ceil(width * scale)` by
`ceil(height * scale)` pixels; the device transform maps inches to pixels
with y flipped (page y up, image y down). The background is `Scene::background`
(white when set, transparent when `None`).

### SVG: our own writer

No crate: `resvg`/`usvg` are MPL-2.0 (not allowed outside `vsd-binary`,
ADR 0003/`deny.toml`) and the remaining SVG crates add little over writing
XML ourselves. The writer emits a small, fixed subset: `svg`, `g`
(transform), `path` (`d`), `defs`/`linearGradient`/`radialGradient`/`pattern`,
`clipPath`, and text as **glyph outlines** (`path` elements from
`Font::outline`), never `<text>`. Glyph outlines make the SVG
self-contained: no font is needed at view time and no viewer text engine can
change the result.

```rust
pub struct SvgOptions { pub scale: f64 }
/// Writes a scene as a standalone SVG document (viewBox in inches, scaled).
pub fn render_svg(scene: &Scene, opts: &SvgOptions) -> Result<String, Error>;
```

Numbers are written with fixed 6-decimal formatting (inches scaled to
pixels by `scale`), attributes in a fixed order, so the output is
byte-stable.

### Determinism

- **SVG is deterministic**: fixed formatting, fixed order, no timestamps.
- **PNG is deterministic per platform**: `tiny-skia` is pure Rust, but
  curve flattening uses `f64` trigonometry (`sin`, `cos`, `sqrt`) whose
  least bits differ between platforms' libm. Same machine, same bytes;
  across platforms, possibly sub-pixel differences.
- Golden tests therefore compare with a **tolerance metric**, not bytes
  (ADR 0027), and the committed goldens are generated on one reference
  platform (macOS arm64) with the metric absorbing cross-platform noise.

### Options summary

| Option | Where | Meaning |
|---|---|---|
| `scale` | PNG, SVG | pixels per inch (default 96) |
| `antialias` | PNG | on by default; off renders hard edges |
| `background` | scene | `Some(Rgba)` or transparent |
| `strict` | CLI | warnings become a non-zero exit (ADR 0021/0026) |

## Consequences

- `cargo deny` stays green: `tiny-skia` is BSD-3-Clause; no MPL crate outside
  `vsd-binary`.
- The wasm build gains `tiny-skia` through `ferrogram-render`; the web app
  can render PNGs in the browser in Phase 8.
- Our own SVG writer is ~1 small module and exactly what the scene emits;
  nothing to trim.

## Alternatives considered

- **`resvg`/`usvg`.** Rejected: MPL-2.0 (deny.toml forbids it outside
  `vsd-binary`) and it wants to own the DOM.
- **`svg` crate (write-only).** Possible, but the surface we need is a
  handful of elements; string building with one escaping helper is less
  dependency than the crate. Revisit if the subset grows.
- **Text as `<text>` in SVG.** Rejected: viewer-dependent rendering breaks
  goldens and "same input, same bytes".
- **Byte-equal goldens everywhere.** Rejected: libm differences would make
  CI flaky; tolerance is the standard practice.
- **Rendering at a fixed 1:1 scale only.** Rejected: the CLI must render
  print-scale and thumbnails.

## Amendments

### Amended 2026-10-15, P3-16: one scale cap for both backends

The review (P3-12 minor 5) found the two backends treating `scale`
differently: PNG capped the raster and reported a huge `--scale` as
`TooLarge` (as if the document were too big), SVG wrote absurd
dimensions. One shared cap now applies to both (the constant
`render::MAX_DEVICE_EXTENT`, 16 384 pixels per side):

- `Error::InvalidOptions` (new) when `scale` is not finite, not
  positive, or **over the cap in pixels per inch** (a scale no page of
  one inch can fit): the option is wrong for any scene.
- `Error::TooLarge` when `page size x scale` exceeds the cap per side
  (or the raster/byte caps apply): the document does not fit at that
  scale.

The CLI keeps exit 3 for both (ADR 0026) and reports the difference in
its message ("the render options are not valid" vs "output is larger
than the allowed caps"). `BuildOptions::scale` (the builder's flattening
tolerance) keeps its documented fallback for a non-finite or
non-positive value; the backends are the enforcement point.
