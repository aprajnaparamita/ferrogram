//! Selection handles: move, resize and rotate (ADR 0042).
//!
//! Handles live at screen-size anchors around the selection bounds; the
//! caller converts the tolerance from screen pixels with the zoom. The
//! pointer tool ([`crate::tools::pointer`], P5-05) owns the drag that
//! follows a hit.
//!
//! The scene helpers here are shared by the pointer tool and the snapper:
//! one page scene build, one shape-to-node mapping and one selection
//! frame (the `render::build_selection_overlay` rules, ADR 0041/0042).

use std::collections::BTreeSet;

use ferrogram_commands::AppCtx;
use ferrogram_geom::{Angle, Point, Rect, Transform};
use ferrogram_model::{Document, ShapeId};
use ferrogram_render::{
    BuildOptions, NodePath, OverlayMark, Scene, build_scene, build_selection_overlay,
};

use crate::UiError;

/// Which edge or corner a resize handle stretches (ADR 0042).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ResizeEdge {
    /// The top edge.
    North,
    /// The bottom edge.
    South,
    /// The right edge.
    East,
    /// The left edge.
    West,
    /// The top-right corner.
    NorthEast,
    /// The top-left corner.
    NorthWest,
    /// The bottom-right corner.
    SouthEast,
    /// The bottom-left corner.
    SouthWest,
}

/// What a handle does (ADR 0042).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HandleKind {
    /// Move the selection (the interior of the bounds).
    Move,
    /// Stretch one edge or corner.
    Resize(ResizeEdge),
    /// Rotate around the selection's pin (the handle above the bounds).
    Rotate,
}

/// The screen-pixel hit tolerance of a handle (ADR 0042).
pub const TOLERANCE_PX: f64 = 6.0;

/// The rotate handle's offset above the bounds' top edge, in inches.
///
/// A fixed page offset keeps [`handles_for`] and [`handle_at`] agreeing
/// without a zoom parameter; at the default zoom it is about 24 screen
/// pixels. A zoom-dependent screen offset would need a new signature
/// (noted in the task Result).
pub const ROTATE_OFFSET_IN: f64 = 0.25;

/// The handle at `point` for one selected shape (page inches, `zoom` for
/// the screen-pixel tolerance of [`TOLERANCE_PX`]).
///
/// Handles win over the move interior. `None` when the point is on no
/// handle and outside the rotated bounds.
pub fn handle_at(
    point: Point,
    bounds: Rect,
    angle: Angle,
    zoom: f64,
) -> Result<Option<HandleKind>, UiError> {
    if !finite(point) || !finite_rect(bounds) || !angle.radians().is_finite() {
        return Ok(None);
    }
    let zoom = if zoom.is_finite() && zoom > 0.0 {
        zoom
    } else {
        1.0
    };
    let tolerance = TOLERANCE_PX / (zoom * crate::canvas::POINTS_PER_INCH);
    for (kind, anchor) in handles_for(bounds, angle)? {
        let delta = point.sub(anchor);
        if delta.x.hypot(delta.y) <= tolerance {
            return Ok(Some(kind));
        }
    }
    if rotated_bounds(bounds, angle).contains(point) {
        Ok(Some(HandleKind::Move))
    } else {
        Ok(None)
    }
}

/// The handle anchor points of one selection: `(kind, point)` pairs in
/// page inches, rotated with `angle` (ADR 0042).
pub fn handles_for(bounds: Rect, angle: Angle) -> Result<Vec<(HandleKind, Point)>, UiError> {
    if !finite_rect(bounds) {
        return Ok(Vec::new());
    }
    let center = rect_center(bounds);
    let (min, max) = (bounds.min, bounds.max);
    let mid_x = (min.x + max.x) / 2.0;
    let mid_y = (min.y + max.y) / 2.0;
    let rotate = |point: Point| rotate_about(center, angle, point);
    let mut handles = Vec::with_capacity(9);
    for (kind, point) in [
        (
            HandleKind::Resize(ResizeEdge::NorthWest),
            Point::new(min.x, max.y),
        ),
        (
            HandleKind::Resize(ResizeEdge::North),
            Point::new(mid_x, max.y),
        ),
        (
            HandleKind::Resize(ResizeEdge::NorthEast),
            Point::new(max.x, max.y),
        ),
        (
            HandleKind::Resize(ResizeEdge::East),
            Point::new(max.x, mid_y),
        ),
        (
            HandleKind::Resize(ResizeEdge::SouthEast),
            Point::new(max.x, min.y),
        ),
        (
            HandleKind::Resize(ResizeEdge::South),
            Point::new(mid_x, min.y),
        ),
        (
            HandleKind::Resize(ResizeEdge::SouthWest),
            Point::new(min.x, min.y),
        ),
        (
            HandleKind::Resize(ResizeEdge::West),
            Point::new(min.x, mid_y),
        ),
        (
            HandleKind::Rotate,
            Point::new(mid_x, max.y + ROTATE_OFFSET_IN),
        ),
    ] {
        handles.push((kind, rotate(point)));
    }
    Ok(handles)
}

/// The union bounds of the selection in page inches (one shape: its
/// rotated bounds; several: the axis-aligned union).
///
/// An empty selection or one with no drawable scene node is an
/// [`UiError::Io`] naming the state; callers that accept an empty
/// selection use the internal `selection_frame` helper.
pub fn selection_bounds(ctx: &AppCtx) -> Result<Rect, UiError> {
    match selection_frame(ctx, None)? {
        Some((bounds, _angle)) => Ok(bounds),
        None => Err(UiError::Io {
            message: "the selection has no bounds".to_owned(),
        }),
    }
}

/// The selection's handle frame: bounds and rotation.
///
/// One selected shape gives its own rotated bounds; several give the
/// axis-aligned union with a zero angle (the `build_selection_overlay`
/// rule). `None` when the selection is empty or nothing is drawable.
pub(crate) fn selection_frame(
    ctx: &AppCtx,
    scene: Option<&Scene>,
) -> Result<Option<(Rect, Angle)>, UiError> {
    // The caller's page scene when it has one (task P5-11: rebuilding
    // the scene per drag frame blew the ADR 0042 budget).
    let built;
    let scene: &Scene = match scene {
        Some(scene) => scene,
        None => {
            built = scene_for(ctx)?;
            match &built {
                Some(scene) => scene,
                None => return Ok(None),
            }
        }
    };
    let paths = paths_for_shapes(&scene, ctx, &ctx.selection.shapes)?;
    if paths.is_empty() {
        return Ok(None);
    }
    let overlay = build_selection_overlay(&scene, &paths).map_err(render_error)?;
    for mark in overlay.marks {
        if let OverlayMark::Handles { bounds, angle } = mark {
            return Ok(Some((bounds, angle)));
        }
    }
    Ok(None)
}

/// The active page's scene, or `None` with no open document or page.
pub(crate) fn scene_for(ctx: &AppCtx) -> Result<Option<Scene>, UiError> {
    let Some(session) = ctx.active_document() else {
        return Ok(None);
    };
    let Some(page) = session.active_page else {
        return Ok(None);
    };
    build_scene(&session.document, page, &BuildOptions::new())
        .map(Some)
        .map_err(render_error)
}

/// One drawable scene node: the model shape, its page transform and its
/// scene path.
#[derive(Debug, Clone)]
pub(crate) struct NodeEntry {
    /// The model shape behind the node.
    pub shape: ShapeId,
    /// The scene path.
    pub path: NodePath,
    /// The shape-local to page transform.
    pub transform: Transform,
}

/// Every drawable shape node of the scene, in paint order, with its page
/// transform.
pub(crate) fn node_entries(scene: &Scene) -> Vec<NodeEntry> {
    let mut entries = Vec::new();
    collect_entries(
        &scene.nodes,
        &NodePath::new(),
        Transform::identity(),
        0,
        &mut entries,
    );
    entries
}

/// The recursion of [`node_entries`] (depth-capped like the renderer).
fn collect_entries(
    nodes: &[ferrogram_render::scene::Node],
    parent: &NodePath,
    transform: Transform,
    depth: usize,
    entries: &mut Vec<NodeEntry>,
) {
    const MAX_DEPTH: usize = 128;
    if depth >= MAX_DEPTH {
        return;
    }
    for (index, node) in nodes.iter().enumerate() {
        let path = parent.child(index);
        match node {
            ferrogram_render::scene::Node::Group(group) => collect_entries(
                &group.children,
                &path,
                group.transform.then(transform),
                depth + 1,
                entries,
            ),
            ferrogram_render::scene::Node::Shape(shape) => {
                if let Some(id) = shape.shape_id {
                    entries.push(NodeEntry {
                        shape: id,
                        path,
                        transform: shape.transform.then(transform),
                    });
                }
            }
            _ => {}
        }
    }
}

/// The scene paths of the selected shapes and their descendants.
pub(crate) fn paths_for_shapes(
    scene: &Scene,
    ctx: &AppCtx,
    shapes: &[ShapeId],
) -> Result<Vec<NodePath>, UiError> {
    let Some(session) = ctx.active_document() else {
        return Ok(Vec::new());
    };
    let wanted = subtree_ids(&session.document, shapes);
    let mut paths = Vec::new();
    for entry in node_entries(scene) {
        if wanted.contains(&entry.shape) {
            paths.push(entry.path);
        }
    }
    Ok(paths)
}

/// The selected shapes plus every shape below them (group members).
pub(crate) fn subtree_ids(doc: &Document, shapes: &[ShapeId]) -> BTreeSet<ShapeId> {
    let mut wanted = BTreeSet::new();
    let mut stack: Vec<ShapeId> = shapes.to_vec();
    let mut steps = 0usize;
    while let Some(id) = stack.pop() {
        steps += 1;
        if steps > doc.shapes.len().saturating_mul(2).saturating_add(16) {
            break;
        }
        if !wanted.insert(id) {
            continue;
        }
        if let Some(shape) = doc.shape(id) {
            stack.extend(shape.children.iter().copied());
        }
    }
    wanted
}

/// Whether `shape` is one of `roots` or below one (the moving set).
pub(crate) fn is_within(doc: &Document, shape: ShapeId, roots: &[ShapeId]) -> bool {
    let roots = subtree_ids(doc, roots);
    roots.contains(&shape)
}

/// The centre of a rectangle.
pub(crate) fn rect_center(bounds: Rect) -> Point {
    Point::new(
        (bounds.min.x + bounds.max.x) / 2.0,
        (bounds.min.y + bounds.max.y) / 2.0,
    )
}

/// The four corners of `bounds`, rotated about its centre.
pub(crate) fn rotated_corners(bounds: Rect, angle: Angle) -> [Point; 4] {
    let center = rect_center(bounds);
    [
        rotate_about(center, angle, bounds.min),
        rotate_about(center, angle, Point::new(bounds.max.x, bounds.min.y)),
        rotate_about(center, angle, bounds.max),
        rotate_about(center, angle, Point::new(bounds.min.x, bounds.max.y)),
    ]
}

/// The axis-aligned bounds of the rotated rectangle.
pub(crate) fn rotated_bounds(bounds: Rect, angle: Angle) -> Rect {
    let corners = rotated_corners(bounds, angle);
    let mut min = corners[0];
    let mut max = corners[0];
    for point in corners.iter().skip(1) {
        min = Point::new(min.x.min(point.x), min.y.min(point.y));
        max = Point::new(max.x.max(point.x), max.y.max(point.y));
    }
    Rect::new(min, max)
}

/// Rotates `point` about `center` by `angle`.
pub(crate) fn rotate_about(center: Point, angle: Angle, point: Point) -> Point {
    let (sin, cos) = angle.radians().sin_cos();
    let dx = point.x - center.x;
    let dy = point.y - center.y;
    Point::new(
        center.x + dx * cos - dy * sin,
        center.y + dx * sin + dy * cos,
    )
}

/// A render failure as a [`UiError`].
pub(crate) fn render_error(error: ferrogram_render::Error) -> UiError {
    UiError::Render {
        message: format!("the scene build failed: {error}"),
    }
}

/// True when both coordinates are finite.
fn finite(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

/// True when all four rectangle coordinates are finite.
fn finite_rect(bounds: Rect) -> bool {
    finite(bounds.min) && finite(bounds.max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferrogram_commands::{DocumentSession, Settings};
    use ferrogram_model::{
        Cell, CellName, CellValue, Document, Page, PageId, Row, Section, SectionName, Shape,
    };

    fn cell(name: &str, value: f64) -> Cell {
        let mut cell = Cell::new(CellName::new(name));
        cell.value = Some(CellValue::Number(value));
        cell
    }

    fn shape_document(angle: f64) -> (AppCtx, ShapeId) {
        let mut doc = Document::new();
        let mut shape = Shape::new(ShapeId { index: 0 }, 1);
        shape.sheet.cells = vec![
            cell("Width", 2.0),
            cell("Height", 1.0),
            cell("PinX", 2.0),
            cell("PinY", 3.0),
            cell("LocPinX", 1.0),
            cell("LocPinY", 0.5),
            cell("Angle", angle),
        ];
        let mut geometry = Section::new(SectionName::from_static("Geometry"));
        for (ix, (kind, x, y)) in [
            ("MoveTo", 0.0, 0.0),
            ("LineTo", 2.0, 0.0),
            ("LineTo", 2.0, 1.0),
            ("LineTo", 0.0, 1.0),
            ("LineTo", 0.0, 0.0),
        ]
        .into_iter()
        .enumerate()
        {
            let mut row = Row::new();
            row.row_type = Some(kind.to_owned());
            row.ix = Some(i32::try_from(ix).unwrap_or(0));
            row.cells = vec![cell("X", x), cell("Y", y)];
            geometry.rows.push(row);
        }
        shape.sheet.sections.push(geometry);
        let id = doc.add_shape(shape);
        let mut page = Page::new(PageId { index: 0 }, 1);
        page.shapes.push(id);
        doc.pages.push(page);
        let mut app = AppCtx::new(Settings::default());
        app.open_document(DocumentSession::new(doc, None));
        app.selection.shapes = vec![id];
        app.selection.anchor = Some(id);
        app.selection.page = Some(PageId { index: 0 });
        (app, id)
    }

    #[test]
    fn handles_sit_on_the_edges_corners_and_above() {
        let bounds = Rect::new(Point::new(0.0, 0.0), Point::new(2.0, 1.0));
        let handles = handles_for(bounds, Angle::from_radians(0.0)).expect("handles");
        assert_eq!(handles.len(), 9);
        assert_eq!(handles[0].0, HandleKind::Resize(ResizeEdge::NorthWest));
        assert_eq!(handles[0].1, Point::new(0.0, 1.0));
        assert_eq!(handles[8].0, HandleKind::Rotate);
        assert_eq!(handles[8].1, Point::new(1.0, 1.0 + ROTATE_OFFSET_IN));
    }

    #[test]
    fn a_rotated_handle_set_turns_about_the_centre() {
        let bounds = Rect::new(Point::new(0.0, 0.0), Point::new(2.0, 2.0));
        let handles = handles_for(bounds, Angle::from_radians(core::f64::consts::FRAC_PI_2))
            .expect("handles");
        let (_, north) = handles[1];
        // A positive angle turns counterclockwise in page coordinates
        // (y up), so the north handle moves to the west side.
        assert!((north.x - 0.0).abs() < 1.0e-9, "{north:?}");
        assert!((north.y - 1.0).abs() < 1.0e-9, "{north:?}");
    }

    #[test]
    fn handle_hits_prefer_the_handle_and_cover_the_interior() {
        let bounds = Rect::new(Point::new(0.0, 0.0), Point::new(2.0, 1.0));
        let angle = Angle::from_radians(0.0);
        assert_eq!(
            handle_at(Point::new(2.0, 0.5), bounds, angle, 1.0).expect("hit"),
            Some(HandleKind::Resize(ResizeEdge::East))
        );
        assert_eq!(
            handle_at(Point::new(1.0, 0.5), bounds, angle, 1.0).expect("hit"),
            Some(HandleKind::Move)
        );
        assert_eq!(
            handle_at(Point::new(4.0, 4.0), bounds, angle, 1.0).expect("miss"),
            None
        );
    }

    #[test]
    fn the_handle_tolerance_follows_the_zoom() {
        let bounds = Rect::new(Point::new(0.0, 0.0), Point::new(2.0, 1.0));
        let angle = Angle::from_radians(0.0);
        let near = Point::new(2.0 + 0.04, 0.5);
        assert_eq!(
            handle_at(near, bounds, angle, 1.0).expect("hit"),
            Some(HandleKind::Resize(ResizeEdge::East))
        );
        assert_eq!(
            handle_at(near, bounds, angle, 4.0).expect("miss"),
            None,
            "6 px is 1/16 in at zoom 1 but 1/64 in at zoom 4"
        );
    }

    #[test]
    fn a_non_finite_query_never_hits() {
        let bounds = Rect::new(Point::new(0.0, 0.0), Point::new(2.0, 1.0));
        assert_eq!(
            handle_at(Point::new(f64::NAN, 0.0), bounds, Angle::default(), 1.0).expect("no panic"),
            None
        );
        assert!(
            handles_for(
                Rect::new(Point::new(f64::NAN, 0.0), Point::new(1.0, 1.0)),
                Angle::default()
            )
            .expect("no panic")
            .is_empty()
        );
    }

    #[test]
    fn the_selection_frame_comes_from_the_scene() {
        let (app, _id) = shape_document(0.0);
        let bounds = selection_bounds(&app).expect("bounds");
        assert!((bounds.width() - 2.0).abs() < 1.0e-9);
        assert!((bounds.height() - 1.0).abs() < 1.0e-9);
        let (frame_bounds, angle) = selection_frame(&app, None).expect("frame").expect("some");
        assert_eq!(frame_bounds, bounds);
        assert_eq!(angle.radians(), 0.0);
    }

    #[test]
    fn an_empty_selection_has_no_bounds() {
        let (mut app, _id) = shape_document(0.0);
        app.selection.clear();
        assert!(selection_bounds(&app).is_err());
        assert_eq!(selection_frame(&app, None).expect("no error"), None);
        assert!(
            paths_for_shapes(&Scene::new(1.0, 1.0), &app, &[])
                .expect("paths")
                .is_empty()
        );
    }

    #[test]
    fn an_invalid_zoom_still_hits_with_the_default_scale() {
        let bounds = Rect::new(Point::new(0.0, 0.0), Point::new(2.0, 1.0));
        assert_eq!(
            handle_at(Point::new(2.0, 0.5), bounds, Angle::default(), f64::NAN).expect("no error"),
            Some(HandleKind::Resize(ResizeEdge::East))
        );
        assert_eq!(
            handle_at(
                Point::new(2.0, 0.5),
                bounds,
                Angle::from_radians(f64::NAN),
                1.0
            )
            .expect("no error"),
            None
        );
    }

    #[test]
    fn an_empty_app_has_no_scene_or_frame() {
        let app = AppCtx::new(Settings::default());
        assert_eq!(scene_for(&app).expect("no error"), None);
        assert_eq!(selection_frame(&app, None).expect("no error"), None);
        assert!(selection_bounds(&app).is_err());
        assert!(
            paths_for_shapes(&Scene::new(1.0, 1.0), &app, &[])
                .expect("no error")
                .is_empty()
        );
        let mut app = app;
        let mut doc = Document::new();
        doc.pages.push(Page::new(PageId { index: 0 }, 1));
        app.open_document(DocumentSession::new(doc, None));
        app.active_document_mut().expect("session").active_page = None;
        assert_eq!(scene_for(&app).expect("no error"), None);
    }

    #[test]
    fn a_cyclic_group_chain_is_walked_once() {
        let (mut app, group) = shape_document(0.0);
        {
            let session = app.active_document_mut().expect("session");
            if let Some(entry) = session.document.shape_mut(group) {
                entry.children.push(group);
            }
        }
        let doc = &app.active_document().expect("session").document;
        let wanted = subtree_ids(doc, &[group]);
        assert_eq!(wanted.len(), 1);
        assert_eq!(subtree_ids(doc, &[ShapeId { index: 99 }]).len(), 1);
    }

    #[test]
    fn render_errors_map_into_ui_errors() {
        let error = render_error(ferrogram_render::Error::TooLarge);
        assert!(matches!(error, UiError::Render { .. }), "{error:?}");
    }

    #[test]
    fn a_selected_group_selects_its_members() {
        let (mut app, group) = shape_document(0.0);
        let session = app.active_document_mut().expect("session");
        let child = Shape::new(ShapeId { index: 0 }, 2);
        let child = session.document.add_shape(child);
        if let Some(entry) = session.document.shape_mut(group) {
            entry.children.push(child);
        }
        if let Some(entry) = session.document.shape_mut(child) {
            entry.parent = Some(group);
        }
        app.selection.shapes = vec![group];
        let wanted = subtree_ids(&app.active_document().expect("session").document, &[group]);
        assert!(wanted.contains(&group));
        assert!(wanted.contains(&child));
        assert!(is_within(
            &app.active_document().expect("session").document,
            child,
            &[group]
        ));
    }
}
