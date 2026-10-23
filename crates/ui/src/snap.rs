//! Snapping policy (ADR 0043): which candidates exist, in what priority,
//! and the snap distance in screen pixels.
//!
//! The math lives in `larrygram_geom::snap` (L0); this module gathers the
//! candidates from the document (grid lines, shape edges and centres,
//! connection points) and applies the priority of ADR 0043:
//! connection point, then alignment, then grid. Alt disables snapping.
//!
//! Both axes snap independently: a move snaps the drag **delta** (the
//! selection's relative offsets stay), and a point snap snaps its x and y
//! with the same priority rule.

use larrygram_commands::AppCtx;
use larrygram_geom::{Point, SnapKind, SnapResult, snap_axis, snap_grid};
use larrygram_model::ShapeId;
use larrygram_render::node_bounds;
use larrygram_render::scene::Scene;

use crate::UiError;
use crate::handles::{is_within, node_entries, rect_center, scene_for, selection_frame};

/// The snapping settings (ADR 0043).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapSettings {
    /// The grid spacing, in inches (the canvas default: 1/8 in).
    pub grid: f64,
    /// The snap distance, in screen pixels (ADR 0043: 8 px).
    pub distance_px: f64,
    /// Snap to the grid.
    pub to_grid: bool,
    /// Snap to shape edges and centres (alignment).
    pub to_shapes: bool,
    /// Snap to connection points (glue candidates).
    pub to_connection_points: bool,
}

impl Default for SnapSettings {
    fn default() -> Self {
        Self {
            grid: 0.125,
            distance_px: 8.0,
            to_grid: true,
            to_shapes: true,
            to_connection_points: true,
        }
    }
}

/// The snapper used by the drag tools (ADR 0043).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Snapper {
    /// The snap settings.
    pub settings: SnapSettings,
}

impl Snapper {
    /// A snapper with the given settings.
    pub const fn new(settings: SnapSettings) -> Self {
        Self { settings }
    }

    /// Snaps a dragged selection's delta (relative offsets of the shapes
    /// are kept, ADR 0043).
    ///
    /// `zoom` converts the pixel snap distance to inches. `alt_held`
    /// disables snapping for this call (ADR 0043). `scene` is the
    /// caller's page scene (the pointer tool caches one per change
    /// counter); `None` builds one (task P5-11: building it per drag
    /// frame blew the ADR 0042 budget on a 1000-shape page).
    pub fn snap_move(
        &self,
        ctx: &AppCtx,
        scene: Option<&Scene>,
        moving: &[ShapeId],
        delta: Point,
        zoom: f64,
        alt_held: bool,
    ) -> Result<SnapResult, UiError> {
        if alt_held || !delta.x.is_finite() || !delta.y.is_finite() {
            return Ok(SnapResult::none(delta));
        }
        let Some(max_distance) = self.max_distance(zoom) else {
            return Ok(SnapResult::none(delta));
        };
        let built;
        let scene: &Scene = match scene {
            Some(scene) => scene,
            None => {
                built = scene_for(ctx)?;
                match &built {
                    Some(scene) => scene,
                    None => return Ok(SnapResult::none(delta)),
                }
            }
        };
        let Some((bounds, _angle)) = selection_frame(ctx, Some(scene))? else {
            return Ok(SnapResult::none(delta));
        };
        let Some(session) = ctx.active_document() else {
            return Ok(SnapResult::none(delta));
        };
        let doc = &session.document;

        let mut x_candidates: Vec<(SnapKind, f64)> = Vec::new();
        let mut y_candidates: Vec<(SnapKind, f64)> = Vec::new();
        let moving_entries: Vec<_> = node_entries(scene)
            .into_iter()
            .filter(|entry| is_within(doc, entry.shape, moving))
            .collect();
        let stationary: Vec<_> = node_entries(scene)
            .into_iter()
            .filter(|entry| !is_within(doc, entry.shape, moving))
            .collect();

        // Alignment: the moving selection's edges and centre against the
        // stationary shapes' edges and centres (one axis at a time).
        for entry in &stationary {
            let Ok(other) = node_bounds(scene, &entry.path) else {
                continue;
            };
            let center = rect_center(other);
            if self.settings.to_shapes {
                for value in [other.min.x, center.x, other.max.x] {
                    for reference in [bounds.min.x, rect_center(bounds).x, bounds.max.x] {
                        x_candidates.push((SnapKind::Alignment, value - reference));
                    }
                }
                for value in [other.min.y, center.y, other.max.y] {
                    for reference in [bounds.min.y, rect_center(bounds).y, bounds.max.y] {
                        y_candidates.push((SnapKind::Alignment, value - reference));
                    }
                }
            }
            if self.settings.to_connection_points {
                for point in connection_points(doc, entry) {
                    for reference in [bounds.min.x, rect_center(bounds).x, bounds.max.x] {
                        x_candidates.push((SnapKind::ConnectionPoint, point.x - reference));
                    }
                    for reference in [bounds.min.y, rect_center(bounds).y, bounds.max.y] {
                        y_candidates.push((SnapKind::ConnectionPoint, point.y - reference));
                    }
                }
            }
        }

        // Connection points of the moving shapes can also land on a
        // stationary connection point. The moving points are read once
        // (task P5-11: recomputing them per stationary shape made the
        // pass O(n * c * m * c) and blew the ADR 0042 frame budget).
        if self.settings.to_connection_points {
            let moving_points: Vec<_> = moving_entries
                .iter()
                .flat_map(|entry| connection_points(doc, entry))
                .collect();
            for entry in &stationary {
                for target in connection_points(doc, entry) {
                    for source in &moving_points {
                        x_candidates.push((SnapKind::ConnectionPoint, target.x - source.x));
                        y_candidates.push((SnapKind::ConnectionPoint, target.y - source.y));
                    }
                }
            }
        }

        // Grid: every moving reference line quantized.
        if self.settings.to_grid && self.settings.grid.is_finite() && self.settings.grid > 0.0 {
            for reference in [bounds.min.x, rect_center(bounds).x, bounds.max.x] {
                let snapped =
                    snap_grid(reference + delta.x, self.settings.grid).map_err(geom_error)?;
                x_candidates.push((SnapKind::Grid, snapped - reference));
            }
            for reference in [bounds.min.y, rect_center(bounds).y, bounds.max.y] {
                let snapped =
                    snap_grid(reference + delta.y, self.settings.grid).map_err(geom_error)?;
                y_candidates.push((SnapKind::Grid, snapped - reference));
            }
        }
        let x = snap_axis_priority(delta.x, &x_candidates, max_distance)?;
        let y = snap_axis_priority(delta.y, &y_candidates, max_distance)?;
        let point = Point::new(
            x.map_or(delta.x, |(value, _)| value),
            y.map_or(delta.y, |(value, _)| value),
        );
        let kind = higher(x.map(|(_, kind)| kind), y.map(|(_, kind)| kind));
        Ok(SnapResult { point, kind })
    }

    /// Snaps one point (drawing tools, connector endpoints, drops).
    ///
    /// `scene` is the caller's page scene or `None` to build one (the
    /// same rule as [`Snapper::snap_move`]).
    pub fn snap_point(
        &self,
        ctx: &AppCtx,
        scene: Option<&Scene>,
        point: Point,
        zoom: f64,
        alt_held: bool,
    ) -> Result<SnapResult, UiError> {
        if alt_held || !point.x.is_finite() || !point.y.is_finite() {
            return Ok(SnapResult::none(point));
        }
        let Some(max_distance) = self.max_distance(zoom) else {
            return Ok(SnapResult::none(point));
        };
        let built;
        let scene: &Scene = match scene {
            Some(scene) => scene,
            None => {
                built = scene_for(ctx)?;
                match &built {
                    Some(scene) => scene,
                    None => return Ok(SnapResult::none(point)),
                }
            }
        };
        let Some(session) = ctx.active_document() else {
            return Ok(SnapResult::none(point));
        };
        let doc = &session.document;
        let mut x_candidates: Vec<(SnapKind, f64)> = Vec::new();
        let mut y_candidates: Vec<(SnapKind, f64)> = Vec::new();
        for entry in node_entries(scene) {
            let Ok(bounds) = node_bounds(scene, &entry.path) else {
                continue;
            };
            let center = rect_center(bounds);
            if self.settings.to_shapes {
                x_candidates.extend(
                    [bounds.min.x, center.x, bounds.max.x]
                        .into_iter()
                        .map(|value| (SnapKind::Alignment, value)),
                );
                y_candidates.extend(
                    [bounds.min.y, center.y, bounds.max.y]
                        .into_iter()
                        .map(|value| (SnapKind::Alignment, value)),
                );
            }
            if self.settings.to_connection_points {
                for cp in connection_points(doc, &entry) {
                    x_candidates.push((SnapKind::ConnectionPoint, cp.x));
                    y_candidates.push((SnapKind::ConnectionPoint, cp.y));
                }
            }
        }
        if self.settings.to_grid && self.settings.grid.is_finite() && self.settings.grid > 0.0 {
            let x = snap_grid(point.x, self.settings.grid).map_err(geom_error)?;
            let y = snap_grid(point.y, self.settings.grid).map_err(geom_error)?;
            x_candidates.push((SnapKind::Grid, x));
            y_candidates.push((SnapKind::Grid, y));
        }
        let x = snap_axis_priority(point.x, &x_candidates, max_distance)?;
        let y = snap_axis_priority(point.y, &y_candidates, max_distance)?;
        let point = Point::new(
            x.map_or(point.x, |(value, _)| value),
            y.map_or(point.y, |(value, _)| value),
        );
        let kind = higher(x.map(|(_, kind)| kind), y.map(|(_, kind)| kind));
        Ok(SnapResult { point, kind })
    }

    /// The snap distance in inches, or `None` when snapping cannot run.
    fn max_distance(&self, zoom: f64) -> Option<f64> {
        let zoom = if zoom.is_finite() && zoom > 0.0 {
            zoom
        } else {
            1.0
        };
        let distance = self.settings.distance_px / (zoom * crate::canvas::POINTS_PER_INCH);
        if distance.is_finite() && distance >= 0.0 {
            Some(distance)
        } else {
            None
        }
    }
}

/// The connection points of one scene node, in page inches.
///
/// The model helper returns the shape's own `Connection` rows or the rows
/// inherited from its master chain, scaled to the instance's size
/// ([`larrygram_model::Document::connection_points`]). The node transform
/// maps the local points to the page.
pub(crate) fn connection_points(
    doc: &larrygram_model::Document,
    entry: &crate::handles::NodeEntry,
) -> Vec<Point> {
    doc.connection_points(entry.shape)
        .into_iter()
        .map(|point| entry.transform.apply(point))
        .filter(|point| point.x.is_finite() && point.y.is_finite())
        .collect()
}

/// The nearest candidate of the highest kind within `max_distance`
/// (ADR 0043's cross-kind priority).
fn snap_axis_priority(
    value: f64,
    candidates: &[(SnapKind, f64)],
    max_distance: f64,
) -> Result<Option<(f64, SnapKind)>, UiError> {
    for kind in [
        SnapKind::ConnectionPoint,
        SnapKind::Alignment,
        SnapKind::Grid,
    ] {
        let of_kind: Vec<(SnapKind, f64)> = candidates
            .iter()
            .filter(|(candidate_kind, _)| *candidate_kind == kind)
            .copied()
            .collect();
        if let Some(found) = snap_axis(value, &of_kind, max_distance).map_err(geom_error)? {
            return Ok(Some(found));
        }
    }
    Ok(None)
}

/// The higher-priority kind of two optional kinds.
fn higher(a: Option<SnapKind>, b: Option<SnapKind>) -> Option<SnapKind> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

/// A geom failure as a [`UiError`] (task P5-19 review F8: the geometry
/// variant, never [`UiError::Io`] — no IO is involved).
fn geom_error(error: larrygram_geom::Error) -> UiError {
    UiError::Geometry {
        message: format!("snap math failed: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use larrygram_commands::{DocumentSession, Settings};
    use larrygram_model::{
        Cell, CellName, CellValue, Document, Page, PageId, Row, Section, SectionName, Shape,
    };

    fn cell(name: &str, value: f64) -> Cell {
        let mut cell = Cell::new(CellName::new(name));
        cell.value = Some(CellValue::Number(value));
        cell
    }

    fn shape(id: u32, pin: f64) -> Shape {
        let mut shape = Shape::new(ShapeId { index: id }, i32::try_from(id).unwrap_or(0));
        shape.sheet.cells = vec![
            cell("Width", 2.0),
            cell("Height", 1.0),
            cell("PinX", pin),
            cell("PinY", 2.0),
            cell("LocPinX", 1.0),
            cell("LocPinY", 0.5),
            cell("Angle", 0.0),
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
        shape
    }

    fn fixture() -> (AppCtx, ShapeId, ShapeId) {
        let mut doc = Document::new();
        let first = doc.add_shape(shape(0, 2.0));
        let second = doc.add_shape(shape(0, 7.0));
        let mut page = Page::new(PageId { index: 0 }, 1);
        page.shapes = vec![first, second];
        doc.pages.push(page);
        let mut app = AppCtx::new(Settings::default());
        app.open_document(DocumentSession::new(doc, None));
        app.selection.shapes = vec![first];
        app.selection.anchor = Some(first);
        (app, first, second)
    }

    #[test]
    fn alt_disables_snapping() {
        let (app, first, _) = fixture();
        let snapper = Snapper::default();
        let result = snapper
            .snap_move(&app, None, &[first], Point::new(0.01, 0.0), 1.0, true)
            .expect("no error");
        assert_eq!(result, SnapResult::none(Point::new(0.01, 0.0)));
        let point = snapper
            .snap_point(&app, None, Point::new(0.01, 0.01), 1.0, true)
            .expect("no error");
        assert_eq!(point, SnapResult::none(Point::new(0.01, 0.01)));
    }

    #[test]
    fn a_move_delta_snaps_to_the_grid() {
        let (app, first, _) = fixture();
        let snapper = Snapper::new(SnapSettings {
            to_shapes: false,
            to_connection_points: false,
            ..SnapSettings::default()
        });
        let result = snapper
            .snap_move(&app, None, &[first], Point::new(0.01, 0.0), 1.0, false)
            .expect("no error");
        assert_eq!(result.point, Point::new(0.0, 0.0));
        assert_eq!(result.kind, Some(SnapKind::Grid));
    }

    #[test]
    fn alignment_beats_the_grid_but_a_connection_point_beats_both() {
        let (mut app, first, second) = fixture();
        {
            let session = app.active_document_mut().expect("session");
            let shape = session.document.shape_mut(second).expect("shape");
            let mut section = Section::new(SectionName::from_static("Connection"));
            let mut row = Row::new();
            row.row_type = Some("Connection".to_owned());
            row.cells = vec![cell("X", 1.0), cell("Y", 5.0)];
            section.rows.push(row);
            shape.sheet.sections.push(section);
        }
        let snapper = Snapper::default();
        // The second shape's left edge is at 6; the first shape's right
        // edge is at 3, so a delta of 3 aligns them exactly.
        let result = snapper
            .snap_move(&app, None, &[first], Point::new(3.001, 0.0), 1.0, false)
            .expect("no error");
        assert!((result.point.x - 3.0).abs() < 1.0e-9, "{result:?}");
        assert_eq!(result.kind, Some(SnapKind::Alignment));

        // Its centre connection point at 7 is nearer to a delta of 5.99.
        let result = snapper
            .snap_move(&app, None, &[first], Point::new(5.99, 0.0), 1.0, false)
            .expect("no error");
        assert!((result.point.x - 6.0).abs() < 1.0e-9, "{result:?}");
        assert_eq!(result.kind, Some(SnapKind::ConnectionPoint));
    }

    #[test]
    fn point_snapping_uses_the_same_priority_and_grid() {
        let (app, _first, _second) = fixture();
        let snapper = Snapper::default();
        let result = snapper
            .snap_point(&app, None, Point::new(0.01, 0.01), 1.0, false)
            .expect("no error");
        assert_eq!(result.point, Point::new(0.0, 0.0));
        assert_eq!(result.kind, Some(SnapKind::Grid));
    }

    #[test]
    fn settings_can_turn_the_targets_off() {
        let (app, first, _) = fixture();
        let snapper = Snapper::new(SnapSettings {
            to_grid: false,
            to_shapes: false,
            to_connection_points: false,
            ..SnapSettings::default()
        });
        let result = snapper
            .snap_move(&app, None, &[first], Point::new(0.01, 0.01), 1.0, false)
            .expect("no error");
        assert_eq!(result, SnapResult::none(Point::new(0.01, 0.01)));
    }

    #[test]
    fn a_context_without_a_page_returns_the_unchanged_value() {
        let snapper = Snapper::default();
        let ctx = AppCtx::new(Settings::default());
        let delta = Point::new(0.01, 0.01);
        assert_eq!(
            snapper
                .snap_move(&ctx, None, &[], delta, 1.0, false)
                .expect("no error"),
            SnapResult::none(delta)
        );
        assert_eq!(
            snapper
                .snap_point(&ctx, None, delta, 1.0, false)
                .expect("no error"),
            SnapResult::none(delta)
        );
    }

    #[test]
    fn no_selection_has_nothing_to_snap() {
        let (mut app, _first, _second) = fixture();
        app.selection.clear();
        let snapper = Snapper::default();
        let delta = Point::new(0.01, 0.01);
        assert_eq!(
            snapper
                .snap_move(&app, None, &[], delta, 1.0, false)
                .expect("no error"),
            SnapResult::none(delta)
        );
    }

    #[test]
    fn moving_connection_points_can_land_on_a_stationary_one() {
        let (mut app, first, second) = fixture();
        for shape in [first, second] {
            let session = app.active_document_mut().expect("session");
            let entry = session.document.shape_mut(shape).expect("shape");
            let mut section = Section::new(SectionName::from_static("Connection"));
            let mut row = Row::new();
            row.row_type = Some("Connection".to_owned());
            row.cells = vec![cell("X", 1.0), cell("Y", 0.5)];
            section.rows.push(row);
            entry.sheet.sections.push(section);
        }
        let snapper = Snapper::default();
        // The first shape's connection point at (2, 2) lands on the
        // second's at (7, 2): delta 5.
        let result = snapper
            .snap_move(&app, None, &[first], Point::new(5.01, 0.0), 1.0, false)
            .expect("no error");
        assert!((result.point.x - 5.0).abs() < 1.0e-9, "{result:?}");
        assert_eq!(result.kind, Some(SnapKind::ConnectionPoint));
    }

    #[test]
    fn malformed_connection_rows_are_skipped() {
        let (mut app, _first, second) = fixture();
        {
            let session = app.active_document_mut().expect("session");
            let entry = session.document.shape_mut(second).expect("shape");
            let mut section = Section::new(SectionName::from_static("Connection"));
            let mut missing = Row::new();
            missing.cells = vec![cell("X", 1.0)];
            let mut bad = Row::new();
            bad.cells = vec![Cell {
                name: CellName::new("X"),
                formula: None,
                value: Some(CellValue::Str("nope".to_owned())),
                unit: larrygram_geom::Unit::Inches,
                preserved: Default::default(),
            }];
            section.rows.push(missing);
            section.rows.push(bad);
            entry.sheet.sections.push(section);
        }
        let scene = scene_for(&app).expect("scene").expect("page");
        let entries = node_entries(&scene);
        assert!(entries.iter().all(|entry| {
            connection_points(&app.active_document().expect("session").document, entry).is_empty()
        }));
        let stale = crate::handles::NodeEntry {
            shape: ShapeId { index: 99 },
            path: larrygram_render::NodePath::new(),
            transform: larrygram_geom::Transform::identity(),
        };
        assert!(
            connection_points(&app.active_document().expect("session").document, &stale).is_empty()
        );
    }

    #[test]
    fn the_kind_priority_helpers_cover_every_pair() {
        assert_eq!(
            higher(Some(SnapKind::Grid), Some(SnapKind::Alignment)),
            Some(SnapKind::Alignment)
        );
        assert_eq!(higher(Some(SnapKind::Grid), None), Some(SnapKind::Grid));
        assert_eq!(
            higher(None, Some(SnapKind::Alignment)),
            Some(SnapKind::Alignment)
        );
        assert_eq!(higher(None, None), None);
        let error = geom_error(larrygram_geom::Error::Singular);
        assert!(matches!(error, UiError::Geometry { .. }));
    }

    #[test]
    fn a_non_finite_delta_or_zoom_never_panics() {
        let (app, first, _) = fixture();
        let snapper = Snapper::default();
        let result = snapper
            .snap_move(&app, None, &[first], Point::new(f64::NAN, 0.0), 1.0, false)
            .expect("no error");
        assert_eq!(result.kind, None);
        for zoom in [f64::NAN, 0.0, -1.0] {
            let result = snapper
                .snap_move(&app, None, &[first], Point::new(0.01, 0.01), zoom, false)
                .expect("no error");
            assert!(result.point.x.is_finite() && result.point.y.is_finite());
            let result = snapper
                .snap_point(&app, None, Point::new(0.01, 0.01), zoom, false)
                .expect("no error");
            assert!(result.point.x.is_finite() && result.point.y.is_finite());
        }
    }
}
