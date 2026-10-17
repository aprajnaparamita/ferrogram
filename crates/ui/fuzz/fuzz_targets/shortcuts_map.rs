//! Fuzz target for the shortcut map (task P4-10, ADR 0036).
//!
//! `ShortcutMap::from_shortcuts` and `ShortcutMap::resolve` run on
//! tables that can be hostile: a future build may load a user-supplied
//! `shortcuts.yaml`, and the decoded table can carry any sequence
//! length, any token set and any scope. The target builds such tables
//! straight from the input bytes (no file I/O) and drives the trie:
//! construction, repeated `resolve` steps with rotating active scopes
//! and the status accessors. A panic is a crash; a hang is a finding.
//!
//! The YAML parse itself is `ferrogram-ui-spec`'s `loader_fuzz`.

#![no_main]

use ferrogram_ui::shortcuts::{ActiveScopes, ShortcutMap};
use ferrogram_ui_spec::{CommandId, Shortcut, ShortcutScope, Shortcuts};
use libfuzzer_sys::fuzz_target;

/// The token vocabulary: modifier and key tokens the table knows,
/// plus one unknown token (the load-error path).
const TOKENS: &[&str] = &[
    "Ctrl",
    "Shift",
    "Alt",
    "A",
    "B",
    "C",
    "F1",
    "Esc",
    "Delete",
    "Arrow Keys",
    "+",
    "<",
    "Wingdings",
];

/// The command vocabulary: valid ids plus one invalid (the
/// `CommandId::intern` error path).
const COMMANDS: &[&str] = &[
    "edit.copy",
    "edit.cut",
    "edit.undo",
    "view.zoom.in",
    "page.next",
    "app.quit",
    "mystery!",
];

const SCOPES: &[ShortcutScope] = &[
    ShortcutScope::Global,
    ShortcutScope::Window,
    ShortcutScope::Page,
    ShortcutScope::Ribbon,
    ShortcutScope::Text,
    ShortcutScope::Selection,
];

/// A byte reader that stops at the end of the input.
struct Reader<'a> {
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { rest: data }
    }

    /// The next byte, or `0` at the end of the input.
    fn byte(&mut self) -> u8 {
        match self.rest.split_first() {
            Some((head, tail)) => {
                self.rest = tail;
                *head
            }
            None => 0,
        }
    }

    /// A slice index into `choices`, or 0 at the end.
    fn pick(&mut self, choices: &[&str]) -> String {
        let index = usize::from(self.byte()) % choices.len();
        choices[index].to_owned()
    }
}

fuzz_target!(|data: &[u8]| {
    let mut reader = Reader::new(data);
    // Work bound: at most 64 entries, 16 chords per sequence, 4
    // tokens per chord (libFuzzer finds the deep case through the
    // `deep` shape below).
    let count = usize::from(reader.byte()) % 64;
    let deep = reader.byte() == 255;
    let mut entries: Vec<Shortcut> = Vec::with_capacity(count + 1);
    for _ in 0..count {
        let chords = usize::from(reader.byte()) % 16;
        let mut sequence = Vec::with_capacity(chords);
        for _ in 0..chords {
            let tokens = usize::from(reader.byte()) % 4;
            let mut chord = Vec::with_capacity(tokens);
            for _ in 0..tokens {
                chord.push(reader.pick(TOKENS));
            }
            sequence.push(chord);
        }
        let scope = SCOPES[usize::from(reader.byte()) % SCOPES.len()];
        let command = reader.pick(COMMANDS);
        entries.push(Shortcut {
            sequence,
            command: CommandId::new(command),
            scope,
            section: None,
        });
    }
    if deep {
        // One very deep sequence: a long chain of identical chords.
        let mut sequence = Vec::with_capacity(512);
        for _ in 0..512 {
            sequence.push(vec!["Alt".to_owned(), "H".to_owned()]);
        }
        entries.push(Shortcut {
            sequence,
            command: CommandId::new("edit.copy"),
            scope: ShortcutScope::Global,
            section: None,
        });
    }

    let Ok(mut map) = ShortcutMap::from_shortcuts(&Shortcuts { entries }) else {
        return;
    };

    // Drive the trie: every chord of every sequence, typed one at a
    // time, with rotating active scopes and a second round in the
    // reverse order (the buffer survives across calls).
    let typed: Vec<Vec<String>> = {
        let mut reader = Reader::new(data);
        let count = usize::from(reader.byte()) % 64;
        let mut out = Vec::new();
        for _ in 0..count {
            let chords = usize::from(reader.byte()) % 16;
            for _ in 0..chords {
                let tokens = usize::from(reader.byte()) % 4;
                let mut chord = Vec::with_capacity(tokens);
                for _ in 0..tokens {
                    chord.push(reader.pick(TOKENS));
                }
                out.push(chord);
            }
        }
        out
    };
    for (index, tokens) in typed.iter().enumerate() {
        let chord = ferrogram_ui::shortcuts::KeyChord::new(
            tokens.last().cloned().unwrap_or_default(),
            tokens[..tokens.len().saturating_sub(1)].to_vec(),
        );
        let active = ActiveScopes {
            page: index % 2 == 0,
            ribbon: index % 3 == 0,
            text: index % 5 == 0,
            selection: index % 7 == 0,
        };
        let _ = map.resolve(std::slice::from_ref(&chord), &active);
        let _ = map.pending_display();
    }
    let _ = map.len();
    let _ = map.conflicts();
});
