//! Label-related statements (`label:` declarations and goto targets).
//!
//! Synchronisation helpers for labels share `statements::recovery` utilities.

use super::*;

parser_impl! {
    pub(super) fn parse_labeled_statement(&mut self, start_pos: Option<usize>) -> Option<Statement> {
        let label = self.consume_identifier("expected label name")?;
        if !self.expect_punctuation(':') {
            return None;
        }

        let statement = if let Some(stmt) = self.parse_statement() {
            Box::new(stmt)
        } else {
            self.push_error(
                "expected statement after label",
                self.peek().map(|token| token.span),
            );
            Box::new(Statement::new(None, StatementKind::Empty))
        };

        let span = self.make_span(start_pos);
        Some(Statement::new(
            span,
                        StatementKind::Labeled { label, statement },
        ))
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn labels_module_placeholder_scaffolding() {
        assert!(true);
    }
}
