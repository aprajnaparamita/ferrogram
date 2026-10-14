# 0020: Scene graph

- Status: accepted
- Date: 2026-10-14
- Task: P3-01

## Context

Phase 3 renders the model to PNG and SVG. Phase 4 draws the same content on
an egui canvas. All three need one intermediate representation between
`ferrogram-model` (L1) and the backends: the scene graph, in
`ferrogram-render` (L3, ADR 0002). It must express paths, paints, strokes,
line ends, shadows, text and clipping, and it must carry the shape hierarchy
transforms already resolved into page coordinates.

## Decision

### Types (`render::scene`)

Plain data, no behaviour beyond trivial constructors:

```rust
pub struct Rgba { pub r: u8, pub g: u8, pub b: u8, pub a: u8 }

pub enum Paint {
    Solid(Rgba),
    LinearGradient { start: Point, end: Point, stops: Vec<GradientStop> },
    RadialGradient { center: Point, radius: f64, stops: Vec<GradientStop> },
    Pattern(PatternPaint),
}
pub struct GradientStop { pub offset: f64, pub color: Rgba }
pub struct PatternPaint {
    pub foreground: Rgba, pub background: Rgba,
    pub tile: geom::Path, pub tile_size: geom::Size,
}

pub enum LineCap { Butt, Round, Square }
pub enum LineJoin { Miter, Round, Bevel }

pub struct LineEnd {
    pub shape: geom::Path,   // unit shape, scaled by width/length
    pub width: f64,          // inches
    pub length: f64,         // inches
    pub filled: bool,
    pub color: Option<Rgba>, // None: use the stroke paint
}
pub struct Stroke {
    pub paint: Paint,
    pub width: f64,          // inches
    pub dash: Vec<f64>,      // inches; empty = solid
    pub cap: LineCap,
    pub join: LineJoin,
    pub miter_limit: f64,
    pub start: Option<LineEnd>,
    pub end: Option<LineEnd>,
}
pub struct Shadow { pub color: Rgba, pub offset: Point, pub blur: f64 }
pub struct Clip { pub path: geom::Path }

pub struct TextNode {
    pub fonts: Vec<text::Font>,   // font indexes used by the glyphs below
    pub layout: text::Layout,
    pub color: Rgba,
    pub origin: Point,
}

pub struct Shape {
    pub transform: geom::Transform,
    pub path: geom::Path,
    pub fill: Option<Paint>,
    pub stroke: Option<Stroke>,
    pub shadow: Option<Shadow>,
    pub clip: Option<Clip>,
    pub text: Option<TextNode>,
}
pub struct Group {
    pub transform: geom::Transform,
    pub clip: Option<Clip>,
    pub children: Vec<Node>,
}
pub enum Node { Group(Group), Shape(Box<Shape>) }  // Shape boxed: keeps the enum small

pub struct Scene {
    pub width: f64,              // page width, inches
    pub height: f64,             // page height, inches
    pub background: Option<Rgba>,
    pub nodes: Vec<Node>,
    pub warnings: Vec<model::CellWarning>,
}
```

Paths are `ferrogram_geom::Path` (ADR 0022) in **page coordinates, in
inches**; the backend applies only its device transform (scale, y flip for
PNG). Text glyphs carry a font index into `TextNode::fonts` and an em-unit
outline from `text::Font::outline` (ADR 0024).

### Transforms from the shape hierarchy

The builder (`render::build`) composes each shape's transform from its
resolved cells, exactly as Visio composes them:

```
page = translate(PinX, PinY) · rotate(Angle) · scale(FlipX ? -1 : 1, FlipY ? -1 : 1)
       · translate(-LocPinX, -LocPinY)
```

applied to shape-local coordinates (the shape occupies `(0, 0)` to
`(Width, Height)` in local space). A group's transform is the same
composition and multiplies its children's; sub-shapes compose through their
parent chain (the arena links of ADR 0011). All values come from the
inheritance lookup (ADR 0013/0019), never from the local cell alone, so
master and style values apply.

### How rendering reads cells

Every cell goes through `ferrogram_model::CellReader` (ADR 0021) on the
shape's `SheetRef`: resolved value plus a warning when the cached value is
missing or wrong. Geometry rows come from `Document::shape_geometry`
(ADR 0019). Layer visibility comes from the page's `Layer` rows and the
shape's `LayerMember`; invisible layers drop their shapes. A background
page is rendered as a leading `Group` of the page's scene.

### Builder output

`build_scene(document, page, options) -> Result<Scene, Error>` walks the
page's top-level shapes in file order, recursing into groups with a depth
cap (128, the writer's cap) and a visited set per background chain. It
never panics: bad values become warnings and defaults (ADR 0021).

## Consequences

- PNG (P3-06), SVG (P3-07) and later the egui canvas consume one type;
  backends stay thin.
- The scene is device-independent: no pixels, no fonts rasterized.
- Visual-style indices (line pattern 12, arrowhead 4, fill pattern 27) are
  resolved **before** the scene, by `render::styles` (ADR 0023): the scene
  holds concrete dashes and arrowhead paths, so backends know nothing about
  Visio indices.
- `Scene::warnings` carries the ADR 0021 warnings to the caller and the
  CLI.

## Alternatives considered

- **Render directly from the model in each backend.** Rejected: three copies
  of inheritance and geometry logic (KICKOFF wants the scene graph shared
  with the UI).
- **A retained-mode renderer trait instead of data.** Rejected: data is
  testable without a device, snapshotable and serializable later.
- **Transforms baked into the paths at build time.** Rejected: the UI needs
  node-level transforms for selection and hit testing (Phase 5), and
  baking loses the hierarchy.
- **Indices in the scene, patterns resolved in the backends.** Rejected:
  every backend would need the style tables, and golden tests would compare
  resolved shapes instead of intent.
