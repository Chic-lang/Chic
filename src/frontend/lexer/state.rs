use super::{Keyword, LexOutput, Token, TokenKind, diagnostics, numeric};
use crate::frontend::diagnostics::{DiagnosticSink, FileId, Span};
use crate::frontend::literals::{
    CharLiteral, StringLiteralKind, parse_char_literal, parse_string_literal,
};
use crate::unicode::identifier;

pub(super) fn run(source: &str) -> LexOutput {
    run_with_file(source, FileId::UNKNOWN)
}

pub(super) fn run_with_file(source: &str, file_id: FileId) -> LexOutput {
    let mut lexer = Lexer::new(source, file_id);
    lexer.lex_all();
    lexer.finish()
}

pub(super) struct Lexer<'a> {
    pub(super) source: &'a str,
    pub(super) iter: core::str::CharIndices<'a>,
    pub(super) lookahead: Option<(usize, char)>,
    pub(super) tokens: Vec<Token>,
    pub(super) diagnostics: DiagnosticSink,
    pub(super) file_id: FileId,
}

impl<'a> Lexer<'a> {
    #[must_use]
    pub(super) fn new(source: &'a str, file_id: FileId) -> Self {
        let mut iter = source.char_indices();
        let lookahead = iter.next();
        Self {
            source,
            iter,
            lookahead,
            tokens: Vec::new(),
            diagnostics: DiagnosticSink::new("LEX"),
            file_id,
        }
    }

    fn finish(self) -> LexOutput {
        let Lexer {
            tokens,
            diagnostics,
            file_id,
            ..
        } = self;
        LexOutput {
            tokens,
            diagnostics: diagnostics.into_vec(),
            file_id,
        }
    }

    pub(super) fn lex_all(&mut self) {
        while let Some((start, ch)) = self.lookahead {
            match ch {
                c if c.is_ascii_whitespace() => {
                    self.consume_whitespace();
                }
                c if identifier::is_identifier_start(c) => {
                    self.consume_identifier(start);
                }
                c if c.is_ascii_digit() => {
                    self.consume_number(start);
                }
                '"' => {
                    self.consume_string_literal(start, &['"'], StringLiteralKind::Regular);
                }
                '\'' => {
                    self.consume_char_literal(start);
                }
                '@' => {
                    if let Some((_, next)) = self.peek_char_offset(0) {
                        if next == '$' {
                            if let Some((_, third)) = self.peek_char_offset(1)
                                && third == '"'
                            {
                                self.consume_string_literal(
                                    start,
                                    &['@', '$', '"'],
                                    StringLiteralKind::InterpolatedVerbatim,
                                );
                                continue;
                            }
                        } else if next == '"' {
                            self.consume_string_literal(
                                start,
                                &['@', '"'],
                                StringLiteralKind::Verbatim,
                            );
                            continue;
                        }
                    }
                    self.emit_single_char_token(start, ch, TokenKind::Punctuation(ch));
                    self.bump();
                }
                '$' => {
                    if let Some((_, next)) = self.peek_char_offset(0) {
                        if next == '@' {
                            if let Some((_, third)) = self.peek_char_offset(1)
                                && third == '"'
                            {
                                self.consume_string_literal(
                                    start,
                                    &['$', '@', '"'],
                                    StringLiteralKind::InterpolatedVerbatim,
                                );
                                continue;
                            }
                        } else if next == '"' {
                            self.consume_string_literal(
                                start,
                                &['$', '"'],
                                StringLiteralKind::Interpolated,
                            );
                            continue;
                        }
                    }
                    self.emit_single_char_token(start, ch, TokenKind::Unknown(ch));
                    self.bump();
                }
                '/' => {
                    self.consume_slash(start);
                }
                '.' => {
                    if let Some((second_idx, second)) = self.peek_char_offset(0) {
                        if second == '.' {
                            let third = self.peek_char_offset(1);
                            self.bump();
                            self.bump();
                            if let Some((third_idx, third_ch)) = third {
                                if third_ch == '.' {
                                    self.bump();
                                    let end = third_idx + '.'.len_utf8();
                                    self.emit(start, end, TokenKind::Operator("..."));
                                    continue;
                                }
                                if third_ch == '=' {
                                    self.bump();
                                    let end = start + 3;
                                    self.emit(start, end, TokenKind::Operator("..="));
                                    continue;
                                }
                            }
                            let end = second_idx + '.'.len_utf8();
                            self.emit(start, end, TokenKind::Operator(".."));
                            continue;
                        }
                    }
                    self.emit_single_char_token(start, ch, TokenKind::Punctuation(ch));
                    self.bump();
                }
                '{' | '}' | '(' | ')' | '[' | ']' | ';' | ',' | ':' | '#' => {
                    self.emit_single_char_token(start, ch, TokenKind::Punctuation(ch));
                    self.bump();
                }
                '?' => {
                    self.consume_question(start);
                }
                '=' | '+' | '-' | '*' | '%' | '!' | '<' | '>' | '&' | '|' | '^' | '~' => {
                    self.consume_operator(start, ch);
                }
                _ => {
                    if let Some(reason) = identifier::disallowed_reason(ch) {
                        diagnostics::report_invalid_identifier_char(self, start, ch, reason, None);
                    }
                    self.emit_single_char_token(start, ch, TokenKind::Unknown(ch));
                    self.bump();
                }
            }
        }
    }

    pub(super) fn bump(&mut self) {
        self.lookahead = self.iter.next();
    }

    pub(super) fn slice(&self, start: usize, end: usize) -> &str {
        &self.source[start..end]
    }

    pub(super) fn emit(&mut self, start: usize, end: usize, kind: TokenKind) {
        self.tokens.push(Token {
            kind,
            lexeme: self.slice(start, end).to_string(),
            span: Span::in_file(self.file_id, start, end),
        });
    }

    pub(super) fn emit_single_char_token(&mut self, start: usize, ch: char, kind: TokenKind) {
        let end = start + ch.len_utf8();
        self.tokens.push(Token {
            kind,
            lexeme: ch.to_string(),
            span: Span::in_file(self.file_id, start, end),
        });
    }

    fn consume_identifier(&mut self, start: usize) {
        let mut end = start;
        while let Some((idx, ch)) = self.lookahead {
            if identifier::is_identifier_continue(ch)
                || identifier::is_forbidden_identifier_control(ch)
                || identifier::looks_like_identifier(ch)
                    && identifier::disallowed_reason(ch).is_some()
            {
                end = idx + ch.len_utf8();
                self.bump();
            } else {
                break;
            }
        }

        let raw = self.slice(start, end).to_string();
        let status = identifier::analyse_identifier(&raw);
        if let Some(disallowed) = status.disallowed {
            diagnostics::report_invalid_identifier_char(
                self,
                start + disallowed.offset,
                disallowed.ch,
                disallowed.property,
                status.suggestion.as_deref(),
            );
        }
        if status.was_normalized {
            diagnostics::report_identifier_not_normalized(
                self,
                Span::in_file(self.file_id, start, end),
                &status.normalized,
            );
        }

        let ident = status.normalized;
        if let Some(keyword) = Keyword::from_ident(&ident) {
            let span = Span::in_file(self.file_id, start, end);
            self.tokens.push(Token {
                kind: TokenKind::Keyword(keyword),
                lexeme: ident,
                span,
            });
        } else {
            let span = Span::in_file(self.file_id, start, end);
            self.tokens.push(Token {
                kind: TokenKind::Identifier,
                lexeme: ident,
                span,
            });
        }
    }

    fn consume_number(&mut self, start: usize) {
        let scan = numeric::scan_numeric_literal(self, start);
        self.emit(start, scan.end, TokenKind::NumberLiteral(scan.literal));
    }

    fn consume_string_literal(&mut self, start: usize, prefix: &[char], kind: StringLiteralKind) {
        let mut last_index = start;
        for expected in prefix {
            let Some((idx, ch)) = self.lookahead else {
                self.emit_single_char_token(start, *expected, TokenKind::Unknown(*expected));
                return;
            };
            debug_assert_eq!(
                ch, *expected,
                "string literal prefix mismatch: expected `{expected}`, found `{ch}`"
            );
            last_index = idx + ch.len_utf8();
            self.bump();
        }

        let content_start = self
            .lookahead
            .map(|(idx, _)| idx)
            .unwrap_or_else(|| self.source.len());
        let mut content_end = content_start;
        let mut end = last_index;
        let mut terminated = false;
        let mut escaped = false;

        while let Some((idx, ch)) = self.lookahead {
            self.bump();
            end = idx + ch.len_utf8();
            match kind {
                StringLiteralKind::Verbatim | StringLiteralKind::InterpolatedVerbatim => {
                    if ch == '"' {
                        if let Some((_, next)) = self.lookahead {
                            if next == '"' {
                                self.bump();
                                end = end + '"'.len_utf8();
                                content_end = end;
                                continue;
                            }
                        }
                        terminated = true;
                        content_end = idx;
                        break;
                    }
                    content_end = end;
                }
                _ => {
                    if ch == '"' && !escaped {
                        terminated = true;
                        content_end = idx;
                        break;
                    }
                    if ch == '\\' && !escaped {
                        escaped = true;
                    } else {
                        escaped = false;
                    }
                    content_end = end;
                }
            }
        }

        if !terminated {
            diagnostics::report_simple_error(
                self,
                "unterminated string literal",
                Span::new(start, end),
            );
        }

        let content = if content_end >= content_start {
            self.slice(content_start, content_end)
        } else {
            ""
        };

        let (literal, errors) = parse_string_literal(content, kind);
        self.emit(start, end, TokenKind::StringLiteral(literal));
        self.report_literal_errors(start, content_start, Some(kind), errors);
    }

    fn consume_char_literal(&mut self, start: usize) {
        let Some((open_idx, ch)) = self.lookahead else {
            return;
        };
        debug_assert_eq!(ch, '\'');
        self.bump();

        let content_start = self
            .lookahead
            .map(|(idx, _)| idx)
            .unwrap_or_else(|| self.source.len());
        let mut content_end = content_start;
        let mut end = open_idx + ch.len_utf8();
        let mut terminated = false;
        let mut escaped = false;

        while let Some((idx, next)) = self.lookahead {
            self.bump();
            end = idx + next.len_utf8();
            if next == '\'' && !escaped {
                terminated = true;
                content_end = idx;
                break;
            }
            if next == '\\' && !escaped {
                escaped = true;
            } else {
                escaped = false;
            }
            content_end = end;
        }

        if !terminated {
            diagnostics::report_simple_error(
                self,
                "unterminated character literal",
                Span::new(start, end),
            );
        }

        let content = if content_end >= content_start {
            self.slice(content_start, content_end)
        } else {
            ""
        };

        let (literal, errors) = parse_char_literal(content);
        self.report_literal_errors(start, content_start, None, errors);
        let value: CharLiteral = literal.unwrap_or_default();
        self.emit(start, end, TokenKind::CharLiteral(value));
    }

    fn consume_operator(&mut self, start: usize, first: char) {
        let maybe_second = self.peek_next_char().map(|(_, ch)| ch);
        let operator = match (first, maybe_second) {
            ('-', Some('>')) => {
                self.bump();
                self.bump();
                "->"
            }
            ('+', Some('+')) => {
                self.bump();
                self.bump();
                "++"
            }
            ('-', Some('-')) => {
                self.bump();
                self.bump();
                "--"
            }
            ('=', Some('>')) => {
                self.bump();
                self.bump();
                "=>"
            }
            ('+', Some('=')) => {
                self.bump();
                self.bump();
                "+="
            }
            ('-', Some('=')) => {
                self.bump();
                self.bump();
                "-="
            }
            ('*', Some('=')) => {
                self.bump();
                self.bump();
                "*="
            }
            ('/', Some('=')) => {
                self.bump();
                self.bump();
                "/="
            }
            ('%', Some('=')) => {
                self.bump();
                self.bump();
                "%="
            }
            ('=', Some('=')) => {
                self.bump();
                self.bump();
                "=="
            }
            ('!', Some('=')) => {
                self.bump();
                self.bump();
                "!="
            }
            ('<', Some('=')) => {
                self.bump();
                self.bump();
                "<="
            }
            ('>', Some('=')) => {
                self.bump();
                self.bump();
                ">="
            }
            ('<', Some('<')) => {
                self.bump();
                if matches!(self.iter.clone().next(), Some((_, '='))) {
                    self.bump();
                    self.bump();
                    "<<="
                } else {
                    self.bump();
                    "<<"
                }
            }
            ('>', Some('>')) => {
                self.bump();
                if matches!(self.iter.clone().next(), Some((_, '='))) {
                    self.bump();
                    self.bump();
                    ">>="
                } else {
                    self.bump();
                    ">>"
                }
            }
            ('&', Some('=')) => {
                self.bump();
                self.bump();
                "&="
            }
            ('&', Some('&')) => {
                self.bump();
                self.bump();
                "&&"
            }
            ('|', Some('|')) => {
                self.bump();
                self.bump();
                "||"
            }
            ('|', Some('=')) => {
                self.bump();
                self.bump();
                "|="
            }
            ('^', Some('=')) => {
                self.bump();
                self.bump();
                "^="
            }
            _ => {
                self.bump();
                match first {
                    '=' => "=",
                    '+' => "+",
                    '-' => "-",
                    '*' => "*",
                    '%' => "%",
                    '!' => "!",
                    '<' => "<",
                    '>' => ">",
                    '&' => "&",
                    '|' => "|",
                    '^' => "^",
                    '~' => "~",
                    '?' => "?",
                    _ => unreachable!("handled via match arm"),
                }
            }
        };

        let end = start + operator.len();

        let kind = match operator {
            "<" => TokenKind::Punctuation('<'),
            ">" => TokenKind::Punctuation('>'),
            _ => TokenKind::Operator(operator),
        };

        self.tokens.push(Token {
            kind,
            lexeme: operator.to_string(),
            span: Span::new(start, end),
        });
    }

    fn consume_question(&mut self, start: usize) {
        if let Some((_, '?')) = self.peek_next_char() {
            self.bump();
            self.bump();
            if matches!(self.lookahead, Some((_, '='))) {
                self.bump();
                let end = start + 3;
                self.tokens.push(Token {
                    kind: TokenKind::Operator("??="),
                    lexeme: "??=".to_string(),
                    span: Span::new(start, end),
                });
            } else {
                let end = start + 2;
                self.tokens.push(Token {
                    kind: TokenKind::Operator("??"),
                    lexeme: "??".to_string(),
                    span: Span::new(start, end),
                });
            }
        } else {
            self.emit_single_char_token(start, '?', TokenKind::Punctuation('?'));
            self.bump();
        }
    }

    pub(super) fn peek_next_char(&self) -> Option<(usize, char)> {
        self.iter.clone().next()
    }

    pub(super) fn peek_char_offset(&self, offset: usize) -> Option<(usize, char)> {
        let mut iter = self.iter.clone();
        iter.nth(offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::lexer::{NumericBase, NumericLiteralKind, NumericLiteralSuffix};
    use crate::frontend::literals::StringLiteralContents;

    fn non_trivia(tokens: &[Token]) -> Vec<&TokenKind> {
        tokens
            .iter()
            .filter(|token| {
                !matches!(
                    token.kind,
                    TokenKind::Whitespace | TokenKind::Comment | TokenKind::DocComment
                )
            })
            .map(|token| &token.kind)
            .collect()
    }

    #[test]
    fn lexes_identifier_sequence() {
        let output = super::super::lex("alpha beta");
        let idents: Vec<_> = output
            .tokens
            .iter()
            .filter(|token| matches!(token.kind, TokenKind::Identifier))
            .collect();
        assert_eq!(idents.len(), 2);
        assert!(output.diagnostics.is_empty());
    }

    #[test]
    fn lexes_ellipsis_as_operator() {
        let output = super::super::lex("int foo(int a, ...);");
        let tokens = non_trivia(&output.tokens);
        assert!(
            tokens
                .iter()
                .any(|token| matches!(token, TokenKind::Operator("..."))),
            "expected ellipsis operator token in {:?}",
            tokens
        );
    }

    #[test]
    fn lexes_generic_type_tokens() {
        let output = super::super::lex("Span<List<int>> data;");
        assert!(output.diagnostics.is_empty());
        let mut punct = output
            .tokens
            .iter()
            .filter_map(|token| match token.kind {
                TokenKind::Punctuation(ch) => Some(ch),
                _ => None,
            })
            .collect::<Vec<_>>();
        punct.retain(|ch| *ch == '<' || *ch == '>');
        assert!(punct.contains(&'<'));
        let has_close = punct.contains(&'>')
            || output
                .tokens
                .iter()
                .any(|token| matches!(token.kind, TokenKind::Operator(op) if op == ">>"));
        assert!(has_close);
    }

    #[test]
    fn lexes_qualified_namespace() {
        let output = super::super::lex("namespace Outer.Inner;");
        assert!(output.diagnostics.is_empty());
        let kinds = non_trivia(&output.tokens);
        assert!(matches!(kinds[0], TokenKind::Keyword(Keyword::Namespace)));
        assert!(matches!(kinds[1], TokenKind::Identifier));
        assert!(matches!(kinds[2], TokenKind::Punctuation('.')));
        assert!(matches!(kinds[3], TokenKind::Identifier));
        assert!(matches!(kinds[4], TokenKind::Punctuation(';')));
    }

    #[test]
    fn lexes_increment_and_decrement() {
        let output = super::super::lex("x++; --y;");
        assert!(output.diagnostics.is_empty());
        let mut seen = output
            .tokens
            .iter()
            .filter_map(|token| match token.kind {
                TokenKind::Operator(op) => Some(op),
                _ => None,
            })
            .collect::<Vec<_>>();
        seen.sort_unstable();
        assert!(seen.contains(&"++"));
        assert!(seen.contains(&"--"));
    }

    #[test]
    fn lexes_fat_arrow_operator() {
        let output = super::super::lex("value => Transform(value);");
        assert!(output.diagnostics.is_empty());
        let mut operators: Vec<&str> = output
            .tokens
            .iter()
            .filter_map(|token| match token.kind {
                TokenKind::Operator(op) => Some(op),
                _ => None,
            })
            .collect();
        operators.sort_unstable();
        assert!(operators.contains(&"=>"));
    }

    #[test]
    fn lexes_char_literal() {
        let output = super::super::lex("'\\u0041'");
        assert!(output.diagnostics.is_empty());
        let kinds = non_trivia(&output.tokens);
        assert_eq!(kinds.len(), 1);
        match kinds[0] {
            TokenKind::CharLiteral(literal) => assert_eq!(literal.value, 'A' as u16),
            other => panic!("expected char literal, found {other:?}"),
        }
    }

    #[test]
    fn lexes_verbatim_string_literal() {
        let output = super::super::lex(r#"@"Line ""quoted"" text""#);
        assert!(output.diagnostics.is_empty());
        let kinds = non_trivia(&output.tokens);
        assert_eq!(kinds.len(), 1);
        match kinds[0] {
            TokenKind::StringLiteral(literal) => match &literal.contents {
                StringLiteralContents::Simple(text) => {
                    assert_eq!(text, "Line \"quoted\" text")
                }
                other => panic!("expected simple string contents, found {other:?}"),
            },
            other => panic!("expected string literal, found {other:?}"),
        }
    }

    #[test]
    fn lexes_compound_assignment() {
        let output = super::super::lex("x += 1; y >>= 2; z ^= value; w <<= 1;");
        assert!(output.diagnostics.is_empty());
        let ops: Vec<&str> = output
            .tokens
            .iter()
            .filter_map(|token| match token.kind {
                TokenKind::Operator(op) => Some(op),
                _ => None,
            })
            .collect();
        assert!(ops.contains(&"+="));
        assert!(ops.contains(&"^="));
        assert!(ops.contains(&"<<="));
        assert!(ops.contains(&">>="));
    }

    #[test]
    fn lexes_bitwise_operators() {
        let output = super::super::lex("~x & y | z ^ mask << 2 >> 1;");
        assert!(output.diagnostics.is_empty());
        let ops: Vec<&str> = output
            .tokens
            .iter()
            .filter_map(|token| match token.kind {
                TokenKind::Operator(op) => Some(op),
                _ => None,
            })
            .collect();
        assert!(ops.contains(&"~"));
        assert!(ops.contains(&"&"));
        assert!(ops.contains(&"|"));
        assert!(ops.contains(&"^"));
        assert!(ops.contains(&"<<"));
        assert!(ops.contains(&">>"));
    }

    #[test]
    fn lexes_null_coalescing() {
        let output = super::super::lex("var value = left ?? right; other ??= default;");
        assert!(output.diagnostics.is_empty());
        let ops: Vec<&str> = output
            .tokens
            .iter()
            .filter_map(|token| match token.kind {
                TokenKind::Operator(op) => Some(op),
                _ => None,
            })
            .collect();
        assert!(ops.contains(&"??"));
        assert!(ops.contains(&"??="));
    }

    #[test]
    fn lexes_unicode_identifiers() {
        let output = super::super::lex("int число = 数据 + مقدار;");
        assert!(output.diagnostics.is_empty());
        let idents: Vec<&str> = output
            .tokens
            .iter()
            .filter_map(|token| match token.kind {
                TokenKind::Identifier => Some(token.lexeme.as_str()),
                _ => None,
            })
            .collect();
        assert!(idents.contains(&"число"));
        assert!(idents.contains(&"数据"));
        assert!(idents.contains(&"مقدار"));
    }

    #[test]
    fn lexes_doc_comment_token() {
        let output = super::super::lex("/// summary\nlet value = 42;");
        assert!(output.diagnostics.is_empty());
        assert!(matches!(
            output.tokens.first().map(|t| &t.kind),
            Some(TokenKind::DocComment)
        ));
        assert_eq!(output.tokens[0].lexeme, "/// summary");
    }

    #[test]
    fn lexes_block_comment() {
        let output = super::super::lex("/* hello */ int x;");
        assert!(output.diagnostics.is_empty());
        assert!(
            output
                .tokens
                .iter()
                .any(|token| matches!(token.kind, TokenKind::Comment))
        );
    }

    #[test]
    fn reports_unterminated_block_comment() {
        let output = super::super::lex("/* oops");
        assert_eq!(output.diagnostics.len(), 1);
        assert!(matches!(
            output.diagnostics[0].message.as_str(),
            "unterminated block comment"
        ));
    }

    #[test]
    fn lexes_nested_block_comment() {
        let output = super::super::lex("/* outer /* inner */ done */ let x = 0;");
        assert!(output.diagnostics.is_empty());
        let comments: Vec<&TokenKind> = output
            .tokens
            .iter()
            .filter_map(|token| match token.kind {
                TokenKind::Comment => Some(&token.kind),
                _ => None,
            })
            .collect();
        assert_eq!(comments.len(), 1);
    }

    #[test]
    fn lexes_hex_integer_with_unsigned_suffix() {
        let output = super::super::lex("0xFFu");
        assert!(output.diagnostics.is_empty());
        let kinds = non_trivia(&output.tokens);
        assert_eq!(kinds.len(), 1);
        assert!(matches!(kinds[0], TokenKind::NumberLiteral(_)));
        assert_eq!(output.tokens[0].lexeme, "0xFFu");
    }

    #[test]
    fn lexes_decimal_zero_with_unsigned_suffix() {
        let output = super::super::lex("0u");
        assert!(output.diagnostics.is_empty());
        let kinds = non_trivia(&output.tokens);
        assert_eq!(kinds.len(), 1);
        assert!(matches!(kinds[0], TokenKind::NumberLiteral(_)));
        assert_eq!(output.tokens[0].lexeme, "0u");
    }

    #[test]
    fn lexes_decimal_literal_with_m_suffix() {
        let output = super::super::lex("10.25m");
        assert!(output.diagnostics.is_empty());
        let kinds = non_trivia(&output.tokens);
        assert_eq!(kinds.len(), 1);
        match kinds[0] {
            TokenKind::NumberLiteral(literal) => {
                assert_eq!(literal.kind, NumericLiteralKind::Decimal);
                assert_eq!(literal.integer, "10");
                assert_eq!(literal.fraction, Some(String::from("25")));
                assert_eq!(literal.suffix, Some(NumericLiteralSuffix::Decimal));
            }
            other => panic!("expected number literal, found {other:?}"),
        }
        assert_eq!(output.tokens[0].lexeme, "10.25m");
    }

    #[test]
    fn lexes_decimal_literal_with_uppercase_m_suffix() {
        let output = super::super::lex("42.0M");
        assert!(
            output.diagnostics.is_empty(),
            "unexpected diagnostics: {:#?}",
            output.diagnostics
        );
        let kinds = non_trivia(&output.tokens);
        assert_eq!(kinds.len(), 1);
        match kinds[0] {
            TokenKind::NumberLiteral(literal) => {
                assert_eq!(literal.kind, NumericLiteralKind::Decimal);
                assert_eq!(literal.integer, "42");
                assert_eq!(literal.fraction.as_deref(), Some("0"));
                assert_eq!(literal.suffix, Some(NumericLiteralSuffix::Decimal));
            }
            other => panic!("expected number literal, found {other:?}"),
        }
        assert_eq!(output.tokens[0].lexeme, "42.0M");
    }

    #[test]
    fn lexes_binary_literal_with_suffix_and_separators() {
        let output = super::super::lex("0b1010_0001u8");
        assert!(output.diagnostics.is_empty());
        let kinds = non_trivia(&output.tokens);
        assert_eq!(kinds.len(), 1);
        match kinds[0] {
            TokenKind::NumberLiteral(literal) => {
                assert_eq!(literal.base, NumericBase::Binary);
                assert_eq!(literal.kind, NumericLiteralKind::Integer);
                assert_eq!(literal.integer, "10100001");
                assert_eq!(
                    literal.suffix,
                    Some(NumericLiteralSuffix::U8),
                    "expected u8 suffix"
                );
                assert!(literal.errors.is_empty());
            }
            other => panic!("expected number literal, found {other:?}"),
        }
    }

    #[test]
    fn lexes_quad_float_suffix() {
        let output = super::super::lex("1.0f128");
        assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
        let literal = match &non_trivia(&output.tokens)[0] {
            TokenKind::NumberLiteral(lit) => lit,
            other => panic!("expected number literal, found {other:?}"),
        };
        assert_eq!(literal.kind, NumericLiteralKind::Float);
        assert_eq!(literal.suffix, Some(NumericLiteralSuffix::F128));
    }

    #[test]
    fn lexes_half_float_suffix() {
        let output = super::super::lex("2.5f16");
        assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
        let literal = match &non_trivia(&output.tokens)[0] {
            TokenKind::NumberLiteral(lit) => lit,
            other => panic!("expected number literal, found {other:?}"),
        };
        assert_eq!(literal.kind, NumericLiteralKind::Float);
        assert_eq!(literal.suffix, Some(NumericLiteralSuffix::F16));
    }

    #[test]
    fn lexes_float_literal_with_suffix_and_separators() {
        let output = super::super::lex("123_456.78_90f32");
        assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
        let kinds = non_trivia(&output.tokens);
        assert_eq!(kinds.len(), 1);
        match kinds[0] {
            TokenKind::NumberLiteral(literal) => {
                assert_eq!(literal.kind, NumericLiteralKind::Float);
                assert_eq!(literal.integer, "123456");
                assert_eq!(literal.fraction, Some(String::from("7890")));
                assert_eq!(
                    literal.suffix,
                    Some(NumericLiteralSuffix::F32),
                    "expected f32 suffix"
                );
            }
            other => panic!("expected number literal, found {other:?}"),
        }
    }

    #[test]
    fn reports_separator_adjacent_to_decimal_point() {
        let output = super::super::lex("1_.0");
        assert!(!output.diagnostics.is_empty());
        assert!(output.diagnostics.iter().any(|diag| {
            diag.message
                .contains("digit separator `_` cannot appear next to a decimal point")
        }));
    }

    #[test]
    fn reports_suffix_not_allowed_on_fractional_literal() {
        let output = super::super::lex("1.0u32");
        assert!(!output.diagnostics.is_empty());
        assert!(output.diagnostics.iter().any(|diag| {
            diag.message
                .contains("suffix `u32` cannot be applied to a literal with a fractional part")
        }));
    }

    #[test]
    fn reports_missing_digits_after_prefix() {
        let output = super::super::lex("0x");
        assert_eq!(output.diagnostics.len(), 1);
        assert!(
            output.diagnostics[0]
                .message
                .contains("numeric literal requires at least one digit")
        );
    }

    #[test]
    fn normalizes_identifier_to_nfc() {
        let output = super::super::lex("A\u{030A}");
        let ident = output
            .tokens
            .iter()
            .find(|token| matches!(token.kind, TokenKind::Identifier))
            .expect("expected identifier token");
        assert_eq!(ident.lexeme, "Å");
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diag| diag.message.contains("NFC-normalised"))
        );
    }

    #[test]
    fn rejects_pattern_whitespace_identifier() {
        let output = super::super::lex("\u{0300}");
        assert!(!output.diagnostics.is_empty());
        assert!(
            output
                .tokens
                .iter()
                .any(|token| matches!(token.kind, TokenKind::Unknown(_)))
        );
    }
}
