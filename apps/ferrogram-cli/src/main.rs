//! Ferrogram command-line tool: `inspect`, `render`, `export` (ADR 0026).
//!
//! `run-command` arrives with the Phase 4 command registry. Part of
//! Ferrogram. Alpha.

#![forbid(unsafe_code)]

use ferrogram_cli::cli;

fn main() {
    match cli::main_from(std::env::args_os().skip(1)) {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(cli::exit::RENDER);
        }
    }
}
