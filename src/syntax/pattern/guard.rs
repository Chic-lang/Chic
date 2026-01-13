//! Guard and logical pattern parsing: handles `and`/`or` composition, unary
//! `not`, and relational comparisons. Invariants:
//! - Binary patterns associate left-to-right with the same precedence as the
//!   core pattern parser.
//! - Relational patterns capture the source slice for guard expressions so
//!   downstream diagnostics retain accurate spans.

use super::*;

impl PatternParser {
    pub(super) fn parse_or_pattern(&mut self) -> Result<PatternNode, PatternParseError> {
        let mut pattern = self.parse_and_pattern()?;
        while self.consume_keyword(Keyword::Or) {
            let rhs = self.parse_and_pattern()?;
            pattern = PatternNode::Binary {
                op: PatternBinaryOp::Or,
                left: Box::new(pattern),
                right: Box::new(rhs),
            };
        }
        Ok(pattern)
    }

    pub(super) fn parse_and_pattern(&mut self) -> Result<PatternNode, PatternParseError> {
        let mut pattern = self.parse_not_pattern()?;
        while self.consume_keyword(Keyword::And) {
            let rhs = self.parse_not_pattern()?;
            pattern = PatternNode::Binary {
                op: PatternBinaryOp::And,
                left: Box::new(pattern),
                right: Box::new(rhs),
            };
        }
        Ok(pattern)
    }

    pub(super) fn parse_not_pattern(&mut self) -> Result<PatternNode, PatternParseError> {
        if self.consume_keyword(Keyword::Not) {
            let inner = self.parse_not_pattern()?;
            return Ok(PatternNode::Not(Box::new(inner)));
        }
        self.parse_primary_pattern()
    }

    pub(super) fn parse_relational_pattern(&mut self) -> Result<PatternNode, PatternParseError> {
        let token = self
            .advance()
            .ok_or_else(|| self.error("expected relational operator"))?;
        let op = self.relational_operator(&token)?;
        let expr = self.parse_relational_expression()?;
        Ok(PatternNode::Relational { op, expr })
    }

    fn relational_operator(&self, token: &Token) -> Result<RelationalOp, PatternParseError> {
        match &token.kind {
            TokenKind::Operator("<") | TokenKind::Punctuation('<') => Ok(RelationalOp::Less),
            TokenKind::Operator("<=") => Ok(RelationalOp::LessEqual),
            TokenKind::Operator(">") | TokenKind::Punctuation('>') => Ok(RelationalOp::Greater),
            TokenKind::Operator(">=") => Ok(RelationalOp::GreaterEqual),
            _ => Err(self.error("expected relational operator")),
        }
    }

    fn parse_relational_expression(&mut self) -> Result<PatternExpression, PatternParseError> {
        let start = self.index;
        if start >= self.tokens.len() {
            return Err(self.error("expected expression after relational operator"));
        }

        let mut depth = DelimiterDepth::default();
        while let Some(token) = self.peek() {
            if depth.should_stop(token) {
                break;
            }
            depth.record(token);
            self.advance();
        }

        if self.index == start {
            return Err(self.error("expected expression after relational operator"));
        }

        Ok(self.expression_from_tokens(start, self.index))
    }
}
