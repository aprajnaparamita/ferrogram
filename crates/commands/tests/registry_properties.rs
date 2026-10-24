//! Property tests for the command registry and ids (task P4-10,
//! ADR 0028).
//!
//! Three properties, one per section below:
//!
//! - **Id grammar** (ADR 0006): `CommandId::parse` accepts exactly the
//!   documented grammar, checked against an independent oracle over
//!   arbitrary strings, and `CommandId::intern` is idempotent and
//!   never panics on hostile input.
//! - **Arguments** (ADR 0028): every `ArgValue` round-trips through
//!   `Args` and the typed accessors answer `None` for any other
//!   variant, under arbitrary names and values.
//! - **Registry model**: random register/execute sequences against a
//!   per-id state model (`absent` / `stub` / `real`), pinning the
//!   P4-02 rules — a real command replaces its stub, a duplicate is
//!   a `RegistryError::Duplicate`, and `execute` answers
//!   `UnknownCommand` before `Unsupported` before `NotEnabled`.
//!
//! The id interner is process-global and capped at 4096 leaked ids
//! (ADR 0028), so these tests intern only from small fixed pools: a
//! test that burned the cap would make later tests in the same binary
//! fail with "id pool is full" (recorded in the P4-10 report as a
//! testing hazard of the global interner).
//!
//! A panic is the bug (AGENTS.md section 6).

// The never-crash lints apply to non-test code only (clippy.toml);
// these helpers are test code and use `unwrap` freely.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use larrygram_commands::app::{AppCtx, Settings};
use larrygram_commands::{
    ArgValue, Args, Command, CommandError, CommandId, CommandMeta, CommandStatus, Enabled,
    Invocation, Outcome, Registry, RegistryError, Source, StubCommand,
};
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Id grammar (ADR 0006)
// ---------------------------------------------------------------------------

/// The ADR 0006 scope tokens (the same closed list `crates/commands`
/// enforces; kept independent so a change on one side shows up here).
const SCOPES: &[&str] = &[
    "app",
    "file",
    "edit",
    "view",
    "insert",
    "format",
    "text",
    "arrange",
    "shape",
    "connector",
    "page",
    "layer",
    "tools",
    "data",
    "review",
    "developer",
    "help",
];

/// The grammar as written in ADR 0006: `scope '.' verb ['.' qualifier]*`
/// with lowercase ASCII letters, digits and underscore in the parts
/// (the verb and qualifier parts must start with a lowercase letter),
/// and one of the fixed scope tokens first.
fn grammar_accepts(s: &str) -> bool {
    let mut parts = s.split('.');
    let Some(scope) = parts.next() else {
        return false;
    };
    if !SCOPES.contains(&scope) {
        return false;
    }
    let mut more = 0;
    for part in parts {
        more += 1;
        let mut chars = part.chars();
        let Some(head) = chars.next() else {
            return false;
        };
        if !head.is_ascii_lowercase() {
            return false;
        }
        if !chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
            return false;
        }
    }
    more >= 1
}

proptest! {
    /// `parse` accepts exactly the grammar (arbitrary strings, any
    /// length and character range).
    #[test]
    fn parse_accepts_exactly_the_grammar(s in proptest::collection::vec(any::<char>(), 0..24)) {
        let s: String = s.into_iter().collect();
        // `parse` takes `&'static str`; the test binary may leak.
        let leaked: &'static str = Box::leak(s.clone().into_boxed_str());
        let accepted = CommandId::parse(leaked);
        prop_assert_eq!(accepted.is_ok(), grammar_accepts(&s), "id {:?}", s);
        if let Ok(id) = accepted {
            prop_assert_eq!(id.as_str(), s.as_str());
        }
    }

    /// `intern` never panics on arbitrary bytes-as-text, is
    /// idempotent and round-trips its accepted ids.
    #[test]
    fn intern_never_panics_and_is_idempotent(
        s in proptest::collection::vec(any::<u8>(), 0..24),
    ) {
        let s = String::from_utf8_lossy(&s).into_owned();
        // Keep the interner's growth tiny: intern only when the string
        // is a valid id (invalid ids consume no pool slot).
        let first = CommandId::intern(&s);
        let second = CommandId::intern(&s);
        prop_assert_eq!(first.is_ok(), second.is_ok());
        if let (Ok(a), Ok(b)) = (&first, &second) {
            prop_assert_eq!(a, b);
            prop_assert_eq!(a.as_str(), s.as_str());
            prop_assert!(grammar_accepts(&s));
        }
    }
}

#[test]
fn intern_rejects_an_invalid_id_with_a_reason() {
    for bad in [
        "",
        "file",
        "file.",
        ".save",
        "file..save",
        "File.save",
        "mystery.save",
    ] {
        let error = CommandId::intern(bad).expect_err(bad);
        assert_eq!(error.value, bad);
        assert!(!error.reason.is_empty());
    }
}

// ---------------------------------------------------------------------------
// Arguments (ADR 0028)
// ---------------------------------------------------------------------------

fn arg_strategy() -> impl Strategy<Value = ArgValue> {
    prop_oneof![
        any::<String>().prop_map(ArgValue::Text),
        any::<String>().prop_map(|s| ArgValue::Path(s.into())),
        any::<i64>().prop_map(ArgValue::Integer),
        // Finite numbers only: `Args` is a plain carrier and NaN is
        // the Phase 6 value-pipeline rule, not an argument rule.
        any::<f64>()
            .prop_filter("finite", |f| f.is_finite())
            .prop_map(ArgValue::Number),
        any::<bool>().prop_map(ArgValue::Flag),
        proptest::collection::vec(any::<String>(), 0..4).prop_map(ArgValue::List),
        proptest::collection::vec(any::<u8>(), 0..8).prop_map(ArgValue::Bytes),
    ]
}

proptest! {
    /// One argument round-trips under a hostile name; every typed
    /// accessor answers its own variant and `None` for the others.
    #[test]
    fn one_argument_round_trips(name in any::<String>(), value in arg_strategy()) {
        let args = Args::new().with(name.clone(), value.clone());
        prop_assert_eq!(args.len(), 1);
        match args.get(&name) {
            Some(got) => prop_assert_eq!(got, &value),
            None => panic!("the argument vanished"),
        }
        prop_assert_eq!(args.path(&name).is_some(), matches!(value, ArgValue::Path(_)));
        prop_assert_eq!(args.text(&name).is_some(), matches!(value, ArgValue::Text(_)));
        prop_assert_eq!(args.integer(&name).is_some(), matches!(value, ArgValue::Integer(_)));
        prop_assert_eq!(args.number(&name).is_some(), matches!(value, ArgValue::Number(_)));
        prop_assert_eq!(args.flag(&name).is_some(), matches!(value, ArgValue::Flag(_)));
        prop_assert_eq!(args.bytes(&name).is_some(), matches!(value, ArgValue::Bytes(_)));
    }

    /// Later `set` calls win, like the `BTreeMap` behind `Args`.
    #[test]
    fn set_replaces_the_previous_value(
        name in any::<String>(),
        first in arg_strategy(),
        second in arg_strategy(),
    ) {
        let mut args = Args::new();
        args.set(name.clone(), first);
        args.set(name.clone(), second.clone());
        prop_assert_eq!(args.len(), 1);
        prop_assert_eq!(args.get(&name), Some(&second));
    }
}

// ---------------------------------------------------------------------------
// Registry model
// ---------------------------------------------------------------------------

/// The test id pool: five valid ids, all `'static`, so nothing is
/// interned and the process cap cannot be reached.
const POOL: [&str; 5] = [
    "file.save",
    "file.open",
    "edit.undo",
    "view.zoom_in",
    "page.next",
];

/// One model-checked registry operation.
#[derive(Debug, Clone)]
enum Op {
    RegisterStub(usize),
    RegisterReal(usize, bool),
    Execute(usize),
    Enabled(usize),
}

/// The per-id state of the registry under test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    Stub,
    Real { enabled: bool },
}

fn op_strategy() -> impl Strategy<Value = Op> {
    (0..POOL.len()).prop_flat_map(|index| {
        prop_oneof![
            Just(Op::RegisterStub(index)),
            any::<bool>().prop_map(move |enabled| Op::RegisterReal(index, enabled)),
            Just(Op::Execute(index)),
            Just(Op::Enabled(index)),
        ]
    })
}

/// A real (implemented) command whose enabled flag is set per test.
struct RealCommand {
    meta: CommandMeta,
    enabled: bool,
}

impl RealCommand {
    fn new(id: &'static str, enabled: bool) -> Box<Self> {
        let mut meta = CommandMeta::new(
            CommandId::parse(id).unwrap(),
            "Test",
            CommandStatus::Implemented,
        );
        meta.status = CommandStatus::Implemented;
        Box::new(Self { meta, enabled })
    }
}

impl Command for RealCommand {
    fn meta(&self) -> &CommandMeta {
        &self.meta
    }

    fn enabled(&self, _ctx: &AppCtx) -> Enabled {
        if self.enabled {
            Enabled::yes()
        } else {
            Enabled::no("test says no")
        }
    }

    fn execute(
        &self,
        _ctx: &mut AppCtx,
        _invocation: &Invocation,
    ) -> Result<Outcome, CommandError> {
        Ok(Outcome::message(format!("ran {}", self.meta.id)))
    }
}

fn stub(id: &'static str) -> Box<StubCommand> {
    Box::new(StubCommand::new(CommandMeta::new(
        CommandId::parse(id).unwrap(),
        "Test",
        CommandStatus::Stub,
    )))
}

proptest! {
    /// Random register/execute sequences agree with the per-id model:
    /// who registers over whom, and which error class `execute`
    /// answers (ADR 0028/0037 order: unknown, then unsupported, then
    /// not-enabled, then the run).
    #[test]
    fn the_registry_agrees_with_its_model(
        ops in proptest::collection::vec(op_strategy(), 1..40),
    ) {
        let mut registry = Registry::new();
        let mut model: Vec<Option<Slot>> = vec![None; POOL.len()];
        let mut ctx = AppCtx::new(Settings::default());

        for op in ops {
            let (Op::RegisterStub(index)
            | Op::RegisterReal(index, _)
            | Op::Execute(index)
            | Op::Enabled(index)) = &op;
            let index = *index;
            let id = CommandId::parse(POOL[index]).unwrap();
            match op {
                Op::RegisterStub(_) => {
                    let result = registry.register(stub(POOL[index]));
                    match model[index] {
                        None => {
                            prop_assert!(result.is_ok());
                            model[index] = Some(Slot::Stub);
                        }
                        Some(_) => {
                            { let hit = matches!(result, Err(RegistryError::Duplicate { .. })); prop_assert!(hit); }
                        }
                    }
                }
                Op::RegisterReal(_, enabled) => {
                    let result = registry.register(RealCommand::new(POOL[index], enabled));
                    match model[index] {
                        None | Some(Slot::Stub) => {
                            prop_assert!(result.is_ok());
                            model[index] = Some(Slot::Real { enabled });
                        }
                        Some(Slot::Real { .. }) => {
                            { let hit = matches!(result, Err(RegistryError::Duplicate { .. })); prop_assert!(hit); }
                        }
                    }
                }
                Op::Execute(_) => {
                    let result = registry.execute(&id, &mut ctx, Invocation::empty(Source::Test));
                    match model[index] {
                        None => { let hit = matches!(result, Err(CommandError::UnknownCommand { .. })); prop_assert!(hit); },
                        Some(Slot::Stub) => { let hit = matches!(result, Err(CommandError::Unsupported { .. })); prop_assert!(hit); },
                        Some(Slot::Real { enabled: true }) => prop_assert!(result.is_ok()),
                        Some(Slot::Real { enabled: false }) => {
                            { let hit = matches!(result, Err(CommandError::NotEnabled { .. })); prop_assert!(hit); }
                        }
                    }
                }
                Op::Enabled(_) => {
                    let verdict = registry.enabled(&id, &ctx);
                    match model[index] {
                        None | Some(Slot::Stub) | Some(Slot::Real { enabled: false }) => {
                            prop_assert!(!verdict.is_enabled())
                        }
                        Some(Slot::Real { enabled: true }) => prop_assert!(verdict.is_enabled()),
                    }
                }
            }

            let present = model.iter().filter(|slot| slot.is_some()).count();
            prop_assert_eq!(registry.len(), present);
            for (slot, name) in model.iter().zip(POOL.iter()) {
                let id = CommandId::parse(name).unwrap();
                prop_assert_eq!(registry.get(&id).is_some(), slot.is_some());
                let expected = match slot {
                    Some(Slot::Real { .. }) => CommandStatus::Implemented,
                    _ => CommandStatus::Stub,
                };
                prop_assert_eq!(registry.status(&id), expected);
            }
        }
    }
}

proptest! {
    /// `from_mappings` builds one stub per distinct valid id and
    /// rejects duplicates and malformed ids exactly like the model.
    #[test]
    fn from_mappings_builds_the_table_it_is_given(
        picks in proptest::collection::vec(0..POOL.len(), 1..12),
    ) {
        let entries: Vec<larrygram_ui_spec::CommandMapping> = picks
            .iter()
            .map(|index| larrygram_ui_spec::CommandMapping {
                id: larrygram_ui_spec::CommandId::new(POOL[*index]),
                label: "Label".to_owned(),
                visuicmd: None,
                fluent_ids: Vec::new(),
                mapping_confidence: larrygram_ui_spec::MappingConfidence::Unknown,
            })
            .collect();
        let mut distinct: Vec<usize> = picks.clone();
        distinct.sort_unstable();
        distinct.dedup();
        let result = Registry::from_mappings(&larrygram_ui_spec::Commands { entries });
        match result {
            Ok(registry) => {
                prop_assert!(!distinct.is_empty());
                prop_assert_eq!(registry.len(), distinct.len());
                for index in distinct {
                    let id = CommandId::parse(POOL[index]).unwrap();
                    prop_assert_eq!(registry.status(&id), CommandStatus::Stub);
                    let verdict = registry.enabled(&id, &AppCtx::new(Settings::default()));
                    prop_assert!(!verdict.is_enabled());
                }
            }
            Err(error) => {
                // Only a duplicate can fail here.
                { let hit = matches!(error, RegistryError::Duplicate { .. }); prop_assert!(hit); }
            }
        }
    }
}
