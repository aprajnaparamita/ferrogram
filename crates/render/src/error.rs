//! The crate error type.

use std::fmt;

/// A rendering error.
///
/// Every fallible operation in this crate returns `Result<T, Error>`. The
/// crate never panics on input (AGENTS.md section 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// An output dimension or byte count exceeds the caps for
    /// input-derived values (AGENTS.md section 6).
    TooLarge,
    /// The operation is not implemented yet (a Phase 3 stub).
    NotImplemented,
    /// The committed visual style data does not parse (ADR 0023). This
    /// is a build bug in `crates/render/data/visual-styles.yaml`, not
    /// file input.
    InvalidStyleData,
    /// The model rejected a document reference (a stale shape or page id).
    /// The builder only walks ids it got from the document, so this is a
    /// caller bug (ADR 0021: a resolving error is not file data).
    Model(ferrogram_model::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::TooLarge => write!(f, "output is larger than the allowed caps"),
            Error::NotImplemented => write!(f, "not implemented yet"),
            Error::InvalidStyleData => write!(f, "committed visual style data is not valid"),
            Error::Model(error) => write!(f, "model error: {error}"),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_describes_the_error() {
        assert_eq!(
            Error::TooLarge.to_string(),
            "output is larger than the allowed caps"
        );
        assert_eq!(Error::NotImplemented.to_string(), "not implemented yet");
        assert_eq!(
            Error::InvalidStyleData.to_string(),
            "committed visual style data is not valid"
        );
        assert_eq!(
            Error::Model(ferrogram_model::Error::UnknownShape(
                ferrogram_model::ShapeId { index: 1 }
            ))
            .to_string(),
            "model error: unknown shape id 1"
        );
    }
}
