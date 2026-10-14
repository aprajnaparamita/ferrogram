//! The crate error type.

use std::fmt;

/// A package or mapping error.
///
/// The crate never panics on input (AGENTS.md section 6): malformed input
/// yields an error, never a crash.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// An input exceeded one of the [`crate::Limits`] caps.
    LimitExceeded {
        /// The name of the cap, e.g. `"max_part_uncompressed"`.
        limit: &'static str,
        /// The offending value.
        value: u64,
        /// The cap.
        max: u64,
    },
    /// A required part is missing from the package.
    MissingPart(String),
    /// A relationship is missing, dangling or unparseable.
    BadRelationship(String),
    /// The container or a part is malformed (bad zip, bad XML, bad content
    /// types). The message never carries file content.
    InvalidPackage(String),
    /// A [`ferrogram_model`] operation failed while mapping.
    Model(ferrogram_model::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::LimitExceeded { limit, value, max } => {
                write!(f, "limit `{limit}` exceeded: {value} > {max}")
            }
            Error::MissingPart(part) => write!(f, "missing part: {part}"),
            Error::BadRelationship(why) => write!(f, "bad relationship: {why}"),
            Error::InvalidPackage(why) => write!(f, "invalid package: {why}"),
            Error::Model(e) => write!(f, "model error: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<ferrogram_model::Error> for Error {
    fn from(e: ferrogram_model::Error) -> Self {
        Error::Model(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_covers_every_variant() {
        assert_eq!(
            Error::LimitExceeded {
                limit: "max_entries",
                value: 9,
                max: 8
            }
            .to_string(),
            "limit `max_entries` exceeded: 9 > 8"
        );
        assert_eq!(
            Error::MissingPart("/visio/document.xml".to_owned()).to_string(),
            "missing part: /visio/document.xml"
        );
        assert_eq!(
            Error::BadRelationship("dangling".to_owned()).to_string(),
            "bad relationship: dangling"
        );
        assert_eq!(
            Error::InvalidPackage("bad zip".to_owned()).to_string(),
            "invalid package: bad zip"
        );
        assert_eq!(
            Error::Model(ferrogram_model::Error::UnknownShape(
                ferrogram_model::ShapeId { index: 2 }
            ))
            .to_string(),
            "model error: unknown shape id 2"
        );
    }

    #[test]
    fn model_errors_convert() {
        let model: ferrogram_model::Error =
            ferrogram_model::Error::UnknownShape(ferrogram_model::ShapeId { index: 2 });
        let e: Error = model.into();
        assert_eq!(e, Error::Model(model));
    }
}
