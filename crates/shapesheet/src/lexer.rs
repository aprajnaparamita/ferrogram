//! The formula lexer (ADR 0051).
//!
//! One formula is one line of source text without the leading `=`. The
//! lexer is bounded like every other part of the crate: input length and
//! token count have limits, and a limit is an [`Error::Syntax`], never a
//! panic (AGENTS.md section 6).
//!
//! The tokens carry what the parser assembles into [`crate::CellRef`]
//! and [`crate::NumberLiteral`] values. A number token holds the
//! magnitude only: a unit suffix is its own token, and the mixed
//! fraction `5 1/2` folds into one magnitude here. A quoted name
//! (`'2-C'`) scans as [`TokenKind::Ident`] with the name text as written
//! between the quotes; the parser reads the spelling from the source
//! span, so quote escapes survive.

use crate::error::{Error, Span, SyntaxKind};

/// The longest formula source text the parser accepts, in bytes.
pub const MAX_SOURCE_LEN: usize = 65_536;

/// The most tokens one formula may have (the end marker is not counted).
pub const MAX_TOKENS: usize = 4_096;

/// The deepest nested construct (`()`, `[]`, unary and `^` chains) the
/// parser accepts.
pub const MAX_NESTING: usize = 32;

/// One lexical token with its source span (ADR 0051).
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// Where the token sits in the source.
    pub span: Span,
    /// What the token is.
    pub kind: TokenKind,
}

/// What a token is (ADR 0051).
///
/// Identifiers, names and unit suffixes are carried as written; the
/// parser assembles them into [`crate::CellRef`] and
/// [`crate::NumberLiteral`] values.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum TokenKind {
    /// A number literal's magnitude (`2`, `0.5`, `1e-3`, `5 1/2` folded
    /// to 5.5). The unit suffix is a separate token.
    Number(f64),
    /// A quoted string body, unescaped (`abc` of `"abc"`).
    Str(String),
    /// An identifier or name part (`IF`, `Width`, `Sheet.4`'s `Sheet`).
    ///
    /// A quoted name carries its text as written between the quotes
    /// (`2-C` of `'2-C'`, doubled quotes not folded).
    Ident(String),
    /// `TRUE` or `FALSE` (any case).
    Bool(bool),
    /// A punctuation or operator character (`+`, `(`, `,`, `.`).
    ///
    /// `'`, `"` and `°` appear here only as the unit suffixes of a
    /// number (`5'`, `5"`, `5°`); everywhere else they open a quoted
    /// name or a string.
    Symbol(char),
    /// The end of the source.
    End,
}

/// Splits `source` into tokens.
///
/// Returns [`Error::Syntax`] with the offending [`Span`] on bad input or
/// when a limit ([`MAX_SOURCE_LEN`], [`MAX_TOKENS`]) is exceeded.
pub fn tokenize(source: &str) -> Result<Vec<Token>, Error> {
    Lexer::new(source).scan()
}

/// A half-open byte range over `source`.
const fn span_at(start: usize, end: usize) -> Span {
    Span::new(byte_offset(start), byte_offset(end))
}

/// Saturating `usize` to [`Span`]'s `u32` offsets.
const fn byte_offset(at: usize) -> u32 {
    if at > u32::MAX as usize {
        u32::MAX
    } else {
        at as u32
    }
}

/// The scanning state: the source, the byte offset and the tokens.
struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    tokens: Vec<Token>,
}

impl<'a> Lexer<'a> {
    /// A lexer over `source`, at its start.
    fn new(source: &'a str) -> Self {
        Self {
            src: source,
            pos: 0,
            tokens: Vec::new(),
        }
    }

    /// Scans every token, then the end marker.
    fn scan(mut self) -> Result<Vec<Token>, Error> {
        if self.src.len() > MAX_SOURCE_LEN {
            return Err(Error::Syntax {
                span: span_at(0, self.src.len()),
                kind: SyntaxKind::InputTooLong,
            });
        }
        loop {
            self.skip_whitespace();
            let start = self.pos;
            let Some(c) = self.peek() else {
                break;
            };
            match c {
                '0'..='9' => self.scan_number(start)?,
                '"' => {
                    if self.number_ends_at(start) {
                        self.bump();
                        self.push(span_at(start, self.pos), TokenKind::Symbol('"'))?;
                    } else {
                        self.scan_string(start)?;
                    }
                }
                '\'' => {
                    if self.number_ends_at(start) {
                        self.bump();
                        self.push(span_at(start, self.pos), TokenKind::Symbol('\''))?;
                    } else {
                        self.scan_quoted_name(start)?;
                    }
                }
                c if is_ident_start(c) => self.scan_ident(start)?,
                '+' | '-' | '*' | '/' | '^' | '&' | '=' | '<' | '>' | '(' | ')' | '[' | ']'
                | '.' | ',' | '!' | '%' | '°' => {
                    self.bump();
                    self.push(span_at(start, self.pos), TokenKind::Symbol(c))?;
                }
                _ => {
                    self.bump();
                    return Err(Error::Syntax {
                        span: span_at(start, self.pos),
                        kind: SyntaxKind::UnexpectedToken,
                    });
                }
            }
        }
        let end = self.pos;
        self.tokens.push(Token {
            span: span_at(end, end),
            kind: TokenKind::End,
        });
        Ok(self.tokens)
    }

    /// The character at the cursor, if any.
    fn peek(&self) -> Option<char> {
        char_at(self.src, self.pos)
    }

    /// Consumes one character.
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    /// Skips spaces, tabs and line breaks (a formula is one line, but
    /// the lexer does not enforce that).
    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.bump();
        }
    }

    /// True when the last token is a number ending at `at`: the case in
    /// which `'`, `"` and `°` are unit suffixes (`5'`, `5"`, `5°`).
    fn number_ends_at(&self, at: usize) -> bool {
        self.tokens.last().is_some_and(|t| {
            matches!(t.kind, TokenKind::Number(_)) && t.span.end == byte_offset(at)
        })
    }

    /// Appends one token, enforcing [`MAX_TOKENS`].
    fn push(&mut self, span: Span, kind: TokenKind) -> Result<(), Error> {
        if self.tokens.len() >= MAX_TOKENS {
            return Err(Error::Syntax {
                span,
                kind: SyntaxKind::TooManyTokens,
            });
        }
        self.tokens.push(Token { span, kind });
        Ok(())
    }

    /// An error covering `start .. end`.
    fn err(&self, start: usize, end: usize, kind: SyntaxKind) -> Error {
        Error::Syntax {
            span: span_at(start, end),
            kind,
        }
    }

    /// Scans `digits ["." digits] [exponent]` and a following mixed
    /// fraction (`5 1/2` is 5.5, ADR 0051).
    fn scan_number(&mut self, start: usize) -> Result<(), Error> {
        let mut at = self.pos;
        at = skip_digits(self.src, at);
        if char_at(self.src, at) == Some('.')
            && matches!(char_at(self.src, at + 1), Some(c) if c.is_ascii_digit())
        {
            at = skip_digits(self.src, at + 1);
        }
        // An exponent only when `e` really starts one (`2ed` is 2 with
        // the elapsed-days unit, not a number `2e`).
        if matches!(char_at(self.src, at), Some('e' | 'E')) {
            let mut exponent_end = at + 1;
            if matches!(char_at(self.src, exponent_end), Some('+' | '-')) {
                exponent_end += 1;
            }
            let digits_end = skip_digits(self.src, exponent_end);
            if digits_end > exponent_end {
                at = digits_end;
            }
        }
        let mut end = at;
        let mut value = self.float_at(start, end)?;
        if let Some((fraction, fraction_end)) = self.scan_mixed(end)? {
            value += fraction;
            end = fraction_end;
        }
        if !value.is_finite() {
            return Err(self.err(start, end, SyntaxKind::InvalidNumber));
        }
        self.pos = end;
        self.push(span_at(start, end), TokenKind::Number(value))
    }

    /// Looks for `whitespace digits "/" digits` after a number and folds
    /// it into that number (`5 1/2` is 5.5). Leaves the cursor alone;
    /// returns the fraction and the offset past it. A zero denominator
    /// is [`SyntaxKind::InvalidNumber`].
    fn scan_mixed(&self, after: usize) -> Result<Option<(f64, usize)>, Error> {
        let src = self.src;
        let mut at = after;
        let whitespace_start = at;
        while matches!(char_at(src, at), Some(c) if c.is_whitespace()) {
            at += 1;
        }
        if at == whitespace_start {
            return Ok(None);
        }
        let numerator_start = at;
        at = skip_digits(src, at);
        if at == numerator_start || char_at(src, at) != Some('/') {
            return Ok(None);
        }
        let denominator_start = at + 1;
        let end = skip_digits(src, denominator_start);
        if end == denominator_start {
            return Ok(None);
        }
        let numerator = self.float_at(numerator_start, denominator_start - 1)?;
        let denominator = self.float_at(denominator_start, end)?;
        if denominator == 0.0 {
            return Err(self.err(numerator_start, end, SyntaxKind::InvalidNumber));
        }
        Ok(Some((numerator / denominator, end)))
    }

    /// Parses `start .. end` as a finite `f64`.
    fn float_at(&self, start: usize, end: usize) -> Result<f64, Error> {
        let text = self.src.get(start..end).unwrap_or_default();
        match text.parse::<f64>() {
            Ok(value) if value.is_finite() => Ok(value),
            _ => Err(self.err(start, end, SyntaxKind::InvalidNumber)),
        }
    }

    /// Scans a word: `TRUE` and `FALSE` become [`TokenKind::Bool`],
    /// anything else [`TokenKind::Ident`].
    fn scan_ident(&mut self, start: usize) -> Result<(), Error> {
        while matches!(self.peek(), Some(c) if is_ident_continue(c)) {
            self.bump();
        }
        let end = self.pos;
        let text = self.src.get(start..end).unwrap_or_default();
        let kind = if text.eq_ignore_ascii_case("TRUE") {
            TokenKind::Bool(true)
        } else if text.eq_ignore_ascii_case("FALSE") {
            TokenKind::Bool(false)
        } else {
            TokenKind::Ident(text.to_owned())
        };
        self.push(span_at(start, end), kind)
    }

    /// Scans `"..."`, where a doubled quote is one quote (ADR 0051).
    fn scan_string(&mut self, start: usize) -> Result<(), Error> {
        self.bump();
        let mut text = String::new();
        loop {
            match self.peek() {
                None => return Err(self.err(start, self.pos, SyntaxKind::UnterminatedString)),
                Some('"') => {
                    self.bump();
                    if self.peek() == Some('"') {
                        text.push('"');
                        self.bump();
                    } else {
                        break;
                    }
                }
                Some(c) => {
                    text.push(c);
                    self.bump();
                }
            }
        }
        self.push(span_at(start, self.pos), TokenKind::Str(text))
    }

    /// Scans `'...'`, a name with nonstandard characters, where `''` is
    /// one quote (ADR 0051). The text keeps its spelling.
    fn scan_quoted_name(&mut self, start: usize) -> Result<(), Error> {
        self.bump();
        let mut text = String::new();
        loop {
            match self.peek() {
                None => return Err(self.err(start, self.pos, SyntaxKind::UnterminatedString)),
                Some('\'') => {
                    self.bump();
                    if self.peek() == Some('\'') {
                        text.push_str("''");
                        self.bump();
                    } else {
                        break;
                    }
                }
                Some(c) => {
                    text.push(c);
                    self.bump();
                }
            }
        }
        self.push(span_at(start, self.pos), TokenKind::Ident(text))
    }
}

/// The character at byte `at`, if `at` is on a character boundary.
fn char_at(src: &str, at: usize) -> Option<char> {
    src.get(at..).and_then(|rest| rest.chars().next())
}

/// Byte offset past the run of ASCII digits starting at `at`.
fn skip_digits(src: &str, at: usize) -> usize {
    let mut at = at;
    while matches!(char_at(src, at), Some(c) if c.is_ascii_digit()) {
        at += 1;
    }
    at
}

/// An identifier starts with a letter or `_` (a quoted name covers the
/// names with other characters).
fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

/// An identifier continues with a letter, a digit or `_`.
fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<TokenKind> {
        tokenize(source)
            .expect("the source lexes")
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    fn error(source: &str) -> Error {
        tokenize(source).expect_err("the source does not lex")
    }

    /// The [`SyntaxKind`] the source fails with.
    fn syntax_kind(source: &str) -> SyntaxKind {
        match error(source) {
            Error::Syntax { kind, .. } => kind,
            other => panic!("expected a syntax error, got {other:?}"),
        }
    }

    #[test]
    fn a_formula_is_numbers_symbols_and_an_end() {
        assert_eq!(
            kinds("1+1"),
            vec![
                TokenKind::Number(1.0),
                TokenKind::Symbol('+'),
                TokenKind::Number(1.0),
                TokenKind::End
            ]
        );
    }

    #[test]
    fn a_number_keeps_its_magnitude_only() {
        assert_eq!(
            kinds("2 0.5 1e-3 1.7E-4"),
            vec![
                TokenKind::Number(2.0),
                TokenKind::Number(0.5),
                TokenKind::Number(0.001),
                TokenKind::Number(0.00017),
                TokenKind::End
            ]
        );
    }

    #[test]
    fn a_mixed_fraction_folds_into_one_magnitude() {
        assert_eq!(kinds("5 1/2")[0], TokenKind::Number(5.5));
        // The fraction needs whitespace in front of it: `51/2` is a
        // division like any other.
        assert_eq!(
            kinds("51/2"),
            vec![
                TokenKind::Number(51.0),
                TokenKind::Symbol('/'),
                TokenKind::Number(2.0),
                TokenKind::End
            ]
        );
    }

    #[test]
    fn an_exponent_only_starts_with_a_digit() {
        // `2ed` is 2 with the elapsed-days unit, `2e3` is 2000.
        assert_eq!(
            kinds("2ed"),
            vec![
                TokenKind::Number(2.0),
                TokenKind::Ident(String::from("ed")),
                TokenKind::End
            ]
        );
        assert_eq!(kinds("2e3")[0], TokenKind::Number(2000.0));
    }

    #[test]
    fn a_string_keeps_its_unescaped_body() {
        // The source `"say ""hi"""` holds `say "hi"`.
        assert_eq!(
            kinds("\"say \"\"hi\"\"\"")[0],
            TokenKind::Str(String::from("say \"hi\""))
        );
        assert_eq!(kinds("\"\"")[0], TokenKind::Str(String::new()));
    }

    #[test]
    fn a_quoted_name_keeps_its_text_as_written() {
        assert_eq!(kinds("'2-C'")[0], TokenKind::Ident(String::from("2-C")));
        // `''` is one quote inside a name and is not folded.
        assert_eq!(kinds("'a''b'")[0], TokenKind::Ident(String::from("a''b")));
    }

    #[test]
    fn true_and_false_are_booleans_in_any_case() {
        assert_eq!(
            kinds("TRUE false"),
            vec![
                TokenKind::Bool(true),
                TokenKind::Bool(false),
                TokenKind::End
            ]
        );
    }

    #[test]
    fn a_quote_right_after_a_number_is_a_unit_suffix() {
        assert_eq!(
            kinds("5'"),
            vec![
                TokenKind::Number(5.0),
                TokenKind::Symbol('\''),
                TokenKind::End
            ]
        );
        assert_eq!(
            kinds("5\""),
            vec![
                TokenKind::Number(5.0),
                TokenKind::Symbol('"'),
                TokenKind::End
            ]
        );
        assert_eq!(
            kinds("5°"),
            vec![
                TokenKind::Number(5.0),
                TokenKind::Symbol('°'),
                TokenKind::End
            ]
        );
        // With a gap the quote opens a name or a string instead.
        assert_eq!(kinds(r#"5 "x""#)[1], TokenKind::Str(String::from("x")));
        assert_eq!(kinds("5 'x'")[1], TokenKind::Ident(String::from("x")));
    }

    #[test]
    fn a_unit_suffix_is_a_word_next_to_the_number() {
        assert_eq!(
            kinds("10MM"),
            vec![
                TokenKind::Number(10.0),
                TokenKind::Ident(String::from("MM")),
                TokenKind::End
            ]
        );
        assert_eq!(
            kinds("0DA"),
            vec![
                TokenKind::Number(0.0),
                TokenKind::Ident(String::from("DA")),
                TokenKind::End
            ]
        );
    }

    #[test]
    fn every_symbol_is_its_own_token() {
        assert_eq!(
            kinds("<> <= >= = + - * / ^ & ( ) [ ] . , ! %"),
            vec![
                TokenKind::Symbol('<'),
                TokenKind::Symbol('>'),
                TokenKind::Symbol('<'),
                TokenKind::Symbol('='),
                TokenKind::Symbol('>'),
                TokenKind::Symbol('='),
                TokenKind::Symbol('='),
                TokenKind::Symbol('+'),
                TokenKind::Symbol('-'),
                TokenKind::Symbol('*'),
                TokenKind::Symbol('/'),
                TokenKind::Symbol('^'),
                TokenKind::Symbol('&'),
                TokenKind::Symbol('('),
                TokenKind::Symbol(')'),
                TokenKind::Symbol('['),
                TokenKind::Symbol(']'),
                TokenKind::Symbol('.'),
                TokenKind::Symbol(','),
                TokenKind::Symbol('!'),
                TokenKind::Symbol('%'),
                TokenKind::End
            ]
        );
    }

    #[test]
    fn a_token_carries_its_span() {
        let tokens = tokenize("Width*2").expect("the source lexes");
        let (first, rest) = tokens.split_first().expect("one token");
        assert_eq!(first.span, Span::new(0, 5));
        assert_eq!(first.kind, TokenKind::Ident(String::from("Width")));
        assert_eq!(rest.len(), 3);
    }

    #[test]
    fn an_unterminated_string_is_an_error() {
        let bad = error(r#""abc"#);
        assert_eq!(
            bad,
            Error::Syntax {
                span: Span::new(0, 4),
                kind: SyntaxKind::UnterminatedString
            }
        );
        let bad_name = error("'abc");
        assert_eq!(
            bad_name,
            Error::Syntax {
                span: Span::new(0, 4),
                kind: SyntaxKind::UnterminatedString
            }
        );
    }

    #[test]
    fn an_unknown_character_is_an_error() {
        assert_eq!(
            error("1 $ 2"),
            Error::Syntax {
                span: Span::new(2, 3),
                kind: SyntaxKind::UnexpectedToken
            }
        );
    }

    #[test]
    fn a_number_that_overflows_is_an_error() {
        assert_eq!(syntax_kind("1e400"), SyntaxKind::InvalidNumber);
        // A mixed fraction over a zero denominator is the same error.
        assert_eq!(syntax_kind("5 1/0"), SyntaxKind::InvalidNumber);
    }

    #[test]
    fn the_input_length_limit_is_a_syntax_error() {
        let long = "1".repeat(MAX_SOURCE_LEN + 1);
        assert_eq!(
            error(&long),
            Error::Syntax {
                span: Span::new(0, byte_offset(long.len())),
                kind: SyntaxKind::InputTooLong
            }
        );
        // Exactly at the limit is fine.
        let exact = format!("\"{}\"", "a".repeat(MAX_SOURCE_LEN - 2));
        assert!(tokenize(&exact).is_ok());
    }

    #[test]
    fn the_token_count_limit_is_a_syntax_error() {
        let many = "1 ".repeat(MAX_TOKENS + 1);
        assert_eq!(syntax_kind(&many), SyntaxKind::TooManyTokens);
        let just_enough = "1 ".repeat(MAX_TOKENS);
        assert_eq!(tokenize(&just_enough).map(|t| t.len()), Ok(MAX_TOKENS + 1));
    }

    #[test]
    fn the_limits_are_ordered() {
        const {
            assert!(MAX_SOURCE_LEN > 0);
            assert!(MAX_TOKENS > 0);
            assert!(MAX_NESTING > 0);
            assert!(MAX_NESTING < MAX_TOKENS);
        }
    }

    #[test]
    fn every_token_kind_exists() {
        let kinds = [
            TokenKind::Number(0.0),
            TokenKind::Str(String::new()),
            TokenKind::Ident(String::from("Width")),
            TokenKind::Bool(false),
            TokenKind::Symbol('+'),
            TokenKind::End,
        ];
        assert_eq!(kinds.len(), 6);
    }
}
