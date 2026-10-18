//! The crate error type.

use std::fmt;

/// A geometry error.
///
/// Every fallible operation in this crate returns `Result<T, Error>`. The
/// crate never panics on input (AGENTS.md section 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The unit does not measure the expected kind of quantity.
    ///
    /// For example, [`crate::Unit::Degrees`] where a length is expected.
    WrongUnit,
    /// The transform is not invertible: its linear part is singular
    /// (determinant zero, or non-finite).
    Singular,
    /// The operation is not implemented yet (a Phase 5 interface stub,
    /// task P5-01).
    NotImplemented,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::WrongUnit => write!(f, "unit does not measure the expected kind of quantity"),
            Error::Singular => write!(f, "transform is singular and cannot be inverted"),
            Error::NotImplemented => write!(f, "not implemented yet"),
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
            Error::WrongUnit.to_string(),
            "unit does not measure the expected kind of quantity"
        );
        assert_eq!(
            Error::Singular.to_string(),
            "transform is singular and cannot be inverted"
        );
        assert_eq!(Error::NotImplemented.to_string(), "not implemented yet");
    }

    #[test]
    fn is_a_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(Error::WrongUnit);
        assert!(e.to_string().contains("unit"));
    }

    #[test]
    fn errors_are_distinct() {
        assert_ne!(Error::WrongUnit, Error::Singular);
    }
}
