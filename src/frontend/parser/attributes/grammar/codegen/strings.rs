use super::*;
use crate::frontend::diagnostics::Span;
use crate::frontend::lexer::TokenKind;
use crate::frontend::literals::{StringLiteral, StringLiteralContents, StringSegment};

parser_impl! {
    pub(in crate::frontend::parser) fn parse_attribute_string_argument(
        &mut self,
        attr_name: &str,
        required: bool,
    ) -> Option<(String, Option<Span>)> {
        if self.consume_punctuation('(') {
            let value = self.expect_attribute_string_literal(attr_name);
            if !self.expect_punctuation(')') {
                self.skip_balanced('(', ')');
            }
            return value.map(|(text, span)| (text, Some(span)));
        }

        match self.peek().cloned() {
            Some(token) if matches!(token.kind, TokenKind::StringLiteral(_)) => {
                let token = self.advance().expect("peeked token must exist");
                if let TokenKind::StringLiteral(literal) = token.kind {
                    let text = match self.attribute_literal_text(
                        attr_name,
                        literal,
                        Some(token.span),
                    ) {
                        Some(text) => text,
                        None => return None,
                    };
                    Some((text, Some(token.span)))
                } else {
                    unreachable!("token kind changed during advance");
                }
            }
            Some(token) if required => {
                self.push_error(
                    format!("expected string literal for `@{attr_name}` attribute"),
                    Some(token.span),
                );
                None
            }
            None if required => {
                self.push_error(
                    format!("expected string literal for `@{attr_name}` attribute"),
                    None,
                );
                None
            }
            _ => None,
        }
    }

    pub(in crate::frontend::parser) fn expect_attribute_string_literal(
        &mut self,
        attr_name: &str,
    ) -> Option<(String, Span)> {
        match self.peek().cloned() {
            Some(token) if matches!(token.kind, TokenKind::StringLiteral(_)) => {
                let token = self.advance().expect("peeked token must exist");
                if let TokenKind::StringLiteral(literal) = token.kind {
                    let span = token.span;
                    self.attribute_literal_text(attr_name, literal, Some(span))
                        .map(|text| (text, span))
                } else {
                    unreachable!("token kind changed during advance");
                }
            }
            Some(token) => {
                self.push_error(
                    format!("expected string literal for `@{attr_name}` attribute"),
                    Some(token.span),
                );
                None
            }
            None => {
                self.push_error(
                    format!("expected string literal for `@{attr_name}` attribute"),
                    None,
                );
                None
            }
        }
    }

    pub(in crate::frontend::parser) fn attribute_literal_text(
        &mut self,
        attr_name: &str,
        literal: StringLiteral,
        span: Option<Span>,
    ) -> Option<String> {
        match literal.contents {
            StringLiteralContents::Simple(text) => Some(text),
            StringLiteralContents::Interpolated(segments) => {
                let mut buffer = String::new();
                for segment in segments {
                    match segment {
                        StringSegment::Text(text) => buffer.push_str(&text),
                        StringSegment::Interpolation(_) => {
                            self.push_error(
                                format!(
                                    "interpolated string literals are not supported in `@{attr_name}`",
                                ),
                                span,
                            );
                            return None;
                        }
                    }
                }
                Some(buffer)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::literals::{InterpolationSegment, StringLiteralKind, StringSegment};
    use crate::frontend::parser::Parser;
    use crate::frontend::parser::tests::fixtures::lex_tokens;

    fn parser_for(source: &str) -> Parser<'_> {
        let lex = lex_tokens(source);
        Parser::new(source, lex)
    }

    #[test]
    fn parse_attribute_string_argument_handles_literal_without_parentheses() {
        let mut parser = parser_for("\"value\"");
        let (value, _) = parser
            .parse_attribute_string_argument("test", true)
            .expect("expected string literal");
        assert_eq!(value, "value");
    }

    #[test]
    fn parse_attribute_string_argument_rejects_numeric_literal() {
        let mut parser = parser_for("(123)");
        assert!(
            parser
                .parse_attribute_string_argument("test", true)
                .is_none()
        );
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("expected string literal")),
            "expected missing string literal diagnostic, found {diagnostics:?}"
        );
    }

    #[test]
    fn attribute_literal_text_rejects_interpolated_segments() {
        let mut parser = parser_for("");
        let literal = StringLiteral {
            kind: StringLiteralKind::Interpolated,
            contents: StringLiteralContents::Interpolated(vec![StringSegment::Interpolation(
                InterpolationSegment {
                    expression: "value".into(),
                    alignment: None,
                    format: None,
                    expression_offset: 0,
                    expression_len: 5,
                },
            )]),
        };
        assert!(
            parser
                .attribute_literal_text("test", literal, None)
                .is_none(),
            "expected interpolation to be rejected"
        );
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics.iter().any(|diag| diag
                .message
                .contains("interpolated string literals are not supported")),
            "expected interpolation diagnostic: {diagnostics:?}"
        );
    }

    #[test]
    fn parse_attribute_string_argument_accepts_parenthesized_literal() {
        let mut parser = parser_for("(\"value\")");
        let (value, span) = parser
            .parse_attribute_string_argument("test", true)
            .expect("expected string literal");
        assert_eq!(value, "value");
        assert!(span.is_some());
    }

    #[test]
    fn parse_attribute_string_argument_respects_optional_flag() {
        let mut parser = parser_for("identifier");
        assert!(
            parser
                .parse_attribute_string_argument("test", false)
                .is_none(),
            "expected optional argument to return None"
        );
    }

    #[test]
    fn expect_attribute_string_literal_reports_error() {
        let mut parser = parser_for("identifier");
        assert!(parser.expect_attribute_string_literal("test").is_none());
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("expected string literal")),
            "expected missing literal diagnostic: {diagnostics:?}"
        );
    }
}
