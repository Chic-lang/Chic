//! Resource-bound statements (`using`, `lock`, `checked`, `unchecked`, `fixed`, `unsafe`).
//!
//! Resource parsing delegates attribute validation to `statements::recovery` helpers.

use super::*;

parser_impl! {
    #[allow(clippy::too_many_lines)]
    pub(super) fn parse_using_statement(&mut self, start_pos: Option<usize>) -> Option<Statement> {
        if self.check_punctuation('(') {
            self.advance();
            let attrs = self.collect_attributes();
            let resource = self.parse_using_resource(attrs, ')', false)?;

            if !self.expect_punctuation(')') {
                return None;
            }

            let body = Box::new(self.parse_embedded_statement()?);
            let span = self.make_span(start_pos);

            Some(Statement::new(
                span,
                                StatementKind::Using(UsingStatement {
                    resource,
                    body: Some(body),
                }),
            ))
        } else {
            let attrs = self.collect_attributes();
            let resource = self.parse_using_resource(attrs, ';', true)?;
            if !self.expect_punctuation(';') {
                return None;
            }
            let span = self.make_span(start_pos);
            Some(Statement::new(
                span,
                                StatementKind::Using(UsingStatement {
                    resource,
                    body: None,
                }),
            ))
        }
    }

    pub(super) fn parse_lock_statement(&mut self, start_pos: Option<usize>) -> Option<Statement> {
        let expression = self.parse_parenthesized_expression("lock")?;
        let body = Box::new(self.parse_embedded_statement()?);
        let span = self.make_span(start_pos);
        Some(Statement::new(
            span,
                        StatementKind::Lock { expression, body },
        ))
    }

    pub(super) fn parse_checked_statement(&mut self, start_pos: Option<usize>) -> Option<Statement> {
        if !self.check_punctuation('{') {
            let span = self.peek().map(|token| token.span);
            self.push_error("checked statement requires a block", span);
            return None;
        }
        let block = self.parse_block()?;
        let span = self.make_span(start_pos);
        Some(Statement::new(span, StatementKind::Checked { body: block }))
    }

    pub(super) fn parse_atomic_statement(&mut self, start_pos: Option<usize>) -> Option<Statement> {
        let ordering = if self.check_punctuation('(') {
            Some(self.parse_parenthesized_expression("atomic")?)
        } else {
            None
        };

        if !self.check_punctuation('{') {
            let span = self.peek().map(|token| token.span);
            self.push_error("atomic statement requires a block", span);
            return None;
        }
        let block = self.parse_block()?;
        let span = self.make_span(start_pos);
        Some(Statement::new(
            span,
            StatementKind::Atomic {
                ordering,
                body: block,
            },
        ))
    }

    pub(super) fn parse_unchecked_statement(&mut self, start_pos: Option<usize>) -> Option<Statement> {
        if !self.check_punctuation('{') {
            let span = self.peek().map(|token| token.span);
            self.push_error("unchecked statement requires a block", span);
            return None;
        }
        let block = self.parse_block()?;
        let span = self.make_span(start_pos);
        Some(Statement::new(span, StatementKind::Unchecked { body: block }))
    }

    pub(super) fn parse_fixed_statement(&mut self, start_pos: Option<usize>) -> Option<Statement> {
        if !self.expect_punctuation('(') {
            return None;
        }

        let attrs = self.collect_attributes();
        let Some(kind) = self.detect_local_declaration() else {
            let span = self.peek().map(|token| token.span);
            self.push_error("expected pointer declaration in fixed statement", span);
            return None;
        };

        let mut decl = match kind {
            LocalDeclStart::Typed {
                ty,
                ty_start,
                name_index,
            } => self.parse_rejected_typed_local(ty, ty_start, name_index, ')', true)?,
            other => {
                let mut decl = self.parse_variable_declaration_with_kind(other, ')', true)?;
                if attrs.builtin.pin {
                    decl.is_pinned = true;
                }
                if !self.expect_punctuation(')') {
                    return None;
                }
                decl
            }
        };
        if attrs.builtin.pin {
            decl.is_pinned = true;
        }

        let body = Box::new(self.parse_embedded_statement()?);
        let span = self.make_span(start_pos);
        Some(Statement::new(
            span,
                        StatementKind::Fixed(FixedStatement {
                declaration: decl,
                body,
            }),
        ))
    }

    pub(super) fn parse_unsafe_statement(&mut self, start_pos: Option<usize>) -> Option<Statement> {
        let body = Box::new(self.parse_embedded_statement()?);
        let span = self.make_span(start_pos);
        Some(Statement::new(span, StatementKind::Unsafe { body }))
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn resource_module_placeholder_scaffolding() {
        assert!(true);
    }
}
