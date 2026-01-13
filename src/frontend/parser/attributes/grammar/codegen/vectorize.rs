use super::*;
use crate::frontend::ast::VectorizeHint;
use crate::frontend::lexer::TokenKind;

parser_impl! {
    pub(in crate::frontend::parser) fn parse_vectorize_attribute(
        &mut self,
        attr_span: Option<Span>,
    ) -> Option<VectorizeHint> {
        let has_parens = self.consume_punctuation('(');
        let Some(token) = self.peek().cloned() else {
            self.push_error(
                "`@vectorize` attribute requires a target such as `decimal`",
                attr_span,
            );
            if has_parens {
                self.skip_balanced('(', ')');
            }
            return None;
        };

        if has_parens && matches!(token.kind, TokenKind::Punctuation(')')) {
            self.push_error(
                "`@vectorize` attribute requires a target such as `decimal`",
                Some(token.span),
            );
            self.advance();
            return None;
        }

        let (target, target_span) = match token.kind {
            TokenKind::Identifier => {
                self.advance();
                (token.lexeme.to_ascii_lowercase(), token.span)
            }
            TokenKind::StringLiteral(literal) => {
                self.advance();
                match self.attribute_literal_text("vectorize", literal, Some(token.span)) {
                    Some(text) => (text.to_ascii_lowercase(), token.span),
                    None => return None,
                }
            }
            _ => {
                self.push_error(
                    "`@vectorize` attribute requires an identifier target like `decimal`",
                    Some(token.span),
                );
                if has_parens {
                    self.skip_balanced('(', ')');
                }
                return None;
            }
        };

        if has_parens && !self.expect_punctuation(')') {
            self.skip_balanced('(', ')');
        }

        if target != "decimal" {
            self.push_error(
                format!("`@vectorize` only supports `decimal`, found `{target}`"),
                Some(target_span),
            );
            if has_parens {
                self.skip_balanced('(', ')');
            }
            return None;
        }

        Some(VectorizeHint::Decimal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::parser::Parser;
    use crate::frontend::parser::tests::fixtures::lex_tokens;

    fn parser_for(source: &str) -> Parser<'_> {
        let lex = lex_tokens(source);
        Parser::new(source, lex)
    }

    #[test]
    fn parse_vectorize_attribute_accepts_string_literal() {
        let mut parser = parser_for("(\"decimal\")");
        let hint = parser
            .parse_vectorize_attribute(Some(Span::new(0, 0)))
            .expect("expected vectorize hint");
        assert!(matches!(hint, VectorizeHint::Decimal));
    }

    #[test]
    fn parse_vectorize_attribute_accepts_identifier_without_parentheses() {
        let mut parser = parser_for("decimal");
        let hint = parser
            .parse_vectorize_attribute(None)
            .expect("expected vectorize hint");
        assert!(matches!(hint, VectorizeHint::Decimal));
    }

    #[test]
    fn parse_vectorize_attribute_rejects_non_decimal_target() {
        let mut parser = parser_for("(foo)");
        assert!(parser.parse_vectorize_attribute(None).is_none());
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("only supports `decimal`")),
            "expected vectorize diagnostic, found {diagnostics:?}"
        );
    }

    #[test]
    fn parse_vectorize_attribute_reports_missing_target() {
        let mut parser = parser_for("()");
        assert!(parser.parse_vectorize_attribute(None).is_none());
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics
                .iter()
                .any(|diag| diag.message.contains("requires a target")),
            "expected missing target diagnostic, found {diagnostics:?}"
        );
    }
}
