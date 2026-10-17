//! Shortcut-trie and chord-normalization edge cases (task P4-10,
//! ADR 0036 as amended at the P4-08 and P4-14 merges).
//!
//! `tests/shortcuts_dispatch.rs` (P4-08/P4-14) covers the happy paths
//! and the committed table; this file hunts the edges:
//!
//! - chord normalization: modifier order, duplicates, shifted symbol
//!   keys (`+`, `<`, `>`) absorbing `Shift` on both sides;
//! - load-time errors: an empty sequence, a chord with no key or two
//!   keys, an unknown token, a bad command id;
//! - trie behavior: a sequence that is a prefix of longer ones
//!   (deferred completion, mismatch, timeout), state across `resolve`
//!   calls, unmatched chords left alone;
//! - scope precedence: the most specific active binding wins and an
//!   inactive binding is still consumed;
//! - the text-edit defer rule (ADR 0036, amended twice): egui's
//!   editing chords are never stolen, every other bound `Ctrl`/`Alt`
//!   chord dispatches inside a text edit;
//! - hostile tables: deep sequences and thousands of entries build and
//!   dispatch without a panic or a hang.
//!
//! A panic is the bug (AGENTS.md section 6).

// The never-crash lints apply to non-test code only (clippy.toml);
// these helpers are test code and use `expect` freely.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use egui::{Context, Event, Key, Modifiers, RawInput};
use ferrogram_commands::{AppCtx, Settings};
use ferrogram_ui::Intent;
use ferrogram_ui::shortcuts::{
    ActiveScopes, KeyChord, SEQUENCE_TIMEOUT_SECONDS, ShortcutMap, dispatch, token_is_known,
};
use ferrogram_ui_spec::{CommandId as SpecCommandId, Shortcut, ShortcutScope, Shortcuts};

// ---------------------------------------------------------------------------
// Helpers (the `shortcuts_dispatch.rs` pattern)
// ---------------------------------------------------------------------------

#[allow(clippy::expect_used)] // test helper: a failure here is the bug
fn shortcut(tokens: &[&str], command: &str, scope: ShortcutScope) -> Shortcut {
    Shortcut {
        sequence: vec![tokens.iter().map(|t| (*t).to_owned()).collect()],
        command: SpecCommandId::parse(command).expect("id"),
        scope,
        section: None,
    }
}

#[allow(clippy::expect_used)] // test helper: a failure here is the bug
fn multi_shortcut(chords: &[&[&str]], command: &str, scope: ShortcutScope) -> Shortcut {
    Shortcut {
        sequence: chords
            .iter()
            .map(|chord| chord.iter().map(|t| (*t).to_owned()).collect())
            .collect(),
        command: SpecCommandId::parse(command).expect("id"),
        scope,
        section: None,
    }
}

#[allow(clippy::expect_used)] // test helper: a failure here is the bug
fn map_of(entries: Vec<Shortcut>) -> ShortcutMap {
    ShortcutMap::from_shortcuts(&Shortcuts { entries }).expect("map")
}

fn key_event(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

fn mods(ctrl: bool, shift: bool, alt: bool) -> Modifiers {
    Modifiers {
        ctrl,
        shift,
        alt,
        ..Default::default()
    }
}

/// One frame: `dispatch` first (before the widgets, ADR 0036), then an
/// optional focused text edit.
struct Frame {
    intent: Option<Intent>,
    events_left: usize,
    pending: bool,
}

#[allow(clippy::expect_used)] // test helper: a failure here is the bug
fn frame(
    ctx: &Context,
    map: &mut ShortcutMap,
    app: &AppCtx,
    scope: ShortcutScope,
    events: Vec<Event>,
    time: f64,
    with_text_edit: bool,
) -> Frame {
    let mut text = String::from("hello");
    let mut frame = Frame {
        intent: None,
        events_left: 0,
        pending: false,
    };
    let mut output = ctx.run_ui(
        RawInput {
            events,
            time: Some(time),
            ..Default::default()
        },
        |ctx| {
            frame.intent = dispatch(ctx, map, app, scope).expect("dispatch");
            frame.events_left = ctx.input(|input| input.events.len());
            frame.pending = map.pending();
            if with_text_edit {
                let mut edit_id = None;
                egui::CentralPanel::default().show(ctx, |ui| {
                    let output = egui::TextEdit::singleline(&mut text).show(ui);
                    edit_id = Some(output.response.id);
                });
                if let Some(id) = edit_id {
                    ctx.memory_mut(|memory| memory.request_focus(id));
                }
            }
        },
    );
    // No texture atlas in these tests: drop the delta deliberately
    // (epaint panics on an unapplied `TexturesDelta`).
    output.textures_delta.clear();
    frame
}

// ---------------------------------------------------------------------------
// Chord normalization
// ---------------------------------------------------------------------------

#[test]
fn modifiers_normalize_to_the_canonical_order_without_duplicates() {
    let chord = KeyChord::new(
        "C",
        vec!["Alt".into(), "Ctrl".into(), "Alt".into(), "Shift".into()],
    );
    assert_eq!(chord.key, "C");
    assert_eq!(chord.modifiers, vec!["Ctrl", "Shift", "Alt"]);
    // Any input order normalizes the same.
    let same = KeyChord::new("C", vec!["Shift".into(), "Alt".into(), "Ctrl".into()]);
    assert_eq!(chord, same);
}

#[test]
fn a_shifted_symbol_key_absorbs_shift_on_both_sides() {
    // File side: `Ctrl+Shift+<` and `Ctrl+<` are one chord.
    let with_shift = KeyChord::new("<", vec!["Ctrl".into(), "Shift".into()]);
    let without = KeyChord::new("<", vec!["Ctrl".into()]);
    assert_eq!(with_shift, without);
    // Same for `+` and `>`.
    assert_eq!(
        KeyChord::new("+", vec!["Ctrl".into(), "Shift".into()]),
        KeyChord::new("+", vec!["Ctrl".into()])
    );
    assert_eq!(
        KeyChord::new(">", vec!["Shift".into()]),
        KeyChord::new(">", vec![])
    );
    // A normal key keeps its Shift.
    assert_ne!(
        KeyChord::new("C", vec!["Ctrl".into(), "Shift".into()]),
        KeyChord::new("C", vec!["Ctrl".into()])
    );
}

#[test]
fn the_event_side_normalizes_like_the_file_side() {
    // `+` typed three ways: the Plus key, Shift+Equals, and the file's
    // `Ctrl++`. All one binding.
    let mut map = map_of(vec![shortcut(
        &["Ctrl", "+"],
        "edit.copy",
        ShortcutScope::Global,
    )]);
    let app = AppCtx::new(Settings::default());
    let ctx = Context::default();
    for (key, modifiers) in [
        (Key::Plus, mods(true, false, false)),
        (Key::Equals, mods(true, true, false)),
    ] {
        let result = frame(
            &ctx,
            &mut map,
            &app,
            ShortcutScope::Global,
            vec![key_event(key, modifiers)],
            0.0,
            false,
        );
        let intent = result.intent.expect("the binding fires");
        assert_eq!(intent.command.as_str(), "edit.copy");
    }
    // `<` typed as Shift+Comma with Ctrl.
    let mut map = map_of(vec![shortcut(
        &["Ctrl", "Shift", "<"],
        "edit.cut",
        ShortcutScope::Global,
    )]);
    let result = frame(
        &ctx,
        &mut map,
        &app,
        ShortcutScope::Global,
        vec![key_event(Key::Comma, mods(true, true, false))],
        1.0,
        false,
    );
    assert_eq!(
        result
            .intent
            .expect("`Ctrl+Shift+<` fires")
            .command
            .as_str(),
        "edit.cut"
    );
}

#[test]
fn a_chord_token_set_is_order_insensitive() {
    // `shortcuts.yaml` writes an unordered token set per chord.
    let mut map = map_of(vec![shortcut(
        &["G", "Ctrl"],
        "edit.copy",
        ShortcutScope::Global,
    )]);
    let app = AppCtx::new(Settings::default());
    let ctx = Context::default();
    let result = frame(
        &ctx,
        &mut map,
        &app,
        ShortcutScope::Global,
        vec![key_event(Key::G, mods(true, false, false))],
        0.0,
        false,
    );
    assert_eq!(result.intent.expect("fires").command.as_str(), "edit.copy");
}

// ---------------------------------------------------------------------------
// Load-time errors (ADR 0036: an unknown token is an error, never a gap)
// ---------------------------------------------------------------------------

#[test]
fn every_broken_table_load_is_an_error_naming_shortcuts_yaml() {
    // The last two carry a raw (unvalidated) command id: the map must
    // reject them at load, not dispatch them later.
    let broken = vec![
        multi_shortcut(&[], "edit.copy", ShortcutScope::Global),
        shortcut(&["Ctrl"], "edit.copy", ShortcutScope::Global),
        shortcut(&["Ctrl", "C", "V"], "edit.copy", ShortcutScope::Global),
        shortcut(&["Ctrl", "Wingdings"], "edit.copy", ShortcutScope::Global),
        Shortcut {
            sequence: vec![vec!["Ctrl".to_owned(), "S".to_owned()]],
            command: SpecCommandId::new("mystery.save"),
            scope: ShortcutScope::Global,
            section: None,
        },
        Shortcut {
            sequence: vec![vec!["Ctrl".to_owned(), "S".to_owned()]],
            command: SpecCommandId::new("not.an.id!"),
            scope: ShortcutScope::Global,
            section: None,
        },
    ];
    for entry in broken {
        let error = ShortcutMap::from_shortcuts(&Shortcuts {
            entries: vec![entry],
        })
        .expect_err("a broken table must not load");
        let text = error.to_string();
        assert!(text.contains("shortcuts.yaml"), "{text}");
    }
}

#[test]
fn the_token_table_knows_its_own_tokens_and_nothing_else() {
    for known in [
        "Ctrl",
        "Shift",
        "Alt",
        "A",
        "F1",
        "Esc",
        "Arrow Keys",
        "+",
        "<",
        ">",
    ] {
        assert!(token_is_known(known), "{known}");
    }
    for unknown in [
        "",
        "ctrl",
        "COMMAND",
        "Wingdings",
        "ArrowKeys",
        "Ctrl+G",
        "é",
    ] {
        assert!(!token_is_known(unknown), "{unknown}");
    }
}

// ---------------------------------------------------------------------------
// Trie behavior
// ---------------------------------------------------------------------------

#[test]
fn a_prefix_sequence_defers_and_fires_on_a_mismatch() {
    // `Alt+H` is a prefix of `Alt+H, F`: typing it must wait.
    let mut map = map_of(vec![
        shortcut(&["Alt", "H"], "view.zoom.in", ShortcutScope::Global),
        multi_shortcut(
            &[&["Alt", "H"], &["F"]],
            "view.zoom.out",
            ShortcutScope::Global,
        ),
    ]);
    let app = AppCtx::new(Settings::default());
    let ctx = Context::default();

    let first = frame(
        &ctx,
        &mut map,
        &app,
        ShortcutScope::Global,
        vec![key_event(Key::H, mods(false, false, true))],
        0.0,
        false,
    );
    assert!(first.intent.is_none(), "the prefix waits");
    assert!(first.pending);

    // A chord that continues nothing fires the held-back completion.
    // The mismatching chord itself is dropped (documented behavior;
    // see the P4-10 report): it is consumed with the completion.
    let second = frame(
        &ctx,
        &mut map,
        &app,
        ShortcutScope::Global,
        vec![key_event(Key::G, mods(true, false, false))],
        0.1,
        false,
    );
    assert_eq!(
        second
            .intent
            .expect("the deferred prefix fires")
            .command
            .as_str(),
        "view.zoom.in"
    );
    assert!(!second.pending);

    // The dropped chord is gone; typing it again starts fresh.
    let third = frame(
        &ctx,
        &mut map,
        &app,
        ShortcutScope::Global,
        vec![key_event(Key::G, mods(true, false, false))],
        0.2,
        false,
    );
    assert!(third.intent.is_none());
}

#[test]
fn the_longer_sequence_of_a_prefix_pair_completes() {
    let mut map = map_of(vec![
        shortcut(&["Alt", "H"], "view.zoom.in", ShortcutScope::Global),
        multi_shortcut(
            &[&["Alt", "H"], &["F"]],
            "view.zoom.out",
            ShortcutScope::Global,
        ),
    ]);
    let app = AppCtx::new(Settings::default());
    let ctx = Context::default();
    let result = frame(
        &ctx,
        &mut map,
        &app,
        ShortcutScope::Global,
        vec![
            key_event(Key::H, mods(false, false, true)),
            key_event(Key::F, mods(false, false, false)),
        ],
        0.0,
        false,
    );
    assert_eq!(
        result
            .intent
            .expect("the long sequence fires")
            .command
            .as_str(),
        "view.zoom.out"
    );
}

#[test]
fn a_prefix_sequence_fires_when_the_longer_one_times_out() {
    let mut map = map_of(vec![
        shortcut(&["Alt", "H"], "view.zoom.in", ShortcutScope::Global),
        multi_shortcut(
            &[&["Alt", "H"], &["F"]],
            "view.zoom.out",
            ShortcutScope::Global,
        ),
    ]);
    let app = AppCtx::new(Settings::default());
    let ctx = Context::default();
    let first = frame(
        &ctx,
        &mut map,
        &app,
        ShortcutScope::Global,
        vec![key_event(Key::H, mods(false, false, true))],
        0.0,
        false,
    );
    assert!(first.pending);

    // Before the timeout nothing fires.
    let early = frame(
        &ctx,
        &mut map,
        &app,
        ShortcutScope::Global,
        Vec::new(),
        1.0,
        false,
    );
    assert!(early.intent.is_none());
    assert!(early.pending);

    // After `SEQUENCE_TIMEOUT_SECONDS` the deferred completion fires
    // on a frame with no key events (ADR 0036).
    let late = frame(
        &ctx,
        &mut map,
        &app,
        ShortcutScope::Global,
        Vec::new(),
        SEQUENCE_TIMEOUT_SECONDS + 0.5,
        false,
    );
    assert_eq!(
        late.intent
            .expect("the timeout fires the prefix")
            .command
            .as_str(),
        "view.zoom.in"
    );
    assert!(!late.pending);
}

#[test]
fn resolve_keeps_half_typed_state_across_calls() {
    // `resolve` is timeless (P4-08 deviation): the state persists and
    // no timeout is applied to it.
    let mut map = map_of(vec![multi_shortcut(
        &[&["Alt", "H"], &["F"], &["O"]],
        "view.zoom.in",
        ShortcutScope::Global,
    )]);
    let active = ActiveScopes::default();
    let chord = |key: &str, alt: bool| {
        KeyChord::new(
            key,
            if alt {
                vec!["Alt".to_owned()]
            } else {
                Vec::new()
            },
        )
    };
    assert!(map.resolve(&[chord("H", true)], &active).unwrap().is_none());
    assert!(map.pending());
    assert_eq!(map.pending_display(), "Alt+H");
    assert!(
        map.resolve(&[chord("F", false)], &active)
            .unwrap()
            .is_none()
    );
    assert_eq!(map.pending_display(), "Alt+H, F");
    let intent = map
        .resolve(&[chord("O", false)], &active)
        .unwrap()
        .expect("the sequence completes");
    assert_eq!(intent.command.as_str(), "view.zoom.in");
    assert!(!map.pending());
}

#[test]
fn an_unmatched_chord_is_left_alone_and_consumes_nothing() {
    let mut map = map_of(vec![shortcut(
        &["Ctrl", "G"],
        "edit.copy",
        ShortcutScope::Global,
    )]);
    let app = AppCtx::new(Settings::default());
    let ctx = Context::default();
    let result = frame(
        &ctx,
        &mut map,
        &app,
        ShortcutScope::Global,
        vec![key_event(Key::Q, mods(true, false, false))],
        0.0,
        false,
    );
    assert!(result.intent.is_none());
    assert_eq!(result.events_left, 1, "the chord survives for the widgets");
}

#[test]
fn a_completed_sequence_with_no_active_binding_is_still_consumed() {
    // ADR 0036: the chord is swallowed even when its scope is off, so
    // a page shortcut cannot leak into a text field as typing.
    let mut map = map_of(vec![shortcut(
        &["Ctrl", "G"],
        "edit.copy",
        ShortcutScope::Page,
    )]);
    let app = AppCtx::new(Settings::default());
    let ctx = Context::default();
    let result = frame(
        &ctx,
        &mut map,
        &app,
        ShortcutScope::Global,
        vec![key_event(Key::G, mods(true, false, false))],
        0.0,
        false,
    );
    assert!(result.intent.is_none());
    assert_eq!(result.events_left, 0);
}

// ---------------------------------------------------------------------------
// Scope precedence (P4-14)
// ---------------------------------------------------------------------------

#[test]
fn the_most_specific_active_scope_wins() {
    let mut map = map_of(vec![
        shortcut(&["Ctrl", "G"], "edit.copy", ShortcutScope::Global),
        shortcut(&["Ctrl", "G"], "edit.cut", ShortcutScope::Window),
        shortcut(&["Ctrl", "G"], "edit.paste", ShortcutScope::Page),
        shortcut(&["Ctrl", "G"], "view.zoom.in", ShortcutScope::Ribbon),
        shortcut(&["Ctrl", "G"], "view.zoom.out", ShortcutScope::Text),
        shortcut(&["Ctrl", "G"], "page.next", ShortcutScope::Selection),
    ]);
    let chord = [KeyChord::new("G", vec!["Ctrl".to_owned()])];
    let cases: &[(ActiveScopes, &str)] = &[
        (ActiveScopes::default(), "edit.cut"),
        (
            ActiveScopes {
                page: true,
                ..ActiveScopes::default()
            },
            "edit.paste",
        ),
        (
            ActiveScopes {
                ribbon: true,
                ..ActiveScopes::default()
            },
            "view.zoom.in",
        ),
        (
            ActiveScopes {
                text: true,
                ..ActiveScopes::default()
            },
            "view.zoom.out",
        ),
        (
            ActiveScopes {
                selection: true,
                ..ActiveScopes::default()
            },
            "page.next",
        ),
        (
            ActiveScopes {
                page: true,
                ribbon: true,
                text: true,
                selection: true,
            },
            "view.zoom.out",
        ),
    ];
    for (active, expected) in cases {
        let intent = map
            .resolve(&chord, active)
            .unwrap()
            .unwrap_or_else(|| panic!("no binding for {active:?}"));
        assert_eq!(intent.command.as_str(), *expected, "for {active:?}");
    }
}

#[test]
fn active_scopes_always_include_global_and_window() {
    let active = ActiveScopes::default();
    assert!(active.contains(ShortcutScope::Global));
    assert!(active.contains(ShortcutScope::Window));
    assert!(!active.contains(ShortcutScope::Page));
    assert!(!active.contains(ShortcutScope::Text));
    assert!(!active.contains(ShortcutScope::Selection));
    assert!(!active.contains(ShortcutScope::Ribbon));
}

#[test]
fn an_exact_repeat_is_silent_and_a_conflict_warns_once() {
    let map = map_of(vec![
        shortcut(&["Ctrl", "G"], "edit.copy", ShortcutScope::Global),
        shortcut(&["Ctrl", "G"], "edit.copy", ShortcutScope::Global),
        shortcut(&["Ctrl", "G"], "edit.cut", ShortcutScope::Global),
        shortcut(&["Ctrl", "G"], "edit.paste", ShortcutScope::Global),
    ]);
    assert_eq!(map.len(), 1);
    assert_eq!(map.conflicts().len(), 2, "one line per dropped conflict");
}

// ---------------------------------------------------------------------------
// The text-edit defer rule (ADR 0036, amended at the P4-14 merge)
// ---------------------------------------------------------------------------

#[test]
fn egui_editing_chords_are_never_stolen_inside_a_text_edit() {
    // The chords egui's text edit owns: Ctrl+A/C/V/X/Z/Y/Backspace/
    // Delete and Ctrl+Shift+Z. They pass through unconsumed.
    let mut map = map_of(vec![
        shortcut(&["Ctrl", "A"], "edit.select_all", ShortcutScope::Text),
        shortcut(&["Ctrl", "C"], "edit.copy", ShortcutScope::Text),
        shortcut(&["Ctrl", "V"], "edit.paste", ShortcutScope::Text),
        shortcut(&["Ctrl", "X"], "edit.cut", ShortcutScope::Text),
        shortcut(&["Ctrl", "Z"], "edit.undo", ShortcutScope::Text),
        shortcut(&["Ctrl", "Y"], "edit.redo", ShortcutScope::Text),
        shortcut(
            &["Ctrl", "Backspace"],
            "edit.delete_back_word",
            ShortcutScope::Text,
        ),
        shortcut(&["Ctrl", "Delete"], "edit.delete_line", ShortcutScope::Text),
        multi_shortcut(&[&["Ctrl", "Shift", "Z"]], "edit.redo", ShortcutScope::Text),
    ]);
    let app = AppCtx::new(Settings::default());
    let ctx = Context::default();
    // One warm-up frame: the shortcut pass runs before the widgets,
    // so `text_edit_focused` comes from the previous frame (P4-14).
    let warm = frame(
        &ctx,
        &mut map,
        &app,
        ShortcutScope::Text,
        Vec::new(),
        -1.0,
        true,
    );
    assert!(warm.intent.is_none());
    for (key, modifiers) in [
        (Key::A, mods(true, false, false)),
        (Key::C, mods(true, false, false)),
        (Key::V, mods(true, false, false)),
        (Key::X, mods(true, false, false)),
        (Key::Z, mods(true, false, false)),
        (Key::Y, mods(true, false, false)),
        (Key::Backspace, mods(true, false, false)),
        (Key::Delete, mods(true, false, false)),
        (Key::Z, mods(true, true, false)),
    ] {
        let result = frame(
            &ctx,
            &mut map,
            &app,
            ShortcutScope::Text,
            vec![key_event(key, modifiers)],
            0.0,
            true,
        );
        assert!(result.intent.is_none(), "{key:?} must defer to egui");
        assert_eq!(result.events_left, 1, "{key:?} must reach the text edit");
    }
}

#[test]
fn other_bound_chords_dispatch_inside_a_text_edit() {
    // The P4-14 rule: only the chords in `defers_to_egui` defer.
    // `Ctrl+Shift+C` and `Alt` chords are ours even in a text edit.
    let mut map = map_of(vec![
        shortcut(
            &["Ctrl", "Shift", "C"],
            "tools.format_painter",
            ShortcutScope::Text,
        ),
        shortcut(&["Alt", "F6"], "view.zoom.in", ShortcutScope::Text),
    ]);
    let app = AppCtx::new(Settings::default());
    let ctx = Context::default();
    // Warm-up frame (see the test above): focus comes from the
    // previous frame.
    let warm = frame(
        &ctx,
        &mut map,
        &app,
        ShortcutScope::Text,
        Vec::new(),
        -1.0,
        true,
    );
    assert!(warm.intent.is_none());
    for (key, modifiers, expected) in [
        (Key::C, mods(true, true, false), "tools.format_painter"),
        (Key::F6, mods(false, false, true), "view.zoom.in"),
    ] {
        let result = frame(
            &ctx,
            &mut map,
            &app,
            ShortcutScope::Text,
            vec![key_event(key, modifiers)],
            0.0,
            true,
        );
        let intent = result.intent.expect("the chord dispatches");
        assert_eq!(intent.command.as_str(), expected);
    }
    // Plain typing is never matched or consumed.
    let result = frame(
        &ctx,
        &mut map,
        &app,
        ShortcutScope::Text,
        vec![
            key_event(Key::A, mods(false, false, false)),
            key_event(Key::B, mods(false, true, false)),
        ],
        1.0,
        true,
    );
    assert!(result.intent.is_none());
    assert_eq!(result.events_left, 2);
}

// ---------------------------------------------------------------------------
// Hostile tables (huge and deep)
// ---------------------------------------------------------------------------

#[test]
fn a_five_hundred_chord_sequence_builds_and_completes() {
    let long: Vec<Vec<String>> = (0..500)
        .map(|_| vec!["Alt".to_owned(), "H".to_owned()])
        .collect();
    let entry = Shortcut {
        sequence: long,
        command: SpecCommandId::parse("edit.copy").unwrap(),
        scope: ShortcutScope::Global,
        section: None,
    };
    let mut map = map_of(vec![entry]);
    assert_eq!(map.len(), 1);
    let active = ActiveScopes::default();
    let chord = [KeyChord::new("H", vec!["Alt".to_owned()])];
    for step in 0..499 {
        assert!(
            map.resolve(&chord, &active).unwrap().is_none(),
            "step {step} must wait"
        );
    }
    let intent = map.resolve(&chord, &active).unwrap().expect("completes");
    assert_eq!(intent.command.as_str(), "edit.copy");
}

#[test]
fn two_thousand_entries_over_few_commands_build() {
    // 2000 distinct sequences over 8 command ids: the map builds and
    // dispatches without a panic or a hang. The id interner sees 8
    // ids, not 2000 (see the P4-10 report's testing hazard).
    let letters: &[&str] = &[
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R",
        "S", "T", "U", "V", "W", "X", "Y", "Z",
    ];
    let digits: &[&str] = &["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
    let mut entries = Vec::new();
    'outer: for f in 1..=12 {
        for letter in letters {
            for digit in digits {
                if entries.len() == 2000 {
                    break 'outer;
                }
                let command = match entries.len() % 8 {
                    0 => "edit.copy",
                    1 => "edit.cut",
                    2 => "edit.paste",
                    3 => "edit.undo",
                    4 => "edit.redo",
                    5 => "view.zoom.in",
                    6 => "view.zoom.out",
                    _ => "page.next",
                };
                entries.push(Shortcut {
                    sequence: vec![
                        vec!["Alt".to_owned(), format!("F{f}")],
                        vec![(*letter).to_owned()],
                        vec![(*digit).to_owned()],
                    ],
                    command: SpecCommandId::parse(command).unwrap(),
                    scope: ShortcutScope::Global,
                    section: None,
                });
            }
        }
    }
    assert_eq!(entries.len(), 2000);
    let mut map = map_of(entries);
    assert_eq!(map.len(), 2000);
    assert!(map.conflicts().is_empty());
    // The first and a late entry dispatch.
    let active = ActiveScopes::default();
    let typed = [
        KeyChord::new("F1", vec!["Alt".to_owned()]),
        KeyChord::new("A", vec![]),
        KeyChord::new("0", vec![]),
    ];
    let intent = map.resolve(&typed, &active).unwrap().expect("entry 0");
    assert_eq!(intent.command.as_str(), "edit.copy");
    let late = [
        KeyChord::new("F2", vec!["Alt".to_owned()]),
        KeyChord::new("B", vec![]),
        KeyChord::new("5", vec![]),
    ];
    assert!(map.resolve(&late, &active).unwrap().is_some());
}

#[test]
fn a_table_with_many_modifier_duplicates_still_loads() {
    let entry = shortcut(
        &["Ctrl", "Ctrl", "Ctrl", "Shift", "Shift", "Alt", "Alt", "G"],
        "edit.copy",
        ShortcutScope::Global,
    );
    let map = map_of(vec![entry]);
    assert_eq!(map.len(), 1);
}

#[test]
fn an_empty_table_is_an_empty_map() {
    let map = map_of(Vec::new());
    assert!(map.is_empty());
    assert_eq!(map.len(), 0);
    assert!(!map.pending());
    assert!(map.conflicts().is_empty());
    assert_eq!(map.pending_chords().len(), 0);
}
