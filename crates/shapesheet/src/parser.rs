//! The formula parser (ADR 0051).
//!
//! [`parse`] turns one formula source string into an [`Expr`], with spans
//! into the source for error reporting. The grammar, the reference forms
//! and the input limits are decided in ADR 0051; the tree is
//! [`crate::ast`].
//!
//! Parsing never panics: bad input is [`Error::Syntax`] with a span, and
//! nesting past [`MAX_NESTING`] is a syntax error before any recursion
//! grows (AGENTS.md section 6).
//!
//! Two rules the EBNF of ADR 0051 leaves open, resolved here:
//!
//! - `%` is the unary percentage operator of `about-operators.md`
//!   (`37%` is 0.37), written before or after its operand. The `%` unit
//!   spelling of `about-units-of-measure.md` is written only in the
//!   implicit form `5 [%, d]`, where it is kept on the literal.
//! - A mixed fraction folds in the lexer (`5 1/2` is 5.5) and a unit
//!   suffix follows the number, in that order: `5 1/2 in`.
//!
//! Everything else is kept as written: a reference's parts, a function
//! name, a number's unit and its measurement-system flag. An argument
//! list has no omissions: `F(a,,b)` is a spanned syntax error (the
//! corpus writes one only inside the unknown `FORMATEX`).
use crate::ast::{BinaryOp, Expr, ExprKind, Literal, NumberLiteral, UnaryOp};
use crate::error::{Error, Span, SyntaxKind};
use crate::formula::{CellRef, Formula};
use crate::lexer::{MAX_NESTING, Token, TokenKind, tokenize};

/// Parses one formula source text into an expression tree.
///
/// The source has no leading `=`, as [`Formula`] stores it. The limits
/// are [`crate::lexer::MAX_SOURCE_LEN`] (checked while tokenizing) and
/// [`MAX_NESTING`]; over either is a [`Error::Syntax`], never a panic.
pub fn parse(source: &str) -> Result<Expr, Error> {
    let tokens = tokenize(source)?;
    let mut parser = Parser {
        src: source,
        tokens,
        at: 0,
    };
    let expr = parser.parse_expr(0)?;
    match parser.peek() {
        None | Some(TokenKind::End) => Ok(expr),
        Some(_) => Err(parser.error_here(SyntaxKind::UnexpectedToken)),
    }
}

/// Parses the source text of a [`Formula`] into an expression tree.
///
/// Same contract as [`parse`]; this wrapper is the seam the recalc pass
/// uses (ADR 0055).
pub fn parse_formula(formula: &Formula) -> Result<Expr, Error> {
    parse(formula.as_str())
}

/// The alternative spelling of a comparison operator
/// (`about-operators.md`: `_GT_`, `_GE_`, `_LT_`, `_LE_`, `_EQ_`, `_NE_`).
fn alt_operator(name: &str) -> Option<BinaryOp> {
    if name.eq_ignore_ascii_case("_GT_") {
        Some(BinaryOp::Gt)
    } else if name.eq_ignore_ascii_case("_GE_") {
        Some(BinaryOp::Ge)
    } else if name.eq_ignore_ascii_case("_LT_") {
        Some(BinaryOp::Lt)
    } else if name.eq_ignore_ascii_case("_LE_") {
        Some(BinaryOp::Le)
    } else if name.eq_ignore_ascii_case("_EQ_") {
        Some(BinaryOp::Eq)
    } else if name.eq_ignore_ascii_case("_NE_") {
        Some(BinaryOp::Ne)
    } else {
        None
    }
}

/// The measurement-system flag of the implicit unit form: one of
/// `A`, `D`, `E`, `P`, `T` in either case (`about-units-of-measure.md`).
fn is_flag(c: char) -> bool {
    matches!(c.to_ascii_lowercase(), 'a' | 'd' | 'e' | 'p' | 't')
}

/// A binary node over already-parsed operands, spanning both.
fn binary(op: BinaryOp, left: Expr, right: Expr) -> Expr {
    let span = Span::new(left.span.start, right.span.end);
    Expr {
        span,
        kind: ExprKind::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
    }
}

/// One reference segment: its text as written and its dot-separated
/// name pieces.
struct Segment {
    /// The whole segment (`Sheet.4`, `Geometry1.X2`, `Masters[Bearing]`).
    text: String,
    /// The pieces between the dots (`Geometry1`, `X2`).
    pieces: Vec<String>,
}

/// What [`Parser::take_implicit_unit`] found.
struct ImplicitUnit {
    /// The unit as written.
    unit: Option<String>,
    /// The measurement-system flag as written.
    flag: Option<char>,
    /// Offset past the closing bracket.
    end: u32,
}

/// The cursor over the token stream.
struct Parser<'a> {
    src: &'a str,
    tokens: Vec<Token>,
    at: usize,
}

impl<'a> Parser<'a> {
    /// The kind of the current token.
    fn peek(&self) -> Option<&TokenKind> {
        self.tokens.get(self.at).map(|t| &t.kind)
    }

    /// The kind of the token after the current one.
    fn peek_second(&self) -> Option<&TokenKind> {
        self.tokens.get(self.at + 1).map(|t| &t.kind)
    }

    /// Consumes the current token.
    fn bump_token(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.at).cloned();
        if token.is_some() {
            self.at += 1;
        }
        token
    }

    /// The span of the current token (the end marker's at the end).
    fn current_span(&self) -> Span {
        match self.tokens.get(self.at).or_else(|| self.tokens.last()) {
            Some(token) => token.span,
            None => Span::empty(offset(self.src.len())),
        }
    }

    /// The span of the last consumed token.
    fn previous_span(&self) -> Span {
        match self.at.checked_sub(1).and_then(|i| self.tokens.get(i)) {
            Some(token) => token.span,
            None => Span::empty(0),
        }
    }

    /// The source text a span covers.
    fn text_of(&self, span: Span) -> &'a str {
        let src = self.src;
        src.get(span.start as usize..span.end as usize)
            .unwrap_or_default()
    }

    /// The source text of the current token.
    fn current_text(&self) -> &'a str {
        self.text_of(self.current_span())
    }

    /// An error at the current token.
    fn error_here(&self, kind: SyntaxKind) -> Error {
        Error::Syntax {
            span: self.current_span(),
            kind,
        }
    }

    /// An error covering `span`.
    fn error_at(&self, span: Span, kind: SyntaxKind) -> Error {
        Error::Syntax { span, kind }
    }

    /// Refuses to descend past [`MAX_NESTING`].
    fn enter(&self, depth: usize) -> Result<(), Error> {
        if depth > MAX_NESTING {
            Err(self.error_here(SyntaxKind::NestingTooDeep))
        } else {
            Ok(())
        }
    }

    /// True when the current token is a quoted name (`'2-C'`).
    fn peek_is_quoted(&self) -> bool {
        self.current_text().starts_with('\'')
    }

    /// True when the current token is an alternative comparison
    /// operator (`_GT_` and family).
    fn peek_is_alt_operator(&self) -> bool {
        match self.peek() {
            Some(TokenKind::Ident(name)) => alt_operator(name).is_some(),
            _ => false,
        }
    }

    /// The expression grammar: one `expr` of ADR 0051.
    fn parse_expr(&mut self, depth: usize) -> Result<Expr, Error> {
        self.parse_equality(depth)
    }

    /// `comparison`, the lowest precedence (ADR 0051): the equality
    /// operators bind looser than the ordering ones.
    fn parse_equality(&mut self, depth: usize) -> Result<Expr, Error> {
        let mut left = self.parse_relational(depth)?;
        while let Some(op) = self.take_equality_operator() {
            let right = self.parse_relational(depth)?;
            left = binary(op, left, right);
        }
        Ok(left)
    }

    /// The equality operator at the cursor and its width in tokens.
    fn equality_operator(&self) -> Option<(BinaryOp, usize)> {
        match (self.peek(), self.peek_second()) {
            (Some(TokenKind::Symbol('=')), _) => Some((BinaryOp::Eq, 1)),
            (Some(TokenKind::Symbol('<')), Some(TokenKind::Symbol('>'))) => Some((BinaryOp::Ne, 2)),
            (Some(TokenKind::Ident(name)), _) => match alt_operator(name) {
                Some(op @ (BinaryOp::Eq | BinaryOp::Ne)) => Some((op, 1)),
                _ => None,
            },
            _ => None,
        }
    }

    /// Consumes `=`, `<>`, `_EQ_` or `_NE_`, if present.
    fn take_equality_operator(&mut self) -> Option<BinaryOp> {
        let (op, width) = self.equality_operator()?;
        for _ in 0..width {
            self.bump_token();
        }
        Some(op)
    }

    /// The ordering comparisons, one precedence level up.
    fn parse_relational(&mut self, depth: usize) -> Result<Expr, Error> {
        let mut left = self.parse_concat(depth)?;
        while let Some(op) = self.take_relational_operator() {
            let right = self.parse_concat(depth)?;
            left = binary(op, left, right);
        }
        Ok(left)
    }

    /// The ordering operator at the cursor and its width in tokens.
    ///
    /// A bare `<` in front of a `>` is not an operator: that pair is
    /// `<>`, the equality operator [`Self::parse_equality`] takes.
    fn relational_operator(&self) -> Option<(BinaryOp, usize)> {
        match (self.peek(), self.peek_second()) {
            (Some(TokenKind::Symbol('<')), Some(TokenKind::Symbol('>'))) => None,
            (Some(TokenKind::Symbol('<')), Some(TokenKind::Symbol('='))) => Some((BinaryOp::Le, 2)),
            (Some(TokenKind::Symbol('<')), _) => Some((BinaryOp::Lt, 1)),
            (Some(TokenKind::Symbol('>')), Some(TokenKind::Symbol('='))) => Some((BinaryOp::Ge, 2)),
            (Some(TokenKind::Symbol('>')), _) => Some((BinaryOp::Gt, 1)),
            (Some(TokenKind::Ident(name)), _) => match alt_operator(name) {
                Some(op @ (BinaryOp::Gt | BinaryOp::Ge | BinaryOp::Lt | BinaryOp::Le)) => {
                    Some((op, 1))
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// Consumes `<`, `<=`, `>`, `>=`, `_LT_`, `_LE_`, `_GT_` or `_GE_`.
    fn take_relational_operator(&mut self) -> Option<BinaryOp> {
        let (op, width) = self.relational_operator()?;
        for _ in 0..width {
            self.bump_token();
        }
        Some(op)
    }

    /// `&`, string concatenation.
    fn parse_concat(&mut self, depth: usize) -> Result<Expr, Error> {
        let mut left = self.parse_additive(depth)?;
        while matches!(self.peek(), Some(TokenKind::Symbol('&'))) {
            self.bump_token();
            let right = self.parse_additive(depth)?;
            left = binary(BinaryOp::Concat, left, right);
        }
        Ok(left)
    }

    /// `+` and `-`.
    fn parse_additive(&mut self, depth: usize) -> Result<Expr, Error> {
        let mut left = self.parse_multiplicative(depth)?;
        loop {
            let op = match self.peek() {
                Some(TokenKind::Symbol('+')) => BinaryOp::Add,
                Some(TokenKind::Symbol('-')) => BinaryOp::Sub,
                _ => break,
            };
            self.bump_token();
            let right = self.parse_multiplicative(depth)?;
            left = binary(op, left, right);
        }
        Ok(left)
    }

    /// `*` and `/`.
    fn parse_multiplicative(&mut self, depth: usize) -> Result<Expr, Error> {
        let mut left = self.parse_power(depth)?;
        loop {
            let op = match self.peek() {
                Some(TokenKind::Symbol('*')) => BinaryOp::Mul,
                Some(TokenKind::Symbol('/')) => BinaryOp::Div,
                _ => break,
            };
            self.bump_token();
            let right = self.parse_power(depth)?;
            left = binary(op, left, right);
        }
        Ok(left)
    }

    /// `^`, exponentiation, right-associative (ADR 0051).
    fn parse_power(&mut self, depth: usize) -> Result<Expr, Error> {
        let left = self.parse_unary(depth)?;
        if matches!(self.peek(), Some(TokenKind::Symbol('^'))) {
            self.bump_token();
            let right = self.parse_power(depth + 1)?;
            return Ok(binary(BinaryOp::Pow, left, right));
        }
        Ok(left)
    }

    /// The unary operators `+`, `-` and `%` (either side for `%`).
    fn parse_unary(&mut self, depth: usize) -> Result<Expr, Error> {
        self.enter(depth)?;
        let start = self.current_span().start;
        let op = match self.peek() {
            Some(TokenKind::Symbol('+')) => Some(UnaryOp::Plus),
            Some(TokenKind::Symbol('-')) => Some(UnaryOp::Minus),
            Some(TokenKind::Symbol('%')) => Some(UnaryOp::Percent),
            _ => None,
        };
        if let Some(op) = op {
            self.bump_token();
            let operand = self.parse_unary(depth + 1)?;
            let span = Span::new(start, operand.span.end);
            return Ok(Expr {
                span,
                kind: ExprKind::Unary {
                    op,
                    operand: Box::new(operand),
                },
            });
        }
        self.parse_postfix(depth)
    }

    /// `%` written after its operand (`37%` is 0.37).
    fn parse_postfix(&mut self, depth: usize) -> Result<Expr, Error> {
        let mut expr = self.parse_primary(depth)?;
        while matches!(self.peek(), Some(TokenKind::Symbol('%'))) {
            let span = Span::new(expr.span.start, self.current_span().end);
            self.bump_token();
            expr = Expr {
                span,
                kind: ExprKind::Unary {
                    op: UnaryOp::Percent,
                    operand: Box::new(expr),
                },
            };
        }
        Ok(expr)
    }

    /// Literals, calls, references and parenthesised expressions.
    fn parse_primary(&mut self, depth: usize) -> Result<Expr, Error> {
        let start = self.current_span().start;
        let kind = self.peek().cloned();
        match kind {
            None => Err(self.error_here(SyntaxKind::UnexpectedEnd)),
            Some(TokenKind::End) => Err(self.error_here(SyntaxKind::UnexpectedEnd)),
            Some(TokenKind::Number(_)) => self.parse_number(),
            Some(TokenKind::Str(text)) => {
                let span = self.current_span();
                self.bump_token();
                Ok(Expr {
                    span,
                    kind: ExprKind::Literal(Literal::Str(text)),
                })
            }
            Some(TokenKind::Bool(value)) => {
                let span = self.current_span();
                self.bump_token();
                Ok(Expr {
                    span,
                    kind: ExprKind::Literal(Literal::Bool(value)),
                })
            }
            Some(TokenKind::Symbol('(')) => {
                self.bump_token();
                let inner = self.parse_expr(depth + 1)?;
                if matches!(self.peek(), Some(TokenKind::Symbol(')'))) {
                    let span = Span::new(start, self.current_span().end);
                    self.bump_token();
                    Ok(Expr {
                        span,
                        kind: inner.kind,
                    })
                } else if self.at_end() {
                    Err(self.error_here(SyntaxKind::UnexpectedEnd))
                } else {
                    Err(self.error_here(SyntaxKind::UnexpectedToken))
                }
            }
            Some(TokenKind::Ident(_)) => self.parse_ident_expression(depth),
            Some(_) => Err(self.error_here(SyntaxKind::UnexpectedToken)),
        }
    }

    /// True at the end marker or past the last token.
    fn at_end(&self) -> bool {
        matches!(self.peek(), None | Some(TokenKind::End))
    }

    /// A function call or a cell reference.
    ///
    /// A call whose result names a sheet or a cell is a reference part
    /// (`LISTSHEETREF()!User.Direction`, `Sheet.30!SheetRef()`), not a
    /// call of its own.
    fn parse_ident_expression(&mut self, depth: usize) -> Result<Expr, Error> {
        let starts_a_call =
            !self.peek_is_quoted() && matches!(self.peek_second(), Some(TokenKind::Symbol('(')));
        if starts_a_call && !self.call_continues_a_reference() {
            self.parse_call(depth)
        } else {
            self.parse_reference(depth)
        }
    }

    /// True when the `NAME(` at the cursor has a `!` or `.` after its
    /// closing parenthesis: the call is one part of a reference.
    fn call_continues_a_reference(&self) -> bool {
        let Some(after) = self.call_end() else {
            return false;
        };
        matches!(
            self.tokens.get(after).map(|t| &t.kind),
            Some(TokenKind::Symbol('!' | '.'))
        )
    }

    /// The index just past the closing `)` of the `NAME(` at the cursor.
    fn call_end(&self) -> Option<usize> {
        if !matches!(self.peek(), Some(TokenKind::Ident(_)))
            || !matches!(self.peek_second(), Some(TokenKind::Symbol('(')))
        {
            return None;
        }
        let mut at = self.at + 2;
        let mut level = 1usize;
        while level > 0 {
            match self.tokens.get(at).map(|t| &t.kind) {
                Some(TokenKind::Symbol('(')) => level += 1,
                Some(TokenKind::Symbol(')')) => level -= 1,
                Some(_) => {}
                None => return None,
            }
            at += 1;
        }
        Some(at)
    }

    /// `NAME(args)`: the parentheses are required even without
    /// arguments (`about-functions.md`), and no argument may be left
    /// out (`F(a,,b)` is an error).
    fn parse_call(&mut self, depth: usize) -> Result<Expr, Error> {
        let name_token = self
            .bump_token()
            .ok_or_else(|| self.error_here(SyntaxKind::UnexpectedEnd))?;
        let name = self.text_of(name_token.span).to_owned();
        self.bump_token(); // `(`
        let mut args = Vec::new();
        if !matches!(self.peek(), Some(TokenKind::Symbol(')'))) {
            loop {
                args.push(self.parse_expr(depth + 1)?);
                if matches!(self.peek(), Some(TokenKind::Symbol(','))) {
                    self.bump_token();
                    continue;
                }
                break;
            }
        }
        if matches!(self.peek(), Some(TokenKind::Symbol(')'))) {
            let end = self.current_span().end;
            self.bump_token();
            Ok(Expr {
                span: Span::new(name_token.span.start, end),
                kind: ExprKind::Call { name, args },
            })
        } else if self.at_end() {
            Err(self.error_here(SyntaxKind::UnexpectedEnd))
        } else {
            Err(self.error_here(SyntaxKind::UnexpectedToken))
        }
    }

    /// A reference in any of its forms (ADR 0054), kept as written: the
    /// sheet parts before the `!` separators, and the section and name
    /// parts of the last segment.
    fn parse_reference(&mut self, depth: usize) -> Result<Expr, Error> {
        let start = self.current_span().start;
        let mut sheets: Vec<String> = Vec::new();
        let mut segment = self.parse_segment(depth)?;
        while matches!(self.peek(), Some(TokenKind::Symbol('!'))) {
            self.bump_token();
            sheets.push(segment.text);
            segment = self.parse_segment(depth)?;
        }
        let end = self.previous_span().end;
        let span = Span::new(start, end);
        let mut pieces = segment.pieces.into_iter();
        let Some(first) = pieces.next() else {
            return Err(self.error_at(span, SyntaxKind::InvalidReference));
        };
        let names: Vec<String> = pieces.collect();
        let (section, names) = if names.is_empty() {
            (None, vec![first])
        } else {
            (Some(first), names)
        };
        Ok(Expr {
            span,
            kind: ExprKind::Ref(CellRef {
                sheets,
                section,
                names,
            }),
        })
    }

    /// One segment: dot-separated name pieces.
    fn parse_segment(&mut self, depth: usize) -> Result<Segment, Error> {
        let mut pieces = Vec::new();
        loop {
            pieces.push(self.parse_name_piece(depth)?);
            if matches!(self.peek(), Some(TokenKind::Symbol('.'))) {
                self.bump_token();
                continue;
            }
            break;
        }
        let text = pieces.join(".");
        Ok(Segment { text, pieces })
    }

    /// One name piece: a word, a number or a call, with its bracket
    /// suffix (`Font[3]`, `Masters[Bearing]`, `Pages[Sheet-3]`).
    ///
    /// A call used as a part keeps its own text (`Sheet.30!SheetRef()`);
    /// what such a part means is a resolution rule (ADR 0054).
    fn parse_name_piece(&mut self, depth: usize) -> Result<String, Error> {
        let mut text = match self.peek() {
            Some(TokenKind::Ident(_))
                if matches!(self.peek_second(), Some(TokenKind::Symbol('('))) =>
            {
                let call = self.parse_call(depth)?;
                self.text_of(call.span).to_owned()
            }
            Some(TokenKind::Ident(_)) | Some(TokenKind::Number(_)) => {
                let text = self.current_text().to_owned();
                self.bump_token();
                text
            }
            _ => return Err(self.error_here(SyntaxKind::InvalidReference)),
        };
        while matches!(self.peek(), Some(TokenKind::Symbol('['))) {
            text.push_str(&self.take_bracket_group(depth)?);
        }
        Ok(text)
    }

    /// Takes `[ ... ]` with balanced brackets and returns its text as
    /// written. Bracket contents are opaque: a name may hold anything.
    fn take_bracket_group(&mut self, depth: usize) -> Result<String, Error> {
        let start = self.current_span().start;
        let mut level = 0usize;
        loop {
            if self.at_end() {
                return Err(self.error_here(SyntaxKind::UnexpectedEnd));
            }
            if matches!(self.peek(), Some(TokenKind::Symbol('['))) {
                level += 1;
                self.enter(depth + level)?;
                self.bump_token();
                continue;
            }
            if matches!(self.peek(), Some(TokenKind::Symbol(']'))) {
                let end = self.current_span().end;
                self.bump_token();
                level = level.saturating_sub(1);
                if level == 0 {
                    let span = Span::new(start, end);
                    return Ok(self.text_of(span).to_owned());
                }
                continue;
            }
            self.bump_token();
        }
    }

    /// A number literal: the magnitude, the unit suffix and, for the
    /// implicit form `5 [in, d]`, the measurement-system flag.
    fn parse_number(&mut self) -> Result<Expr, Error> {
        let token = self
            .bump_token()
            .ok_or_else(|| self.error_here(SyntaxKind::UnexpectedEnd))?;
        let start = token.span.start;
        let mut end = token.span.end;
        let value = match token.kind {
            TokenKind::Number(value) => value,
            _ => return Err(self.error_at(token.span, SyntaxKind::UnexpectedToken)),
        };
        let mut unit = None;
        let mut flag = None;
        match self.peek().cloned() {
            Some(TokenKind::Ident(_)) if !self.peek_is_alt_operator() && !self.peek_is_quoted() => {
                let (text, span) = self.take_unit_word()?;
                unit = Some(text);
                end = span.end;
            }
            Some(TokenKind::Symbol(c @ ('\'' | '"' | '°'))) => {
                unit = Some(c.to_string());
                end = self.current_span().end;
                self.bump_token();
            }
            Some(TokenKind::Symbol('[')) => {
                let implicit = self.take_implicit_unit()?;
                unit = implicit.unit;
                flag = implicit.flag;
                end = implicit.end;
            }
            _ => {}
        }
        Ok(Expr {
            span: Span::new(start, end),
            kind: ExprKind::Literal(Literal::Number(NumberLiteral { value, unit, flag })),
        })
    }

    /// A unit spelled with words: `in`, `MM`, or a compound like
    /// `FEET/INCH` when its parts are written next to each other.
    fn take_unit_word(&mut self) -> Result<(String, Span), Error> {
        let first = self.current_span();
        let mut text = self.current_text().to_owned();
        let mut span = first;
        self.bump_token();
        loop {
            let slash = self.current_span();
            if !matches!(self.peek(), Some(TokenKind::Symbol('/'))) || slash.start != span.end {
                break;
            }
            let Some(TokenKind::Ident(_)) = self.peek_second() else {
                break;
            };
            let Some(word) = self.tokens.get(self.at + 1).map(|t| t.span) else {
                break;
            };
            if word.start != slash.end {
                break;
            }
            self.bump_token(); // `/`
            text.push('/');
            text.push_str(self.text_of(word));
            self.bump_token(); // the word
            span = Span::new(first.start, word.end);
        }
        Ok((text, span))
    }

    /// The implicit form `number[unit, flag]` (`5[deg,A]`).
    fn take_implicit_unit(&mut self) -> Result<ImplicitUnit, Error> {
        self.bump_token(); // `[`
        let unit = match self.peek().cloned() {
            Some(TokenKind::Ident(_)) if !self.peek_is_alt_operator() && !self.peek_is_quoted() => {
                self.take_unit_word()?.0
            }
            Some(TokenKind::Symbol(c @ ('%' | '\'' | '"' | '°'))) => {
                self.bump_token();
                c.to_string()
            }
            _ => return Err(self.error_here(SyntaxKind::InvalidUnit)),
        };
        let mut flag = None;
        if matches!(self.peek(), Some(TokenKind::Symbol(','))) {
            self.bump_token();
            flag = Some(self.take_flag()?);
        }
        if matches!(self.peek(), Some(TokenKind::Symbol(']'))) {
            let end = self.current_span().end;
            self.bump_token();
            Ok(ImplicitUnit {
                unit: Some(unit),
                flag,
                end,
            })
        } else if self.at_end() {
            Err(self.error_here(SyntaxKind::UnexpectedEnd))
        } else {
            Err(self.error_here(SyntaxKind::InvalidUnit))
        }
    }

    /// The one-letter measurement-system flag of the implicit form.
    fn take_flag(&mut self) -> Result<char, Error> {
        let text = self.current_text();
        let mut chars = text.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) if is_flag(c) => {
                self.bump_token();
                Ok(c)
            }
            _ => Err(self.error_here(SyntaxKind::InvalidUnit)),
        }
    }
}

/// Saturating `usize` to [`Span`]'s `u32` offsets.
const fn offset(at: usize) -> u32 {
    if at > u32::MAX as usize {
        u32::MAX
    } else {
        at as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn number(value: f64, unit: Option<&str>, flag: Option<char>) -> Expr {
        Expr {
            span: Span::default(),
            kind: ExprKind::Literal(Literal::Number(NumberLiteral {
                value,
                unit: unit.map(str::to_owned),
                flag,
            })),
        }
    }

    fn binary_of(op: BinaryOp, left: Expr, right: Expr) -> Expr {
        Expr {
            span: Span::default(),
            kind: ExprKind::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            },
        }
    }

    fn tree(source: &str) -> Expr {
        // `Expr` equality ignores spans (ADR 0051), so the parse itself
        // is the shape to compare.
        parse(source).expect("the source parses")
    }

    fn error(source: &str) -> Error {
        parse(source).expect_err("the source does not parse")
    }

    fn syntax_kind(source: &str) -> SyntaxKind {
        match error(source) {
            Error::Syntax { kind, .. } => kind,
            other => panic!("expected a syntax error, got {other:?}"),
        }
    }

    #[test]
    fn a_formula_is_one_expression() {
        assert_eq!(
            tree("Width*0.5"),
            binary_of(BinaryOp::Mul, tree("Width"), number(0.5, None, None))
        );
    }

    #[test]
    fn an_empty_formula_is_an_error() {
        assert_eq!(syntax_kind(""), SyntaxKind::UnexpectedEnd);
    }

    #[test]
    fn trailing_input_is_an_error() {
        assert_eq!(syntax_kind("1 2"), SyntaxKind::UnexpectedToken);
        assert_eq!(syntax_kind("Width PinX"), SyntaxKind::UnexpectedToken);
    }

    #[test]
    fn a_number_keeps_its_unit_and_flag() {
        assert_eq!(tree("10MM"), number(10.0, Some("MM"), None));
        assert_eq!(tree("0DA"), number(0.0, Some("DA"), None));
        assert_eq!(tree("0.5 in"), number(0.5, Some("in"), None));
        assert_eq!(tree("5'"), number(5.0, Some("'"), None));
        assert_eq!(tree("5\""), number(5.0, Some("\""), None));
        assert_eq!(tree("5°"), number(5.0, Some("°"), None));
    }

    #[test]
    fn the_implicit_form_keeps_the_flag() {
        assert_eq!(tree("5[deg,A]"), number(5.0, Some("deg"), Some('A')));
        assert_eq!(tree("2 [in, d]"), number(2.0, Some("in"), Some('d')));
        assert_eq!(tree("5[%]"), number(5.0, Some("%"), None));
    }

    #[test]
    fn a_mixed_fraction_folds_and_still_takes_a_unit() {
        assert_eq!(tree("5 1/2"), number(5.5, None, None));
        assert_eq!(tree("5 1/2 in"), number(5.5, Some("in"), None));
    }

    #[test]
    fn a_compound_unit_glues_when_written_together() {
        assert_eq!(tree("3FEET/INCH"), number(3.0, Some("FEET/INCH"), None));
        // With a gap the word after `/` is an operand, not a unit.
        assert!(parse("3 FEET / INCH").is_ok());
    }

    #[test]
    fn percent_is_the_unary_percentage_operator() {
        let expected = Expr {
            span: Span::default(),
            kind: ExprKind::Unary {
                op: UnaryOp::Percent,
                operand: Box::new(number(37.0, None, None)),
            },
        };
        assert_eq!(tree("37%"), expected);
        assert_eq!(tree("%37"), expected);
    }

    #[test]
    fn every_reference_form_of_the_table_parses() {
        for source in [
            "Width",
            "Wheel!Angle",
            "Frame.2!Height",
            "Char.Size[3]",
            "Scratch.B3",
            "Sheet.12!FillForegnd",
            "Masters[Bearing]!Hub!Geometry1.X1",
            "ThePage!User.Anchor_Point",
            "Pages[Sheet-3]!Sheet.4!BeginX",
            "Styles!Ledger!LineColor",
            "TheDoc!DocScale",
            "'2-C'!LineColor",
        ] {
            assert!(parse(source).is_ok(), "{source} should parse");
        }
    }

    #[test]
    fn a_reference_keeps_its_parts_as_written() {
        let expr = tree("Masters[Bearing]!Hub!Geometry1.X1");
        let ExprKind::Ref(r) = expr.kind else {
            panic!("expected a reference");
        };
        assert_eq!(
            r.sheets,
            vec![String::from("Masters[Bearing]"), String::from("Hub")]
        );
        assert_eq!(r.section.as_deref(), Some("Geometry1"));
        assert_eq!(r.names, vec![String::from("X1")]);
    }

    #[test]
    fn a_quoted_name_keeps_its_quotes_in_the_part() {
        let expr = tree("'2-C'!LineColor");
        let ExprKind::Ref(r) = expr.kind else {
            panic!("expected a reference");
        };
        assert_eq!(r.sheets, vec![String::from("'2-C'")]);
        assert_eq!(r.names, vec![String::from("LineColor")]);
    }

    #[test]
    fn a_dotted_name_splits_into_section_and_names() {
        let expr = tree("Controls.Row_1.Y");
        let ExprKind::Ref(r) = expr.kind else {
            panic!("expected a reference");
        };
        assert_eq!(r.section.as_deref(), Some("Controls"));
        assert_eq!(r.names, vec![String::from("Row_1"), String::from("Y")]);
    }

    #[test]
    fn a_call_needs_parentheses() {
        let expr = tree("DEFAULTEVENT()");
        assert_eq!(
            expr,
            Expr {
                span: Span::default(),
                kind: ExprKind::Call {
                    name: String::from("DEFAULTEVENT"),
                    args: Vec::new()
                }
            }
        );
        // Without parentheses the name is a reference.
        assert!(matches!(tree("Width").kind, ExprKind::Ref(_)));
    }

    #[test]
    fn an_unknown_function_name_is_still_a_call() {
        let expr = tree("THEMEVAL(1, 2)");
        let ExprKind::Call { name, args } = expr.kind else {
            panic!("expected a call");
        };
        assert_eq!(name, "THEMEVAL");
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn an_unknown_reference_form_is_still_a_reference() {
        let expr = tree("Some.New.Fangled[3].Name");
        assert!(matches!(expr.kind, ExprKind::Ref(_)));
    }

    #[test]
    fn a_call_can_be_a_reference_part() {
        // A sheet part that is a call (the corpus writes these through
        // `LISTSHEETREF` and `CONTAINERSHEETREF`).
        let expr = tree("LISTSHEETREF()!User.MSVSDLISTDIRECTION");
        let ExprKind::Ref(r) = expr.kind else {
            panic!("expected a reference");
        };
        assert_eq!(r.sheets, vec![String::from("LISTSHEETREF()")]);
        assert_eq!(r.section.as_deref(), Some("User"));
        assert_eq!(r.names, vec![String::from("MSVSDLISTDIRECTION")]);
        // A name part that is a call.
        let expr = tree("Sheet.30!SheetRef()");
        let ExprKind::Ref(r) = expr.kind else {
            panic!("expected a reference");
        };
        assert_eq!(r.names, vec![String::from("SheetRef()")]);
        // On its own the same text is a call, not a reference.
        assert!(matches!(tree("SheetRef()").kind, ExprKind::Call { .. }));
        // The arguments keep their own spelling.
        let expr = tree(r#"CONTAINERSHEETREF(1,"Swimlane")!User.VISHEADINGTEXT"#);
        let ExprKind::Ref(r) = expr.kind else {
            panic!("expected a reference");
        };
        assert_eq!(
            r.sheets,
            vec![String::from("CONTAINERSHEETREF(1,\"Swimlane\")")]
        );
    }

    #[test]
    fn the_alternative_operators_map_to_the_same_six() {
        assert_eq!(tree("5 _GT_ 2"), tree("5>2"));
        assert_eq!(tree("5 _GE_ 2"), tree("5>=2"));
        assert_eq!(tree("5 _LT_ 2"), tree("5<2"));
        assert_eq!(tree("5 _LE_ 2"), tree("5<=2"));
        assert_eq!(tree("5 _EQ_ 2"), tree("5=2"));
        assert_eq!(tree("5 _NE_ 2"), tree("5<>2"));
    }

    #[test]
    fn strings_unfold_their_doubled_quotes() {
        let expr = tree("\"say \"\"hi\"\"\"");
        assert_eq!(
            expr,
            Expr {
                span: Span::default(),
                kind: ExprKind::Literal(Literal::Str(String::from("say \"hi\"")))
            }
        );
    }

    #[test]
    fn a_malformed_formula_is_a_spanned_syntax_error() {
        assert_eq!(
            error("1+"),
            Error::Syntax {
                span: Span::new(2, 2),
                kind: SyntaxKind::UnexpectedEnd
            }
        );
        assert_eq!(
            error("(Width"),
            Error::Syntax {
                span: Span::new(6, 6),
                kind: SyntaxKind::UnexpectedEnd
            }
        );
        assert_eq!(
            error("Width)"),
            Error::Syntax {
                span: Span::new(5, 6),
                kind: SyntaxKind::UnexpectedToken
            }
        );
    }

    #[test]
    fn an_omitted_argument_is_an_error() {
        assert_eq!(syntax_kind("FORMATEX(a,b,,c)"), SyntaxKind::UnexpectedToken);
        assert_eq!(syntax_kind("IF(a,)"), SyntaxKind::UnexpectedToken);
        assert_eq!(syntax_kind("SETF(,)"), SyntaxKind::UnexpectedToken);
    }

    #[test]
    fn a_malformed_unit_is_an_invalid_unit() {
        assert_eq!(syntax_kind("5[]"), SyntaxKind::InvalidUnit);
        assert_eq!(syntax_kind("5[in,]"), SyntaxKind::InvalidUnit);
        assert_eq!(syntax_kind("5[in,XX]"), SyntaxKind::InvalidUnit);
        assert_eq!(syntax_kind("5[in,1]"), SyntaxKind::InvalidUnit);
        assert_eq!(syntax_kind("5[in d]"), SyntaxKind::InvalidUnit);
    }

    #[test]
    fn a_malformed_reference_is_an_invalid_reference() {
        assert_eq!(syntax_kind("Width.!PinX"), SyntaxKind::InvalidReference);
        // `!` cannot start a primary at all.
        assert_eq!(syntax_kind("!Width"), SyntaxKind::UnexpectedToken);
        assert_eq!(syntax_kind("Sheet.4!"), SyntaxKind::InvalidReference);
    }

    #[test]
    fn nesting_past_the_limit_is_a_syntax_error() {
        let deep = format!("{}1{}", "(".repeat(33), ")".repeat(33));
        assert_eq!(syntax_kind(&deep), SyntaxKind::NestingTooDeep);
        let max = format!("{}1{}", "(".repeat(32), ")".repeat(32));
        assert!(parse(&max).is_ok());
    }

    #[test]
    fn deep_operator_chains_hit_the_nesting_limit() {
        assert_eq!(
            syntax_kind(&format!("{}1", "-".repeat(33))),
            SyntaxKind::NestingTooDeep
        );
        assert_eq!(
            syntax_kind(&format!("{}2", "2^".repeat(33))),
            SyntaxKind::NestingTooDeep
        );
    }

    #[test]
    fn a_flat_operator_chain_is_not_nesting() {
        let sum = (0..64).map(|_| "1").collect::<Vec<_>>().join("+");
        assert!(parse(&sum).is_ok());
    }

    #[test]
    fn an_unbalanced_bracket_is_an_error() {
        assert_eq!(syntax_kind("Char.Size[3"), SyntaxKind::UnexpectedEnd);
        assert_eq!(syntax_kind("5[in"), SyntaxKind::UnexpectedEnd);
    }

    #[test]
    fn parse_formula_reads_the_source_text() {
        let f = Formula::new("Width*0.5");
        assert_eq!(
            parse_formula(&f).expect("the formula parses"),
            tree("Width*0.5")
        );
    }

    #[test]
    fn spans_cover_the_source_text() {
        let expr = parse("Width*0.5 in").expect("the source parses");
        assert_eq!(expr.span, Span::new(0, 12));
        let ExprKind::Binary { right, .. } = expr.kind else {
            panic!("expected a binary expression");
        };
        assert_eq!(right.span, Span::new(6, 12));
    }
}
