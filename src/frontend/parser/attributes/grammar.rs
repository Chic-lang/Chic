//! Attribute grammar orchestrator; delegates builtin handling to the focused
//! `layout`, `codegen`, and `diagnostic` modules while keeping shared helpers in
//! `shared`.

use super::*;
use crate::frontend::ast::AttributeKind;
use crate::frontend::lexer::TokenKind;

mod codegen;
mod conditional;
mod diagnostic;
mod layout;
mod shared;

parser_impl! {
    pub(in crate::frontend::parser) fn skip_attributes(&mut self) {
        loop {
            self.stash_leading_doc();
            let Some(token) = self.peek() else {
                return;
            };
            if token.kind != TokenKind::Punctuation('@') {
                return;
            }
            self.advance();
            if matches!(self.peek(), Some(t) if matches!(t.kind, TokenKind::Identifier)) {
                self.advance();
            }
            if self.consume_punctuation('(') {
                self.skip_balanced('(', ')');
            }
        }
    }

    pub(in crate::frontend::parser) fn collect_attributes(&mut self) -> CollectedAttributes {
        let mut attrs = CollectedAttributes::default();
        loop {
            let Some(token) = self.peek().cloned() else {
                break;
            };
            if !matches!(token.kind, TokenKind::Punctuation('@')) {
                break;
            }
            self.advance();

            let attr_start = token.span.start;
            let Some((name, _name_span)) = self.parse_attribute_name() else {
                continue;
            };
            let lowered = name.to_ascii_lowercase();
            let handled_builtin =
                diagnostic::handle_diagnostic_attribute(self, lowered.as_str(), token.span, &mut attrs)
                || conditional::handle_conditional_attribute(self, lowered.as_str(), token.span, &mut attrs)
                || layout::handle_layout_attribute(self, lowered.as_str(), token.span, &mut attrs)
                || codegen::handle_codegen_attribute(self, lowered.as_str(), token.span, &mut attrs);

            if handled_builtin {
                let end = self.last_span.map_or(token.span.end, |span| span.end);
                let attribute =
                    self.make_attribute(name.clone(), attr_start, end, AttributeKind::Builtin);
                attrs.push(attribute);
                continue;
            }

            if self.consume_punctuation('(') {
                self.skip_balanced('(', ')');
            }

            let end = self.last_span.map_or(token.span.end, |span| span.end);
            let attribute =
                self.make_attribute(name.clone(), attr_start, end, AttributeKind::Macro);
            attrs.push(attribute);
        }
        attrs
    }
}

#[cfg(test)]
mod tests {
    use crate::frontend::lexer::TokenKind;
    use crate::frontend::parser::Parser;
    use crate::frontend::parser::tests::fixtures::lex_tokens;

    fn parser_for(source: &str) -> Parser<'_> {
        let lex = lex_tokens(source);
        Parser::new(source, lex)
    }

    #[test]
    fn skip_attributes_consumes_repeated_attributes() {
        let mut parser = parser_for("@foo()\n@bar()\n");
        parser.skip_attributes();
        assert!(parser.is_at_end(), "expected all tokens to be consumed");
    }

    #[test]
    fn collect_attributes_retains_macro_attributes() {
        let mut parser = parser_for("@custom()\nlet value = 1;");
        let attrs = parser.collect_attributes();
        assert!(
            !attrs.list.is_empty(),
            "expected macro attribute to be recorded"
        );
        let attr = &attrs.list[0];
        assert_eq!(attr.name, "custom");
        let next = parser.peek().expect("expected token after attributes");
        assert!(
            !matches!(next.kind, TokenKind::Punctuation('@')),
            "attributes should have been consumed"
        );
    }
}
