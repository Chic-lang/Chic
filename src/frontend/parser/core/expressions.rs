use super::*;
use crate::syntax::expr::ExprNode;

parser_impl! {
    pub(in crate::frontend::parser) fn build_expression(&mut self, text: String, span: Option<Span>) -> Expression {
        if text.trim().is_empty() {
            return Expression::new(text, span);
        }
        if text
            .trim_start()
            .starts_with("new ")
            && text.contains("(*")
        {
            return Expression::new(text, span);
        }
        let trimmed = text.trim_start();
        if trimmed == "default" || trimmed.starts_with("default(") {
            let keyword_offset = text.find("default").unwrap_or(0);
            let keyword_span = span.map(|sp| {
                Span::in_file(
                    sp.file_id,
                    sp.start + keyword_offset,
                    sp.start + keyword_offset + "default".len(),
                )
            });
            let mut explicit_type = None;
            let mut type_span = None;
            if trimmed.starts_with("default(") && text.contains('(') && text.contains(')') {
                if let (Some(open), Some(close)) = (text.find('('), text.rfind(')')) {
                    if close > open {
                        let inner = &text[open + 1..close];
                        let trimmed_inner = inner.trim();
                        if !trimmed_inner.is_empty() {
                            explicit_type = Some(trimmed_inner.to_string());
                            if let Some(sp) = span {
                                let inner_offset =
                                    inner.find(trimmed_inner).unwrap_or_default() + open + 1;
                                let start = sp.start + inner_offset;
                                let end = start + trimmed_inner.len();
                                type_span = Some(Span::in_file(sp.file_id, start, end));
                            }
                        }
                    }
                }
            }
            let default_expr = crate::syntax::expr::builders::DefaultExpr {
                explicit_type,
                keyword_span,
                type_span,
            };
            return Expression::with_node(text, span, ExprNode::Default(default_expr));
        }
        if text.contains("++") || text.contains("--") || text.contains("(*") {
            return Expression::new(text, span);
        }
        match parse_expression(&text) {
            Ok(node) => Expression::with_node(text, span, node),
            Err(err) => {
                let diag_span = combine_expression_span(span, err.span);
                self.push_error(err.message.clone(), diag_span);
                Expression::new(text, span)
            }
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "Expression scanning handles nested delimiters; refactor once expression parser is shared."
    )]
    pub(in crate::frontend::parser) fn collect_expression_bounds(&mut self, terminators: &[char]) -> (String, Option<Span>) {
        let start_index = self.index;
        if self.is_at_end() {
            return (String::new(), None);
        }

        let mut depth_paren = 0usize;
        let mut depth_brace = 0usize;
        let mut depth_bracket = 0usize;
        let mut depth_angle = 0usize;

        while let Some(token) = self.peek() {
            match token.kind {
                TokenKind::Punctuation('(') => {
                    depth_paren += 1;
                    self.advance();
                }
                TokenKind::Punctuation(')') => {
                    if depth_paren == 0 {
                        break;
                    }
                    depth_paren -= 1;
                    self.advance();
                }
                TokenKind::Punctuation('{') => {
                    depth_brace += 1;
                    self.advance();
                }
                TokenKind::Punctuation('}') => {
                    if depth_brace == 0 {
                        break;
                    }
                    depth_brace -= 1;
                    self.advance();
                }
                TokenKind::Punctuation('[') => {
                    depth_bracket += 1;
                    self.advance();
                }
                TokenKind::Punctuation(']') => {
                    if depth_bracket == 0 {
                        break;
                    }
                    depth_bracket -= 1;
                    self.advance();
                }
                TokenKind::Punctuation('<') => {
                    depth_angle = depth_angle.saturating_add(1);
                    self.advance();
                }
                TokenKind::Punctuation('>') => {
                    if depth_angle > 0 {
                        depth_angle -= 1;
                    }
                    self.advance();
                }
                TokenKind::Operator(op) if op.chars().all(|ch| ch == '<') => {
                    if depth_angle > 0 {
                        depth_angle = depth_angle.saturating_add(op.len());
                    }
                    self.advance();
                }
                TokenKind::Operator(op) if op.chars().all(|ch| ch == '>') => {
                    let count = op.len();
                    if depth_angle >= count {
                        depth_angle -= count;
                    } else {
                        depth_angle = 0;
                    }
                    self.advance();
                }
                TokenKind::Punctuation(ch)
                    if terminators.contains(&ch)
                        && depth_paren == 0
                        && depth_brace == 0
                        && depth_bracket == 0
                        && (ch != ',' || depth_angle == 0) =>
                {
                    break;
                }
                _ => {
                    self.advance();
                }
            }
        }

        let end_index = self.index;
        let span = self.span_from_indices(start_index, end_index);
        let text = span.map(|sp| self.text_from_span(sp)).unwrap_or_default();
        (text, span)
    }

    pub(in crate::frontend::parser) fn collect_expression_until(&mut self, terminators: &[char]) -> Expression {
        let (text, span) = self.collect_expression_bounds(terminators);
        if span.is_none() && text.is_empty() {
            return Expression::new(String::new(), None);
        }
        self.build_expression(text, span)
    }

    pub(in crate::frontend::parser) fn collect_pattern_expression_until(&mut self, terminators: &[char]) -> Expression {
        let (text, span) = self.collect_expression_bounds(terminators);
        Expression::new(text, span)
    }

    pub(in crate::frontend::parser) fn collect_expression_list_until(&mut self, terminator: char) -> Vec<Expression> {
        let mut expressions = Vec::new();
        while !self.check_punctuation(terminator) && !self.is_at_end() {
            let expr = self.collect_expression_until(&[',', terminator]);
            if expr.span.is_none() && expr.text.is_empty() {
                break;
            }
            expressions.push(expr);
            if self.check_punctuation(',') {
                self.advance();
                continue;
            }
            break;
        }
        expressions
    }

    pub(in crate::frontend::parser) fn parse_parenthesized_expression(&mut self, context: &str) -> Option<Expression> {
        if !self.expect_punctuation('(') {
            return None;
        }
        let start_index = self.index;
        let mut depth = 1usize;

        while let Some(token) = self.advance() {
            match token.kind {
                TokenKind::Punctuation('(') => depth += 1,
                TokenKind::Punctuation(')') => {
                    depth -= 1;
                    if depth == 0 {
                        let end_index = self.index - 1;
                        let span = self.span_from_indices(start_index, end_index);
                        let text = span.map(|sp| self.text_from_span(sp)).unwrap_or_default();
                        return Some(self.build_expression(text, span));
                    }
                }
                _ => {}
            }
        }

        self.push_error(
            format!("unterminated {context} expression"),
            self.peek().map(|token| token.span),
        );
        None
    }
}
