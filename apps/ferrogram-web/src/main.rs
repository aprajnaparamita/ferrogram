//! Ferrogram web app (wasm, built with trunk) (ADR 0030).
//!
//! The wasm twin of `apps/ferrogram`: same shell, same registry, with
//! file upload and download instead of native dialogs (ADR 0036).
//!
//! Alpha status: the eframe web runner lands in task P4-03; `run` is a
//! stub.

#![forbid(unsafe_code)]

use std::fmt;

/// A startup failure (one line; never a panic, AGENTS.md section 6).
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

/// Builds the registry, loads both layouts and starts the eframe web
/// runner (ADR 0030, ADR 0031).
///
/// Not implemented yet (P4-03).
fn run() -> Result<(), AppError> {
    Err(AppError {
        message: "the web app is not implemented yet (P4-03)".to_owned(),
    })
}

/// Entry point of the wasm build.
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
