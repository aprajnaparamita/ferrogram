//! Typed geometry rows and paths (ADR 0019).
//!
//! The `T` attribute of a `Geometry` section row names its kind ([MS-VSDX]
//! `GeometryRow_Type`); this module types a row's cached cells into a
//! [`GeometryRow`] and a section into a [`GeometryPath`]. The `Rel*` rows
//! keep their values as fractions of the shape's width and height; the
//! scene builder multiplies (ADR 0022). `NurbsTo` and `PolylineTo` decode
//! their formula-encoded cached data (`NURBS(...)` text and the A-cell
//! point list) without evaluating any formula (Phase 6).

use ferrogram_geom::{Angle, Point};

use crate::document::Document;
use crate::error::Error;
use crate::id::ShapeId;
use crate::values::CellWarning;

/// One control point of a [`NurbsData`] curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NurbsDataPoint {
    /// The x coordinate, a fraction of the width (`x_type` 0) or a local
    /// coordinate (`x_type` 1).
    pub x: f64,
    /// The y coordinate, a fraction of the height (`y_type` 0) or a local
    /// coordinate (`y_type` 1).
    pub y: f64,
    /// The knot of this control point.
    pub knot: f64,
    /// The weight of this control point.
    pub weight: f64,
}

/// The decoded data of a `NURBSTo` row's E-cell `NURBS()` formula (ADR 0022).
#[derive(Debug, Clone, PartialEq)]
pub struct NurbsData {
    /// The last knot of the formula.
    pub knot_last: f64,
    /// The degree of the spline.
    pub degree: u32,
    /// The `xType` of the formula: 0 fraction of width, 1 local.
    pub x_type: u32,
    /// The `yType` of the formula: 0 fraction of height, 1 local.
    pub y_type: u32,
    /// The control points with their knots and weights, in order.
    pub points: Vec<NurbsDataPoint>,
}

/// The decoded point list of a `PolylineTo` row's A cell (ADR 0022).
#[derive(Debug, Clone, PartialEq)]
pub struct PolylineData {
    /// The points, in order.
    pub points: Vec<Point>,
}

/// One typed geometry row (ADR 0019).
///
/// Values are as cached in the file (canonical units, ADR 0011); the
/// `Rel*` rows hold fractions of the shape's width and height (ADR 0022).
/// Cell semantics per row are documented in ADR 0022 and the ShapeSheet
/// reference.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum GeometryRow {
    /// `MoveTo`: starts a subpath at (x, y).
    MoveTo {
        /// The x coordinate.
        x: f64,
        /// The y coordinate.
        y: f64,
    },
    /// `LineTo`: a straight segment to (x, y).
    LineTo {
        /// The x coordinate.
        x: f64,
        /// The y coordinate.
        y: f64,
    },
    /// `ArcTo`: a circular arc to (x, y) with the given bow.
    ArcTo {
        /// The x coordinate of the end vertex.
        x: f64,
        /// The y coordinate of the end vertex.
        y: f64,
        /// The bow (sagitta) of the arc.
        bow: f64,
    },
    /// `EllipticalArcTo`: an elliptical arc to (x, y).
    EllipticalArcTo {
        /// The x coordinate of the end vertex.
        x: f64,
        /// The y coordinate of the end vertex.
        y: f64,
        /// The control point (A, B), a point on the arc.
        ctrl: Point,
        /// The angle of the major axis (C).
        angle: Angle,
        /// The major/minor ratio (D).
        ratio: f64,
    },
    /// `NURBSTo`: a NURBS segment ending at (x, y).
    NurbsTo {
        /// The x coordinate of the last control point.
        x: f64,
        /// The y coordinate of the last control point.
        y: f64,
        /// The second-to-last knot (A).
        second_last_knot: f64,
        /// The last weight (B).
        last_weight: f64,
        /// The first knot (C).
        first_knot: f64,
        /// The first weight (D).
        first_weight: f64,
        /// The decoded E-cell formula, when present and parsable.
        data: Option<NurbsData>,
    },
    /// `PolylineTo`: a polyline ending at (x, y).
    PolylineTo {
        /// The x coordinate of the end vertex.
        x: f64,
        /// The y coordinate of the end vertex.
        y: f64,
        /// The decoded A-cell point list, when present and parsable.
        data: Option<PolylineData>,
    },
    /// `SplineStart`: opens a B-spline; (x, y) is its second control point.
    SplineStart {
        /// The x coordinate of the second control point.
        x: f64,
        /// The y coordinate of the second control point.
        y: f64,
        /// The second knot (A).
        second_knot: f64,
        /// The first knot (B).
        first_knot: f64,
        /// The last knot (C).
        last_knot: f64,
        /// The degree (D), 1 to 25.
        degree: i32,
    },
    /// `SplineKnot`: one more control point and knot of the open spline.
    SplineKnot {
        /// The x coordinate of the control point.
        x: f64,
        /// The y coordinate of the control point.
        y: f64,
        /// The knot (A).
        knot: f64,
    },
    /// `Ellipse`: an ellipse by centre and two conjugate semi-diameter
    /// endpoints (ADR 0022).
    Ellipse {
        /// The centre (X, Y).
        center: Point,
        /// The first semi-diameter endpoint (A, B).
        a: Point,
        /// The second semi-diameter endpoint (C, D).
        b: Point,
    },
    /// `InfiniteLine`: the line through (p1, p2).
    InfiniteLine {
        /// The first point (X, Y).
        p1: Point,
        /// The second point (A, B).
        p2: Point,
    },
    /// `RelMoveTo`: `MoveTo` with coordinates as width/height fractions.
    RelMoveTo {
        /// The x fraction of the width.
        x: f64,
        /// The y fraction of the height.
        y: f64,
    },
    /// `RelLineTo`: `LineTo` with coordinates as width/height fractions.
    RelLineTo {
        /// The x fraction of the width.
        x: f64,
        /// The y fraction of the height.
        y: f64,
    },
    /// `RelQuadBezTo`: a quadratic Bézier with fraction coordinates.
    RelQuadBezTo {
        /// The x fraction of the end vertex.
        x: f64,
        /// The y fraction of the end vertex.
        y: f64,
        /// The control point (A, B) as fractions.
        ctrl: Point,
    },
    /// `RelCubBezTo`: a cubic Bézier with fraction coordinates.
    RelCubBezTo {
        /// The x fraction of the end vertex.
        x: f64,
        /// The y fraction of the end vertex.
        y: f64,
        /// The first control point (A, B) as fractions.
        ctrl1: Point,
        /// The second control point (C, D) as fractions.
        ctrl2: Point,
    },
    /// `RelEllipticalArcTo`: `EllipticalArcTo` with fraction coordinates.
    RelEllipticalArcTo {
        /// The x fraction of the end vertex.
        x: f64,
        /// The y fraction of the end vertex.
        y: f64,
        /// The control point (A, B) as fractions.
        ctrl: Point,
        /// The angle of the major axis (C).
        angle: Angle,
        /// The major/minor ratio (D).
        ratio: f64,
    },
    /// A row type this model does not map; its cells stay in the row
    /// (ADR 0019).
    Unknown {
        /// The `T` attribute as written.
        row_type: String,
    },
}

/// One typed `Geometry` section (ADR 0019).
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryPath {
    /// The rows, in file order.
    pub rows: Vec<GeometryRow>,
    /// The section's `NoFill` cell.
    pub no_fill: bool,
    /// The section's `NoLine` cell.
    pub no_line: bool,
    /// The section's `NoShow` cell.
    pub no_show: bool,
}

/// The typed geometry of one shape, with row-level inheritance resolved
/// (ADR 0019) and the cached-value policy applied (ADR 0021).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ShapeGeometry {
    /// The `Geometry` sections, in file order.
    pub paths: Vec<GeometryPath>,
    /// The warnings recorded while reading the rows (ADR 0021).
    pub warnings: Vec<CellWarning>,
}

impl Document {
    /// The typed geometry of one shape, with row-level inheritance
    /// resolved (ADR 0019): the shape's own rows where it has any, the
    /// master chain's rows otherwise.
    ///
    /// Returns [`Error::NotImplemented`] until the model work lands
    /// (P3-02).
    pub fn shape_geometry(&self, shape: ShapeId) -> Result<ShapeGeometry, Error> {
        let _ = shape;
        let _ = self;
        Err(Error::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_variants_hold_their_data() {
        let r = GeometryRow::ArcTo {
            x: 1.0,
            y: 2.0,
            bow: 0.5,
        };
        assert!(matches!(r, GeometryRow::ArcTo { bow, .. } if bow == 0.5));
        let e = GeometryRow::Ellipse {
            center: Point::new(0.0, 0.0),
            a: Point::new(1.0, 0.0),
            b: Point::new(0.0, 1.0),
        };
        assert!(matches!(
            e,
            GeometryRow::Ellipse {
                a: Point { x: 1.0, y: 0.0 },
                ..
            }
        ));
        assert_eq!(
            GeometryRow::Unknown {
                row_type: "Whatever".to_owned()
            },
            GeometryRow::Unknown {
                row_type: "Whatever".to_owned()
            }
        );
    }

    #[test]
    fn nurbs_data_holds_its_points() {
        let d = NurbsData {
            knot_last: 1.0,
            degree: 3,
            x_type: 1,
            y_type: 0,
            points: vec![NurbsDataPoint {
                x: 0.0,
                y: 1.0,
                knot: 0.5,
                weight: 1.0,
            }],
        };
        assert_eq!(d.degree, 3);
        assert_eq!(d.points.len(), 1);
        let p = PolylineData {
            points: vec![Point::new(0.0, 0.0)],
        };
        assert_eq!(p.points.len(), 1);
    }

    #[test]
    fn geometry_path_defaults_are_false() {
        let p = GeometryPath {
            rows: Vec::new(),
            no_fill: false,
            no_line: false,
            no_show: false,
        };
        assert!(p.rows.is_empty());
        let g = ShapeGeometry::default();
        assert!(g.paths.is_empty());
        assert!(g.warnings.is_empty());
    }

    #[test]
    fn shape_geometry_is_not_implemented_yet() {
        let doc = Document::new();
        assert_eq!(
            doc.shape_geometry(ShapeId { index: 0 }),
            Err(Error::NotImplemented)
        );
    }
}
