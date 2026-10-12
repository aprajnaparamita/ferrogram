//! Shared helpers: workspace root, running commands, cargo metadata.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Result type used across xtask; errors are human-readable messages.
pub type Result<T> = std::result::Result<T, String>;

/// The workspace root (parent of the xtask crate).
pub fn root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.parent().map(Path::to_path_buf).unwrap_or(manifest)
}

/// The cargo binary that invoked us, so toolchain overrides carry through.
pub fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

/// Runs `cmd`, inheriting stdio. Returns an error naming the command on failure.
pub fn run(cmd: &mut Command) -> Result<()> {
    let shown = format!("{cmd:?}");
    let status = cmd
        .current_dir(root())
        .status()
        .map_err(|e| format!("failed to start {shown}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{shown} exited with {status}"))
    }
}

/// Runs `cmd` and captures stdout as a string.
pub fn output(cmd: &mut Command) -> Result<String> {
    let shown = format!("{cmd:?}");
    let out = cmd
        .current_dir(root())
        .output()
        .map_err(|e| format!("failed to start {shown}: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "{shown} exited with {}:\n{}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    String::from_utf8(out.stdout).map_err(|e| format!("{shown} printed non-UTF-8: {e}"))
}

/// Returns true if `cargo <sub> --version` works, i.e. the subcommand is installed.
pub fn has_cargo_subcommand(sub: &str) -> bool {
    Command::new(cargo())
        .args([sub, "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// A workspace package from `cargo metadata`.
#[derive(Debug, Clone)]
pub struct Package {
    /// Package name.
    pub name: String,
    /// Directory containing Cargo.toml.
    pub dir: PathBuf,
    /// Names of direct dependencies (normal, dev and build).
    pub deps: Vec<String>,
}

/// Lists workspace packages via `cargo metadata --no-deps`.
pub fn workspace_packages() -> Result<Vec<Package>> {
    let json =
        output(Command::new(cargo()).args(["metadata", "--format-version", "1", "--no-deps"]))?;
    parse_metadata(&json)
}

/// Parses the JSON printed by `cargo metadata --no-deps`.
pub fn parse_metadata(json: &str) -> Result<Vec<Package>> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("bad cargo metadata JSON: {e}"))?;
    let pkgs = v
        .get("packages")
        .and_then(|p| p.as_array())
        .ok_or("cargo metadata: no `packages` array")?;
    let mut out = Vec::new();
    for p in pkgs {
        let name = p
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or("cargo metadata: package without name")?;
        let manifest = p
            .get("manifest_path")
            .and_then(|m| m.as_str())
            .ok_or("cargo metadata: package without manifest_path")?;
        let dir = Path::new(manifest)
            .parent()
            .map(Path::to_path_buf)
            .ok_or("cargo metadata: manifest_path has no parent")?;
        let deps = p
            .get("dependencies")
            .and_then(|d| d.as_array())
            .map(|ds| {
                ds.iter()
                    .filter_map(|d| d.get("name").and_then(|n| n.as_str()))
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        out.push(Package {
            name: name.to_owned(),
            dir,
            deps,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_metadata() {
        let json = r#"{"packages":[{"name":"a","manifest_path":"/w/crates/a/Cargo.toml",
            "dependencies":[{"name":"b"},{"name":"serde"}]}]}"#;
        let pkgs = parse_metadata(json).unwrap();
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].name, "a");
        assert_eq!(pkgs[0].dir, PathBuf::from("/w/crates/a"));
        assert_eq!(pkgs[0].deps, vec!["b", "serde"]);
    }

    #[test]
    fn rejects_bad_metadata() {
        assert!(parse_metadata("not json").is_err());
        assert!(parse_metadata("{}").is_err());
        assert!(parse_metadata(r#"{"packages":[{}]}"#).is_err());
    }
}
