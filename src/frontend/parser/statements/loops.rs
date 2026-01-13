//! Loop statement parsing: handles `while`, `do-while`, `for`, and `foreach` constructs.
//!
//! Shared initializer and iterator recovery helpers live in `statements::recovery`.

use super::*;

parser_impl! {
    pub(super) fn parse_while_statement(&mut self, start_pos: Option<usize>) -> Option<Statement> {
        let condition = self.parse_parenthesized_expression("while")?;
        let body = Box::new(self.parse_embedded_statement()?);
        let span = self.make_span(start_pos);
        Some(Statement::new(
            span,
                        StatementKind::While { condition, body },
        ))
    }

    pub(super) fn parse_do_statement(&mut self, start_pos: Option<usize>) -> Option<Statement> {
        let body = Box::new(self.parse_embedded_statement()?);
        if !self.match_keyword(Keyword::While) {
            let span = self.peek().map(|token| token.span);
            self.push_error("expected 'while' after do body", span);
            return None;
        }
        let condition = self.parse_parenthesized_expression("do-while")?;
        if !self.expect_punctuation(';') {
            return None;
        }
        let span = self.make_span(start_pos);
        Some(Statement::new(
            span,
                        StatementKind::DoWhile { body, condition },
        ))
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn parse_for_statement(&mut self, start_pos: Option<usize>) -> Option<Statement> {
        if !self.expect_punctuation('(') {
            return None;
        }

        let initializer = self.parse_for_initializer()?;

        let condition = if self.check_punctuation(';') {
            self.advance();
            None
        } else {
            let expr = self.collect_pattern_expression_until(&[';']);
            if !self.expect_punctuation(';') {
                return None;
            }
            if expr.span.is_some() || !expr.text.is_empty() {
                Some(expr)
            } else {
                None
            }
        };

        let iterator = self.parse_iteration_list();

        if !self.expect_punctuation(')') {
            return None;
        }

        let body = Box::new(self.parse_embedded_statement()?);
        let span = self.make_span(start_pos);
        Some(Statement::new(
            span,
                        StatementKind::For(ForStatement {
                initializer,
                condition,
                iterator,
                body,
            }),
        ))
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn parse_foreach_statement(&mut self, start_pos: Option<usize>) -> Option<Statement> {
        if !self.expect_punctuation('(') {
            return None;
        }

        let binding_start = self.index;
        let mut depth = 0usize;
        while let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Punctuation('(') => {
                    depth += 1;
                    self.advance();
                }
                TokenKind::Punctuation(')') => {
                    if depth == 0 {
                        self.push_error("expected 'in' in foreach", Some(token.span));
                        return None;
                    }
                    depth -= 1;
                    self.advance();
                }
                TokenKind::Keyword(Keyword::In) if depth == 0 => break,
                _ => {
                    self.advance();
                }
            }
        }

        let binding_end = self.index;
        let binding_span = self.span_from_indices(binding_start, binding_end);
        let binding_text = binding_span
            .map(|sp| self.text_from_span(sp))
            .unwrap_or_default()
            .trim()
            .to_string();

        self.ensure_foreach_binding_uses_let_or_var(&binding_text, binding_span);

        if !self.match_keyword(Keyword::In) {
            let span = self.peek().map(|token| token.span);
            self.push_error("expected 'in' in foreach statement", span);
            return None;
        }

        let expression = self.collect_expression_until(&[')']);
        if !self.expect_punctuation(')') {
            return None;
        }

        let body = Box::new(self.parse_embedded_statement()?);
        let span = self.make_span(start_pos);
        Some(Statement::new(
            span,
                        StatementKind::Foreach(ForeachStatement {
                binding: binding_text,
                binding_span,
                expression,
                body,
            }),
        ))
    }
}

parser_impl! {
    fn ensure_foreach_binding_uses_let_or_var(
        &mut self,
        binding_text: &str,
        binding_span: Option<Span>,
    ) {
        let tokens = lex(binding_text)
            .tokens
            .into_iter()
            .filter(|token| {
                !matches!(
                    token.kind,
                    TokenKind::Whitespace | TokenKind::Comment | TokenKind::Unknown(_)
                )
            })
            .collect::<Vec<_>>();
        if tokens.is_empty() {
            return;
        }

        let mut index = 0usize;
        if matches!(tokens.get(index), Some(Token { kind: TokenKind::Keyword(Keyword::In), .. })) {
            index += 1;
        }
        if matches!(tokens.get(index), Some(Token { kind: TokenKind::Keyword(Keyword::Ref), .. })) {
            index += 1;
            if tokens.get(index).is_some_and(|token| {
                matches!(token.kind, TokenKind::Identifier)
                    && token.lexeme.eq_ignore_ascii_case("readonly")
            }) {
                index += 1;
            }
        }

        match tokens.get(index) {
            Some(Token {
                kind: TokenKind::Keyword(Keyword::Let | Keyword::Var),
                ..
            }) => {}
            Some(token) => {
                let name_token = tokens.get(index + 1);
                let decl = VariableDeclaration {
                    modifier: VariableModifier::Var,
                    type_annotation: None,
                    declarators: vec![VariableDeclarator {
                        name: name_token
                            .map(|tok| tok.lexeme.clone())
                            .unwrap_or_else(|| "binding".to_string()),
                        initializer: None,
                    }],
                    is_pinned: false,
                };
                self.emit_typed_local_error(
                    Some(binding_span.unwrap_or(token.span)),
                    name_token.map(|tok| tok.span),
                    Some(&decl),
                );
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn loops_module_placeholder_scaffolding() {
        assert!(true);
    }
}
