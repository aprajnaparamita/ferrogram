//! Structural comparison against libvisio's `vsd2xhtml` (task P3-10,
//! ADR 0027 section 4 and ADR 0010).
//!
//! This test file is oracle-only: it consumes the CLI tool's stdout as an
//! independent opinion on the same file, and holds no libvisio code and
//! no description of libvisio's output format beyond parsing counts and
//! path coordinates out of the XHTML (an open SVG vocabulary). The oracle
//! is built by `cargo xtask oracle build` (a dev-only harness under
//! `tools/libvisio-oracle/`); the test skips with a message when
//! `LARRYGRAM_LIBVISIO_ORACLE` is unset, and `LARRYGRAM_REQUIRE_ORACLE=1`
//! makes a missing oracle a failure (ADR 0010).
//!
//! What is compared, per page: the page count, the shape count (the
//! oracle's `<svg:g>` groups), the path count (the oracle's `<svg:path>`
//! elements against the scene's filled and stroked shape paths) and the
//! overall drawing bounding box within 0.05 inches (the two renderers
//! disagree about stroke extents). Files the oracle cannot parse record
//! the skip and fall back to the corpus render test.

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use larrygram_geom::{Point, Segment, Transform};
use larrygram_render::scene::{Group, Node, Scene, Shape};
use larrygram_vsdx::{Limits, read_document};

/// The bounding box tolerance, in inches (ADR 0027).
const BBOX_TOLERANCE: f64 = 0.05;

/// The oracle tools directory when `LARRYGRAM_LIBVISIO_ORACLE` names an
/// existing directory; `None` when it is unset, empty or missing (ADR
/// 0010).
pub fn oracle_dir() -> Option<PathBuf> {
    oracle_dir_from(std::env::var_os("LARRYGRAM_LIBVISIO_ORACLE"))
}

/// Whether `LARRYGRAM_REQUIRE_ORACLE=1` turns a missing oracle into a
/// test failure instead of a skip (set in CI on Linux and macOS).
pub fn require_oracle() -> bool {
    require_from(std::env::var("LARRYGRAM_REQUIRE_ORACLE").as_deref().ok())
}

/// [`oracle_dir`] with an explicit value for the environment variable.
fn oracle_dir_from(var: Option<std::ffi::OsString>) -> Option<PathBuf> {
    let dir = PathBuf::from(var?);
    dir.is_dir().then_some(dir)
}

/// [`require_oracle`] with an explicit value for the environment variable.
fn require_from(var: Option<&str>) -> bool {
    var == Some("1")
}

/// Runs `<oracle>/vsd2xhtml` on one file and returns its stdout.
fn run_vsd2xhtml(oracle: &Path, file: &Path) -> Option<String> {
    let output = Command::new(oracle.join("vsd2xhtml"))
        .arg(file)
        .output()
        .ok()?;
    if !output.status.success() {
        eprintln!(
            "skipped {}: vsd2xhtml exited with {}",
            file.display(),
            output.status
        );
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    if text.contains("<svg:svg") {
        Some(text)
    } else {
        eprintln!("skipped {}: vsd2xhtml printed no drawing", file.display());
        None
    }
}

/// An axis-aligned box in page coordinates, in inches, y down.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Box {
    /// The smallest x.
    min_x: f64,
    /// The smallest y.
    min_y: f64,
    /// The largest x.
    max_x: f64,
    /// The largest y.
    max_y: f64,
}

impl Box {
    /// The box of one point.
    fn of_point(p: Point) -> Self {
        Self {
            min_x: p.x,
            min_y: p.y,
            max_x: p.x,
            max_y: p.y,
        }
    }

    /// Grows the box to hold the point.
    fn add(&mut self, p: Point) {
        self.min_x = self.min_x.min(p.x);
        self.min_y = self.min_y.min(p.y);
        self.max_x = self.max_x.max(p.x);
        self.max_y = self.max_y.max(p.y);
    }

    /// The union of two boxes.
    fn union(self, other: Self) -> Self {
        Self {
            min_x: self.min_x.min(other.min_x),
            min_y: self.min_y.min(other.min_y),
            max_x: self.max_x.max(other.max_x),
            max_y: self.max_y.max(other.max_y),
        }
    }

    /// Whether every edge is within `tolerance` of `other`.
    fn near(self, other: Self, tolerance: f64) -> bool {
        (self.min_x - other.min_x).abs() <= tolerance
            && (self.min_y - other.min_y).abs() <= tolerance
            && (self.max_x - other.max_x).abs() <= tolerance
            && (self.max_y - other.max_y).abs() <= tolerance
    }

    /// The box's width and height.
    fn size(self) -> (f64, f64) {
        (self.max_x - self.min_x, self.max_y - self.min_y)
    }
}

/// One page of the oracle's XHTML: its counts and drawing box.
#[derive(Debug, Clone)]
struct OraclePage {
    /// The `<svg:g>` count.
    shapes: usize,
    /// The `<svg:path>` count.
    paths: usize,
    /// The drawing box in inches (y down), when every path parsed.
    bounds: Option<Box>,
    /// The page size in inches, from the `svg` element.
    size: (f64, f64),
    /// Why the page skipped a comparison, when it did.
    skip: Option<String>,
}

/// The `<svg:svg ...>...</svg:svg>` blocks of one XHTML document.
fn svg_blocks(text: &str) -> Vec<&str> {
    let mut blocks = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("<svg:svg") {
        let tail = rest.get(start..).unwrap_or(rest);
        let end = tail.find("</svg:svg>").map(|at| at + "</svg:svg>".len());
        let block = tail.get(..end.unwrap_or(tail.len())).unwrap_or(tail);
        blocks.push(block);
        rest = tail.get(block.len()..).unwrap_or("");
    }
    blocks
}

/// The value of attribute `name` of the opening tag of `block`.
fn tag_attribute<'a>(block: &'a str, name: &str) -> Option<&'a str> {
    let open_end = block.find('>')?;
    let open = block.get(..open_end)?;
    let start = open.find(&format!("{name}=\""))? + name.len() + 2;
    let rest = open.get(start..)?;
    let end = rest.find('"')?;
    rest.get(..end)
}

/// The `width`/`height` of an `svg` element, in inches.
fn page_size(block: &str) -> (f64, f64) {
    let length = |name: &str| -> f64 {
        tag_attribute(block, name)
            .and_then(|value| value.strip_suffix("in"))
            .and_then(|value| value.trim().parse::<f64>().ok())
            .unwrap_or(0.0)
    };
    (length("width"), length("height"))
}

/// The viewBox scale of `block`: output units per inch.
fn units_per_inch(block: &str) -> f64 {
    let (width_in, _) = page_size(block);
    let view_box = tag_attribute(block, "viewBox").unwrap_or("");
    let width_units = view_box
        .split_whitespace()
        .nth(2)
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0);
    if width_in > 0.0 && width_units > 0.0 {
        width_units / width_in
    } else {
        72.0
    }
}

/// The oracle's reading of one `<svg:svg>` block.
fn parse_block(block: &str) -> OraclePage {
    let shapes = block.matches("<svg:g").count();
    let paths = block.matches("<svg:path").count();
    let scale = units_per_inch(block);
    let mut skip = None;
    let mut bounds = Some(Vec::new());
    for d in match_attributes(block, "d") {
        match path_points(&d, scale) {
            Some(mut points) => {
                if let Some(all) = bounds.as_mut() {
                    all.append(&mut points);
                }
            }
            None => {
                skip = Some(format!(
                    "the path data holds a command outside M/L/A/Z ({:.40}...)",
                    d.trim()
                ));
                bounds = None;
                break;
            }
        }
    }
    let mut merged: Option<Box> = None;
    if let Some(points) = bounds {
        for point in points {
            let box_of_point = Box::of_point(point);
            merged = Some(match merged {
                Some(existing) => existing.union(box_of_point),
                None => box_of_point,
            });
        }
    }
    OraclePage {
        shapes,
        paths,
        bounds: merged,
        size: page_size(block),
        skip,
    }
}

/// The values of every whitespace-delimited `name="..."` attribute in
/// `text` (so `d="` never matches the tail of `id="`).
fn match_attributes(text: &str, name: &str) -> Vec<String> {
    let needle = format!("{name}=\"");
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(&needle) {
        let boundary = start == 0
            || rest
                .as_bytes()
                .get(start.wrapping_sub(1))
                .is_some_and(|c| c.is_ascii_whitespace());
        let after = start + needle.len();
        let tail = rest.get(after..).unwrap_or("");
        let end = tail.find('"').unwrap_or(tail.len());
        if boundary && let Some(value) = tail.get(..end) {
            found.push(value.to_owned());
        }
        rest = rest.get(after + end + 1..).unwrap_or("");
    }
    found
}

/// The on-curve points of one SVG path's `d` data, in inches (y down).
///
/// The commands are the ones the oracle emits: `M`, `L`, `Z` and `A`
/// (numbers may be glued to the command letter). An elliptical arc is
/// sampled at 33 points of its true extent (the SVG
/// endpoint-to-centre parameterization). Any other command returns
/// `None`: the caller records the skip.
fn path_points(d: &str, scale: f64) -> Option<Vec<Point>> {
    let tokens = path_tokens(d)?;
    let mut points = Vec::new();
    let mut current = Point::new(0.0, 0.0);
    let mut start = current;
    let mut index = 0usize;
    while index < tokens.len() {
        let Some(Tok::Cmd(command)) = tokens.get(index) else {
            return None;
        };
        index += 1;
        let number = |at: usize| -> Option<f64> {
            match tokens.get(at) {
                Some(Tok::Num(value)) => Some(*value),
                _ => None,
            }
        };
        match command {
            'M' | 'L' => {
                let x = number(index)? / scale;
                let y = number(index + 1)? / scale;
                index += 2;
                current = Point::new(x, y);
                points.push(current);
                if *command == 'M' {
                    start = current;
                }
            }
            'A' => {
                let rx = number(index)? / scale;
                let ry = number(index + 1)? / scale;
                let rotation = number(index + 2)?;
                let large = number(index + 3)? != 0.0;
                let sweep = number(index + 4)? != 0.0;
                let to = Point::new(number(index + 5)? / scale, number(index + 6)? / scale);
                index += 7;
                points.extend(sample_arc(current, to, rx, ry, rotation, large, sweep));
                current = to;
            }
            'Z' => current = start,
            _ => return None,
        }
    }
    Some(points)
}

/// One token of a path's `d` data.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Tok {
    /// A command letter (`M`, `L`, `A`, `Z`).
    Cmd(char),
    /// A number.
    Num(f64),
}

/// The command letters and numbers of one path's `d` data. A letter may
/// be glued to the number that follows it (`M31.18`); anything else
/// returns `None`.
fn path_tokens(d: &str) -> Option<Vec<Tok>> {
    let mut tokens = Vec::new();
    let mut number = String::new();
    let flush = |number: &mut String, tokens: &mut Vec<Tok>| -> Option<()> {
        if number.is_empty() {
            return Some(());
        }
        let parsed = number.parse::<f64>().ok()?;
        tokens.push(Tok::Num(parsed));
        number.clear();
        Some(())
    };
    for c in d.chars() {
        if c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E' {
            number.push(c);
        } else if c.is_ascii_whitespace() || c == ',' {
            flush(&mut number, &mut tokens)?;
        } else if c.is_ascii_alphabetic() {
            flush(&mut number, &mut tokens)?;
            tokens.push(Tok::Cmd(c));
        } else {
            return None;
        }
    }
    flush(&mut number, &mut tokens)?;
    Some(tokens)
}

/// The sampled points of one SVG elliptical arc from `from` to `to`.
///
/// The parameterization is the SVG implementation note (endpoint to
/// centre); the sample count is fixed at 33, which keeps the box error
/// far below the comparison tolerance.
fn sample_arc(
    from: Point,
    to: Point,
    rx: f64,
    ry: f64,
    rotation_deg: f64,
    large: bool,
    sweep: bool,
) -> Vec<Point> {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if from == to || rx <= 0.0 || ry <= 0.0 {
        return vec![to];
    }
    let phi = rotation_deg.to_radians();
    let (sin_phi, cos_phi) = phi.sin_cos();
    let dx = (from.x - to.x) / 2.0;
    let dy = (from.y - to.y) / 2.0;
    let x1 = cos_phi * dx + sin_phi * dy;
    let y1 = -sin_phi * dx + cos_phi * dy;
    let lambda = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    if lambda > 1.0 {
        let scale = lambda.sqrt();
        rx *= scale;
        ry *= scale;
    }
    let numerator = (rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1).max(0.0);
    let denominator = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let sign = if large != sweep { 1.0 } else { -1.0 };
    let factor = if denominator > 0.0 {
        sign * (numerator / denominator).sqrt()
    } else {
        0.0
    };
    let cx1 = factor * rx * y1 / ry;
    let cy1 = -factor * ry * x1 / rx;
    let cx = cos_phi * cx1 - sin_phi * cy1 + (from.x + to.x) / 2.0;
    let cy = sin_phi * cx1 + cos_phi * cy1 + (from.y + to.y) / 2.0;
    let ux = (x1 - cx1) / rx;
    let uy = (y1 - cy1) / ry;
    let vx = (-x1 - cx1) / rx;
    let vy = (-y1 - cy1) / ry;
    let start_angle = uy.atan2(ux);
    let mut extent = (vx.atan2(vy) - uy.atan2(ux)).rem_euclid(std::f64::consts::TAU);
    if sweep && extent > 0.0 {
        extent -= std::f64::consts::TAU;
    }
    if !sweep && extent < 0.0 {
        extent += std::f64::consts::TAU;
    }
    let mut points = Vec::with_capacity(33);
    for step in 0..=32 {
        let angle = start_angle + extent * f64::from(step) / 32.0;
        let (sin_a, cos_a) = angle.sin_cos();
        points.push(Point::new(
            cos_phi * rx * cos_a - sin_phi * ry * sin_a + cx,
            sin_phi * rx * cos_a + cos_phi * ry * sin_a + cy,
        ));
    }
    points
}

/// The scene's counts and drawing box (y down, like the oracle's SVG).
fn scene_summary(scene: &Scene) -> SceneSummary {
    let mut summary = SceneSummary {
        shapes: 0,
        paths: 0,
        bounds: None,
        text: false,
        groups: false,
        divergent: false,
    };
    walk_nodes(
        &scene.nodes,
        Transform::identity(),
        scene.height,
        &mut summary,
    );
    summary
}

/// What one scene draws, in the oracle's terms.
#[derive(Debug, Clone)]
struct SceneSummary {
    /// The `Node::Shape` count.
    shapes: usize,
    /// The filled and stroked shape paths.
    paths: usize,
    /// The drawing box in inches (y down), when geometry exists.
    bounds: Option<Box>,
    /// Whether any shape carries text (the oracle draws text as extra
    /// paths and groups of its own).
    text: bool,
    /// Whether the scene has group nodes (group shapes flatten
    /// differently in the two renderers).
    groups: bool,
    /// Whether any shape does not draw exactly one filled and stroked
    /// path (a themed paint that fell back, or an empty shape): on such
    /// a page the two renderers draw different object sets and the
    /// counts are not comparable.
    divergent: bool,
}

/// Recurses the node tree, composing transforms.
fn walk_nodes(nodes: &[Node], parent: Transform, height: f64, summary: &mut SceneSummary) {
    for node in nodes {
        match node {
            Node::Group(Group {
                transform,
                children,
                ..
            }) => {
                summary.groups = true;
                walk_nodes(children, parent.then(*transform), height, summary);
            }
            Node::Shape(shape) => {
                summary.shapes += 1;
                let fill = shape.fill.is_some();
                let stroke = shape.stroke.is_some();
                if fill && stroke {
                    summary.paths += 2;
                } else {
                    summary.paths += usize::from(fill) + usize::from(stroke);
                    summary.divergent = true;
                }
                if shape.path.is_empty() && shape.text.is_none() {
                    summary.divergent = true;
                }
                if shape.text.is_some() {
                    summary.text = true;
                }
                add_shape_bounds(
                    shape,
                    parent.then(shape.transform),
                    height,
                    &mut summary.bounds,
                );
            }
            _ => {}
        }
    }
}

/// Grows the drawing box by one shape's path (sampled, y down).
fn add_shape_bounds(shape: &Shape, transform: Transform, height: f64, bounds: &mut Option<Box>) {
    let mut current = Point::new(0.0, 0.0);
    for segment in shape.path.segments() {
        for point in sample_segment(current, segment) {
            current = point;
            let page = transform.apply(point);
            let down = Point::new(page.x, height - page.y);
            let mut grown = bounds.unwrap_or_else(|| Box::of_point(down));
            grown.add(down);
            *bounds = Some(grown);
        }
    }
}

/// The sampled points of one path segment (17 samples of any curve),
/// given the segment's start point.
fn sample_segment(current: Point, segment: &Segment) -> Vec<Point> {
    match segment {
        Segment::MoveTo(p) | Segment::LineTo(p) => vec![*p],
        Segment::QuadTo { ctrl, to } => (1..=16)
            .map(|step| {
                let t = f64::from(step) / 16.0;
                let u = 1.0 - t;
                Point::new(
                    u * u * current.x + 2.0 * u * t * ctrl.x + t * t * to.x,
                    u * u * current.y + 2.0 * u * t * ctrl.y + t * t * to.y,
                )
            })
            .collect(),
        Segment::CubicTo { ctrl1, ctrl2, to } => (1..=16)
            .map(|step| {
                let t = f64::from(step) / 16.0;
                let u = 1.0 - t;
                Point::new(
                    u * u * u * current.x
                        + 3.0 * u * u * t * ctrl1.x
                        + 3.0 * u * t * t * ctrl2.x
                        + t * t * t * to.x,
                    u * u * u * current.y
                        + 3.0 * u * u * t * ctrl1.y
                        + 3.0 * u * t * t * ctrl2.y
                        + t * t * t * to.y,
                )
            })
            .collect(),
        Segment::Close => vec![],
        _ => vec![],
    }
}

/// The corpus files that are malformed by design (`corpus/PROVENANCE.md`):
/// the oracle emits an empty or duplicate page for them, so only "no
/// panic, no drawing" is comparable there.
const KNOWN_MALFORMED: &[&str] = &["recursion-cycle.vsdx", "tab-short-prefix.vsdx"];

/// The structural comparison: page counts for every file, and per-page
/// shape counts, path counts and the drawing box within
/// [`BBOX_TOLERANCE`] where the two renderers draw the same objects.
///
/// A page skips the per-page comparison (recording the reason) when our
/// scene carries text (the oracle draws text as extra paths and groups),
/// when it has group nodes (group shapes flatten differently), or when a
/// shape resolves only one of fill and stroke (a themed paint that fell
/// back on our side). The corpus render test covers the skipped files.
#[test]
fn our_rendering_agrees_with_the_oracle_on_structure() {
    let Some(oracle) = oracle_dir() else {
        assert!(
            !require_oracle(),
            "LARRYGRAM_REQUIRE_ORACLE=1 is set but LARRYGRAM_LIBVISIO_ORACLE is missing"
        );
        eprintln!(
            "skipping: LARRYGRAM_LIBVISIO_ORACLE is not set; see `cargo xtask oracle build` (ADR 0010)"
        );
        return;
    };
    let files = common::corpus_files();
    if common::require_corpus_or(&files) {
        return;
    }
    let mut compared_pages = 0usize;
    let mut parsed_files = 0usize;
    for file in &files {
        let name = file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if KNOWN_MALFORMED.contains(&name.as_str()) {
            eprintln!("skipped {name}: known-malformed corpus file");
            continue;
        }
        let Some(xhtml) = run_vsd2xhtml(&oracle, file) else {
            continue; // the skip reason was recorded by the runner
        };
        parsed_files += 1;
        let bytes = std::fs::read(file).unwrap_or_else(|e| panic!("{name}: {e}"));
        let doc = read_document(&bytes, &Limits::DEFAULT)
            .unwrap_or_else(|e| panic!("{name}: does not read: {e}"));
        let oracle_pages: Vec<OraclePage> =
            svg_blocks(&xhtml).into_iter().map(parse_block).collect();
        let scenes = common::page_scenes(&doc, true);
        assert_eq!(
            oracle_pages.len(),
            scenes.len(),
            "{name}: page counts differ (oracle {} vs ours {})",
            oracle_pages.len(),
            scenes.len()
        );
        for (index, (opage, scene)) in oracle_pages.iter().zip(scenes.iter()).enumerate() {
            let summary = scene_summary(scene);
            if let Some(reason) = skip_reason(opage, &summary) {
                eprintln!("skipped {name} page {index}: {reason}");
                continue;
            }
            assert_eq!(
                opage.shapes, summary.shapes,
                "{name} page {index}: shape counts differ (oracle {} vs ours {})",
                opage.shapes, summary.shapes
            );
            assert_eq!(
                opage.paths, summary.paths,
                "{name} page {index}: path counts differ (oracle {} vs ours {})",
                opage.paths, summary.paths
            );
            let (oracle_box, our_box) = (
                opage.bounds.expect("bounds (skip rule)"),
                summary.bounds.expect("bounds (skip rule)"),
            );
            assert!(
                oracle_box.near(our_box, BBOX_TOLERANCE),
                "{name} page {index}: drawing boxes differ by more than {BBOX_TOLERANCE} in \
                 (oracle {oracle_box:?} vs ours {our_box:?})"
            );
            compared_pages += 1;
        }
    }
    assert!(parsed_files > 0, "the oracle parsed no corpus file");
    assert!(
        compared_pages >= 3,
        "only {compared_pages} pages took the full comparison; the oracle setup looks wrong"
    );
}

/// Why one page skips the per-page comparison, when it does.
fn skip_reason(opage: &OraclePage, summary: &SceneSummary) -> Option<String> {
    if let Some(reason) = &opage.skip {
        return Some(reason.clone());
    }
    if opage.size == (0.0, 0.0) {
        return Some("the oracle reports an empty page".to_owned());
    }
    if summary.text {
        return Some(
            "our scene has text; the oracle draws text as extra paths and groups".to_owned(),
        );
    }
    if summary.groups {
        return Some("our scene has group nodes; group shapes flatten differently".to_owned());
    }
    if summary.divergent {
        return Some(
            "a shape does not draw exactly one filled and stroked path on our side (a fell-back paint or an empty shape)"
                .to_owned(),
        );
    }
    if opage.bounds.is_none() || summary.bounds.is_none() {
        return Some("one side has no drawing box".to_owned());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oracle_dir_finds_only_an_existing_directory() {
        assert_eq!(oracle_dir_from(None), None);
        assert_eq!(oracle_dir_from(Some("no/such/dir".into())), None);
        assert!(oracle_dir_from(Some(".".into())).is_some());
    }

    #[test]
    fn require_oracle_reads_only_the_literal_one() {
        assert!(!require_from(None));
        assert!(!require_from(Some("0")));
        assert!(require_from(Some("1")));
    }

    #[test]
    fn svg_blocks_split_on_the_svg_elements() {
        let text =
            "<body><svg:svg width=\"1in\">a</svg:svg><svg:svg width=\"2in\">b</svg:svg></body>";
        let blocks = svg_blocks(text);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].contains("1in"));
        assert!(blocks[1].contains("2in"));
    }

    #[test]
    fn page_size_and_scale_come_from_the_svg_element() {
        let block =
            "<svg:svg width=\"11.6929in\" height=\"8.2677in\" viewBox=\"0 0 841.8898 595.2756\">";
        assert_eq!(page_size(block), (11.6929, 8.2677));
        assert!((units_per_inch(block) - 72.0).abs() < 1e-3);
    }

    #[test]
    fn path_points_read_moves_lines_and_closes() {
        let d = "M 72.0,144.0\nL 216.0,144.0\nZ";
        let points = path_points(d, 72.0).expect("parses");
        assert_eq!(points, vec![Point::new(1.0, 2.0), Point::new(3.0, 2.0)]);
    }

    #[test]
    fn path_points_refuse_an_unknown_command() {
        assert_eq!(path_points("M 0 0 C 1 1 2 2 3 3", 1.0), None);
        assert_eq!(path_points("M 0 0 Q 1 1 2 2", 1.0), None);
    }

    #[test]
    fn an_oracle_page_counts_its_elements() {
        let block = "<svg:svg width=\"8.5in\" height=\"11in\" viewBox=\"0 0 612 792\">\
                     <svg:g id=\"Layer1000\"><svg:path d=\"M 0,0 L 72,0 Z\"/>\
                     <svg:path d=\"M 0,0 L 0,72 Z\"/></svg:g></svg:svg>";
        let page = parse_block(block);
        assert_eq!(page.shapes, 1);
        assert_eq!(page.paths, 2);
        assert_eq!(page.size, (8.5, 11.0));
        let bounds = page.bounds.expect("bounds");
        assert!((bounds.min_x - 0.0).abs() < 1e-9);
        assert!((bounds.max_y - 1.0).abs() < 1e-9);
    }

    #[test]
    fn an_unparsable_path_records_the_skip() {
        let block = "<svg:svg width=\"8.5in\" height=\"11in\" viewBox=\"0 0 612 792\">\
                     <svg:path d=\"M 0,0 C 1,1 2,2 3,3\"/></svg:svg>";
        let page = parse_block(block);
        assert!(page.bounds.is_none());
        assert!(page.skip.is_some());
    }

    #[test]
    fn boxes_union_and_compare_within_tolerance() {
        let a = Box::of_point(Point::new(0.0, 0.0));
        let mut b = a;
        b.add(Point::new(2.0, 3.0));
        let union = a.union(b);
        assert_eq!(union.size(), (2.0, 3.0));
        assert!(union.near(union, 0.0));
        let mut shifted = union;
        shifted.min_x += 0.04;
        shifted.max_x += 0.04;
        assert!(union.near(shifted, BBOX_TOLERANCE));
        shifted.min_x += 0.04;
        assert!(!union.near(shifted, BBOX_TOLERANCE));
    }

    #[test]
    fn sampled_segments_cover_their_ends() {
        let origin = Point::new(0.0, 0.0);
        let line = sample_segment(origin, &Segment::LineTo(Point::new(1.0, 1.0)));
        assert_eq!(line, vec![Point::new(1.0, 1.0)]);
        let cubic = sample_segment(
            origin,
            &Segment::CubicTo {
                ctrl1: Point::new(0.0, 1.0),
                ctrl2: Point::new(1.0, 1.0),
                to: Point::new(1.0, 0.0),
            },
        );
        assert_eq!(cubic.len(), 16);
        assert_eq!(cubic[15], Point::new(1.0, 0.0));
        // The first sample sits on the curve near the origin.
        assert!(cubic[0].x >= 0.0 && cubic[0].x < 0.1 && cubic[0].y > 0.1);
    }

    #[test]
    fn an_arc_samples_its_whole_extent() {
        // A half circle from (1, 0) to (-1, 0): the sample must reach
        // the top or bottom of the circle, not just its endpoints.
        let points = sample_arc(
            Point::new(1.0, 0.0),
            Point::new(-1.0, 0.0),
            1.0,
            1.0,
            0.0,
            false,
            true,
        );
        assert_eq!(points.len(), 33);
        let extreme = points.iter().map(|p| p.y.abs()).fold(0.0f64, f64::max);
        assert!(extreme > 0.99, "arc never reaches the apex: {extreme}");
    }

    #[test]
    fn tokens_split_glued_commands() {
        assert_eq!(
            path_tokens("M31.18,56.69 L1,2"),
            Some(vec![
                Tok::Cmd('M'),
                Tok::Num(31.18),
                Tok::Num(56.69),
                Tok::Cmd('L'),
                Tok::Num(1.0),
                Tok::Num(2.0)
            ])
        );
        // Unknown letters tokenize but do not parse as path commands.
        assert!(path_tokens("M 0 0 L x 1").is_some());
        assert_eq!(path_points("M 0 0 L x 1", 1.0), None);
    }

    #[test]
    fn match_attributes_never_matches_inside_another_name() {
        let text = "<svg:g id=\"Layer1000\"><svg:path d=\"M 0 0\"/></svg:g>";
        assert_eq!(match_attributes(text, "d"), vec!["M 0 0".to_owned()]);
    }
}
