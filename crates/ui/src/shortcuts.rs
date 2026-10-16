//! Keyboard shortcut dispatch (ADR 0036): chord sequences from
//! `ui-spec/shortcuts.yaml`, scoped by focus, dispatched as [`Intent`]s.
//!
//! Owned by task P4-08. Alpha status (task P4-01): the interface is
//! defined as stubs.

use egui::Context;
use ferrogram_commands::AppCtx;
use ferrogram_ui_spec::{ShortcutScope, Shortcuts};

use crate::{Intent, UiError};

/// One chord: modifiers plus one key, as `shortcuts.yaml` token strings
/// (`Ctrl`, `Shift`, `Alt`, `A`, `F1`, ...; ADR 0036).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyChord {
    /// The key token (last token of the chord).
    pub key: String,
    /// The modifier tokens, in `shortcuts.yaml` order.
    pub modifiers: Vec<String>,
}

impl KeyChord {
    /// A chord from its key and modifiers.
    pub fn new(key: impl Into<String>, modifiers: Vec<String>) -> Self {
        Self {
            key: key.into(),
            modifiers,
        }
    }
}

/// The shortcut map: typed chords to command intents (ADR 0036).
///
/// Multi-chord sequences are stateful: `resolve` records the chords
/// typed so far in `pending` and resets on a mismatch (P4-08).
pub struct ShortcutMap {
    entries: usize,
    pending: Vec<KeyChord>,
}

impl ShortcutMap {
    /// Builds the map from `ui-spec/shortcuts.yaml` (ADR 0007).
    ///
    /// Not implemented yet (P4-08).
    pub fn from_shortcuts(shortcuts: &Shortcuts) -> Result<ShortcutMap, UiError> {
        let _ = shortcuts;
        Err(UiError::NotImplemented {
            what: "ShortcutMap::from_shortcuts",
        })
    }

    /// Matches typed chords against the map; `Some` when a sequence
    /// completes (ADR 0036).
    ///
    /// Not implemented yet (P4-08).
    pub fn resolve(&mut self, chords: &[KeyChord]) -> Result<Option<Intent>, UiError> {
        let _ = chords;
        Err(UiError::NotImplemented {
            what: "ShortcutMap::resolve",
        })
    }

    /// Whether a multi-chord sequence is half-typed (status bar hint).
    pub fn pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Number of shortcut sequences in the map.
    pub fn len(&self) -> usize {
        self.entries
    }

    /// True when the map holds no sequences.
    pub fn is_empty(&self) -> bool {
        self.entries == 0
    }
}

/// Reads egui input, resolves it through `map` and returns the intent
/// for the given scope (ADR 0036).
///
/// Not implemented yet (P4-08).
pub fn dispatch(
    egui_ctx: &Context,
    map: &mut ShortcutMap,
    app: &AppCtx,
    scope: ShortcutScope,
) -> Result<Option<Intent>, UiError> {
    let _ = (egui_ctx, map, app, scope);
    Err(UiError::NotImplemented {
        what: "shortcuts::dispatch",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chord_holds_key_and_modifiers() {
        let chord = KeyChord::new("Z", vec!["Ctrl".into()]);
        assert_eq!(chord.key, "Z");
        assert_eq!(chord.modifiers, vec!["Ctrl".to_owned()]);
    }

    #[test]
    fn the_map_is_a_stub() {
        let mut map = match ShortcutMap::from_shortcuts(&Shortcuts::default()) {
            Ok(map) => map,
            Err(error) => {
                assert!(error.to_string().contains("not implemented"));
                ShortcutMap {
                    entries: 0,
                    pending: Vec::new(),
                }
            }
        };
        assert!(!map.pending());
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
        assert!(map.resolve(&[]).is_err());
    }

    #[test]
    fn dispatch_is_a_stub() {
        let mut map = ShortcutMap {
            entries: 0,
            pending: Vec::new(),
        };
        let ctx = Context::default();
        let app = AppCtx::new(ferrogram_commands::Settings::default());
        let outcome = dispatch(&ctx, &mut map, &app, ShortcutScope::Global);
        assert!(outcome.is_err());
    }
}
