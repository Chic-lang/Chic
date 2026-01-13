use super::{ExprError, ExprParser};
use crate::frontend::ast::Expression;
use crate::frontend::diagnostics::Span;
use crate::frontend::lexer::{Keyword, Token, TokenKind};
use crate::syntax::expr::builders::{
    LambdaBlock, LambdaBody, LambdaExpr, LambdaParam, LambdaParamModifier,
};

impl ExprParser {
    #[allow(clippy::too_many_lines, clippy::collapsible_if)]
    pub(super) fn try_parse_lambda(&mut self) -> Result<Option<LambdaExpr>, ExprError> {
        let checkpoint = self.index;
        let mut is_async = false;
        let mut span_start = None;

        if let Some(token) = self.peek().cloned()
            && matches!(token.kind, TokenKind::Identifier)
            && token.lexeme == "async"
            && self
                .peek_n(1)
                .is_some_and(|next| matches!(next.kind, TokenKind::Punctuation('(')))
        {
            is_async = true;
            span_start = Some(token.span.start);
            self.advance();
        }

        let Some(open_token) = self.peek().cloned() else {
            self.index = checkpoint;
            return Ok(None);
        };

        if !matches!(open_token.kind, TokenKind::Punctuation('(')) {
            self.index = checkpoint;
            return Ok(None);
        }

        if span_start.is_none() {
            span_start = Some(open_token.span.start);
        }

        self.advance();

        if self.locate_lambda_arrow(self.index).is_none() {
            self.index = checkpoint;
            return Ok(None);
        }

        let (params, close_span) = self.parse_lambda_params()?;

        let arrow_token = self.advance().ok_or_else(|| {
            ExprError::new("expected `=>` in lambda expression", Some(close_span))
        })?;
        if !matches!(arrow_token.kind, TokenKind::Operator(op) if op == "=>") {
            self.index = checkpoint;
            return Err(ExprError::new(
                "expected `=>` in lambda expression",
                Some(arrow_token.span),
            ));
        }

        let (body, body_span) = self.parse_lambda_body()?;

        let span_end = body_span
            .or_else(|| self.last_consumed_span())
            .map(|span| span.end);

        let lambda_span = span_start.and_then(|start| span_end.map(|end| Span::new(start, end)));

        Ok(Some(LambdaExpr {
            params,
            captures: Vec::new(),
            body,
            is_async,
            span: lambda_span,
        }))
    }

    pub(super) fn locate_lambda_arrow(&self, mut index: usize) -> Option<(usize, usize)> {
        let mut depth = 1usize;
        while let Some(token) = self.tokens.get(index) {
            match token.kind {
                TokenKind::Punctuation('(') => {
                    depth += 1;
                }
                TokenKind::Punctuation(')') => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        let mut arrow_index = index + 1;
                        while let Some(next) = self.tokens.get(arrow_index) {
                            match &next.kind {
                                TokenKind::Comment | TokenKind::DocComment => {
                                    arrow_index += 1;
                                }
                                TokenKind::Operator(op) if *op == "=>" => {
                                    return Some((index, arrow_index));
                                }
                                _ => return None,
                            }
                        }
                        return None;
                    }
                }
                _ => {}
            }
            index += 1;
        }
        None
    }

    #[allow(clippy::too_many_lines, clippy::collapsible_if)]
    pub(super) fn parse_lambda_params(&mut self) -> Result<(Vec<LambdaParam>, Span), ExprError> {
        let mut params = Vec::new();
        if let Some(close) = self.peek().cloned()
            && matches!(close.kind, TokenKind::Punctuation(')'))
        {
            self.advance();
            return Ok((params, close.span));
        }

        loop {
            let (modifier, modifier_span) = if let Some(token) = self.peek().cloned() {
                match token.kind {
                    TokenKind::Keyword(Keyword::In) => {
                        self.advance();
                        (Some(LambdaParamModifier::In), Some(token.span))
                    }
                    TokenKind::Keyword(Keyword::Ref) => {
                        self.advance();
                        (Some(LambdaParamModifier::Ref), Some(token.span))
                    }
                    TokenKind::Keyword(Keyword::Out) => {
                        self.advance();
                        (Some(LambdaParamModifier::Out), Some(token.span))
                    }
                    _ => (None, None),
                }
            } else {
                (None, None)
            };

            let type_start_index = self.index;
            let first_token = self.peek().cloned().ok_or_else(|| {
                ExprError::new(
                    "expected parameter in lambda expression",
                    modifier_span.or_else(|| self.peek().map(|token| token.span)),
                )
            })?;

            let type_scan = self.scan_type_name(type_start_index);
            let (ty, name_token, begin, end) = if let Some((ty_text, next_index)) = type_scan {
                if let Some(name_tok) = self.tokens.get(next_index).cloned()
                    && matches!(
                        name_tok.kind,
                        TokenKind::Identifier | TokenKind::Keyword(Keyword::Error)
                    )
                {
                    let name_tok = name_tok.clone();
                    let type_start_span = self.tokens.get(type_start_index).map_or_else(
                        || modifier_span.map_or(0, |span| span.end),
                        |token| token.span.start,
                    );
                    let name_span_end = name_tok.span.end;
                    self.index = next_index + 1;
                    let begin = modifier_span.map_or(type_start_span, |span| span.start);
                    (
                        Some(ty_text.trim().to_string()),
                        name_tok,
                        begin,
                        name_span_end,
                    )
                } else if matches!(
                    first_token.kind,
                    TokenKind::Identifier | TokenKind::Keyword(Keyword::Error)
                ) {
                    self.index = type_start_index + 1;
                    let begin = modifier_span.map_or(first_token.span.start, |span| span.start);
                    (None, first_token.clone(), begin, first_token.span.end)
                } else {
                    return Err(ExprError::new(
                        "expected identifier as parameter name in lambda expression",
                        Some(first_token.span),
                    ));
                }
            } else if matches!(
                first_token.kind,
                TokenKind::Identifier | TokenKind::Keyword(Keyword::Error)
            ) {
                self.index = type_start_index + 1;
                let begin = modifier_span.map_or(first_token.span.start, |span| span.start);
                (None, first_token.clone(), begin, first_token.span.end)
            } else {
                return Err(ExprError::new(
                    "expected parameter type in lambda expression",
                    modifier_span.or_else(|| self.peek().map(|token| token.span)),
                ));
            };

            let default = if let Some(token) = self.peek() {
                matches!(token.kind, TokenKind::Operator(op) if op == "=")
            } else {
                false
            };
            let default = if default {
                self.advance();
                let (text, expr_span) = self.collect_expression_segment(&[',', ')']);
                if text.trim().is_empty() {
                    return Err(ExprError::new(
                        "expected default expression after '='",
                        expr_span.or_else(|| self.peek().map(|tok| tok.span)),
                    ));
                }
                match super::parse_expression(&text) {
                    Ok(node) => Some(Expression::with_node(text, expr_span, node)),
                    Err(err) => {
                        return Err(ExprError::new(err.message, err.span.or(expr_span)));
                    }
                }
            } else {
                None
            };

            params.push(LambdaParam {
                modifier,
                ty,
                name: name_token.lexeme.clone(),
                span: Some(Span::new(begin, end)),
                default,
            });

            if let Some(close) = self.peek().cloned()
                && matches!(close.kind, TokenKind::Punctuation(')'))
            {
                self.advance();
                return Ok((params, close.span));
            }

            if !self.expect_punctuation(',') {
                return Err(ExprError::new(
                    "expected ',' or ')' after lambda parameter",
                    self.peek().map(|token| token.span),
                ));
            }
        }
    }

    pub(super) fn parse_lambda_body(&mut self) -> Result<(LambdaBody, Option<Span>), ExprError> {
        let Some(token) = self.peek().cloned() else {
            return Err(ExprError::new("expected lambda body", None));
        };

        if matches!(token.kind, TokenKind::Punctuation('{')) {
            let block = self.parse_lambda_block_body(token)?;
            let span = block.span;
            Ok((LambdaBody::Block(block), span))
        } else {
            let expr = self.parse_assignment()?;
            let span = self.last_consumed_span();
            Ok((LambdaBody::Expression(Box::new(expr)), span))
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    pub(super) fn parse_lambda_block_body(
        &mut self,
        open: Token,
    ) -> Result<LambdaBlock, ExprError> {
        self.advance();
        let mut depth = 1usize;
        let start = open.span.start;
        let mut end_span = open.span;

        while depth > 0 {
            let token = self.advance().ok_or_else(|| {
                ExprError::new("expected `}` to close lambda body", Some(open.span))
            })?;
            match token.kind {
                TokenKind::Punctuation('{') => depth += 1,
                TokenKind::Punctuation('}') => {
                    depth -= 1;
                    end_span = token.span;
                }
                _ => {}
            }
        }

        let end = end_span.end;
        let text = self.source.get(start..end).unwrap_or_default().to_string();

        Ok(LambdaBlock {
            text,
            span: Some(Span::new(start, end)),
        })
    }

    #[allow(clippy::too_many_lines)]
    fn collect_expression_segment(&mut self, terminators: &[char]) -> (String, Option<Span>) {
        let start_index = self.index;
        let mut paren = 0usize;
        let mut brace = 0usize;
        let mut bracket = 0usize;

        while let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Punctuation('(') => {
                    paren += 1;
                    self.advance();
                }
                TokenKind::Punctuation(')') => {
                    if paren == 0 && brace == 0 && bracket == 0 && terminators.contains(&')') {
                        break;
                    }
                    paren = paren.saturating_sub(1);
                    self.advance();
                }
                TokenKind::Punctuation('{') => {
                    brace += 1;
                    self.advance();
                }
                TokenKind::Punctuation('}') => {
                    if brace == 0 && paren == 0 && bracket == 0 && terminators.contains(&'}') {
                        break;
                    }
                    brace = brace.saturating_sub(1);
                    self.advance();
                }
                TokenKind::Punctuation('[') => {
                    bracket += 1;
                    self.advance();
                }
                TokenKind::Punctuation(']') => {
                    if bracket == 0 && paren == 0 && brace == 0 && terminators.contains(&']') {
                        break;
                    }
                    bracket = bracket.saturating_sub(1);
                    self.advance();
                }
                TokenKind::Punctuation(ch)
                    if terminators.contains(&ch) && paren == 0 && brace == 0 && bracket == 0 =>
                {
                    break;
                }
                _ => {
                    self.advance();
                }
            }
        }

        let end_index = self.index;
        let span = self.span_for_range(start_index, end_index);
        let text = span
            .map(|span| self.source[span.start..span.end].to_string())
            .unwrap_or_default();
        (text, span)
    }

    pub(super) fn last_consumed_span(&self) -> Option<Span> {
        self.tokens
            .get(self.index.saturating_sub(1))
            .map(|token| token.span)
    }
}
