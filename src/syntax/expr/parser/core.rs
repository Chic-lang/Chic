use crate::frontend::diagnostics::Span;
use crate::frontend::lexer::{Token, TokenKind};
use crate::syntax::pattern::{PatternAst, PatternParseError, parse_pattern_prefix};

#[derive(Clone, Debug)]
pub struct ExprError {
    pub message: String,
    pub span: Option<Span>,
}

impl ExprError {
    pub fn new(message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

pub(crate) struct ExprParser {
    pub(crate) tokens: Vec<Token>,
    pub(crate) source: String,
    pub(crate) index: usize,
}

impl ExprParser {
    pub(crate) fn new(tokens: Vec<Token>, source: &str) -> Self {
        Self {
            tokens,
            source: source.to_string(),
            index: 0,
        }
    }

    pub(crate) fn expect_end(&mut self) -> Result<(), ExprError> {
        if self.peek().is_some() {
            let Some(token) = self.advance() else {
                return Err(ExprError::new(
                    "unexpected end of input after expression",
                    self.peek().map(|tok| tok.span),
                ));
            };
            Err(ExprError::new(
                format!("unexpected token `{}` after expression", token.lexeme),
                Some(token.span),
            ))
        } else {
            Ok(())
        }
    }

    pub(crate) fn expect_punctuation(&mut self, ch: char) -> bool {
        if let Some(token) = self.peek()
            && matches!(token.kind, TokenKind::Punctuation(c) if c == ch)
        {
            self.advance();
            return true;
        }
        false
    }

    pub(crate) fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    pub(crate) fn peek_n(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.index.saturating_add(offset))
    }

    pub(crate) fn peek_punctuation(&self, ch: char) -> bool {
        self.peek()
            .is_some_and(|token| matches!(token.kind, TokenKind::Punctuation(c) if c == ch))
    }

    pub(crate) fn peek_punctuation_n(&self, offset: usize, ch: char) -> bool {
        self.peek_n(offset)
            .is_some_and(|token| matches!(token.kind, TokenKind::Punctuation(c) if c == ch))
    }

    pub(crate) fn advance(&mut self) -> Option<Token> {
        if self.index >= self.tokens.len() {
            return None;
        }
        let token = self.tokens[self.index].clone();
        self.index += 1;
        Some(token)
    }

    pub(crate) fn parse_pattern_operand(
        &mut self,
        is_span: Option<Span>,
    ) -> Result<PatternAst, ExprError> {
        if self.index >= self.tokens.len() {
            return Err(ExprError::new("expected pattern after `is`", is_span));
        }
        let start = self.index;
        let slice = &self.tokens[start..];
        let (mut pattern, consumed) =
            parse_pattern_prefix(slice, &self.source).map_err(Self::pattern_error)?;
        let end = start + consumed;
        if pattern.span.is_none() {
            pattern.span = self.span_for_range(start, end);
        }
        self.index = end;
        Ok(pattern)
    }

    pub(crate) fn pattern_error(err: PatternParseError) -> ExprError {
        ExprError::new(err.message, err.span)
    }

    pub(crate) fn span_for_range(&self, start: usize, end: usize) -> Option<Span> {
        if start >= end {
            return None;
        }
        let first = self.tokens.get(start)?;
        let last = self.tokens.get(end - 1)?;
        Some(Span::new(first.span.start, last.span.end))
    }
}
