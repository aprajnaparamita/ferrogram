//! Coverage step: runs `cargo llvm-cov`, aggregates line coverage per crate,
//! and checks it against `coverage.toml` thresholds and the ratchet.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::util::{self, Package, Result};

/// Line counts for one crate.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Lines {
    /// Instrumented lines.
    pub count: u64,
    /// Covered lines.
    pub covered: u64,
}

impl Lines {
    /// Percent covered; `None` when the crate has no instrumented lines.
    pub fn percent(self) -> Option<f64> {
        if self.count == 0 {
            None
        } else {
            Some(self.covered as f64 * 100.0 / self.count as f64)
        }
    }
}

/// Thresholds and ratchet values from `coverage.toml`.
#[derive(Debug, Default)]
pub struct Config {
    /// Minimum percent per crate.
    pub thresholds: BTreeMap<String, f64>,
    /// Percent recorded at the previous phase end.
    pub ratchet: BTreeMap<String, f64>,
}

const CONFIG_FILE: &str = "coverage.toml";
/// Tolerance for float noise when comparing to the ratchet.
const EPSILON: f64 = 0.05;

/// Runs coverage, prints a table, and checks it. Optionally rewrites the ratchet.
pub fn run(update_ratchet: bool) -> Result<()> {
    if !util::has_cargo_subcommand("llvm-cov") {
        return Err(
            "cargo-llvm-cov is not installed: `cargo install --locked cargo-llvm-cov`".into(),
        );
    }
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| util::root().join("target"));
    let report = target.join("llvm-cov-summary.json");
    util::run(
        Command::new(util::cargo())
            .args([
                "llvm-cov",
                "--workspace",
                "--all-features",
                "--json",
                "--summary-only",
                "--output-path",
            ])
            .arg(&report),
    )?;
    let json = std::fs::read_to_string(&report)
        .map_err(|e| format!("cannot read {}: {e}", report.display()))?;
    let pkgs = util::workspace_packages()?;
    let per_crate = aggregate(&json, &pkgs)?;
    let config_path = util::root().join(CONFIG_FILE);
    let config_text = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("cannot read {CONFIG_FILE}: {e}"))?;
    let config = parse_config(&config_text)?;

    print_table(&per_crate, &config);
    let failures = check(&per_crate, &config);

    if update_ratchet {
        if !failures.is_empty() {
            return Err("refusing to update the ratchet while coverage fails".into());
        }
        let updated = rewrite_ratchet(&config_text, &per_crate);
        std::fs::write(&config_path, updated)
            .map_err(|e| format!("cannot write {CONFIG_FILE}: {e}"))?;
        println!("coverage: ratchet updated in {CONFIG_FILE}");
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "coverage below target:\n  {}",
            failures.join("\n  ")
        ))
    }
}

/// Sums llvm-cov per-file line counts into per-package totals.
pub fn aggregate(json: &str, pkgs: &[Package]) -> Result<BTreeMap<String, Lines>> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("bad llvm-cov JSON: {e}"))?;
    let files = v
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|d| d.first())
        .and_then(|d| d.get("files"))
        .and_then(|f| f.as_array())
        .ok_or("llvm-cov JSON: missing data[0].files")?;
    let mut out: BTreeMap<String, Lines> = pkgs
        .iter()
        .map(|p| (p.name.clone(), Lines::default()))
        .collect();
    for f in files {
        let Some(name) = f.get("filename").and_then(|n| n.as_str()) else {
            continue;
        };
        let Some(pkg) = owner(Path::new(name), pkgs) else {
            continue;
        };
        let lines = f.get("summary").and_then(|s| s.get("lines"));
        let count = lines
            .and_then(|l| l.get("count"))
            .and_then(|c| c.as_u64())
            .unwrap_or(0);
        let covered = lines
            .and_then(|l| l.get("covered"))
            .and_then(|c| c.as_u64())
            .unwrap_or(0);
        let entry = out.entry(pkg.name.clone()).or_default();
        entry.count += count;
        entry.covered += covered;
    }
    Ok(out)
}

/// The package whose directory is the longest prefix of `file`.
fn owner<'a>(file: &Path, pkgs: &'a [Package]) -> Option<&'a Package> {
    pkgs.iter()
        .filter(|p| file.starts_with(&p.dir))
        .max_by_key(|p| p.dir.components().count())
}

/// Parses `coverage.toml`.
pub fn parse_config(text: &str) -> Result<Config> {
    let table: toml::Table = text
        .parse()
        .map_err(|e| format!("bad {CONFIG_FILE}: {e}"))?;
    let section = |key: &str| -> Result<BTreeMap<String, f64>> {
        let mut out = BTreeMap::new();
        if let Some(t) = table.get(key).and_then(|t| t.as_table()) {
            for (k, v) in t {
                let n = v
                    .as_float()
                    .or_else(|| v.as_integer().map(|i| i as f64))
                    .ok_or_else(|| format!("{CONFIG_FILE}: [{key}].{k} is not a number"))?;
                out.insert(k.clone(), n);
            }
        }
        Ok(out)
    };
    Ok(Config {
        thresholds: section("thresholds")?,
        ratchet: section("ratchet")?,
    })
}

/// Returns one message per crate below its threshold or ratchet.
pub fn check(per_crate: &BTreeMap<String, Lines>, config: &Config) -> Vec<String> {
    let mut failures = Vec::new();
    for (name, lines) in per_crate {
        let Some(pct) = lines.percent() else { continue };
        if let Some(min) = config.thresholds.get(name)
            && pct + EPSILON < *min
        {
            failures.push(format!("{name}: {pct:.1}% < threshold {min:.1}%"));
        }
        if let Some(prev) = config.ratchet.get(name)
            && pct + EPSILON < *prev
        {
            failures.push(format!(
                "{name}: {pct:.1}% < previous phase {prev:.1}% (ratchet)"
            ));
        }
    }
    failures
}

fn print_table(per_crate: &BTreeMap<String, Lines>, config: &Config) {
    println!(
        "{:<24} {:>8} {:>8} {:>8} {:>10} {:>9}",
        "crate", "lines", "covered", "pct", "threshold", "ratchet"
    );
    for (name, l) in per_crate {
        let pct = l.percent().map_or("n/a".to_owned(), |p| format!("{p:.1}"));
        let thr = config
            .thresholds
            .get(name)
            .map_or("-".to_owned(), |t| format!("{t:.1}"));
        let rat = config
            .ratchet
            .get(name)
            .map_or("-".to_owned(), |r| format!("{r:.1}"));
        println!(
            "{name:<24} {:>8} {:>8} {pct:>8} {thr:>10} {rat:>9}",
            l.count, l.covered
        );
    }
}

/// Replaces everything after the `[ratchet]` header with current values,
/// rounded down to one decimal so the ratchet never exceeds what was measured.
pub fn rewrite_ratchet(config_text: &str, per_crate: &BTreeMap<String, Lines>) -> String {
    let mut out = String::new();
    for line in config_text.lines() {
        if line.trim() == "[ratchet]" {
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.push_str("[ratchet]\n");
    for (name, l) in per_crate {
        if let Some(p) = l.percent() {
            let floored = (p * 10.0).floor() / 10.0;
            out.push_str(&format!("{name} = {floored:.1}\n"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkgs() -> Vec<Package> {
        vec![
            Package {
                name: "a".into(),
                dir: "/w/crates/a".into(),
                deps: vec![],
            },
            Package {
                name: "a-sub".into(),
                dir: "/w/crates/a/sub".into(),
                deps: vec![],
            },
            Package {
                name: "empty".into(),
                dir: "/w/crates/empty".into(),
                deps: vec![],
            },
        ]
    }

    const REPORT: &str = r#"{"data":[{"files":[
        {"filename":"/w/crates/a/src/lib.rs","summary":{"lines":{"count":10,"covered":9}}},
        {"filename":"/w/crates/a/src/x.rs","summary":{"lines":{"count":10,"covered":7}}},
        {"filename":"/w/crates/a/sub/src/lib.rs","summary":{"lines":{"count":4,"covered":1}}},
        {"filename":"/elsewhere/dep.rs","summary":{"lines":{"count":100,"covered":0}}}
    ]}]}"#;

    #[test]
    fn aggregates_by_longest_prefix() {
        let m = aggregate(REPORT, &pkgs()).unwrap();
        assert_eq!(
            m["a"],
            Lines {
                count: 20,
                covered: 16
            }
        );
        assert_eq!(
            m["a-sub"],
            Lines {
                count: 4,
                covered: 1
            }
        );
        assert_eq!(m["empty"].percent(), None);
    }

    #[test]
    fn rejects_malformed_report() {
        assert!(aggregate("{}", &pkgs()).is_err());
        assert!(aggregate("nope", &pkgs()).is_err());
    }

    #[test]
    fn checks_thresholds_and_ratchet() {
        let m = aggregate(REPORT, &pkgs()).unwrap();
        let cfg =
            parse_config("[thresholds]\na = 85\nempty = 85\n[ratchet]\na-sub = 30.0\n").unwrap();
        let f = check(&m, &cfg);
        assert_eq!(f.len(), 2, "{f:?}");
        assert!(f.iter().any(|s| s.starts_with("a: 80.0%")));
        assert!(f.iter().any(|s| s.contains("ratchet")));
    }

    #[test]
    fn rejects_non_numeric_config() {
        assert!(parse_config("[thresholds]\na = \"x\"\n").is_err());
        assert!(parse_config("not = = toml").is_err());
    }

    #[test]
    fn rewrites_ratchet_section() {
        let m = aggregate(REPORT, &pkgs()).unwrap();
        let text = "# c\n[thresholds]\na = 1\n\n[ratchet]\nold = 5.0\n";
        let out = rewrite_ratchet(text, &m);
        assert_eq!(
            out,
            "# c\n[thresholds]\na = 1\n\n[ratchet]\na = 80.0\na-sub = 25.0\n"
        );
        let cfg = parse_config(&out).unwrap();
        assert_eq!(cfg.ratchet.get("a"), Some(&80.0));
    }
}
