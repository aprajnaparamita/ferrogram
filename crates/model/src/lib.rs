//! Document model: documents, pages, masters, shapes, stylesheets and
//! ShapeSheet sheets, sections, rows and cells.
//!
//! The model is format-agnostic (layer L1, ADR 0002). It keeps Visio's own
//! identifiers (`ID`, `NameU`, `Name`) on every entity, stores each ShapeSheet
//! cell as its formula, its cached value and its unit separately (ADR 0011),
//! and preserves unknown XML verbatim (ADR 0012). Master and stylesheet
//! inheritance is resolved through a lookup, never flattened on load
//! (ADR 0013).
//!
//! Part of Ferrogram. Alpha.

#![forbid(unsafe_code)]

mod cell;
mod document;
mod error;
pub mod generated;
mod id;
mod lookup;
mod master;
mod name;
mod page;
mod raw;
mod section;
mod shape;
mod sheet;
mod style;

pub use cell::{Cell, CellValue, Color};
pub use document::{Document, DocumentKind, Property};
pub use error::Error;
pub use id::{MasterId, PageId, ShapeId, StyleId};
pub use lookup::{CellLookup, Resolved, ValueSource};
pub use master::Master;
pub use name::{CellName, SectionName};
pub use page::{Connect, Page};
pub use raw::{
    DifferenceKind, Preserved, QName, XmlAttribute, XmlChild, XmlDifference, XmlNode, diff,
};
pub use section::{Row, Section};
pub use shape::Shape;
pub use sheet::Sheet;
pub use style::{Style, Stylesheet};
