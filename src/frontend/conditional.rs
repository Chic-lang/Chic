use crate::frontend::diagnostics::{Diagnostic, Span};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::iter::Peekable;
use std::str::CharIndices;
use std::sync::{LazyLock, RwLock};

/// Value assigned to a conditional compilation define.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefineValue {
    Bool(bool),
    String(String),
}

impl DefineValue {
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            DefineValue::Bool(value) => Some(*value),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            DefineValue::String(value) => Some(value.as_str()),
            _ => None,
        }
    }
}

/// Map of compile-time defines exposed to the frontend.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConditionalDefines {
    values: BTreeMap<String, DefineValue>,
}

impl ConditionalDefines {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, key: impl Into<String>, value: DefineValue) -> Option<DefineValue> {
        self.values.insert(key.into(), value)
    }

    pub fn set_bool(&mut self, key: impl Into<String>, value: bool) {
        self.insert(key.into(), DefineValue::Bool(value));
    }

    pub fn set_string(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.insert(key.into(), DefineValue::String(value.into()));
    }

    #[must_use]
    pub fn is_true(&self, key: &str) -> bool {
        matches!(self.values.get(key), Some(DefineValue::Bool(true)))
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&DefineValue> {
        self.values.get(key)
    }

    #[must_use]
    pub fn iter(&self) -> impl Iterator<Item = (&String, &DefineValue)> {
        self.values.iter()
    }
}

static ACTIVE_DEFINES: LazyLock<RwLock<ConditionalDefines>> =
    LazyLock::new(|| RwLock::new(ConditionalDefines::new()));

/// Snapshot the active conditional defines used by the current compilation pipeline.
#[must_use]
pub fn active_defines() -> ConditionalDefines {
    ACTIVE_DEFINES
        .read()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

/// Record the active conditional defines for the current compilation pipeline.
pub fn set_active_defines(defines: ConditionalDefines) {
    if let Ok(mut guard) = ACTIVE_DEFINES.write() {
        *guard = defines;
    }
}

/// Result of preprocessing conditional directives.
#[derive(Debug, Default)]
pub struct ConditionalPreprocessorResult {
    pub rewritten: Option<String>,
    pub diagnostics: Vec<Diagnostic>,
}

struct ConditionalFrame {
    span: Span,
    parent_active: bool,
    branch_taken: bool,
    active: bool,
    else_consumed: bool,
}

impl ConditionalFrame {
    fn new(span: Span, parent_active: bool, active: bool) -> Self {
        Self {
            span,
            parent_active,
            branch_taken: active,
            active,
            else_consumed: false,
        }
    }
}

/// Preprocess conditional compilation directives, returning a rewritten source string where
/// inactive blocks are replaced with whitespace so spans remain stable.
#[must_use]
pub fn preprocess(source: &str, defines: &ConditionalDefines) -> ConditionalPreprocessorResult {
    if source.is_empty() {
        return ConditionalPreprocessorResult::default();
    }

    let mut output = String::with_capacity(source.len());
    let mut diagnostics = Vec::new();
    let mut frames: Vec<ConditionalFrame> = Vec::new();
    let mut changed = false;
    let mut index = 0;

    while index <= source.len() {
        let Some(line_end_rel) = source[index..].find('\n') else {
            // Process final line without trailing newline.
            process_line(
                source,
                index,
                source.len(),
                defines,
                &mut frames,
                &mut output,
                &mut diagnostics,
                &mut changed,
            );
            break;
        };
        let line_end = index + line_end_rel;
        process_line(
            source,
            index,
            line_end,
            defines,
            &mut frames,
            &mut output,
            &mut diagnostics,
            &mut changed,
        );
        output.push('\n');
        index = line_end + 1;
        if index > source.len() {
            break;
        }
    }

    while let Some(frame) = frames.pop() {
        diagnostics.push(Diagnostic::error(
            "`#if` without matching `#endif`",
            Some(frame.span),
        ));
        changed = true;
    }

    if !changed {
        return ConditionalPreprocessorResult {
            rewritten: None,
            diagnostics,
        };
    }

    ConditionalPreprocessorResult {
        rewritten: Some(output),
        diagnostics,
    }
}

fn process_line(
    source: &str,
    start: usize,
    end: usize,
    defines: &ConditionalDefines,
    frames: &mut Vec<ConditionalFrame>,
    output: &mut String,
    diagnostics: &mut Vec<Diagnostic>,
    changed: &mut bool,
) {
    if start >= end {
        return;
    }
    let line = &source[start..end];
    let trimmed_offset = line
        .char_indices()
        .find(|(_, ch)| !matches!(ch, ' ' | '\t'))
        .map(|(idx, _)| idx)
        .unwrap_or(line.len());
    let trimmed = &line[trimmed_offset..];
    if trimmed.starts_with("#![") {
        let active = current_active(frames);
        if !active {
            *changed = true;
        }
        append_line(line, active, output);
        return;
    }
    if trimmed.starts_with('#') {
        *changed = true;
        handle_directive(
            start + trimmed_offset,
            trimmed,
            defines,
            frames,
            diagnostics,
        );
        mask_line(line, output);
    } else {
        let active = frames.last().map(|frame| frame.active).unwrap_or(true);
        if !active {
            *changed = true;
        }
        append_line(line, active, output);
    }
}

fn handle_directive(
    directive_start: usize,
    trimmed: &str,
    defines: &ConditionalDefines,
    frames: &mut Vec<ConditionalFrame>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut cursor = 1; // skip '#'
    cursor += consume_ascii_whitespace(&trimmed[cursor..]);
    let keyword_start = cursor;
    let keyword_len = trimmed[keyword_start..]
        .chars()
        .take_while(|ch| ch.is_ascii_alphabetic())
        .map(|ch| ch.len_utf8())
        .sum::<usize>();
    cursor += keyword_len;
    if keyword_start == cursor {
        diagnostics.push(Diagnostic::error(
            "expected directive keyword after `#`",
            Some(span_for_directive(directive_start, trimmed)),
        ));
        return;
    }
    let keyword = &trimmed[keyword_start..cursor];
    cursor += consume_ascii_whitespace(&trimmed[cursor..]);
    let expr = trimmed[cursor..].trim_end_matches('\r');
    let expr_offset = directive_start + cursor;
    let keyword_lower = keyword.to_ascii_lowercase();
    match keyword_lower.as_str() {
        "if" => {
            handle_if(
                expr,
                expr_offset,
                directive_start,
                trimmed,
                defines,
                frames,
                diagnostics,
            );
        }
        "elif" => handle_elif(
            expr,
            expr_offset,
            directive_start,
            trimmed,
            defines,
            frames,
            diagnostics,
        ),
        "else" => handle_else(expr, directive_start, trimmed, frames, diagnostics),
        "endif" => handle_endif(expr, directive_start, trimmed, frames, diagnostics),
        _ => diagnostics.push(Diagnostic::error(
            format!("unsupported conditional directive `#{keyword}`"),
            Some(span_for_directive(directive_start, trimmed)),
        )),
    }
}

fn handle_if(
    expr: &str,
    expr_offset: usize,
    directive_start: usize,
    trimmed: &str,
    defines: &ConditionalDefines,
    frames: &mut Vec<ConditionalFrame>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let span = span_for_directive(directive_start, trimmed);
    if expr.is_empty() {
        diagnostics.push(Diagnostic::error("`#if` requires a condition", Some(span)));
        frames.push(ConditionalFrame::new(span, current_active(frames), false));
        return;
    }
    let condition = match evaluate_condition(expr, expr_offset, defines) {
        Ok(value) => value,
        Err(err) => {
            diagnostics.push(err.into());
            false
        }
    };
    let parent_active = current_active(frames);
    let active = parent_active && condition;
    frames.push(ConditionalFrame::new(span, parent_active, active));
}

fn handle_elif(
    expr: &str,
    expr_offset: usize,
    directive_start: usize,
    trimmed: &str,
    defines: &ConditionalDefines,
    frames: &mut Vec<ConditionalFrame>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let span = span_for_directive(directive_start, trimmed);
    let Some(frame) = frames.last_mut() else {
        diagnostics.push(Diagnostic::error(
            "`#elif` without preceding `#if`",
            Some(span),
        ));
        return;
    };
    if frame.else_consumed {
        diagnostics.push(Diagnostic::error(
            "`#elif` cannot appear after `#else`",
            Some(span),
        ));
        frame.active = false;
        return;
    }
    if expr.is_empty() {
        diagnostics.push(Diagnostic::error(
            "`#elif` requires a condition",
            Some(span),
        ));
        frame.active = false;
        return;
    }
    let condition = match evaluate_condition(expr, expr_offset, defines) {
        Ok(value) => value,
        Err(err) => {
            diagnostics.push(err.into());
            false
        }
    };
    let eligible = frame.parent_active && !frame.branch_taken;
    let active = eligible && condition;
    frame.active = active;
    if active {
        frame.branch_taken = true;
    }
}

fn handle_else(
    expr: &str,
    directive_start: usize,
    trimmed: &str,
    frames: &mut Vec<ConditionalFrame>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let span = span_for_directive(directive_start, trimmed);
    if !expr.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "`#else` does not take a condition",
            Some(span),
        ));
    }
    let Some(frame) = frames.last_mut() else {
        diagnostics.push(Diagnostic::error(
            "`#else` without preceding `#if`",
            Some(span),
        ));
        return;
    };
    if frame.else_consumed {
        diagnostics.push(Diagnostic::error(
            "multiple `#else` clauses in the same `#if`",
            Some(span),
        ));
        frame.active = false;
        return;
    }
    frame.else_consumed = true;
    let active = frame.parent_active && !frame.branch_taken;
    frame.active = active;
    if active {
        frame.branch_taken = true;
    }
}

fn handle_endif(
    expr: &str,
    directive_start: usize,
    trimmed: &str,
    frames: &mut Vec<ConditionalFrame>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let span = span_for_directive(directive_start, trimmed);
    if !expr.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "`#endif` cannot take a trailing condition",
            Some(span),
        ));
    }
    if frames.pop().is_none() {
        diagnostics.push(Diagnostic::error(
            "`#endif` without preceding `#if`",
            Some(span),
        ));
    }
}

fn current_active(frames: &[ConditionalFrame]) -> bool {
    frames.last().map(|frame| frame.active).unwrap_or(true)
}

fn append_line(line: &str, active: bool, out: &mut String) {
    if active {
        out.push_str(line);
    } else {
        mask_line(line, out);
    }
}

fn mask_line(line: &str, out: &mut String) {
    for ch in line.chars() {
        match ch {
            '\r' => out.push('\r'),
            '\t' => out.push('\t'),
            _ => out.push(' '),
        }
    }
}

fn consume_ascii_whitespace(segment: &str) -> usize {
    segment
        .as_bytes()
        .iter()
        .take_while(|ch| matches!(**ch, b' ' | b'\t'))
        .count()
}

fn span_for_directive(start: usize, trimmed: &str) -> Span {
    Span::new(start, start + trimmed.trim_end_matches('\r').len())
}

pub(crate) fn evaluate_condition(
    expr: &str,
    base_offset: usize,
    defines: &ConditionalDefines,
) -> Result<bool, ConditionError> {
    ConditionParser::new(expr, base_offset, defines)?.parse()
}

pub(crate) fn evaluate_condition_with_diagnostics(
    expr: &str,
    base_offset: usize,
    defines: &ConditionalDefines,
) -> Result<bool, Diagnostic> {
    evaluate_condition(expr, base_offset, defines).map_err(Into::into)
}

#[derive(Debug)]
pub(crate) struct ConditionError {
    message: String,
    span: Span,
}

impl ConditionError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

impl From<ConditionError> for Diagnostic {
    fn from(err: ConditionError) -> Self {
        Diagnostic::error(err.message, Some(err.span))
    }
}

#[derive(Debug, Clone)]
enum TokenKind {
    Identifier(String),
    StringLiteral(String),
    BoolLiteral(bool),
    And,
    Or,
    Not,
    Eq,
    Ne,
    LParen,
    RParen,
    End,
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    span: Span,
}

struct ConditionLexer<'a> {
    input: &'a str,
    chars: Peekable<CharIndices<'a>>,
    len: usize,
    base_offset: usize,
}

impl<'a> ConditionLexer<'a> {
    fn new(input: &'a str, base_offset: usize) -> Self {
        Self {
            input,
            chars: input.char_indices().peekable(),
            len: input.len(),
            base_offset,
        }
    }

    fn next_token(&mut self) -> Result<Token, ConditionError> {
        self.skip_whitespace();
        let Some((start, ch)) = self.chars.peek().copied() else {
            return Ok(Token {
                kind: TokenKind::End,
                span: Span::new(self.base_offset + self.len, self.base_offset + self.len),
            });
        };
        match ch {
            '(' => self.consume_simple(TokenKind::LParen),
            ')' => self.consume_simple(TokenKind::RParen),
            '&' => self.consume_double('&', TokenKind::And, "&&"),
            '|' => self.consume_double('|', TokenKind::Or, "||"),
            '=' => self.consume_equals(start),
            '!' => self.consume_bang(start),
            '"' => self.consume_string(start),
            _ if ch.is_ascii_alphabetic() || ch == '_' => self.consume_identifier(start),
            _ => Err(ConditionError::new(
                format!("unsupported character `{ch}` in conditional expression"),
                Span::new(
                    self.base_offset + start,
                    self.base_offset + start + ch.len_utf8(),
                ),
            )),
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some((_, ch)) = self.chars.peek() {
            if ch.is_whitespace() {
                self.chars.next();
            } else {
                break;
            }
        }
    }

    fn consume_simple(&mut self, kind: TokenKind) -> Result<Token, ConditionError> {
        if let Some((start, ch)) = self.chars.next() {
            let end = start + ch.len_utf8();
            Ok(Token {
                kind,
                span: Span::new(self.base_offset + start, self.base_offset + end),
            })
        } else {
            unreachable!("peeked before calling consume_simple");
        }
    }

    fn consume_double(
        &mut self,
        expected: char,
        kind: TokenKind,
        repr: &str,
    ) -> Result<Token, ConditionError> {
        let Some((start, _)) = self.chars.next() else {
            unreachable!("peeked before calling consume_double");
        };
        match self.chars.peek() {
            Some((_, ch)) if *ch == expected => {
                self.chars.next();
                let end = start + repr.len();
                Ok(Token {
                    kind,
                    span: Span::new(self.base_offset + start, self.base_offset + end),
                })
            }
            _ => Err(ConditionError::new(
                format!("expected `{repr}` in conditional expression"),
                Span::new(self.base_offset + start, self.base_offset + start + 1),
            )),
        }
    }

    fn consume_equals(&mut self, start: usize) -> Result<Token, ConditionError> {
        self.chars.next();
        if let Some((_, '=')) = self.chars.peek() {
            self.chars.next();
            let end = start + 2;
            return Ok(Token {
                kind: TokenKind::Eq,
                span: Span::new(self.base_offset + start, self.base_offset + end),
            });
        }
        Ok(Token {
            kind: TokenKind::Eq,
            span: Span::new(self.base_offset + start, self.base_offset + start + 1),
        })
    }

    fn consume_bang(&mut self, start: usize) -> Result<Token, ConditionError> {
        self.chars.next();
        if let Some((_, '=')) = self.chars.peek() {
            self.chars.next();
            return Ok(Token {
                kind: TokenKind::Ne,
                span: Span::new(self.base_offset + start, self.base_offset + start + 2),
            });
        }
        Ok(Token {
            kind: TokenKind::Not,
            span: Span::new(self.base_offset + start, self.base_offset + start + 1),
        })
    }

    fn consume_identifier(&mut self, start: usize) -> Result<Token, ConditionError> {
        let Some((_, first)) = self.chars.next() else {
            unreachable!("peeked before calling consume_identifier");
        };
        let mut end = start + first.len_utf8();
        while let Some(&(idx, ch)) = self.chars.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.chars.next();
                end = idx + ch.len_utf8();
            } else {
                break;
            }
        }
        let slice = &self.input[start..end];
        let kind = match slice.to_ascii_lowercase().as_str() {
            "true" => TokenKind::BoolLiteral(true),
            "false" => TokenKind::BoolLiteral(false),
            _ => TokenKind::Identifier(slice.to_string()),
        };
        Ok(Token {
            kind,
            span: Span::new(self.base_offset + start, self.base_offset + end),
        })
    }

    fn consume_string(&mut self, start: usize) -> Result<Token, ConditionError> {
        self.chars.next(); // opening quote
        let mut value = String::new();
        while let Some((idx, ch)) = self.chars.next() {
            if ch == '"' {
                let end = idx + ch.len_utf8();
                return Ok(Token {
                    kind: TokenKind::StringLiteral(value),
                    span: Span::new(self.base_offset + start, self.base_offset + end),
                });
            }
            if ch == '\\' {
                let Some((_, escaped)) = self.chars.next() else {
                    return Err(ConditionError::new(
                        "unterminated escape sequence in conditional string literal",
                        Span::new(self.base_offset + start, self.base_offset + self.len),
                    ));
                };
                value.push(match escaped {
                    '\\' => '\\',
                    '"' => '"',
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    other => other,
                });
                continue;
            }
            value.push(ch);
        }
        Err(ConditionError::new(
            "unterminated string literal in conditional expression",
            Span::new(self.base_offset + start, self.base_offset + self.len),
        ))
    }
}

struct ConditionParser<'a> {
    current: Token,
    lexer: ConditionLexer<'a>,
    defines: &'a ConditionalDefines,
}

impl<'a> ConditionParser<'a> {
    fn new(
        input: &'a str,
        base_offset: usize,
        defines: &'a ConditionalDefines,
    ) -> Result<Self, ConditionError> {
        let mut lexer = ConditionLexer::new(input, base_offset);
        let current = lexer.next_token()?;
        Ok(Self {
            current,
            lexer,
            defines,
        })
    }

    fn parse(mut self) -> Result<bool, ConditionError> {
        let value = self.parse_or()?;
        if !matches!(self.current.kind, TokenKind::End) {
            return Err(ConditionError::new(
                "unexpected tokens after conditional expression",
                self.current.span,
            ));
        }
        Ok(value)
    }

    fn parse_or(&mut self) -> Result<bool, ConditionError> {
        let mut value = self.parse_and()?;
        while matches!(self.current.kind, TokenKind::Or) {
            self.bump()?;
            let rhs = self.parse_and()?;
            value = value || rhs;
        }
        Ok(value)
    }

    fn parse_and(&mut self) -> Result<bool, ConditionError> {
        let mut value = self.parse_equality()?;
        while matches!(self.current.kind, TokenKind::And) {
            self.bump()?;
            let rhs = self.parse_equality()?;
            value = value && rhs;
        }
        Ok(value)
    }

    fn parse_equality(&mut self) -> Result<bool, ConditionError> {
        let mut left = self.parse_unary_value()?;
        loop {
            match self.current.kind {
                TokenKind::Eq => {
                    self.bump()?;
                    let right = self.parse_unary_value()?;
                    let result = compare_values(&left, &right, true)?;
                    left = Value::new_bool(result, Span::new(left.span.start, right.span.end));
                }
                TokenKind::Ne => {
                    self.bump()?;
                    let right = self.parse_unary_value()?;
                    let result = compare_values(&left, &right, false)?;
                    left = Value::new_bool(result, Span::new(left.span.start, right.span.end));
                }
                _ => break,
            }
        }
        left.into_bool()
    }

    fn parse_unary_value(&mut self) -> Result<Value<'a>, ConditionError> {
        match self.current.kind.clone() {
            TokenKind::Not => {
                let span = self.current.span;
                self.bump()?;
                let value = self.parse_unary_value()?;
                let bool_value = value.clone().into_bool()?;
                Ok(Value::new_bool(
                    !bool_value,
                    Span::new(span.start, value.span.end),
                ))
            }
            TokenKind::LParen => {
                let open_span = self.current.span;
                self.bump()?;
                let value = self.parse_or()?;
                match self.current.kind {
                    TokenKind::RParen => {
                        let close_span = self.current.span;
                        self.bump()?;
                        Ok(Value::new_bool(
                            value,
                            Span::new(open_span.start, close_span.end),
                        ))
                    }
                    _ => Err(ConditionError::new(
                        "expected ')' in conditional expression",
                        open_span,
                    )),
                }
            }
            _ => self.parse_primary_value(),
        }
    }

    fn parse_primary_value(&mut self) -> Result<Value<'a>, ConditionError> {
        let token = self.current.clone();
        match token.kind {
            TokenKind::Identifier(name) => {
                self.bump()?;
                Ok(value_from_identifier(name, token.span, self.defines))
            }
            TokenKind::StringLiteral(value) => {
                self.bump()?;
                Ok(Value::new_str(Cow::Owned(value), token.span))
            }
            TokenKind::BoolLiteral(value) => {
                self.bump()?;
                Ok(Value::new_bool(value, token.span))
            }
            _ => Err(ConditionError::new(
                "expected identifier, string literal, or boolean literal",
                token.span,
            )),
        }
    }

    fn bump(&mut self) -> Result<(), ConditionError> {
        self.current = self.lexer.next_token()?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct Value<'a> {
    kind: ValueKind<'a>,
    span: Span,
}

#[derive(Debug, Clone)]
enum ValueKind<'a> {
    Bool(bool),
    Str(Cow<'a, str>),
}

impl<'a> Value<'a> {
    fn new_bool(value: bool, span: Span) -> Self {
        Self {
            kind: ValueKind::Bool(value),
            span,
        }
    }

    fn new_str(value: Cow<'a, str>, span: Span) -> Self {
        Self {
            kind: ValueKind::Str(value),
            span,
        }
    }

    fn into_bool(self) -> Result<bool, ConditionError> {
        match self.kind {
            ValueKind::Bool(value) => Ok(value),
            ValueKind::Str(_) => Err(ConditionError::new(
                "string value used where boolean was expected",
                self.span,
            )),
        }
    }
}

fn compare_values<'a>(
    left: &Value<'a>,
    right: &Value<'a>,
    eq: bool,
) -> Result<bool, ConditionError> {
    match (&left.kind, &right.kind) {
        (ValueKind::Bool(lhs), ValueKind::Bool(rhs)) => {
            Ok(if eq { lhs == rhs } else { lhs != rhs })
        }
        (ValueKind::Str(lhs), ValueKind::Str(rhs)) => Ok(if eq { lhs == rhs } else { lhs != rhs }),
        _ => Err(ConditionError::new(
            "cannot compare boolean and string values",
            Span::new(left.span.start, right.span.end),
        )),
    }
}

fn value_from_identifier<'a>(
    name: String,
    span: Span,
    defines: &'a ConditionalDefines,
) -> Value<'a> {
    match defines.get(&name) {
        Some(DefineValue::Bool(value)) => Value::new_bool(*value, span),
        Some(DefineValue::String(value)) => Value::new_str(Cow::Borrowed(value.as_str()), span),
        None => Value::new_bool(false, span),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_inactive_branch() {
        let mut defines = ConditionalDefines::default();
        defines.set_bool("DEBUG", false);
        defines.set_bool("RELEASE", true);
        let source = "#if DEBUG\nfn debug() {}\n#else\nfn release() {}\n#endif\n";
        let result = preprocess(source, &defines);
        assert!(result.diagnostics.is_empty());
        let rewritten = result.rewritten.expect("rewritten source");
        assert!(rewritten.contains("fn release"));
        assert!(!rewritten.contains("fn debug"));
    }

    #[test]
    fn evaluates_string_comparisons() {
        let mut defines = ConditionalDefines::default();
        defines.set_string("TARGET_OS", "macos");
        let source = "#if TARGET_OS == \"macos\"\nfn main() {}\n#endif";
        let result = preprocess(source, &defines);
        assert!(result.diagnostics.is_empty());
        let rewritten = result.rewritten.expect("rewritten source");
        assert!(rewritten.contains("fn main()"));
    }

    #[test]
    fn reports_unmatched_directive() {
        let defines = ConditionalDefines::default();
        let source = "#if DEBUG\nfn debug() {}";
        let result = preprocess(source, &defines);
        assert_eq!(result.diagnostics.len(), 1);
        assert!(result.rewritten.is_some());
    }

    #[test]
    fn accepts_single_equals_in_conditions() {
        let mut defines = ConditionalDefines::default();
        defines.set_string("TARGET_OS", "macos");
        let value = evaluate_condition_with_diagnostics("TARGET_OS = \"macos\"", 0, &defines)
            .expect("condition should evaluate");
        assert!(value, "single '=' should behave like equality");
    }

    #[test]
    fn preserves_crate_level_attributes() {
        let defines = ConditionalDefines::default();
        let source = "#![no_std]\nnamespace Kernel;";
        let result = preprocess(source, &defines);
        assert!(
            result.rewritten.is_none(),
            "crate attributes should remain untouched by preprocessing"
        );
        assert!(result.diagnostics.is_empty());
    }
}
