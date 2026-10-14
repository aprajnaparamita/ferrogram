//! Ferrogram command-line library: commands, parsing and execution
//! (ADR 0026).
//!
//! The binary in `main.rs` is a thin shell over [`cli`]; tests and, in
//! Phase 8, the control channel call the library directly. Part of
//! Ferrogram. Alpha.

#![forbid(unsafe_code)]

pub mod cli;
