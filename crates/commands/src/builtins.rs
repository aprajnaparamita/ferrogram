//! The built-in registry: one function that registers every implemented
//! command (ADR 0028).
//!
//! The desktop app, the web app, the CLI and the parity generator
//! (ADR 0037) all build their registry here, so "which commands are
//! implemented" has exactly one answer. Implementers add their handlers
//! to this module's `registry` function when their command lands.
//!
//! Alpha status (task P4-01): returns an empty registry until P4-02
//! registers the first commands.

use crate::error::RegistryError;
use crate::registry::Registry;

/// Builds the registry with every implemented command registered over
/// its `commands.yaml` stub (ADR 0028).
///
/// Returns [`RegistryError::Duplicate`] when two handlers claim one id;
/// that is a build bug, not input-dependent behaviour.
pub fn registry() -> Result<Registry, RegistryError> {
    // P4-02 starts registering commands here; until then the registry
    // is empty and every dispatch reports the stub error.
    Ok(Registry::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_builtin_registry_starts_empty() {
        let registry = registry().expect("registry");
        assert!(registry.is_empty());
    }
}
