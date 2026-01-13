use super::*;
use crate::frontend::ast::InlineAttr;
use crate::frontend::lexer::{Token, TokenKind};

parser_impl! {
    pub(in crate::frontend::parser) fn parse_inline_attribute(
        &mut self,
        attr_span: Option<Span>,
    ) -> Option<InlineAttr> {
        if !self.consume_punctuation('(') {
            return Some(InlineAttr::Local);
        }
        if self.consume_punctuation(')') {
            return Some(InlineAttr::Local);
        }
        let (value_text, value_span) = match self.peek().cloned() {
            Some(Token {
                kind: TokenKind::Identifier,
                lexeme,
                span,
            }) => {
                self.advance();
                (lexeme.to_ascii_lowercase(), span)
            }
            Some(token) => {
                self.push_error("expected inline strategy identifier", Some(token.span));
                self.skip_balanced('(', ')');
                return None;
            }
            None => {
                self.push_error("expected inline strategy identifier", attr_span);
                return None;
            }
        };

        if !self.expect_punctuation(')') {
            self.skip_balanced('(', ')');
            return None;
        }

        match value_text.as_str() {
            "local" => Some(InlineAttr::Local),
            "cross" => Some(InlineAttr::Cross),
            _ => {
                self.push_error(
                    "inline attribute must be `local` or `cross`",
                    Some(value_span),
                );
                None
            }
        }
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
    fn parse_inline_attribute_accepts_cross_variant() {
        let mut parser = parser_for("(cross)");
        let attr = parser
            .parse_inline_attribute(None)
            .expect("expected inline attribute");
        assert!(matches!(attr, InlineAttr::Cross));
    }

    #[test]
    fn parse_inline_attribute_defaults_to_local() {
        let mut parser = parser_for("()");
        let attr = parser
            .parse_inline_attribute(None)
            .expect("expected inline attribute");
        assert!(matches!(attr, InlineAttr::Local));
    }

    #[test]
    fn parse_inline_attribute_rejects_unknown_identifier() {
        let mut parser = parser_for("(unknown)");
        assert!(parser.parse_inline_attribute(None).is_none());
        let (diagnostics, _) = parser.finish();
        assert!(
            diagnostics.iter().any(|diag| diag
                .message
                .contains("inline attribute must be `local` or `cross`")),
            "expected inline diagnostic, found {diagnostics:?}"
        );
    }

    #[test]
    fn parse_inline_attribute_defaults_to_local_without_parentheses() {
        let mut parser = parser_for("");
        let attr = parser
            .parse_inline_attribute(None)
            .expect("expected inline attribute");
        assert!(matches!(attr, InlineAttr::Local));
    }

    #[test]
    fn parse_inline_attribute_rejects_non_identifier() {
        let mut parser = parser_for("(\"text\")");
        assert!(parser.parse_inline_attribute(None).is_none());
    }
}
