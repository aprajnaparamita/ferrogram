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
    /// The render options cannot be right for any scene: a `scale` that
    /// is not finite, not positive, or above the device extent cap
    /// (ADR 0025 amendment, P3-16).
    InvalidOptions,
    /// The operation is not implemented yet (a Phase 3 stub).
    NotImplemented,
    /// A scene node path does not resolve to a node in the scene it is
    /// used with: the path is stale or from another scene (ADR 0041).
    InvalidNode,
    /// The committed visual style data does not parse (ADR 0023). This
    /// is a build bug in `crates/render/data/visual-styles.yaml`, not
    /// file input.
    InvalidStyleData,
    /// The model rejected a document reference (a stale shape or page id).
    /// The builder only walks ids it got from the document, so this is a
    /// caller bug (ADR 0021: a resolving error is not file data).
    Model(larrygram_model::Error),
    /// The rendered raster could not be encoded (PNG encoding failed).
    Encode,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::TooLarge => write!(f, "output is larger than the allowed caps"),
            Error::InvalidOptions => write!(f, "the render options are not valid"),
            Error::NotImplemented => write!(f, "not implemented yet"),
            Error::InvalidNode => write!(f, "the scene node path does not resolve"),
            Error::InvalidStyleData => write!(f, "committed visual style data is not valid"),
            Error::Model(error) => write!(f, "model error: {error}"),
            Error::Encode => write!(f, "the output could not be encoded"),
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
        assert_eq!(
            Error::InvalidOptions.to_string(),
            "the render options are not valid"
        );
        assert_eq!(Error::NotImplemented.to_string(), "not implemented yet");
        assert_eq!(
            Error::InvalidNode.to_string(),
            "the scene node path does not resolve"
        );
        assert_eq!(
            Error::InvalidStyleData.to_string(),
            "committed visual style data is not valid"
        );
        assert_eq!(
            Error::Model(larrygram_model::Error::UnknownShape(
                larrygram_model::ShapeId { index: 1 }
            ))
            .to_string(),
            "model error: unknown shape id 1"
        );
        assert_eq!(Error::Encode.to_string(), "the output could not be encoded");
    }
}
