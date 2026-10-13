//! OPC package and XML reader/writer for `.vsdx`, `.vssx` and `.vstx`.
//!
//! Layer L2 (ADR 0002): the crate turns a package into a
//! [`Document`](ferrogram_model::Document) and back, preserving everything it
//! does not model (ADR 0012). MS-VSDX is the source of truth. The package
//! layer (`package`) is pure OPC: zip container, `[Content_Types].xml`,
//! relationships and part access, with hard caps against zip bombs and huge
//! inputs (ADR 0017).
//!
//! Part of Ferrogram. Alpha: function bodies here are `Err` stubs until tasks
//! P2-05 to P2-07 implement them (task P2-01 defines the signatures).

#![forbid(unsafe_code)]
#![deny(clippy::indexing_slicing)]

mod error;
mod package;
mod read;
mod write;

pub use error::Error;
pub use package::{
    ContentType, ContentTypeDecl, ContentTypeKind, Limits, Package, Part, PartData, PartName,
    Relationship, RelationshipId, TargetMode, read_package, write_package,
};
pub use read::{read_document, to_model};
pub use write::{from_model, write_document};
