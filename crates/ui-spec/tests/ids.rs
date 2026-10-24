//! Property tests for the id parsers (P1-14 goal 4).
//!
//! [`CommandId::parse`] and [`FluentId::parse`] must accept exactly
//! the strings their `ui-spec/schema.json` patterns accept. The
//! reference is the `jsonschema` validator over a tiny schema holding
//! the same pattern, so the hand-rolled parsers cannot drift from the
//! schema unnoticed.

use larrygram_ui_spec::{CommandId, FluentId};
use proptest::prelude::*;

/// The `commandId` pattern of `ui-spec/schema.json`.
const COMMAND_ID_PATTERN: &str = r"^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$";

/// The `fluentId` pattern of `ui-spec/schema.json`.
const FLUENT_ID_PATTERN: &str = r"^[A-Za-z_][A-Za-z0-9_]*$";

/// The reference: does the schema `pattern` accept `s`?
#[allow(clippy::unwrap_used, clippy::expect_used)] // test helper; tests may panic
fn schema_pattern_allows(pattern: &str, s: &str) -> bool {
    let schema = serde_json::json!({ "type": "string", "pattern": pattern });
    let validator = jsonschema::validator_for(&schema).expect("a tiny schema compiles");
    validator.is_valid(&serde_json::Value::String(s.to_owned()))
}

/// Printable-ASCII strings of bounded length: the broad input class.
#[allow(clippy::unwrap_used)] // test helper; tests may panic
fn short_ascii() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[ -~]{0,16}").unwrap()
}

/// Identifier-shaped pieces, to make accepted ids common enough for
/// the comparison to be meaningful.
#[allow(clippy::unwrap_used)] // test helper; tests may panic
fn id_pieces() -> impl Strategy<Value = String> {
    proptest::string::string_regex("[A-Za-z0-9_]{0,4}").unwrap()
}

/// Structured candidates: pieces joined by separators, so both
/// grammars get near-misses (bad separators, empty tokens, ...).
#[allow(clippy::unwrap_used)] // test helper; tests may panic
fn candidates() -> impl Strategy<Value = String> {
    const SEPARATORS: [&str; 6] = [".", "..", "-", " ", "", "_"];
    prop_oneof![
        2 => short_ascii(),
        3 => proptest::collection::vec(id_pieces(), 1..5).prop_map(|pieces| {
            let mut out = String::new();
            for (i, piece) in pieces.iter().enumerate() {
                if i > 0 {
                    out.push_str(SEPARATORS[i % SEPARATORS.len()]);
                }
                out.push_str(piece);
            }
            out
        }),
    ]
}

proptest! {
    /// `CommandId::parse` accepts exactly the schema pattern's
    /// language (ADR 0006 grammar).
    #[test]
    fn command_id_parse_agrees_with_the_schema_pattern(s in candidates()) {
        prop_assert_eq!(
            CommandId::parse(&s).is_ok(),
            schema_pattern_allows(COMMAND_ID_PATTERN, &s),
            "disagreement on {:?}", s
        );
    }

    /// `FluentId::parse` accepts exactly `^[A-Za-z_][A-Za-z0-9_]*$`.
    #[test]
    fn fluent_id_parse_agrees_with_the_schema_pattern(s in candidates()) {
        prop_assert_eq!(
            FluentId::parse(&s).is_ok(),
            schema_pattern_allows(FLUENT_ID_PATTERN, &s),
            "disagreement on {:?}", s
        );
    }
}

/// Sanity: the reference and the parsers agree on the documented
/// examples, in both directions.
#[test]
fn parsers_and_reference_agree_on_the_examples() {
    for s in ["file.save", "arrange.align.left", "a.b"] {
        assert!(CommandId::parse(s).is_ok(), "{s}");
        assert!(schema_pattern_allows(COMMAND_ID_PATTERN, s), "{s}");
    }
    for s in ["file", "File.save", "file..save", "file.", ".save", ""] {
        assert!(CommandId::parse(s).is_err(), "{s:?}");
        assert!(!schema_pattern_allows(COMMAND_ID_PATTERN, s), "{s:?}");
    }
    for s in ["FileSave", "_a", "visCmdFileSave"] {
        assert!(FluentId::parse(s).is_ok(), "{s}");
        assert!(schema_pattern_allows(FLUENT_ID_PATTERN, s), "{s}");
    }
    for s in ["2Cut", "Cut.Menu", "Cut Menu", ""] {
        assert!(FluentId::parse(s).is_err(), "{s:?}");
        assert!(!schema_pattern_allows(FLUENT_ID_PATTERN, s), "{s:?}");
    }
}
