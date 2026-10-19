//! Connection points inherited from masters (ADR 0013, ADR 0043).
//!
//! A shape's `Connection` section rows live in the shape's local
//! coordinates. A master instance usually has no rows of its own and
//! inherits the section from its master; a resized instance must keep the
//! inherited points on its own outline, so the master's coordinates are
//! scaled by the instance's size over the master shape's size.
//!
//! The walk follows the master chain of [`Document::master_chain_shapes`]
//! and is bounded like the lookup (ADR 0013): a cyclic chain stops at the
//! first repeat, and at most a fixed rung cap (128) are visited.

use ferrogram_geom::Point;

use crate::cell::CellValue;
use crate::document::Document;
use crate::id::ShapeId;
use crate::section::{Row, Section};
use crate::shape::Shape;

/// The maximum number of master rungs a connection-point walk visits.
///
/// The lookup cuts a cyclic chain through its visited set; this cap is the
/// same belt-and-braces bound the geometry rewrite uses.
const MAX_CHAIN: usize = 128;

impl Document {
    /// The connection points of `shape`, in the shape's local coordinates
    /// (inches).
    ///
    /// The shape's own `Connection` rows win. A shape without rows
    /// inherits them through its master chain: the nearest master shape
    /// that has rows defines the points. Each point is scaled by the
    /// instance's `Width`/`Height` over that master shape's
    /// `Width`/`Height`, so a resized instance keeps its points on its
    /// outline. A zero or missing master size (or instance size) means no
    /// scaling on that axis.
    ///
    /// Rows with a non-finite or missing `X`/`Y` value are skipped. A
    /// stale shape id or a chain without rows yields an empty list. This
    /// function never panics.
    pub fn connection_points(&self, shape: ShapeId) -> Vec<Point> {
        self.connection_points_indexed(shape)
            .into_iter()
            .map(|(_, point)| point)
            .collect()
    }

    /// The connection points of `shape` with their `Connection` row
    /// indexes (`Row/@IX`), in file order.
    ///
    /// A malformed row (missing or non-finite `X`/`Y`) is skipped
    /// without renumbering the rows around it, so a glue record that
    /// names `IX` addresses the same row the file does (task P5-17,
    /// Goal 4). A row without an `IX` is named by its position in the
    /// section, which matches the readers that assign one.
    pub fn connection_points_indexed(&self, shape: ShapeId) -> Vec<(i32, Point)> {
        let Some(entity) = self.shape(shape) else {
            return Vec::new();
        };
        let own = connection_rows(&entity.sheet.sections);
        if !own.is_empty() {
            return indexed_rows(own);
        }

        for master_id in self.master_chain_shapes(shape, MAX_CHAIN) {
            let Some(master) = self.shape(master_id) else {
                continue;
            };
            let rows = connection_rows(&master.sheet.sections);
            if rows.is_empty() {
                continue;
            }
            let (scale_x, scale_y) = self.connection_scale(entity, master);
            return indexed_rows_scaled(rows, scale_x, scale_y);
        }
        Vec::new()
    }

    /// The per-axis scale from a master shape's local box to the
    /// instance's box. A missing or non-finite size, or a zero master
    /// size, leaves that axis unscaled.
    fn connection_scale(&self, instance: &Shape, master: &Shape) -> (f64, f64) {
        (
            axis_scale(
                direct_number(instance, "Width"),
                direct_number(master, "Width"),
            ),
            axis_scale(
                direct_number(instance, "Height"),
                direct_number(master, "Height"),
            ),
        )
    }
}

/// The `Connection` rows of a sheet, across every section of that name.
fn connection_rows(sections: &[Section]) -> Vec<&Row> {
    sections
        .iter()
        .filter(|section| section.name.as_str() == "Connection")
        .flat_map(|section| section.rows.iter())
        .collect()
}

/// The `(IX, point)` pairs of one row list, skipping malformed rows.
fn indexed_rows(rows: Vec<&Row>) -> Vec<(i32, Point)> {
    indexed_rows_scaled(rows, 1.0, 1.0)
}

/// The `(IX, point)` pairs of one row list with per-axis scaling,
/// skipping malformed rows.
fn indexed_rows_scaled(rows: Vec<&Row>, scale_x: f64, scale_y: f64) -> Vec<(i32, Point)> {
    rows.into_iter()
        .enumerate()
        .filter_map(|(position, row)| {
            let point = row_point_scaled(row, scale_x, scale_y)?;
            let ix = row
                .ix
                .filter(|ix| *ix >= 0)
                .unwrap_or_else(|| i32::try_from(position).unwrap_or(i32::MAX));
            Some((ix, point))
        })
        .collect()
}

/// One row's `X`/`Y` scaled by the per-axis factors, or `None` when either
/// value is missing or non-finite.
fn row_point_scaled(row: &Row, scale_x: f64, scale_y: f64) -> Option<Point> {
    let x = row_number(row, "X")?;
    let y = row_number(row, "Y")?;
    let point = Point::new(x * scale_x, y * scale_y);
    if point.x.is_finite() && point.y.is_finite() {
        Some(point)
    } else {
        None
    }
}

/// The cached number of one row cell, or `None`.
fn row_number(row: &Row, name: &str) -> Option<f64> {
    row.cells
        .iter()
        .find(|cell| cell.name.as_str() == name)
        .and_then(|cell| match &cell.value {
            Some(CellValue::Number(value)) => Some(*value),
            _ => None,
        })
}

/// The direct cached number of one shape cell, or `None`.
fn direct_number(shape: &Shape, name: &str) -> Option<f64> {
    shape
        .sheet
        .cells
        .iter()
        .find(|cell| cell.name.as_str() == name)
        .and_then(|cell| match &cell.value {
            Some(CellValue::Number(value)) => Some(*value),
            _ => None,
        })
}

/// One axis' scale factor: the instance size over the master size, or
/// `1.0` when either size is missing, non-finite or the master size is
/// zero.
fn axis_scale(instance: Option<f64>, master: Option<f64>) -> f64 {
    match (instance, master) {
        (Some(instance), Some(master))
            if instance.is_finite() && master.is_finite() && master != 0.0 =>
        {
            instance / master
        }
        _ => 1.0,
    }
}
