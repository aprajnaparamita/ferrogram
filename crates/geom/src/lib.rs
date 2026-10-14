//! Geometry primitives: points, sizes, rectangles, angles, affine transforms
//! and Visio units of measure. Canonical units are inches for lengths and
//! radians for angles (ADR 0011).
//!
//! Phase 2 needs only the types below. Path, arc, NURBS and spline types land
//! with the geometry-row work, because Phase 3 rendering is their first
//! consumer.
//!
//! Part of Ferrogram. Alpha.

#![forbid(unsafe_code)]

mod angle;
mod error;
mod point;
mod rect;
mod transform;
mod units;

pub use angle::Angle;
pub use error::Error;
pub use point::{Point, Size};
pub use rect::Rect;
pub use transform::Transform;
pub use units::{Unit, UnitKind};
