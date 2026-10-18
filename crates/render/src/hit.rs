//! Hit-testing against the scene graph (ADR 0041).
//!
//! The scene is in page inches (ADR 0020); the caller converts the
//! tolerance from screen pixels (`pixels / (zoom * 96)`). Results come
//! topmost first: the caller picks the first hit it can select. Filled
//! shapes are hit on their fill, unfilled shapes on their outline and
//! text; groups are not hit directly (their leaf shapes are, and the
//! caller maps to the top-level ancestor through the model).
//!
//! Task P5-03 implements the bodies. The bodies are
//! `Err(Error::NotImplemented)` stubs until then.

use ferrogram_geom::{Point, Rect};
use ferrogram_model::ShapeId;

use crate::error::Error;
use crate::scene::Scene;

/// The path of one node in the scene tree: the child index at each level
/// (`[0, 2]` = the third child of the first top-level node).
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct NodePath(pub Vec<usize>);

impl NodePath {
    /// The path of a top-level node.
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// The path one level deeper, through child `index`.
    pub fn child(&self, index: usize) -> NodePath {
        let mut indices = self.0.clone();
        indices.push(index);
        NodePath(indices)
    }

    /// The indices, outermost first.
    pub fn indices(&self) -> &[usize] {
        &self.0
    }
}

/// What part of a shape was hit (ADR 0041).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum HitKind {
    /// Inside the fill of a filled shape.
    Fill,
    /// Within tolerance of the stroked outline.
    Stroke,
    /// On the text block.
    Text,
    /// On the shape's bounds only (the marquee case).
    Bounds,
}

/// One hit-test result (ADR 0041).
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    /// The hit node's path in the scene tree.
    pub node: NodePath,
    /// The model shape behind the node, when it has one.
    pub shape_id: Option<ShapeId>,
    /// What part was hit.
    pub kind: HitKind,
    /// Distance from the query point to the hit part, in inches (0 inside
    /// a fill); results sort by it.
    pub distance: f64,
}

/// Options for [`hit_test`] (ADR 0041).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HitOptions {
    /// The outline tolerance, in inches (converted from screen pixels by
    /// the caller).
    pub tolerance: f64,
}

impl HitOptions {
    /// Options with the given tolerance, in inches.
    pub const fn new(tolerance: f64) -> Self {
        Self { tolerance }
    }
}

/// Every shape hit by `point`, topmost first.
///
/// A filled shape is hit when the point is inside its fill or within
/// `tolerance` of its outline; an unfilled shape only on the outline and
/// its text (ADR 0041).
pub fn hit_test(scene: &Scene, point: Point, options: &HitOptions) -> Result<Vec<Hit>, Error> {
    let _ = (scene, point, options);
    Err(Error::NotImplemented)
}

/// Every shape whose transformed bounds intersect `rect`, topmost first
/// (the marquee rule of ADR 0041: intersection, not containment).
pub fn hit_marquee(scene: &Scene, rect: Rect) -> Result<Vec<Hit>, Error> {
    let _ = (scene, rect);
    Err(Error::NotImplemented)
}

/// The transformed bounding box of one node, in page inches (handles and
/// overlays, ADR 0042).
pub fn node_bounds(scene: &Scene, node: &NodePath) -> Result<Rect, Error> {
    let _ = (scene, node);
    Err(Error::NotImplemented)
}

/// The bounds of every node of the scene, in page inches (`view.zoom.fit`
/// and selection-all).
pub fn scene_bounds(scene: &Scene) -> Result<Rect, Error> {
    let _ = scene;
    Err(Error::NotImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_paths_nest() {
        let root = NodePath::new();
        assert!(root.indices().is_empty());
        let child = root.child(2);
        assert_eq!(child.indices(), &[2]);
        assert_eq!(child.child(0).indices(), &[2, 0]);
    }

    #[test]
    fn hits_hold_their_data() {
        let hit = Hit {
            node: NodePath::new(),
            shape_id: Some(ShapeId { index: 3 }),
            kind: HitKind::Stroke,
            distance: 0.01,
        };
        assert_eq!(hit.shape_id, Some(ShapeId { index: 3 }));
        assert_eq!(hit.kind, HitKind::Stroke);
        assert_eq!(HitOptions::new(0.02).tolerance, 0.02);
        assert_eq!(HitOptions::default().tolerance, 0.0);
    }

    /// The P5-01 stubs: replaced by real behaviour tests in task P5-03.
    #[test]
    fn the_p5_01_stubs_are_not_implemented() {
        let scene = Scene::new(1.0, 1.0);
        let p = Point::new(0.0, 0.0);
        assert_eq!(
            hit_test(&scene, p, &HitOptions::default()),
            Err(Error::NotImplemented)
        );
        assert_eq!(
            hit_marquee(&scene, Rect::new(p, Point::new(1.0, 1.0))),
            Err(Error::NotImplemented)
        );
        assert_eq!(
            node_bounds(&scene, &NodePath::new()),
            Err(Error::NotImplemented)
        );
        assert_eq!(scene_bounds(&scene), Err(Error::NotImplemented));
    }
}
