# 0033: Canvas raster backend

- Status: accepted
- Date: 2026-10-16
- Task: P4-01

## Context

Phase 4 draws a page in the egui app: "Drawing canvas: pan, zoom, rulers
and grid", "page tabs and status bar", and the exit criterion "opening
corpus files shows them on the canvas". The scene graph (ADR 0020) already
carries everything needed to draw a page, and `ferrogram-render` (L3) has
PNG and SVG backends (ADR 0025). The options are: a third backend that
turns the scene into egui shapes, a `tiny-skia` raster uploaded as an egui
texture, or both. Phase 5 needs hit-testing and cheap redraw while dragging.

Layering (ADR 0002) forbids `ferrogram-render` (L3) from seeing egui, so
an "egui shapes" backend would live in `crates/ui` (L6) consuming the
scene, or `render` would gain a plain-shape output the UI converts.

## Decision

### Phase 4: raster texture

The canvas renders the page with `tiny-skia` into raw RGBA and uploads it
as one egui texture, drawn in a scrollable area with an egui transform.

`ferrogram-render` gains one public entry point (new module `raster.rs`,
P4-05):

```rust
/// RGBA raster of a scene, row-major, 8 bits per channel, no padding.
pub struct RasterImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// `width * height * 4` bytes, RGBA order.
    pub rgba: Vec<u8>,
}

pub struct RasterOptions {
    /// Pixels per inch (ADR 0025). Default [`build::DEFAULT_SCALE`].
    pub scale: f64,
    /// Anti-aliasing on by default (ADR 0025).
    pub antialias: bool,
}

/// Rasterizes a scene to raw RGBA at `scale` pixels per inch.
pub fn render_raster(scene: &Scene, opts: &RasterOptions) -> Result<RasterImage, Error>;
```

`render_raster` shares the PNG backend's rasterization path (one code
path for pixels; `render_png` encodes the same raster) and the ADR 0025
caps: `Error::InvalidOptions` for a bad `scale`, `Error::TooLarge` past
`MAX_DEVICE_EXTENT`. The PNG goldens therefore cover the canvas pixels
too.

### Caching and redraw

`crates/ui::canvas::CanvasCache` holds the texture handle plus the key it
was rasterized for (document generation counter, page, raster scale).
Rules:

- Document changed or page switched: re-rasterize.
- Zoom changed: re-rasterize only when `zoom` moves more than 10% from the
  rasterized zoom; in between the texture is scaled on screen (panning is
  always free: no re-rasterize).
- Window resized: no re-rasterize (the texture is drawn at device scale).

Rasterizing at `zoom * 96` PPI keeps the view sharp at the settled zoom;
the brief scaled blur between re-rasterizations is accepted in Phase 4.

### The chrome around the canvas

- **Rulers** are drawn as egui vector art (labels in display units from
  `Settings::units`, ticks at 1/8 inch), from `ViewState` pan/zoom. Units
  convert at this edge only (AGENTS.md section 6).
- **Grid** is drawn as egui lines behind the texture when
  `ViewState::show_grid`; snap-to-grid arrives in Phase 5.
- **Pan** is middle-drag or space-drag or scrollbars; **zoom** is Ctrl +
  mouse wheel and the `view.zoom.in`/`view.zoom.out`/`view.zoom.fit`
  commands (settings `ViewState`, ADR 0029).
- **Page tabs** live in a strip under the canvas; clicking activates a
  page (`ViewState`/`DocumentSession::active_page`), `page.next` /
  `page.previous` commands wrap the same change (for shortcuts and MCP).
- **Status bar** shows the page name and size (inches formatted in
  `Settings::units`), the zoom percentage, and the last warning
  (`AppCtx::warnings`).

### Phase 5 upgrade path (recorded, not built)

Hit testing and drag redraw need point lookup in the scene. When Phase 5
needs it, `ferrogram-render` gains `hit_test(scene, point) -> Option<Node>`
and `crates/ui` may gain a vector backend (scene to `egui::Shape`), which
is legal in L6. The raster stays the fidelity reference (same pixels as
the PNG goldens); drags draw transient egui shapes over the cached raster.

## Consequences

- Canvas pixels match the tested PNG backend exactly; the exit-criterion
  test can compare a canvas screenshot region against a golden PNG.
- `render_png` and `render_raster` share one rasterizer; determinism
  notes (ADR 0025) apply unchanged.
- Zoomed-in views are briefly soft between re-rasterizations; accepted.
- No egui dependency below L6 (layers stay green); `render` gains a plain
  RGBA output type only.

## Alternatives considered

- **Vector egui backend now (scene to `egui::Shape`).** Rejected for
  Phase 4: gradients, patterns, line ends and glyph outlines all need
  conversion code that Phase 4's exit criterion does not require, and it
  would fork the pixel path from the PNG goldens. The Phase 5 hit-test
  need may revive it.
- **`wgpu` render pass from the scene.** Rejected (ADR 0030): heavy tree;
  the raster already satisfies the criterion.
- **Emit PNG bytes and decode in the UI.** Rejected: a needless
  encode/decode round trip per redraw; raw RGBA is the natural output of
  the shared rasterizer.
- **SVG scene in a web view.** Rejected: no web view in egui; fidelity
  would diverge from the PNG path.
- **Full-page re-rasterize on every pan.** Rejected: panning must be
  smooth; the texture transform makes it free.
