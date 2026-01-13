//! Exception statement parsing including `try`/`catch`/`finally` and `throw`.
//!
//! Catch filter parsing defers to `statements::recovery` helpers for consistent recovery.

use super::*;

parser_impl! {
    #[expect(
        clippy::too_many_lines,
        reason = "Try/catch parsing encompasses filters and finally handling; slated for helper extraction."
    )]
    pub(super) fn parse_try_statement(&mut self, start_pos: Option<usize>) -> Option<Statement> {
        let body_block = self.parse_block()?;
        let mut catches = Vec::new();

        while self.check_keyword(Keyword::Catch) {
            self.advance();
            let mut type_annotation = None;
            let mut identifier = None;

            if self.consume_punctuation('(') {
                if self.check_punctuation(')') {
                    self.advance();
                } else {
                    type_annotation = self.parse_type_expr();
                    if self
                        .peek()
                        .is_some_and(|tok| matches!(tok.kind, TokenKind::Identifier))
                    {
                        identifier =
                            self.consume_identifier("expected catch variable name");
                    }
                    if !self.expect_punctuation(')') {
                        return None;
                    }
                }
            }

            let filter = match self.parse_catch_filter() {
                Ok(filter) => filter,
                Err(()) => return None,
            };

            let catch_body = self.parse_block()?;
            catches.push(CatchClause {
                type_annotation,
                identifier,
                filter,
                body: catch_body,
            });
        }

        let finally_block = if self.match_keyword(Keyword::Finally) {
            Some(self.parse_block()?)
        } else {
            None
        };

        if catches.is_empty() && finally_block.is_none() {
            self.push_error(
                "try statement requires at least one catch or finally clause",
                None,
            );
            return None;
        }

        let span = self.make_span(start_pos);
        Some(Statement::new(
            span,
                        StatementKind::Try(TryStatement {
                body: body_block,
                catches,
                finally: finally_block,
            }),
        ))
    }

    pub(super) fn parse_throw_statement(&mut self, start_pos: Option<usize>) -> Option<Statement> {
        let expression = if self.check_punctuation(';') {
            None
        } else {
            let expr = self.collect_expression_until(&[';']);
            if expr.span.is_some() || !expr.text.trim().is_empty() {
                Some(expr)
            } else {
                None
            }
        };

        if !self.expect_punctuation(';') {
            return None;
        }

        let span = self.make_span(start_pos);
        Some(Statement::new(span, StatementKind::Throw { expression }))
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn exception_module_placeholder_scaffolding() {
        assert!(true);
    }
}
