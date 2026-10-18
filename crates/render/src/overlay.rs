//! Drag and selection overlays (ADR 0042).
//!
//! The overlay is transient vector art drawn on top of the cached canvas
//! raster: the raster is re-rasterized only when the document change
//! counter moves, so a drag redraws marks, never a full page. The marks
//! are plain data in page inches; `crates/ui` (L6) draws them as egui
//! shapes.
//!
//! Task P5-03 implements [`build_selection_overlay`]; the mark types and
//! the trivial constructors are fixed here by task P5-01.

use ferrogram_geom::{Angle, Point, Rect};

use crate::error::Error;
use crate::hit::NodePath;
use crate::scene::Scene;

/// One overlay mark (ADR 0042).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum OverlayMark {
    /// The selection outline around a shape's bounds, rotated with it.
    SelectionOutline {
        /// The shape's bounds, in page inches.
        bounds: Rect,
        /// The shape's rotation.
        angle: Angle,
    },
    /// The resize and rotate handles of the selection, at their anchor
    /// points (the UI draws the glyphs).
    Handles {
        /// The selection bounds, in page inches.
        bounds: Rect,
        /// The selection rotation.
        angle: Angle,
    },
    /// The marquee rectangle while it is dragged.
    RubberBand(Rect),
    /// A snap guide line (two points, in page inches).
    SnapGuide {
        /// One end of the guide.
        a: Point,
        /// The other end.
        b: Point,
    },
    /// The live preview of a connector being drawn or re-routed.
    ConnectorPreview {
        /// The routed points, in page inches (ADR 0045).
        points: Vec<Point>,
    },
}

/// The transient marks of one frame (ADR 0042).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Overlay {
    /// The marks, in draw order.
    pub marks: Vec<OverlayMark>,
}

impl Overlay {
    /// An empty overlay.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether there is nothing to draw.
    pub fn is_empty(&self) -> bool {
        self.marks.is_empty()
    }
}

/// The selection outline and handle marks for the given nodes (the
/// selection's shapes as scene nodes).
///
/// Handles are laid out around the union bounds of the nodes (one shape:
/// its rotated bounds; several: the axis-aligned union). Task P5-03
/// implements the body.
pub fn build_selection_overlay(scene: &Scene, nodes: &[NodePath]) -> Result<Overlay, Error> {
    let _ = (scene, nodes);
    Err(Error::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlays_hold_their_marks() {
        let overlay = Overlay::new();
        assert!(overlay.is_empty());
        let mut overlay = Overlay::new();
        overlay.marks.push(OverlayMark::RubberBand(Rect::new(
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
        )));
        assert!(!overlay.is_empty());
        assert_eq!(overlay.marks.len(), 1);
    }

    /// The P5-01 stub: replaced by a real test in task P5-03.
    #[test]
    fn the_p5_01_stub_is_not_implemented() {
        let scene = Scene::new(1.0, 1.0);
        assert_eq!(
            build_selection_overlay(&scene, &[NodePath::new()]),
            Err(Error::NotImplemented)
        );
    }
}
