//! Scene graph, PNG and SVG rendering (ADR 0020, ADR 0025).
//!
//! `build_scene` turns one page of a `ferrogram-model` document into a
//! [`Scene`] through the inheritance lookup (ADR 0021), and
//! `render_png`/`render_svg` rasterize or write that scene. The scene is
//! shared with the UI (ADR 0002: `render` is L3).
//!
//! Part of Ferrogram. Alpha.

#![forbid(unsafe_code)]

pub mod build;
mod error;
pub mod png;
pub mod scene;
pub mod styles;
pub mod svg;

pub use build::{BuildOptions, build_scene};
pub use error::Error;
pub use png::{PngOptions, render_png};
pub use scene::{Rgba, Scene};
pub use svg::{SvgOptions, render_svg};
