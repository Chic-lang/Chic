//! Simple statements covering declarations, expressions, returns, yields, and gotos.
//!
//! Terminator handling delegates to `statements::recovery` helpers for consistency.

use super::*;

parser_impl! {
    pub(super) fn parse_block_statement(
        &mut self,
        _start_pos: Option<usize>,
    ) -> Option<Statement> {
        let block = self.parse_block()?;
        let span = block.span;
        Some(Statement::new(span, StatementKind::Block(block)))
    }

    pub(super) fn parse_empty_statement(
        &mut self,
        _start_pos: Option<usize>,
    ) -> Option<Statement> {
        if let Some(token) = self.advance() {
            Some(Statement::new(Some(token.span), StatementKind::Empty))
        } else {
            None
        }
    }

    pub(super) fn parse_break_statement(
        &mut self,
        start_pos: Option<usize>,
    ) -> Option<Statement> {
        if !self.expect_punctuation(';') {
            return None;
        }
        let span = self.make_span(start_pos);
        Some(Statement::new(span, StatementKind::Break))
    }

    pub(super) fn parse_continue_statement(
        &mut self,
        start_pos: Option<usize>,
    ) -> Option<Statement> {
        if !self.expect_punctuation(';') {
            return None;
        }
        let span = self.make_span(start_pos);
        Some(Statement::new(span, StatementKind::Continue))
    }

    pub(super) fn parse_variable_declaration_statement(
        &mut self,
        start_pos: Option<usize>,
        start_kind: LocalDeclStart,
    ) -> Option<Statement> {
        match start_kind {
            LocalDeclStart::Const => {
                self.match_keyword(Keyword::Const);
                let mut declaration = self.parse_const_declaration_body(None, ';')?;
                if !self.expect_punctuation(';') {
                    return None;
                }
                let span = self.make_span(start_pos);
                declaration.span = span;
                Some(Statement::new(
                    span,
                    StatementKind::ConstDeclaration(ConstStatement { declaration }),
                ))
            }
            LocalDeclStart::Typed {
                ty,
                ty_start,
                name_index,
            } => {
                let decl =
                    self.parse_rejected_typed_local(ty, ty_start, name_index, ';', false);
                let span = self.make_span(start_pos);
                Some(Statement::new(
                    span,
                    decl.map_or(StatementKind::Empty, StatementKind::VariableDeclaration),
                ))
            }
            kind => {
                let decl = self.parse_variable_declaration_with_kind(kind, ';', false)?;
                if !self.expect_punctuation(';') {
                    return None;
                }
                let span = self.make_span(start_pos);
                Some(Statement::new(
                    span,
                    StatementKind::VariableDeclaration(decl),
                ))
            }
        }
    }

    pub(super) fn parse_expression_statement(&mut self, start_pos: Option<usize>) -> Option<Statement> {
        let expr = self.parse_expression_until(';')?;
        let span = self.make_span(start_pos);
        Some(Statement::new(span, StatementKind::Expression(expr)))
    }

    pub(super) fn parse_return_statement(&mut self, start_pos: Option<usize>) -> Option<Statement> {
        if self.check_punctuation(';') {
            self.advance();
            let span = self.make_span(start_pos);
            return Some(Statement::new(
                span,
                                StatementKind::Return { expression: None },
            ));
        }

        let expr = self.parse_expression_until(';')?;
        let expr_opt = if expr.span.is_some() || !expr.text.is_empty() {
            Some(expr)
        } else {
            None
        };
        let span = self.make_span(start_pos);
        Some(Statement::new(
            span,
                        StatementKind::Return {
                expression: expr_opt,
            },
        ))
    }

    pub(super) fn parse_yield_statement(&mut self, start_pos: Option<usize>) -> Option<Statement> {
        if self.match_keyword(Keyword::Return) {
            let expr = self.parse_pattern_expression_until(';')?;
            let span = self.make_span(start_pos);
            return Some(Statement::new(
                span,
                                StatementKind::YieldReturn { expression: expr },
            ));
        }

        if self.match_keyword(Keyword::Break) {
            if !self.expect_punctuation(';') {
                return None;
            }
            let span = self.make_span(start_pos);
            return Some(Statement::new(span, StatementKind::YieldBreak));
        }

        let span = self.peek().map(|token| token.span);
        self.push_error("expected 'return' or 'break' after 'yield'", span);
        None
    }

    pub(super) fn parse_goto_statement(&mut self, start_pos: Option<usize>) -> Option<Statement> {
        if self.match_keyword(Keyword::Case) {
            let (pattern, guards) = self.collect_case_pattern_and_guards(&[';']);
            if !self.expect_punctuation(';') {
                return None;
            }
            let span = self.make_span(start_pos);
            return Some(Statement::new(
                span,
                                StatementKind::Goto(GotoStatement {
                    target: GotoTarget::Case { pattern, guards },
                }),
            ));
        }

        if self.match_keyword(Keyword::Default) {
            if !self.expect_punctuation(';') {
                return None;
            }
            let span = self.make_span(start_pos);
            return Some(Statement::new(
                span,
                                StatementKind::Goto(GotoStatement {
                    target: GotoTarget::Default,
                }),
            ));
        }

        let label = self.consume_identifier("expected label after 'goto'")?;
        if !self.expect_punctuation(';') {
            return None;
        }
        let span = self.make_span(start_pos);
        Some(Statement::new(
            span,
                        StatementKind::Goto(GotoStatement {
                target: GotoTarget::Label(label),
            }),
        ))
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn simple_module_placeholder_scaffolding() {
        assert!(true);
    }
}
