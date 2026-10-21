//! The ShapeSheet function table (ADR 0053).
//!
//! One entry per function the evaluator knows: the name as Visio spells
//! it, how many arguments it takes and what kind of function it is. The
//! table is **hand-written** in this module (ADR 0053); its rows are the
//! decided set of the KICKOFF core plus the helpers the corpus needs.
//!
//! A name the table does not hold keeps the cell's cached value and
//! records a structured warning (ADR 0053): an unknown function never
//! fails the document. Task P6-04 fills [`FUNCTIONS`] in and implements
//! [`lookup_function`].

use crate::Error;

/// One row of the function table (ADR 0053).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionDef {
    /// The function name as Visio spells it (`IF`, `GUARD`, `SETF`).
    pub name: &'static str,
    /// The fewest arguments the function takes.
    pub min_args: u8,
    /// The most arguments it takes, or `None` for a variadic function.
    pub max_args: Option<u8>,
    /// What kind of function it is.
    pub kind: FunctionKind,
}

/// What kind of function a [`FunctionDef`] is (ADR 0053).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FunctionKind {
    /// Arithmetic (`MIN`, `ROUND`, `SUM`).
    Arithmetic,
    /// Trigonometry (`SIN`, `ATAN2`).
    Trigonometry,
    /// Logic and control flow (`IF`, `AND`, `NOT`).
    Logic,
    /// Text (`STRSAME`, `SHAPETEXT`).
    Text,
    /// Colour (`RGB`, `LUM`, `TINT`).
    Color,
    /// Reference and lookup (`INDEX`, `LOOKUP`, `SHEETREF`).
    Reference,
    /// Value guards and assignment (`GUARD`, `SETF`, `SETATREF`).
    Guard,
    /// An action with no value (`RUNADDON`, `OPENTEXTWIN`).
    Action,
    /// Reserved for Visio's own use (`_MOD`, `_XFTRIGGER`).
    Internal,
}

/// The function table (ADR 0053). Empty until task P6-04 fills it in.
pub const FUNCTIONS: &[FunctionDef] = &[];

/// Looks a function up by name (case-insensitive per ADR 0053).
///
/// The stub returns [`Error::NotImplemented`] until task P6-04 lands.
pub fn lookup_function(name: &str) -> Result<Option<&'static FunctionDef>, Error> {
    let _ = name;
    Err(Error::NotImplemented("lookup_function"))
}

/// True when the evaluator implements the function (ADR 0053).
///
/// The stub returns `false` until task P6-04 lands.
pub fn is_supported(name: &str) -> bool {
    let _ = name;
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_function_is_a_stub() {
        assert_eq!(
            lookup_function("IF"),
            Err(Error::NotImplemented("lookup_function"))
        );
    }

    #[test]
    fn is_supported_is_false_while_the_table_is_empty() {
        assert!(!is_supported("IF"));
        assert!(!is_supported("GUARD"));
    }

    #[test]
    fn the_table_starts_empty() {
        assert!(FUNCTIONS.is_empty());
    }

    #[test]
    fn a_function_def_holds_its_shape() {
        let d = FunctionDef {
            name: "IF",
            min_args: 2,
            max_args: Some(3),
            kind: FunctionKind::Logic,
        };
        assert_eq!(d.name, "IF");
        assert_eq!(d.min_args, 2);
        assert_eq!(d.max_args, Some(3));
        assert_eq!(d.kind, FunctionKind::Logic);
    }

    #[test]
    fn every_function_kind_exists() {
        let kinds = [
            FunctionKind::Arithmetic,
            FunctionKind::Trigonometry,
            FunctionKind::Logic,
            FunctionKind::Text,
            FunctionKind::Color,
            FunctionKind::Reference,
            FunctionKind::Guard,
            FunctionKind::Action,
            FunctionKind::Internal,
        ];
        assert_eq!(kinds.len(), 9);
    }
}
