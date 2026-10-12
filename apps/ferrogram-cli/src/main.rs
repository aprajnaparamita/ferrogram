//! Ferrogram command-line tool: inspect, render, export, run-command.
//!
//! Alpha placeholder; the real implementation lands in a later phase.

#![forbid(unsafe_code)]

fn main() {
    println!("{}", banner());
}

/// Startup message shown until the real app exists.
fn banner() -> String {
    format!(
        "ferrogram-cli {} (alpha): no subcommands yet",
        env!("CARGO_PKG_VERSION")
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn banner_names_version() {
        assert!(super::banner().contains(env!("CARGO_PKG_VERSION")));
    }
}
