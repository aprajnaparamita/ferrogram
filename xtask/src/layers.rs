//! Crate layering check (ADR 0002).
//!
//! A workspace crate may depend only on workspace crates in strictly lower
//! layers. Crates below the UI layer may not depend on GUI toolkits.

use crate::util::{self, Package, Result};

/// Layer of each workspace package. Unlisted packages fail the check so the
/// table stays complete.
const LAYERS: &[(&str, u8)] = &[
    ("larrygram-geom", 0),
    ("larrygram-ui-spec", 0),
    ("larrygram-model", 1),
    ("larrygram-text", 1),
    ("larrygram-vsdx", 2),
    ("larrygram-vsd-binary", 2),
    ("larrygram-shapesheet", 2),
    ("larrygram-render", 3),
    ("larrygram-commands", 4),
    ("larrygram-control", 5),
    ("larrygram-ui", 6),
    ("larrygram", 7),
    ("larrygram-web", 7),
    ("larrygram-cli", 7),
    ("larrygram-xtask", 9),
];

/// First layer allowed to depend on GUI toolkits.
const UI_LAYER: u8 = 6;

/// GUI crates forbidden below [`UI_LAYER`].
const GUI_CRATES: &[&str] = &[
    "egui",
    "eframe",
    "winit",
    "rfd",
    "egui_kittest",
    "egui-wgpu",
];

fn layer_of(name: &str) -> Option<u8> {
    LAYERS.iter().find(|(n, _)| *n == name).map(|(_, l)| *l)
}

/// Runs the check against the current workspace.
pub fn run() -> Result<()> {
    let pkgs = util::workspace_packages()?;
    let errors = check(&pkgs);
    if errors.is_empty() {
        println!("layers: ok ({} crates)", pkgs.len());
        Ok(())
    } else {
        Err(format!("layering violations:\n  {}", errors.join("\n  ")))
    }
}

/// Returns one message per violation.
pub fn check(pkgs: &[Package]) -> Vec<String> {
    let mut errors = Vec::new();
    for p in pkgs {
        let Some(layer) = layer_of(&p.name) else {
            errors.push(format!(
                "{} has no layer; add it to LAYERS in xtask/src/layers.rs",
                p.name
            ));
            continue;
        };
        for d in &p.deps {
            if let Some(dl) = layer_of(d)
                && dl >= layer
                && p.name != "larrygram-xtask"
            {
                errors.push(format!(
                    "{} (L{layer}) depends on {d} (L{dl}); only lower layers allowed",
                    p.name
                ));
            }
            if layer < UI_LAYER && GUI_CRATES.contains(&d.as_str()) {
                errors.push(format!(
                    "{} (L{layer}) depends on GUI crate {d}; only L{UI_LAYER}+ may",
                    p.name
                ));
            }
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn pkg(name: &str, deps: &[&str]) -> Package {
        Package {
            name: name.into(),
            dir: PathBuf::new(),
            deps: deps.iter().map(|d| (*d).to_owned()).collect(),
        }
    }

    #[test]
    fn allows_downward_deps() {
        let pkgs = [pkg("larrygram-model", &["larrygram-geom", "serde"])];
        assert!(check(&pkgs).is_empty());
    }

    #[test]
    fn rejects_upward_and_same_layer_deps() {
        let pkgs = [
            pkg("larrygram-geom", &["larrygram-model"]),
            pkg("larrygram-vsdx", &["larrygram-shapesheet"]),
        ];
        assert_eq!(check(&pkgs).len(), 2);
    }

    #[test]
    fn rejects_gui_below_ui_layer() {
        let pkgs = [
            pkg("larrygram-render", &["egui"]),
            pkg("larrygram-ui", &["egui"]),
        ];
        let errs = check(&pkgs);
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("larrygram-render"));
    }

    #[test]
    fn rejects_unknown_crate() {
        assert_eq!(check(&[pkg("mystery", &[])]).len(), 1);
    }
}
