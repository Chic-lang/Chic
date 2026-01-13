//! Shared attribute parsing helpers that back the focused grammar modules.

use super::super::{ParsedAttributeArgument, ParsedAttributeValue};
use super::utils;
use super::*;
use crate::frontend::ast::{
    Attribute, AttributeArgument as AstAttributeArgument, AttributeKind, AttributeMacroMetadata,
};
use crate::frontend::diagnostics::Span;
use crate::frontend::lexer::{Keyword, Token, TokenKind};
use crate::frontend::literals::{StringLiteralContents, StringSegment};

parser_impl! {
    pub(in crate::frontend::parser) fn parse_attribute_kv_arguments(
        &mut self,
        attr_name: &str,
    ) -> Option<Vec<ParsedAttributeArgument>> {
        let Some(token) = self.peek().cloned() else {
            self.push_error(
                format!("`@{attr_name}` attribute expects parentheses with arguments"),
                None,
            );
            return None;
        };

        if token.kind != TokenKind::Punctuation('(') {
            self.push_error(
                format!("`@{attr_name}` attribute expects parentheses with arguments"),
                Some(token.span),
            );
            return None;
        }
        self.advance();

        if self.consume_punctuation(')') {
            return Some(Vec::new());
        }

        let mut args = Vec::new();
        loop {
            let Some((key, key_span)) = self.parse_attribute_name() else {
                self.skip_balanced('(', ')');
                return None;
            };
            if !self.consume_operator("=") {
                self.push_error(
                    format!("expected '=' after `{key}` in `@{attr_name}` attribute"),
                    self.peek().map(|t| t.span).or(Some(key_span)),
                );
                self.skip_balanced('(', ')');
                return None;
            }
            let key_lower = key.to_ascii_lowercase();
            let Some((value, value_span)) =
                self.parse_attribute_value(attr_name, &key_lower)
            else {
                self.skip_balanced('(', ')');
                return None;
            };
            let span = Span::in_file(key_span.file_id, key_span.start, value_span.end);
            if args
                .iter()
                .any(|arg: &ParsedAttributeArgument| arg.name == key_lower)
            {
                self.push_error(
                    format!("duplicate `{key_lower}` argument in `@{attr_name}`"),
                    Some(span),
                );
            } else {
                args.push(ParsedAttributeArgument {
                    name: key_lower,
                    value,
                    span,
                });
            }

            if self.consume_punctuation(')') {
                break;
            }
            if !self.consume_punctuation(',') {
                self.push_error(
                    format!("expected ',' or ')' in `@{attr_name}` attribute"),
                    self.peek().map(|t| t.span),
                );
                self.skip_balanced('(', ')');
                return None;
            }
        }

        Some(args)
    }

    pub(in crate::frontend::parser) fn parse_attribute_value(
        &mut self,
        attr_name: &str,
        key: &str,
    ) -> Option<(ParsedAttributeValue, Span)> {
        let Some(token) = self.peek().cloned() else {
            self.push_error(
                format!("expected value for `{key}` in `@{attr_name}` attribute"),
                None,
            );
            return None;
        };

        match token.kind {
            TokenKind::NumberLiteral(ref literal) => {
                self.advance();
                match utils::parse_u64_numeric_literal(literal) {
                    Some(value) => Some((ParsedAttributeValue::Int(value), token.span)),
                    None => {
                        self.push_error(
                            format!(
                                "invalid numeric literal `{}` in `@{attr_name}` attribute",
                                token.lexeme
                            ),
                            Some(token.span),
                        );
                        None
                    }
                }
            }
            TokenKind::StringLiteral(literal) => {
                self.advance();
                match literal.contents {
                    StringLiteralContents::Simple(text) => {
                        Some((ParsedAttributeValue::Str(text), token.span))
                    }
                    StringLiteralContents::Interpolated(segments) => {
                        let mut buffer = String::new();
                        for segment in segments {
                            match segment {
                                StringSegment::Text(text) => buffer.push_str(&text),
                                StringSegment::Interpolation(_) => {
                                    self.push_error(
                                        "interpolated string literals are not supported here",
                                        Some(token.span),
                                    );
                                    return None;
                                }
                            }
                        }
                        Some((ParsedAttributeValue::Str(buffer), token.span))
                    }
                }
            }
            TokenKind::Identifier => {
                self.advance();
                let lowered = token.lexeme.to_ascii_lowercase();
                if lowered == "true" {
                    Some((ParsedAttributeValue::Bool(true), token.span))
                } else if lowered == "false" {
                    Some((ParsedAttributeValue::Bool(false), token.span))
                } else {
                    Some((ParsedAttributeValue::Str(token.lexeme), token.span))
                }
            }
            TokenKind::Keyword(keyword) if matches!(keyword, Keyword::Not) => {
                self.push_error(
                    format!("unexpected keyword in `@{attr_name}` attribute"),
                    Some(token.span),
                );
                None
            }
            _ => {
                self.push_error(
                    format!("unsupported value for `{key}` in `@{attr_name}` attribute"),
                    Some(token.span),
                );
                None
            }
        }
    }

    pub(in crate::frontend::parser) fn make_attribute(
        &self,
        name: String,
        attr_start: usize,
        attr_end: usize,
        kind: AttributeKind,
    ) -> Attribute {
        let span = if attr_end > attr_start {
            Some(Span::in_file(self.file_id, attr_start, attr_end))
        } else {
            None
        };
        let raw = span.and_then(|sp| self.source.get(sp.start..sp.end).map(|s| s.to_string()));
        let arguments = self.parse_attribute_arguments_from_raw(raw.as_deref(), attr_start);
        let tokens = span
            .map(|sp| self.attribute_tokens(sp))
            .unwrap_or_default();
        let macro_metadata = AttributeMacroMetadata::new(matches!(kind, AttributeKind::Macro), tokens);
        Attribute::new(name, arguments, span, raw, kind).with_macro_metadata(macro_metadata)
    }

    pub(in crate::frontend::parser) fn attribute_tokens(&self, span: Span) -> Vec<Token> {
        self.tokens
            .iter()
            .filter(|token| token.span.start >= span.start && token.span.end <= span.end)
            .filter(|token| {
                !matches!(
                    token.kind,
                    TokenKind::Whitespace | TokenKind::Comment | TokenKind::DocComment
                )
            })
            .cloned()
            .collect()
    }

    pub(in crate::frontend::parser) fn parse_attribute_arguments_from_raw(
        &self,
        raw: Option<&str>,
        attr_start: usize,
    ) -> Vec<AstAttributeArgument> {
        let Some(raw) = raw else {
            return Vec::new();
        };

        let open_idx = raw.find('(').unwrap_or(0);
        let mut close_idx = None;
        let mut depth = 0usize;
        let mut in_string: Option<char> = None;
        let mut escape = false;

        for (idx, ch) in raw.char_indices() {
            if idx < open_idx {
                continue;
            }
            if let Some(delim) = in_string {
                if escape {
                    escape = false;
                    continue;
                }
                if ch == '\\' {
                    escape = true;
                    continue;
                }
                if ch == delim {
                    in_string = None;
                }
                continue;
            }
            match ch {
                '(' => depth += 1,
                ')' => {
                    if depth > 0 {
                        depth -= 1;
                    }
                    if depth == 0 {
                        close_idx = Some(idx);
                        break;
                    }
                }
                '"' | '\'' => in_string = Some(ch),
                _ => {}
            }
        }

        let Some(close_idx) = close_idx else {
            return Vec::new();
        };

        let inner = &raw[open_idx + 1..close_idx];
        let base_offset = attr_start + open_idx + 1;
        self.split_attribute_arguments(inner, base_offset)
    }

    pub(in crate::frontend::parser) fn split_attribute_arguments(
        &self,
        text: &str,
        base_offset: usize,
    ) -> Vec<AstAttributeArgument> {
        let mut segments = Vec::new();
        let mut segment_start = 0usize;
        let mut separator_index: Option<usize> = None;
        let mut paren_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut angle_depth = 0usize;
        let mut in_string: Option<char> = None;
        let mut escape = false;

        for (idx, ch) in text.char_indices() {
            if let Some(delim) = in_string {
                if escape {
                    escape = false;
                    continue;
                }
                if ch == '\\' {
                    escape = true;
                    continue;
                }
                if ch == delim {
                    in_string = None;
                }
                continue;
            }

            match ch {
                '(' => paren_depth += 1,
                ')' => {
                    if paren_depth > 0 {
                        paren_depth -= 1;
                    }
                }
                '{' => brace_depth += 1,
                '}' => {
                    if brace_depth > 0 {
                        brace_depth -= 1;
                    }
                }
                '[' => bracket_depth += 1,
                ']' => {
                    if bracket_depth > 0 {
                        bracket_depth -= 1;
                    }
                }
                '<' => angle_depth += 1,
                '>' => {
                    if angle_depth > 0 {
                        angle_depth -= 1;
                    }
                }
                '"' | '\'' => in_string = Some(ch),
                '=' | ':'
                    if paren_depth == 0
                        && brace_depth == 0
                        && bracket_depth == 0
                        && angle_depth == 0
                        && separator_index.is_none() =>
                {
                    separator_index = Some(idx);
                }
                ','
                    if paren_depth == 0
                        && brace_depth == 0
                        && bracket_depth == 0
                        && angle_depth == 0 =>
                {
                    segments.push((segment_start, idx, separator_index));
                    segment_start = idx + ch.len_utf8();
                    separator_index = None;
                }
                _ => {}
            }
        }

        segments.push((segment_start, text.len(), separator_index));

        let mut results = Vec::new();
        for (start, end, sep_index) in segments {
            if start >= end {
                continue;
            }
            let segment = &text[start..end];
            let trimmed_start_str = segment.trim_start();
            let leading = segment.len() - trimmed_start_str.len();
            let trimmed = trimmed_start_str.trim_end();
            if trimmed.is_empty() {
                continue;
            }
            let trailing = trimmed_start_str.len() - trimmed.len();
            let trimmed_start = start + leading;
            let trimmed_end = end - trailing;
            let abs_start = base_offset + trimmed_start;
            let abs_end = base_offset + trimmed_end;
            let span = Some(Span::in_file(self.file_id, abs_start, abs_end));
            let trimmed_slice = &text[trimmed_start..trimmed_end];

            let (name, value) = if let Some(sep) = sep_index {
                if sep >= trimmed_start && sep < trimmed_end {
                    let name_slice = &text[trimmed_start..sep];
                    let value_slice = &text[sep + 1..trimmed_end];
                    let name_text = name_slice.trim();
                    let value_text = value_slice.trim();
                    let name_value = if name_text.is_empty() {
                        None
                    } else {
                        Some(name_text.to_string())
                    };
                    (name_value, value_text.to_string())
                } else {
                    (None, trimmed_slice.to_string())
                }
            } else {
                (None, trimmed_slice.to_string())
            };

            results.push(AstAttributeArgument::new(name, value, span));
        }

        results
    }

    pub(in crate::frontend::parser) fn parse_attribute_name(&mut self) -> Option<(String, Span)> {
        let Some(token) = self.peek().cloned() else {
            self.push_error("expected attribute name", None);
            return None;
        };

        if token.kind != TokenKind::Identifier {
            self.push_error("expected attribute name", Some(token.span));
            return None;
        }

        self.advance();
        let mut name = token.lexeme.clone();
        let mut last_span = token.span;
        while self.consume_punctuation('.') {
            match self.peek().cloned() {
                Some(Token {
                    kind: TokenKind::Identifier,
                    lexeme,
                    span,
                }) => {
                    self.advance();
                    name.push('.');
                    name.push_str(&lexeme);
                    last_span = span;
                }
                Some(other) => {
                    self.push_error(
                        "expected identifier after '.' in attribute name",
                        Some(other.span),
                    );
                    break;
                }
                None => {
                    self.push_error("expected identifier after '.' in attribute name", None);
                    break;
                }
            }
        }
        Some((name, last_span))
    }

    pub(in crate::frontend::parser) fn skip_balanced(&mut self, open: char, close: char) {
        let mut depth = 1usize;
        while let Some(token) = self.advance() {
            match token.kind {
                TokenKind::Punctuation(ch) if ch == open => depth += 1,
                TokenKind::Punctuation(ch) if ch == close => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
        if depth != 0 {
            self.push_error(format!("unterminated delimiter '{open}...{close}'"), None);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::frontend::parser::Parser;
    use crate::frontend::parser::attributes::ParsedAttributeValue;
    use crate::frontend::parser::tests::fixtures::lex_tokens;

    fn parser_for(source: &str) -> Parser<'_> {
        let lex = lex_tokens(source);
        Parser::new(source, lex)
    }

    #[test]
    fn parse_attribute_name_accepts_qualified_segments() {
        let mut parser = parser_for("outer.inner.leaf");
        let (name, _) = parser
            .parse_attribute_name()
            .expect("expected attribute name");
        assert_eq!(name, "outer.inner.leaf");
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics.is_empty(),
            "unexpected diagnostics: {diagnostics:?}"
        );
    }

    #[test]
    fn split_attribute_arguments_handles_nested_calls() {
        let parser = parser_for(" ");
        let args =
            parser.split_attribute_arguments("first = foo(bar, baz(42)), second = value", 10);
        assert_eq!(args.len(), 2, "expected two arguments: {args:?}");
        assert_eq!(args[0].name.as_deref(), Some("first"));
        assert_eq!(args[1].name.as_deref(), Some("second"));
    }

    #[test]
    fn parse_attribute_value_rejects_keywords() {
        let mut parser = parser_for("not");
        assert!(
            parser.parse_attribute_value("test", "value").is_none(),
            "expected keyword to be rejected"
        );
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("unexpected keyword")),
            "expected keyword diagnostic: {diagnostics:?}"
        );
    }

    #[test]
    fn parse_attribute_value_handles_numeric_and_bool_literals() {
        let mut parser = parser_for("42 true");
        let (value, _) = parser
            .parse_attribute_value("test", "number")
            .expect("expected numeric literal");
        assert!(matches!(value, ParsedAttributeValue::Int(42)));

        let (value, _) = parser
            .parse_attribute_value("test", "bool")
            .expect("expected bool literal");
        assert!(matches!(value, ParsedAttributeValue::Bool(true)));
    }

    #[test]
    fn parse_attribute_kv_arguments_reports_duplicate_keys() {
        let mut parser = parser_for("(Foo = 1, Foo = 2)");
        let args = parser
            .parse_attribute_kv_arguments("attr")
            .expect("expected arguments result");
        assert_eq!(args.len(), 1, "expected only the first argument to be kept");
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("duplicate `foo`")),
            "expected duplicate key diagnostic: {diagnostics:?}"
        );
    }
}
