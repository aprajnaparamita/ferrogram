//! The "Basic Flowchart" stencil: 9 masters of our own design (ADR 0034).
//!
//! The masters are the classic flowchart symbol *families* (process,
//! decision, terminator, ...) with coordinates we wrote ourselves: a
//! rectangle, a diamond, a trapezoid, a hexagon, a parallelogram, two
//! rounded rectangles, a document box with a wave bottom edge and a
//! connector line. No shape is traced, copied or derived from Microsoft
//! material (AGENTS.md section 4).

use super::{
    FillDefaults, GeometryRow, LineDefaults, MasterDef, closed_polygon, edge_midpoints,
    rounded_rect,
};

/// A master with our default line (ADR 0023) and two Shape Data rows.
fn master(
    name_u: &'static str,
    name: &'static str,
    width: f64,
    height: f64,
    geometry: Vec<GeometryRow>,
    fill: FillDefaults,
    connection_points: Vec<(f64, f64)>,
) -> MasterDef {
    MasterDef {
        name_u,
        name,
        width,
        height,
        geometry,
        line: LineDefaults::default(),
        fill,
        shape_data: vec![("Process", name), ("Cost", "0")],
        connection_points,
    }
}

/// The "Basic Flowchart" masters, in Shapes window order.
pub(super) fn masters() -> Vec<MasterDef> {
    let filled = FillDefaults::default();
    let stroked = FillDefaults {
        pattern: 0,
        ..FillDefaults::default()
    };
    vec![
        master(
            "Process",
            "Process",
            1.5,
            0.75,
            closed_polygon(&[(0.0, 0.0), (1.5, 0.0), (1.5, 0.75), (0.0, 0.75)]),
            filled,
            edge_midpoints(1.5, 0.75),
        ),
        master(
            "AlternateProcess",
            "Alternate Process",
            1.5,
            0.75,
            rounded_rect(1.5, 0.75, 0.12),
            filled,
            edge_midpoints(1.5, 0.75),
        ),
        master(
            "Decision",
            "Decision",
            1.2,
            0.9,
            closed_polygon(&[(0.6, 0.0), (1.2, 0.45), (0.6, 0.9), (0.0, 0.45)]),
            filled,
            edge_midpoints(1.2, 0.9),
        ),
        master(
            "Terminator",
            "Terminator",
            1.5,
            0.6,
            rounded_rect(1.5, 0.6, 0.3),
            filled,
            edge_midpoints(1.5, 0.6),
        ),
        master(
            "Data",
            "Data",
            1.5,
            0.75,
            closed_polygon(&[(0.3, 0.0), (1.5, 0.0), (1.2, 0.75), (0.0, 0.75)]),
            filled,
            edge_midpoints(1.5, 0.75),
        ),
        master(
            "Document",
            "Document",
            1.5,
            0.8,
            vec![
                GeometryRow::MoveTo { x: 0.0, y: 0.2 },
                GeometryRow::ArcTo {
                    x: 0.75,
                    y: 0.2,
                    bow: -0.08,
                },
                GeometryRow::ArcTo {
                    x: 1.5,
                    y: 0.2,
                    bow: 0.08,
                },
                GeometryRow::LineTo { x: 1.5, y: 0.8 },
                GeometryRow::LineTo { x: 0.0, y: 0.8 },
                GeometryRow::LineTo { x: 0.0, y: 0.2 },
            ],
            filled,
            edge_midpoints(1.5, 0.8),
        ),
        master(
            "ManualOperation",
            "Manual Operation",
            1.5,
            0.75,
            closed_polygon(&[(0.3, 0.0), (1.2, 0.0), (1.5, 0.75), (0.0, 0.75)]),
            filled,
            edge_midpoints(1.5, 0.75),
        ),
        master(
            "Preparation",
            "Preparation",
            1.5,
            0.75,
            closed_polygon(&[
                (0.3, 0.0),
                (1.2, 0.0),
                (1.5, 0.375),
                (1.2, 0.75),
                (0.3, 0.75),
                (0.0, 0.375),
            ]),
            filled,
            edge_midpoints(1.5, 0.75),
        ),
        master(
            "Connector",
            "Connector",
            1.2,
            0.6,
            vec![
                GeometryRow::MoveTo { x: 0.0, y: 0.0 },
                GeometryRow::LineTo { x: 1.2, y: 0.6 },
            ],
            stroked,
            vec![
                (0.6, 0.0),
                (1.2, 0.3),
                (0.6, 0.6),
                (0.0, 0.3),
                (0.0, 0.0),
                (1.2, 0.6),
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stencil_has_nine_masters() {
        let masters = masters();
        assert_eq!(masters.len(), 9);
        assert_eq!(masters[0].name_u, "Process");
        assert_eq!(masters[8].name_u, "Connector");
    }

    #[test]
    fn the_connector_carries_its_ends() {
        let masters = masters();
        let connector = masters
            .iter()
            .find(|master| master.name_u == "Connector")
            .expect("connector");
        assert_eq!(connector.connection_points.len(), 6);
        assert!(connector.connection_points.contains(&(0.0, 0.0)));
        assert!(connector.connection_points.contains(&(1.2, 0.6)));
    }
}
