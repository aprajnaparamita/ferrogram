//! Ferrogram desktop app (eframe/egui) (ADR 0030).
//!
//! The thin shell over `crates/ui` (L7, ADR 0002): it builds the
//! [`ferrogram_commands::Registry`], loads the ui-spec layouts and runs
//! the eframe event loop around `ferrogram_ui::shell::Shell`.
//!
//! Alpha status: the shell lands in task P4-03; `run` is a stub.

#![forbid(unsafe_code)]

use std::fmt;

/// A startup failure (one line; the app never panics, AGENTS.md
/// section 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppError {
    /// The formatted cause.
    pub message: String,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for AppError {}

/// Builds the registry, loads both layouts and runs the eframe loop
/// (ADR 0030, ADR 0031).
///
/// Not implemented yet (P4-03).
fn run() -> Result<(), AppError> {
    Err(AppError {
        message: "the desktop app is not implemented yet (P4-03)".to_owned(),
    })
}

/// Entry point: prints `error: <message>` and exits non-zero on
/// failure (ADR 0026 style).
fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_is_a_stub() {
        let error = run().expect_err("stub");
        assert!(error.to_string().contains("not implemented"));
    }

    #[test]
    fn main_reports_the_stub_error() {
        main();
    }
}
