//! Conditional and selection statement parsing covering `if` and `switch` constructs.
//!
//! Switch label parsing delegates to `statements::recovery` helpers for guard handling.

use super::*;

parser_impl! {
    pub(super) fn parse_if_statement(&mut self, start_pos: Option<usize>) -> Option<Statement> {
        let condition = self.parse_parenthesized_expression("if")?;
        let then_branch = Box::new(self.parse_embedded_statement()?);
        let else_branch = if self.check_keyword(Keyword::Else) {
            self.advance();
            Some(Box::new(self.parse_embedded_statement()?))
        } else {
            None
        };

        let span = self.make_span(start_pos);
        Some(Statement::new(
            span,
                        StatementKind::If(IfStatement {
                condition,
                then_branch,
                else_branch,
            }),
        ))
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn parse_switch_statement(&mut self, start_pos: Option<usize>) -> Option<Statement> {
        let expression = self.parse_parenthesized_expression("switch")?;
        if !self.expect_punctuation('{') {
            return None;
        }

        let mut sections = Vec::new();
        while !self.check_punctuation('}') && !self.is_at_end() {
            let mut labels = Vec::new();
            loop {
                match self.parse_switch_label() {
                    Ok(Some(label)) => labels.push(label),
                    Ok(None) => break,
                    Err(()) => return None,
                }
            }

            if labels.is_empty() {
                let span = self.peek().map(|token| token.span);
                self.push_error("expected 'case' or 'default' label", span);
                self.synchronize_statement();
                if self.check_punctuation('}') {
                    break;
                }
                continue;
            }

            let mut statements = Vec::new();
            while !self.check_keyword(Keyword::Case)
                && !self.check_keyword(Keyword::Default)
                && !self.check_punctuation('}')
            {
                if let Some(stmt) = self.parse_statement() {
                    statements.push(stmt);
                } else {
                    self.synchronize_statement();
                    if self.check_punctuation('}') {
                        break;
                    }
                }
            }

            sections.push(SwitchSection { labels, statements });
        }

        if !self.expect_punctuation('}') {
            return None;
        }

        let span = self.make_span(start_pos);
        Some(Statement::new(
            span,
                        StatementKind::Switch(SwitchStatement { expression, sections }),
        ))
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn selection_module_placeholder_scaffolding() {
        assert!(true);
    }
}
