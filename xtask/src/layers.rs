//! Crate layering check (ADR 0002).
//!
//! A workspace crate may depend only on workspace crates in strictly lower
//! layers. Crates below the UI layer may not depend on GUI toolkits.

use crate::util::{self, Package, Result};

/// Layer of each workspace package. Unlisted packages fail the check so the
/// table stays complete.
const LAYERS: &[(&str, u8)] = &[
    ("ferrogram-geom", 0),
    ("ferrogram-ui-spec", 0),
    ("ferrogram-model", 1),
    ("ferrogram-text", 1),
    ("ferrogram-vsdx", 2),
    ("ferrogram-vsd-binary", 2),
    ("ferrogram-shapesheet", 2),
    ("ferrogram-render", 3),
    ("ferrogram-commands", 4),
    ("ferrogram-control", 5),
    ("ferrogram-ui", 6),
    ("ferrogram", 7),
    ("ferrogram-web", 7),
    ("ferrogram-cli", 7),
    ("xtask", 9),
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
                && p.name != "xtask"
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
        let pkgs = [pkg("ferrogram-model", &["ferrogram-geom", "serde"])];
        assert!(check(&pkgs).is_empty());
    }

    #[test]
    fn rejects_upward_and_same_layer_deps() {
        let pkgs = [
            pkg("ferrogram-geom", &["ferrogram-model"]),
            pkg("ferrogram-vsdx", &["ferrogram-shapesheet"]),
        ];
        assert_eq!(check(&pkgs).len(), 2);
    }

    #[test]
    fn rejects_gui_below_ui_layer() {
        let pkgs = [
            pkg("ferrogram-render", &["egui"]),
            pkg("ferrogram-ui", &["egui"]),
        ];
        let errs = check(&pkgs);
        assert_eq!(errs.len(), 1);
        assert!(errs[0].contains("ferrogram-render"));
    }

    #[test]
    fn rejects_unknown_crate() {
        assert_eq!(check(&[pkg("mystery", &[])]).len(), 1);
    }
}
