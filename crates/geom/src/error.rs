//! The crate error type.

use std::fmt;

/// A geometry error.
///
/// Every fallible operation in this crate returns `Result<T, Error>`. The
/// crate never panics on input (AGENTS.md section 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The operation is a planning stub and is not implemented yet.
    ///
    /// The payload names the operation, e.g. `"Unit::to_canonical"`.
    NotImplemented(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotImplemented(op) => write!(f, "not implemented: {op}"),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_the_operation() {
        let e = Error::NotImplemented("Unit::to_canonical");
        assert_eq!(e.to_string(), "not implemented: Unit::to_canonical");
    }

    #[test]
    fn is_a_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(Error::NotImplemented("x"));
        assert!(e.to_string().contains("not implemented"));
    }
}
