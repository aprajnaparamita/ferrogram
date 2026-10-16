//! Keyboard shortcut dispatch (ADR 0036): chord sequences from
//! `ui-spec/shortcuts.yaml`, scoped by focus, dispatched as [`Intent`]s.
//!
//! Owned by task P4-08; the conflict rule and the per-scope bindings
//! were amended for task P4-14 (ADR 0036 "Conflicts", amended at the
//! P4-08 merge): the conflict key is the chord sequence **and** the
//! scope, so one sequence may be bound in several scopes and each
//! binding fires when its scope is active. When more than one bound
//! scope is active, the most specific wins: `Text`, then `Selection`,
//! `Page`, `Ribbon`, `Window`, `Global`. Two entries with the same
//! sequence and scope: the first in file order wins and one warning is
//! recorded, except when both name the same command (an exact repeat
//! is dropped silently).
//!
//! A **chord** is one key plus its modifier tokens. Both sides are
//! normalized before matching: modifiers are deduplicated and sorted
//! `Ctrl`, `Shift`, `Alt`, and a shifted symbol key (`+`, `<`, `>`)
//! absorbs the `Shift` modifier (on most layouts those symbols are
//! typed with Shift, and `shortcuts.yaml` lists them inconsistently:
//! `Ctrl+Shift+<` but `Ctrl++`).
//!
//! Multi-chord sequences are stateful: [`ShortcutMap`] buffers the
//! chords typed so far and drops the buffer on a mismatch or after
//! [`SEQUENCE_TIMEOUT_SECONDS`] (the timeout is applied by
//! [`dispatch`], which runs once per frame with the egui frame time;
//! [`ShortcutMap::resolve`] alone is timeless).
//!
//! Focus rules ([`dispatch`]): `Global` and `Window` always match,
//! `Page` and `Ribbon` match when the shell reports that surface as
//! focused, `Text` matches only while an egui text edit has focus, and
//! `Selection` matches only with a non-empty selection (never in
//! Phase 4). While a text edit has focus, the chords without `Ctrl` or
//! `Alt` (typing, shift-typing, the caret keys) are never matched or
//! consumed: they pass straight through to egui. A chord with `Ctrl`
//! or `Alt` is matched even inside a text edit, and the most specific
//! active scope decides what it dispatches (P4-14: `Ctrl+C` resolves
//! in a text edit and `Ctrl+Shift+C` picks the `text` binding there).

use std::collections::BTreeMap;
use std::path::PathBuf;

use egui::{Context, Event, Key, Modifiers};
use ferrogram_commands::{AppCtx, CommandId, Invocation, Source};
use ferrogram_ui_spec::{LoadError, ShortcutScope, Shortcuts};

use crate::{Intent, UiError};

/// A multi-chord sequence expires this many seconds after the last
/// typed chord (ADR 0036).
pub const SEQUENCE_TIMEOUT_SECONDS: f64 = 2.0;

/// Pseudo-token for the arrow keys: `shortcuts.yaml` records the
/// held-key note "Arrow Keys" (P1-04), not a direction, so one token
/// covers all four arrow keys.
const ARROWS_TOKEN: &str = "Arrow Keys";

/// Modifier tokens, in canonical order.
const MODIFIER_TOKENS: [&str; 3] = ["Ctrl", "Shift", "Alt"];

/// Key tokens that produce their character with Shift on most
/// layouts. A chord whose key is one of these drops its `Shift`
/// modifier on both sides of the match.
const SHIFTED_SYMBOL_TOKENS: [&str; 3] = ["+", "<", ">"];

/// One chord: modifiers plus one key, as `shortcuts.yaml` token strings
/// (`Ctrl`, `Shift`, `Alt`, `A`, `F1`, ...; ADR 0036).
///
/// The fields are normalized: `modifiers` holds no duplicates and is
/// sorted `Ctrl`, `Shift`, `Alt`, and a shifted symbol key (`+`, `<`,
/// `>`) carries no `Shift` modifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyChord {
    /// The key token (the only non-modifier token of the chord).
    pub key: String,
    /// The modifier tokens, in canonical order (`Ctrl`, `Shift`, `Alt`).
    pub modifiers: Vec<String>,
}

impl KeyChord {
    /// A chord from its key and modifiers, normalized.
    pub fn new(key: impl Into<String>, modifiers: Vec<String>) -> Self {
        normalize_chord(key.into(), modifiers)
    }
}

/// A shortcut entry: what a completed sequence dispatches and where it
/// applies (ADR 0036).
#[derive(Debug, Clone)]
struct Entry {
    /// The command to dispatch.
    command: CommandId,
    /// The scope the shortcut applies in.
    scope: ShortcutScope,
}

impl Entry {
    /// The intent for this entry (ADR 0028 `Source::Shortcut`).
    fn into_intent(self) -> Intent {
        Intent::new(self.command, Invocation::empty(Source::Shortcut))
    }
}

/// One node of the chord-sequence trie.
#[derive(Debug, Default)]
struct Node {
    /// The bindings whose sequence ends here, one per scope (P4-14).
    ends: Vec<Entry>,
    /// Longer sequences continuing from here.
    children: BTreeMap<KeyChord, Node>,
}

/// The shortcut map: typed chords to command intents (ADR 0036).
///
/// Multi-chord sequences are stateful: [`ShortcutMap::resolve`] records
/// the chords typed so far and resets on a mismatch or a timeout.
#[derive(Debug, Default)]
pub struct ShortcutMap {
    /// The chord-sequence trie.
    root: Node,
    /// Number of bindings kept (see [`ShortcutMap::len`]).
    bindings: usize,
    /// The chords typed so far of a half-typed sequence.
    pending: Vec<KeyChord>,
    /// The bindings held back because their sequence is a prefix of
    /// longer ones: the trie waits for the next chord (ADR 0036).
    deferred: Vec<Entry>,
    /// The egui frame time of the last typed chord, for the timeout.
    last_chord_time: Option<f64>,
    /// One line per (sequence, scope) conflict dropped at load (first
    /// wins). An exact repeat is dropped without a line.
    conflicts: Vec<String>,
}

/// The shortcut scopes active at one dispatch (ADR 0036).
///
/// A binding fires when its scope is active: `Global` and `Window`
/// always, `Page` and `Ribbon` when the shell reports that surface as
/// focused, `Text` while an egui text edit has focus, `Selection` with
/// a non-empty selection. When several bindings of one sequence are
/// active, the most specific wins.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActiveScopes {
    /// A drawing page (the canvas) has focus.
    pub page: bool,
    /// The ribbon has focus.
    pub ribbon: bool,
    /// An egui text edit has focus.
    pub text: bool,
    /// Shapes are selected (Phase 5; false in Phase 4).
    pub selection: bool,
}

impl ActiveScopes {
    /// The scopes active for one dispatch: the focused surface, an
    /// egui text edit and a selection (ADR 0036).
    pub fn new(focus: ShortcutScope, text_edit: bool, selection: bool) -> Self {
        Self {
            page: focus == ShortcutScope::Page,
            ribbon: focus == ShortcutScope::Ribbon,
            text: text_edit,
            selection,
        }
    }

    /// Whether a binding of scope `scope` fires (ADR 0036).
    pub fn contains(&self, scope: ShortcutScope) -> bool {
        match scope {
            ShortcutScope::Global | ShortcutScope::Window => true,
            ShortcutScope::Page => self.page,
            ShortcutScope::Ribbon => self.ribbon,
            ShortcutScope::Text => self.text,
            ShortcutScope::Selection => self.selection,
        }
    }
}

impl ShortcutMap {
    /// Builds the map from `ui-spec/shortcuts.yaml` (ADR 0007).
    ///
    /// Entries are inserted in file order, one binding per (chord
    /// sequence, scope) pair: the same sequence may be bound in
    /// several scopes and each binding is kept (P4-14). A repeated
    /// (sequence, scope) pair is dropped first-wins — silently when it
    /// names the same command (an exact repeat), with one line in
    /// [`ShortcutMap::conflicts`] when it names a different command (a
    /// conflict, ADR 0036 amended). An unknown key token, a chord with
    /// no key or with more than one key, and a shortcut with no chord
    /// are load-time errors.
    pub fn from_shortcuts(shortcuts: &Shortcuts) -> Result<ShortcutMap, UiError> {
        let mut map = ShortcutMap::default();
        for shortcut in &shortcuts.entries {
            if shortcut.sequence.is_empty() {
                return Err(load_error("a shortcut needs at least one chord"));
            }
            let mut chords = Vec::with_capacity(shortcut.sequence.len());
            for tokens in &shortcut.sequence {
                chords.push(chord_from_tokens(tokens)?);
            }
            let command = CommandId::intern(shortcut.command.as_str())
                .map_err(|error| load_error(format!("bad command id: {error}")))?;
            let entry = Entry {
                command,
                scope: shortcut.scope,
            };
            let mut node = &mut map.root;
            for chord in &chords {
                node = node.children.entry(chord.clone()).or_default();
            }
            if let Some(kept) = node.ends.iter().find(|kept| kept.scope == entry.scope) {
                if kept.command != entry.command {
                    map.conflicts.push(format!(
                        "conflicting shortcut [{}] (scope {}): {} wins over {}",
                        sequence_display(&chords),
                        kept.scope.as_str(),
                        kept.command,
                        entry.command,
                    ));
                }
                // An exact repeat (same command) is dropped silently.
                continue;
            }
            node.ends.push(entry);
            map.bindings += 1;
        }
        Ok(map)
    }

    /// Matches typed chords against the map; `Some` when a sequence
    /// completes and one of its bindings fires (ADR 0036): the
    /// binding of the most specific active scope.
    ///
    /// The chords are processed in order and the first completed
    /// sequence wins; the remaining chords are dropped. Each call
    /// extends the half-typed sequence recorded by earlier calls. A
    /// completed sequence whose bindings are all inactive resolves to
    /// `None` (the sequence is still consumed).
    pub fn resolve(
        &mut self,
        chords: &[KeyChord],
        active: &ActiveScopes,
    ) -> Result<Option<Intent>, UiError> {
        Ok(self
            .resolve_entries(chords)
            .and_then(|entries| pick_entry(&entries, active))
            .map(Entry::into_intent))
    }

    /// The conflict warnings recorded at load (ADR 0036 amended): one
    /// line per (sequence, scope) pair dropped in favour of the first
    /// entry when the two name different commands. An exact repeat
    /// (same sequence, scope and command) is dropped without a line.
    pub fn conflicts(&self) -> &[String] {
        &self.conflicts
    }

    /// Whether a multi-chord sequence is half-typed (status bar hint).
    ///
    /// The 2 second timeout is applied by [`dispatch`], which runs
    /// every frame; a stale buffer is reported here until then.
    pub fn pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// The chords typed so far of a half-typed sequence.
    pub fn pending_chords(&self) -> &[KeyChord] {
        &self.pending
    }

    /// The half-typed chords as `Alt+H` (the shell's status line).
    pub fn pending_display(&self) -> String {
        sequence_display(&self.pending)
    }

    /// Number of shortcut **bindings** in the map: one per distinct
    /// (chord sequence, scope) pair kept. The same sequence bound in
    /// several scopes counts once per scope. A repeated pair is
    /// dropped first-wins and not counted (an exact repeat silently, a
    /// conflict with one [`ShortcutMap::conflicts`] line).
    pub fn len(&self) -> usize {
        self.bindings
    }

    /// True when the map holds no bindings.
    pub fn is_empty(&self) -> bool {
        self.bindings == 0
    }

    /// Feeds the chords to the trie in order; the first completed
    /// sequence wins and later chords are dropped.
    fn resolve_entries(&mut self, chords: &[KeyChord]) -> Option<Vec<Entry>> {
        let mut fired = None;
        for chord in chords {
            let chord = normalize_chord(chord.key.clone(), chord.modifiers.clone());
            if let Some(entries) = self.step(chord, None) {
                fired = Some(entries);
                break;
            }
        }
        fired
    }

    /// Feeds one typed chord to the trie and returns the bindings it
    /// completed, if any. `now` is the egui frame time of the chord
    /// (when called from [`dispatch`]).
    fn step(&mut self, chord: KeyChord, now: Option<f64>) -> Option<Vec<Entry>> {
        if let Some(now) = now {
            self.last_chord_time = Some(now);
        }
        let mut candidate = self.pending.clone();
        candidate.push(chord.clone());
        match self.probe(&candidate) {
            // Complete, but longer sequences share the prefix: wait.
            Some((true, Some(entries))) => {
                self.pending = candidate;
                self.deferred = entries;
                None
            }
            // Complete and no longer sequence to wait for: fire.
            Some((_, Some(entries))) => {
                self.reset();
                Some(entries)
            }
            // A prefix of longer sequences: keep typing.
            Some((true, None)) => {
                self.pending = candidate;
                None
            }
            // Mismatch: fire the held-back completion, else retry the
            // chord as the start of a fresh sequence.
            _ => {
                if !self.deferred.is_empty() {
                    self.reset();
                    return Some(std::mem::take(&mut self.deferred));
                }
                let had_progress = !self.pending.is_empty();
                self.reset();
                if had_progress {
                    return self.step(chord, now);
                }
                None
            }
        }
    }

    /// Drops a half-typed sequence after its timeout and returns the
    /// completion that was waiting on a longer sequence (ADR 0036: a
    /// sequence that is a prefix of another fires when the longer one
    /// times out).
    fn expire(&mut self, now: f64) -> Vec<Entry> {
        if self.pending.is_empty() && self.deferred.is_empty() {
            self.last_chord_time = None;
            return Vec::new();
        }
        let expired = self
            .last_chord_time
            .is_some_and(|last| now - last > SEQUENCE_TIMEOUT_SECONDS);
        if expired {
            self.reset();
            std::mem::take(&mut self.deferred)
        } else {
            Vec::new()
        }
    }

    /// The trie node at `chords`: `(has_children, bindings)`, or
    /// `None` when the chords are not a path in the trie.
    fn probe(&self, chords: &[KeyChord]) -> Option<(bool, Option<Vec<Entry>>)> {
        let mut node = &self.root;
        for chord in chords {
            node = node.children.get(chord)?;
        }
        let ends = if node.ends.is_empty() {
            None
        } else {
            Some(node.ends.clone())
        };
        Some((!node.children.is_empty(), ends))
    }

    /// Clears the typed-chord buffer and its clock.
    fn reset(&mut self) {
        self.pending.clear();
        self.last_chord_time = None;
    }
}

/// Reads egui input, resolves it through `map` and returns the intent
/// for the given scope (ADR 0036).
///
/// `scope` is the surface the shell reports as focused (`Global` when
/// none of `Page` or `Ribbon` has focus). The bindings of the most
/// specific active scope fire ([`ActiveScopes`], ADR 0036 order). A
/// matched chord is consumed before the widgets see it, even when no
/// binding of its sequence is active; a chord that matches nothing is
/// left alone. While an egui text edit has focus, the chords without
/// `Ctrl` or `Alt` (typing and the caret keys) are never matched or
/// consumed; a chord with `Ctrl` or `Alt` is matched even there.
pub fn dispatch(
    egui_ctx: &Context,
    map: &mut ShortcutMap,
    app: &AppCtx,
    scope: ShortcutScope,
) -> Result<Option<Intent>, UiError> {
    let now = egui_ctx.input(|input| input.time);
    let active = ActiveScopes::new(
        scope,
        egui_ctx.text_edit_focused(),
        !app.selection.is_empty(),
    );
    // A sequence that is only a prefix of longer ones fires when the
    // longer ones time out (no key event on that frame).
    if let Some(entry) = pick_entry(&map.expire(now), &active) {
        return Ok(Some(entry.into_intent()));
    }
    let mut fired: Option<Entry> = None;
    let mut consumed: Vec<usize> = Vec::new();

    egui_ctx.input_mut(|input| {
        for (index, event) in input.events.iter().enumerate() {
            if fired.is_some() {
                break;
            }
            let Event::Key {
                key,
                modifiers,
                pressed: true,
                ..
            } = event
            else {
                continue;
            };
            let Some(chord) = chord_from_event(*key, *modifiers) else {
                continue;
            };
            if active.text && defers_to_egui(&chord) {
                continue;
            }
            let matched = map.step(chord, Some(now));
            if matched.is_some() || map.pending() {
                consumed.push(index);
            }
            if let Some(entries) = matched
                && let Some(entry) = pick_entry(&entries, &active)
            {
                fired = Some(entry);
            }
        }
        if !consumed.is_empty() {
            let mut index = 0usize;
            input.events.retain(|_event| {
                let keep = !consumed.contains(&index);
                index += 1;
                keep
            });
        }
    });

    Ok(fired.map(Entry::into_intent))
}

/// The binding that fires for a completed sequence (ADR 0036): the
/// one of the most specific active scope.
fn pick_entry(entries: &[Entry], active: &ActiveScopes) -> Option<Entry> {
    entries
        .iter()
        .filter(|entry| active.contains(entry.scope))
        .min_by_key(|entry| scope_rank(entry.scope))
        .cloned()
}

/// Scope specificity, lower wins: `Text`, `Selection`, `Page`,
/// `Ribbon`, `Window`, `Global` (ADR 0036).
fn scope_rank(scope: ShortcutScope) -> u8 {
    match scope {
        ShortcutScope::Text => 0,
        ShortcutScope::Selection => 1,
        ShortcutScope::Page => 2,
        ShortcutScope::Ribbon => 3,
        ShortcutScope::Window => 4,
        ShortcutScope::Global => 5,
    }
}

/// Key tokens egui's text edit binds with `Ctrl` (Cmd on macOS):
/// select all, the clipboard, undo/redo and delete word.
const EGUI_CTRL_TEXT_TOKENS: [&str; 8] = ["A", "C", "V", "X", "Z", "Y", "Backspace", "Delete"];

/// Key tokens egui's text edit binds with `Ctrl+Shift`: redo and
/// select by word.
const EGUI_CTRL_SHIFT_TEXT_TOKENS: [&str; 2] = ["Z", ARROWS_TOKEN];

/// Whether egui's text edit owns this chord while it has focus (ADR
/// 0036 "egui's own shortcuts apply only inside text edits"): plain
/// typing, shift-typing and the caret keys, plus egui's own `Ctrl`
/// editing chords (clipboard, select all, undo/redo, delete word).
/// Every other `Ctrl` or `Alt` chord is ours, so `Ctrl+Shift+C` picks
/// the `text` binding inside a text edit.
fn defers_to_egui(chord: &KeyChord) -> bool {
    let has = |name: &str| chord.modifiers.iter().any(|modifier| modifier == name);
    let key = chord.key.as_str();
    match (has("Ctrl"), has("Shift"), has("Alt")) {
        (_, _, true) => false,
        (false, _, false) => true,
        (true, false, false) => EGUI_CTRL_TEXT_TOKENS.contains(&key),
        (true, true, false) => EGUI_CTRL_SHIFT_TEXT_TOKENS.contains(&key),
    }
}

/// True when `token` is a key or modifier token the egui mapping table
/// covers (the token-coverage check of ADR 0036).
pub fn token_is_known(token: &str) -> bool {
    MODIFIER_TOKENS.contains(&token) || token == ARROWS_TOKEN || key_for_token(token).is_some()
}

/// The `shortcuts.yaml` key token of an egui key press, or `None` for
/// keys the table does not cover (their chords never match).
///
/// The token of the shifted symbol keys depends on Shift being held:
/// `Comma` is `,` and (with Shift) `<`; `Period` is `.` and `>`;
/// `Equals` is `=` and (with Shift, on most layouts) `+`.
fn key_token(key: Key, modifiers: Modifiers) -> Option<&'static str> {
    let token = match key {
        Key::ArrowDown | Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp => ARROWS_TOKEN,
        Key::Escape => "Esc",
        Key::Tab => "Tab",
        Key::Backspace => "Backspace",
        Key::Enter => "Enter",
        Key::Space => "Space",
        Key::Insert => "Insert",
        Key::Delete => "Delete",
        Key::Home => "Home",
        Key::End => "End",
        Key::PageUp => "PageUp",
        Key::PageDown => "PageDown",
        Key::A => "A",
        Key::B => "B",
        Key::C => "C",
        Key::D => "D",
        Key::E => "E",
        Key::F => "F",
        Key::G => "G",
        Key::H => "H",
        Key::I => "I",
        Key::J => "J",
        Key::K => "K",
        Key::L => "L",
        Key::M => "M",
        Key::N => "N",
        Key::O => "O",
        Key::P => "P",
        Key::Q => "Q",
        Key::R => "R",
        Key::S => "S",
        Key::T => "T",
        Key::U => "U",
        Key::V => "V",
        Key::W => "W",
        Key::X => "X",
        Key::Y => "Y",
        Key::Z => "Z",
        Key::Num0 => "0",
        Key::Num1 => "1",
        Key::Num2 => "2",
        Key::Num3 => "3",
        Key::Num4 => "4",
        Key::Num5 => "5",
        Key::Num6 => "6",
        Key::Num7 => "7",
        Key::Num8 => "8",
        Key::Num9 => "9",
        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F7 => "F7",
        Key::F8 => "F8",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::F12 => "F12",
        Key::Comma => {
            if modifiers.shift {
                "<"
            } else {
                ","
            }
        }
        Key::Period => {
            if modifiers.shift {
                ">"
            } else {
                "."
            }
        }
        Key::Equals => {
            if modifiers.shift {
                "+"
            } else {
                "="
            }
        }
        Key::Plus => "+",
        Key::Minus => "-",
        Key::Colon => ":",
        Key::Backslash => "\\",
        Key::Slash => "/",
        Key::Pipe => "|",
        Key::Questionmark => "?",
        Key::Exclamationmark => "!",
        Key::OpenBracket => "[",
        Key::CloseBracket => "]",
        Key::OpenCurlyBracket => "{",
        Key::CloseCurlyBracket => "}",
        Key::Backtick => "`",
        Key::Semicolon => ";",
        Key::Quote => "'",
        _ => return None,
    };
    Some(token)
}

/// The egui key a key token maps to, or `None` for tokens the table
/// does not cover (a load-time error in
/// [`ShortcutMap::from_shortcuts`]).
///
/// `ARROWS_TOKEN` is not a single key; [`token_is_known`] answers for
/// it. `<` and `>` are typed on the comma and period keys (with
/// Shift), so they map there.
fn key_for_token(token: &str) -> Option<Key> {
    let key = match token {
        "Esc" => Key::Escape,
        "Tab" => Key::Tab,
        "Backspace" => Key::Backspace,
        "Enter" => Key::Enter,
        "Space" => Key::Space,
        "Insert" => Key::Insert,
        "Delete" => Key::Delete,
        "Home" => Key::Home,
        "End" => Key::End,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        "A" => Key::A,
        "B" => Key::B,
        "C" => Key::C,
        "D" => Key::D,
        "E" => Key::E,
        "F" => Key::F,
        "G" => Key::G,
        "H" => Key::H,
        "I" => Key::I,
        "J" => Key::J,
        "K" => Key::K,
        "L" => Key::L,
        "M" => Key::M,
        "N" => Key::N,
        "O" => Key::O,
        "P" => Key::P,
        "Q" => Key::Q,
        "R" => Key::R,
        "S" => Key::S,
        "T" => Key::T,
        "U" => Key::U,
        "V" => Key::V,
        "W" => Key::W,
        "X" => Key::X,
        "Y" => Key::Y,
        "Z" => Key::Z,
        "0" => Key::Num0,
        "1" => Key::Num1,
        "2" => Key::Num2,
        "3" => Key::Num3,
        "4" => Key::Num4,
        "5" => Key::Num5,
        "6" => Key::Num6,
        "7" => Key::Num7,
        "8" => Key::Num8,
        "9" => Key::Num9,
        "F1" => Key::F1,
        "F2" => Key::F2,
        "F3" => Key::F3,
        "F4" => Key::F4,
        "F5" => Key::F5,
        "F6" => Key::F6,
        "F7" => Key::F7,
        "F8" => Key::F8,
        "F9" => Key::F9,
        "F10" => Key::F10,
        "F11" => Key::F11,
        "F12" => Key::F12,
        "<" | "," => Key::Comma,
        ">" | "." => Key::Period,
        "+" => Key::Plus,
        "=" => Key::Equals,
        "-" => Key::Minus,
        ":" => Key::Colon,
        "\\" => Key::Backslash,
        "/" => Key::Slash,
        "|" => Key::Pipe,
        "?" => Key::Questionmark,
        "!" => Key::Exclamationmark,
        "[" => Key::OpenBracket,
        "]" => Key::CloseBracket,
        "{" => Key::OpenCurlyBracket,
        "}" => Key::CloseCurlyBracket,
        "`" => Key::Backtick,
        ";" => Key::Semicolon,
        "'" => Key::Quote,
        _ => return None,
    };
    Some(key)
}

/// Normalizes one egui key press to a chord: the key token plus the
/// `Ctrl`, `Shift`, `Alt` modifier tokens (`Ctrl` covers the macOS
/// command key; `shortcuts.yaml` is a Windows table). `None` for keys
/// the token table does not cover.
fn chord_from_event(key: Key, modifiers: Modifiers) -> Option<KeyChord> {
    let key_token = key_token(key, modifiers)?;
    let mut mods = Vec::new();
    if modifiers.ctrl || modifiers.command {
        mods.push("Ctrl".to_owned());
    }
    if modifiers.shift {
        mods.push("Shift".to_owned());
    }
    if modifiers.alt {
        mods.push("Alt".to_owned());
    }
    Some(normalize_chord(key_token.to_owned(), mods))
}

/// Parses one chord of `shortcuts.yaml` (an unordered set of key
/// tokens) into a normalized [`KeyChord`].
fn chord_from_tokens(tokens: &[String]) -> Result<KeyChord, UiError> {
    let mut keys: Vec<&String> = Vec::new();
    let mut modifiers: Vec<String> = Vec::new();
    for token in tokens {
        if MODIFIER_TOKENS.contains(&token.as_str()) {
            modifiers.push(token.clone());
        } else {
            keys.push(token);
        }
    }
    let mut keys = keys.into_iter();
    let Some(key) = keys.next() else {
        return Err(load_error(format!(
            "chord [{}] has no key token",
            tokens.join(", ")
        )));
    };
    if keys.next().is_some() {
        return Err(load_error(format!(
            "chord [{}] has more than one key token",
            tokens.join(", ")
        )));
    }
    if !token_is_known(key) {
        return Err(load_error(format!("unknown key token {key:?}")));
    }
    Ok(normalize_chord(key.clone(), modifiers))
}

/// Normalizes a chord: deduplicated modifiers in canonical order, and
/// no `Shift` on a shifted symbol key.
fn normalize_chord(key: String, mut modifiers: Vec<String>) -> KeyChord {
    modifiers.sort_by_key(|token| modifier_rank(token));
    modifiers.dedup();
    if SHIFTED_SYMBOL_TOKENS.contains(&key.as_str()) {
        modifiers.retain(|token| token != "Shift");
    }
    KeyChord { key, modifiers }
}

/// The canonical order of the modifier tokens.
fn modifier_rank(token: &str) -> u8 {
    match token {
        "Ctrl" => 0,
        "Shift" => 1,
        "Alt" => 2,
        _ => 3,
    }
}

/// The chords of one sequence as `Ctrl+Shift+F, F` for warnings.
fn sequence_display(chords: &[KeyChord]) -> String {
    chords
        .iter()
        .map(|chord| {
            let mut parts: Vec<&str> = chord.modifiers.iter().map(String::as_str).collect();
            parts.push(chord.key.as_str());
            parts.join("+")
        })
        .collect::<Vec<String>>()
        .join(", ")
}

/// A load-time error for the shortcut table (ADR 0007 data errors are
/// [`LoadError::Parse`]).
fn load_error(message: impl Into<String>) -> UiError {
    UiError::Layout(LoadError::Parse {
        path: PathBuf::from("shortcuts.yaml"),
        message: message.into(),
    })
}

#[cfg(test)]
mod tests {
    use ferrogram_commands::Settings;
    use ferrogram_ui_spec::CommandId as SpecCommandId;

    use super::*;

    fn shortcut(
        tokens: &[&str],
        command: &str,
        scope: ShortcutScope,
    ) -> ferrogram_ui_spec::Shortcut {
        ferrogram_ui_spec::Shortcut {
            sequence: vec![tokens.iter().map(|t| (*t).to_owned()).collect()],
            command: SpecCommandId::parse(command).expect("id"),
            scope,
            section: None,
        }
    }

    fn multi_shortcut(
        chords: &[&[&str]],
        command: &str,
        scope: ShortcutScope,
    ) -> ferrogram_ui_spec::Shortcut {
        ferrogram_ui_spec::Shortcut {
            sequence: chords
                .iter()
                .map(|chord| chord.iter().map(|t| (*t).to_owned()).collect())
                .collect(),
            command: SpecCommandId::parse(command).expect("id"),
            scope,
            section: None,
        }
    }

    fn shortcuts(entries: Vec<ferrogram_ui_spec::Shortcut>) -> Shortcuts {
        Shortcuts { entries }
    }

    fn chord(key: &str, modifiers: &[&str]) -> KeyChord {
        KeyChord::new(key, modifiers.iter().map(|m| (*m).to_owned()).collect())
    }

    fn app() -> AppCtx {
        AppCtx::new(Settings::default())
    }

    #[test]
    fn a_chord_holds_key_and_modifiers() {
        let chord = KeyChord::new("Z", vec!["Ctrl".into()]);
        assert_eq!(chord.key, "Z");
        assert_eq!(chord.modifiers, vec!["Ctrl".to_owned()]);
    }

    #[test]
    fn modifiers_are_sorted_and_deduplicated() {
        let chord = KeyChord::new("F", vec!["Shift".into(), "Ctrl".into(), "Ctrl".into()]);
        assert_eq!(chord.modifiers, vec!["Ctrl".to_owned(), "Shift".to_owned()]);
    }

    #[test]
    fn a_shifted_symbol_drops_shift_on_both_sides() {
        let from_file = chord_from_tokens(&["Ctrl".to_owned(), "Shift".to_owned(), "<".to_owned()])
            .expect("chord");
        let typed = chord_from_event(
            Key::Comma,
            Modifiers {
                ctrl: true,
                shift: true,
                ..Default::default()
            },
        )
        .expect("chord");
        assert_eq!(from_file, typed);
        assert_eq!(from_file, chord("<", &["Ctrl"]));
    }

    #[test]
    fn the_plus_key_matches_shift_equals_and_numpad_plus() {
        let from_file = chord_from_tokens(&["Ctrl".to_owned(), "+".to_owned()]).expect("chord");
        let shift_equals = chord_from_event(
            Key::Equals,
            Modifiers {
                ctrl: true,
                shift: true,
                ..Default::default()
            },
        )
        .expect("chord");
        let numpad_plus = chord_from_event(
            Key::Plus,
            Modifiers {
                ctrl: true,
                ..Default::default()
            },
        )
        .expect("chord");
        assert_eq!(from_file, shift_equals);
        assert_eq!(from_file, numpad_plus);
    }

    #[test]
    fn the_arrow_keys_share_one_token() {
        for key in [
            Key::ArrowUp,
            Key::ArrowDown,
            Key::ArrowLeft,
            Key::ArrowRight,
        ] {
            let chord = chord_from_event(key, Modifiers::default()).expect("chord");
            assert_eq!(chord.key, ARROWS_TOKEN);
        }
        let from_file = chord_from_tokens(&[ARROWS_TOKEN.to_owned()]).expect("chord");
        assert_eq!(from_file.key, ARROWS_TOKEN);
    }

    #[test]
    fn an_unknown_key_token_is_a_load_error() {
        let error = chord_from_tokens(&["Hyper".to_owned()]).expect_err("error");
        assert!(error.to_string().contains("unknown key token"));
    }

    #[test]
    fn a_chord_without_a_key_or_with_two_keys_is_a_load_error() {
        let empty = chord_from_tokens(&["Ctrl".to_owned()]).expect_err("error");
        assert!(empty.to_string().contains("no key token"));
        let two = chord_from_tokens(&["A".to_owned(), "B".to_owned()]).expect_err("error");
        assert!(two.to_string().contains("more than one key token"));
    }

    #[test]
    fn every_token_of_the_table_is_known() {
        for token in MODIFIER_TOKENS {
            assert!(token_is_known(token));
        }
        assert!(token_is_known(ARROWS_TOKEN));
        for token in [
            "A",
            "Z",
            "0",
            "9",
            "F1",
            "F12",
            "Esc",
            "Tab",
            "Backspace",
            "Delete",
            "Enter",
            "Space",
            "Insert",
            "Home",
            "End",
            "PageUp",
            "PageDown",
            "+",
            "-",
            "=",
            "<",
            ">",
            ",",
            ".",
            ":",
            ";",
            "'",
            "?",
            "!",
            "[",
            "]",
            "{",
            "}",
            "\\",
            "/",
            "|",
            "`",
        ] {
            assert!(key_for_token(token).is_some(), "{token} missing");
            assert!(token_is_known(token), "{token} unknown");
        }
        assert!(!token_is_known("Hyper"));
        assert!(!token_is_known("F13"));
    }

    #[test]
    fn an_uncovered_key_has_no_token() {
        assert_eq!(key_token(Key::F13, Modifiers::default()), None);
        assert!(chord_from_event(Key::F13, Modifiers::default()).is_none());
    }

    #[test]
    fn the_map_loads_and_resolves_single_chords() {
        let map_source = shortcuts(vec![
            shortcut(&["Ctrl", "Z"], "edit.undo", ShortcutScope::Global),
            shortcut(&["Ctrl", "I"], "format.text.italic", ShortcutScope::Text),
        ]);
        let mut map = ShortcutMap::from_shortcuts(&map_source).expect("map");
        assert_eq!(map.len(), 2);
        assert!(!map.is_empty());
        assert!(map.conflicts().is_empty());
        let intent = map
            .resolve(&[chord("Z", &["Ctrl"])], &ActiveScopes::default())
            .expect("resolve")
            .expect("intent");
        assert_eq!(intent.command.as_str(), "edit.undo");
        assert_eq!(intent.invocation.source, Source::Shortcut);
        assert!(!map.pending());
    }

    #[test]
    fn a_leaf_holds_one_binding_per_scope() {
        let map_source = shortcuts(vec![
            shortcut(&["Ctrl", "C"], "edit.copy", ShortcutScope::Global),
            shortcut(&["Ctrl", "C"], "edit.copy", ShortcutScope::Window),
            shortcut(&["Ctrl", "C"], "edit.copy", ShortcutScope::Page),
        ]);
        let mut map = ShortcutMap::from_shortcuts(&map_source).expect("map");
        assert_eq!(map.len(), 3, "one binding per (sequence, scope) pair");
        assert!(map.conflicts().is_empty());
        // All three scopes name the same command: it fires anywhere.
        for active in [
            ActiveScopes::default(),
            ActiveScopes::new(ShortcutScope::Page, false, false),
            ActiveScopes::new(ShortcutScope::Global, true, false),
        ] {
            let intent = map
                .resolve(&[chord("C", &["Ctrl"])], &active)
                .expect("resolve")
                .expect("intent");
            assert_eq!(intent.command.as_str(), "edit.copy");
        }
    }

    #[test]
    fn the_most_specific_active_scope_wins() {
        let map_source = shortcuts(vec![
            shortcut(&["Ctrl", "G"], "arrange.group", ShortcutScope::Global),
            shortcut(&["Ctrl", "G"], "arrange.ungroup", ShortcutScope::Page),
            shortcut(&["Ctrl", "G"], "edit.copy", ShortcutScope::Text),
        ]);
        let mut map = ShortcutMap::from_shortcuts(&map_source).expect("map");
        assert_eq!(map.len(), 3);
        assert!(map.conflicts().is_empty(), "different scopes are legal");

        // Only Global active: the global binding.
        let global = map
            .resolve(&[chord("G", &["Ctrl"])], &ActiveScopes::default())
            .expect("resolve")
            .expect("intent");
        assert_eq!(global.command.as_str(), "arrange.group");

        // Page beats Global (ADR 0036 order).
        let page = map
            .resolve(
                &[chord("G", &["Ctrl"])],
                &ActiveScopes::new(ShortcutScope::Page, false, false),
            )
            .expect("resolve")
            .expect("intent");
        assert_eq!(page.command.as_str(), "arrange.ungroup");

        // Text beats Page and Global.
        let text = map
            .resolve(
                &[chord("G", &["Ctrl"])],
                &ActiveScopes::new(ShortcutScope::Page, true, false),
            )
            .expect("resolve")
            .expect("intent");
        assert_eq!(text.command.as_str(), "edit.copy");
    }

    #[test]
    fn an_exact_repeat_is_dropped_without_a_warning() {
        let map_source = shortcuts(vec![
            shortcut(
                &["Ctrl", "H"],
                "arrange.flip_horizontal",
                ShortcutScope::Global,
            ),
            shortcut(
                &["Ctrl", "H"],
                "arrange.flip_horizontal",
                ShortcutScope::Global,
            ),
            shortcut(
                &["Ctrl", "H"],
                "arrange.flip_horizontal",
                ShortcutScope::Page,
            ),
        ]);
        let map = ShortcutMap::from_shortcuts(&map_source).expect("map");
        assert_eq!(map.len(), 2, "the exact repeat is dropped");
        assert!(map.conflicts().is_empty(), "an exact repeat is silent");
    }

    #[test]
    fn a_real_conflict_is_recorded_once_and_first_wins() {
        let map_source = shortcuts(vec![
            shortcut(&["Ctrl", "G"], "arrange.group", ShortcutScope::Global),
            shortcut(&["Ctrl", "G"], "arrange.ungroup", ShortcutScope::Global),
            shortcut(&["Ctrl", "G"], "edit.copy", ShortcutScope::Global),
        ]);
        let mut map = ShortcutMap::from_shortcuts(&map_source).expect("map");
        assert_eq!(map.len(), 1);
        assert_eq!(map.conflicts().len(), 2);
        assert!(map.conflicts()[0].contains("arrange.group"));
        assert!(map.conflicts()[0].contains("arrange.ungroup"));
        let intent = map
            .resolve(&[chord("G", &["Ctrl"])], &ActiveScopes::default())
            .expect("resolve")
            .expect("intent");
        assert_eq!(intent.command.as_str(), "arrange.group");
    }

    #[test]
    fn a_completed_sequence_with_no_active_binding_resolves_to_nothing() {
        let map_source = shortcuts(vec![shortcut(
            &["Ctrl", "G"],
            "arrange.group",
            ShortcutScope::Selection,
        )]);
        let mut map = ShortcutMap::from_shortcuts(&map_source).expect("map");
        assert!(
            map.resolve(&[chord("G", &["Ctrl"])], &ActiveScopes::default())
                .expect("resolve")
                .is_none()
        );
        assert!(!map.pending());
    }

    #[test]
    fn len_counts_bindings_not_source_entries() {
        let map_source = shortcuts(vec![
            shortcut(&["Ctrl", "C"], "edit.copy", ShortcutScope::Global),
            shortcut(&["Ctrl", "C"], "edit.copy", ShortcutScope::Window),
            shortcut(&["Ctrl", "C"], "edit.copy", ShortcutScope::Window),
            shortcut(&["Ctrl", "G"], "arrange.group", ShortcutScope::Global),
            shortcut(&["Ctrl", "G"], "arrange.ungroup", ShortcutScope::Global),
        ]);
        let map = ShortcutMap::from_shortcuts(&map_source).expect("map");
        // Three bindings: Ctrl+C global, Ctrl+C window (the repeat is
        // dropped) and Ctrl+G global (the conflict is dropped).
        assert_eq!(map.len(), 3);
    }

    #[test]
    fn pending_chords_show_as_a_status_line() {
        let two_chords = multi_shortcut(
            &[&["Alt", "H"], &["F"]],
            "arrange.group",
            ShortcutScope::Global,
        );
        let mut map = ShortcutMap::from_shortcuts(&shortcuts(vec![two_chords])).expect("map");
        assert_eq!(map.pending_display(), "");
        map.step(chord("H", &["Alt"]), Some(1.0));
        assert!(map.pending());
        assert_eq!(map.pending_chords(), &[chord("H", &["Alt"])]);
        assert_eq!(map.pending_display(), "Alt+H");
    }

    #[test]
    fn a_multi_chord_sequence_resolves_after_its_chords() {
        let two_chords = multi_shortcut(
            &[&["Alt", "H"], &["F"]],
            "arrange.group",
            ShortcutScope::Global,
        );
        let mut map = ShortcutMap::from_shortcuts(&shortcuts(vec![two_chords])).expect("map");
        assert!(
            map.resolve(&[chord("H", &["Alt"])], &ActiveScopes::default())
                .expect("resolve")
                .is_none()
        );
        assert!(map.pending());
        let intent = map
            .resolve(&[chord("F", &[])], &ActiveScopes::default())
            .expect("resolve")
            .expect("intent");
        assert_eq!(intent.command.as_str(), "arrange.group");
        assert!(!map.pending());
    }

    #[test]
    fn a_mismatch_resets_the_buffer_and_retries_the_chord() {
        let two_chords = multi_shortcut(
            &[&["Alt", "H"], &["F"]],
            "arrange.group",
            ShortcutScope::Global,
        );
        let single = shortcut(&["Ctrl", "Z"], "edit.undo", ShortcutScope::Global);
        let mut map =
            ShortcutMap::from_shortcuts(&shortcuts(vec![two_chords, single])).expect("map");
        assert!(
            map.resolve(&[chord("H", &["Alt"])], &ActiveScopes::default())
                .expect("resolve")
                .is_none()
        );
        // Ctrl+Z does not continue the sequence: it starts a new one.
        let intent = map
            .resolve(&[chord("Z", &["Ctrl"])], &ActiveScopes::default())
            .expect("resolve")
            .expect("intent");
        assert_eq!(intent.command.as_str(), "edit.undo");
        assert!(!map.pending());
    }

    #[test]
    fn a_prefix_entry_waits_then_fires_on_mismatch() {
        let map_source = shortcuts(vec![
            shortcut(&["F6"], "view.zoom.in", ShortcutScope::Global),
            multi_shortcut(&[&["F6"], &["F6"]], "view.zoom.fit", ShortcutScope::Global),
        ]);
        let mut map = ShortcutMap::from_shortcuts(&map_source).expect("map");
        assert!(
            map.resolve(&[chord("F6", &[])], &ActiveScopes::default())
                .expect("resolve")
                .is_none()
        );
        assert!(map.pending());
        // F5 does not continue the longer sequence: the shorter one fires.
        let intent = map
            .resolve(&[chord("F5", &[])], &ActiveScopes::default())
            .expect("resolve")
            .expect("intent");
        assert_eq!(intent.command.as_str(), "view.zoom.in");
        assert!(!map.pending());
    }

    #[test]
    fn a_deferred_completion_fires_the_most_specific_active_binding() {
        let map_source = shortcuts(vec![
            shortcut(&["F6"], "view.zoom.in", ShortcutScope::Global),
            shortcut(&["F6"], "arrange.nudge", ShortcutScope::Page),
            multi_shortcut(&[&["F6"], &["F6"]], "view.zoom.fit", ShortcutScope::Global),
        ]);
        let mut map = ShortcutMap::from_shortcuts(&map_source).expect("map");
        assert!(
            map.resolve(&[chord("F6", &[])], &ActiveScopes::default())
                .expect("resolve")
                .is_none()
        );
        let intent = map
            .resolve(
                &[chord("F5", &[])],
                &ActiveScopes::new(ShortcutScope::Page, false, false),
            )
            .expect("resolve")
            .expect("intent");
        assert_eq!(intent.command.as_str(), "arrange.nudge");
    }

    #[test]
    fn an_expired_sequence_drops_the_buffer_and_fires_the_deferred_entry() {
        let map_source = shortcuts(vec![
            shortcut(&["F6"], "view.zoom.in", ShortcutScope::Global),
            multi_shortcut(&[&["F6"], &["F6"]], "view.zoom.fit", ShortcutScope::Global),
        ]);
        let mut map = ShortcutMap::from_shortcuts(&map_source).expect("map");
        map.step(chord("F6", &[]), Some(10.0));
        assert!(map.pending());
        assert!(map.expire(11.0).is_empty());
        let entries = map.expire(12.5);
        assert_eq!(entries.len(), 1, "the deferred binding fires");
        assert_eq!(entries[0].command.as_str(), "view.zoom.in");
        assert!(!map.pending());
    }

    #[test]
    fn a_half_typed_sequence_times_out() {
        let two_chords = multi_shortcut(
            &[&["Alt", "H"], &["F"]],
            "arrange.group",
            ShortcutScope::Global,
        );
        let mut map = ShortcutMap::from_shortcuts(&shortcuts(vec![two_chords])).expect("map");
        map.step(chord("H", &["Alt"]), Some(10.0));
        assert!(map.pending());
        assert!(map.expire(10.0 + SEQUENCE_TIMEOUT_SECONDS / 2.0).is_empty());
        assert!(map.pending());
        assert!(map.expire(10.0 + SEQUENCE_TIMEOUT_SECONDS + 0.1).is_empty());
        assert!(!map.pending());
    }

    #[test]
    fn an_empty_table_loads_as_an_empty_map() {
        let mut map = ShortcutMap::from_shortcuts(&Shortcuts::default()).expect("map");
        assert!(map.is_empty());
        assert_eq!(map.len(), 0);
        assert!(!map.pending());
        assert!(
            map.resolve(&[], &ActiveScopes::default())
                .expect("resolve")
                .is_none()
        );
    }

    #[test]
    fn a_shortcut_without_a_chord_is_a_load_error() {
        let empty = ferrogram_ui_spec::Shortcut {
            sequence: Vec::new(),
            command: SpecCommandId::parse("edit.undo").expect("id"),
            scope: ShortcutScope::Global,
            section: None,
        };
        let error = ShortcutMap::from_shortcuts(&shortcuts(vec![empty])).expect_err("error");
        assert!(error.to_string().contains("at least one chord"));
    }

    #[test]
    fn a_bad_token_is_a_load_error() {
        let bad = shortcut(&["Ctrl", "Warp"], "edit.undo", ShortcutScope::Global);
        let error = ShortcutMap::from_shortcuts(&shortcuts(vec![bad])).expect_err("error");
        assert!(error.to_string().contains("unknown key token"));
    }

    #[test]
    fn active_scopes_follow_the_focus() {
        let page = ActiveScopes::new(ShortcutScope::Page, false, false);
        assert!(page.page);
        assert!(!page.ribbon);
        assert!(!page.text);
        assert!(!page.selection);
        let ribbon = ActiveScopes::new(ShortcutScope::Ribbon, true, true);
        assert!(ribbon.ribbon);
        assert!(ribbon.text);
        assert!(ribbon.selection);
        assert!(!ribbon.page);
    }

    #[test]
    fn global_and_window_scopes_always_match() {
        for want in [ShortcutScope::Global, ShortcutScope::Window] {
            for active in [
                ActiveScopes::default(),
                ActiveScopes::new(ShortcutScope::Page, false, false),
                ActiveScopes::new(ShortcutScope::Ribbon, true, false),
                ActiveScopes::new(ShortcutScope::Global, false, true),
            ] {
                assert!(active.contains(want));
            }
        }
    }

    #[test]
    fn page_and_ribbon_need_their_surface() {
        let page = ActiveScopes::new(ShortcutScope::Page, false, false);
        let ribbon = ActiveScopes::new(ShortcutScope::Ribbon, false, false);
        assert!(page.contains(ShortcutScope::Page));
        assert!(!page.contains(ShortcutScope::Ribbon));
        assert!(ribbon.contains(ShortcutScope::Ribbon));
        assert!(!ribbon.contains(ShortcutScope::Page));
    }

    #[test]
    fn text_needs_a_text_edit_and_selection_never_fires_in_phase_4() {
        let with_text = ActiveScopes::new(ShortcutScope::Global, true, false);
        let without_text = ActiveScopes::new(ShortcutScope::Global, false, false);
        assert!(with_text.contains(ShortcutScope::Text));
        assert!(!without_text.contains(ShortcutScope::Text));
        for active in [
            ActiveScopes::default(),
            ActiveScopes::new(ShortcutScope::Page, true, false),
        ] {
            assert!(!active.contains(ShortcutScope::Selection));
        }
    }

    #[test]
    fn plain_typing_and_caret_keys_defer_to_egui() {
        for key in ["A", "1", "-", ARROWS_TOKEN, "Delete", "Esc", "Home"] {
            assert!(defers_to_egui(&chord(key, &[])), "{key}");
        }
        assert!(defers_to_egui(&chord("A", &["Shift"])));
    }

    #[test]
    fn egui_text_editing_chords_defer() {
        for key in ["A", "C", "V", "X", "Z", "Y", "Backspace", "Delete"] {
            assert!(defers_to_egui(&chord(key, &["Ctrl"])), "Ctrl+{key}");
        }
        assert!(defers_to_egui(&chord("Z", &["Ctrl", "Shift"])));
        assert!(defers_to_egui(&chord(ARROWS_TOKEN, &["Ctrl", "Shift"])));
    }

    #[test]
    fn other_ctrl_and_alt_chords_do_not_defer() {
        // Ctrl+Shift+C picks the `text` binding inside a text edit.
        assert!(!defers_to_egui(&chord("C", &["Ctrl", "Shift"])));
        assert!(!defers_to_egui(&chord("I", &["Ctrl"])));
        assert!(!defers_to_egui(&chord("<", &["Ctrl"])));
        assert!(!defers_to_egui(&chord("F4", &["Alt"])));
        assert!(!defers_to_egui(&chord("C", &["Ctrl", "Alt"])));
    }

    #[test]
    fn chord_from_event_maps_modifiers() {
        let typed = chord_from_event(
            Key::F,
            Modifiers {
                ctrl: true,
                shift: true,
                alt: true,
                ..Default::default()
            },
        )
        .expect("chord");
        assert_eq!(typed, chord("F", &["Ctrl", "Shift", "Alt"]));
        // The macOS command key reads as Ctrl (a Windows table).
        let command_key = chord_from_event(
            Key::S,
            Modifiers {
                mac_cmd: true,
                command: true,
                ..Default::default()
            },
        )
        .expect("chord");
        assert_eq!(command_key, chord("S", &["Ctrl"]));
    }

    #[test]
    fn resolve_processes_one_chord_per_call_and_drops_the_rest() {
        let map_source = shortcuts(vec![
            shortcut(&["Ctrl", "Z"], "edit.undo", ShortcutScope::Global),
            shortcut(&["Ctrl", "Y"], "edit.redo", ShortcutScope::Global),
        ]);
        let mut map = ShortcutMap::from_shortcuts(&map_source).expect("map");
        let intent = map
            .resolve(
                &[chord("Z", &["Ctrl"]), chord("Y", &["Ctrl"])],
                &ActiveScopes::default(),
            )
            .expect("resolve")
            .expect("intent");
        assert_eq!(intent.command.as_str(), "edit.undo");
    }

    #[test]
    fn the_context_dispatch_sees_no_keys() {
        let map_source = shortcuts(vec![shortcut(
            &["Ctrl", "Z"],
            "edit.undo",
            ShortcutScope::Global,
        )]);
        let mut map = ShortcutMap::from_shortcuts(&map_source).expect("map");
        let ctx = Context::default();
        let mut outcome = None;
        let mut output = ctx.run_ui(Default::default(), |ctx| {
            outcome = dispatch(ctx, &mut map, &app(), ShortcutScope::Global).ok();
        });
        output.textures_delta.clear();
        assert_eq!(outcome, Some(None));
    }
}
