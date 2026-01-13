use super::{ExprError, ExprNode, ExprParser};
use crate::frontend::diagnostics::Span;
use crate::frontend::lexer::{Keyword, TokenKind};
use crate::mir::{BinOp, UnOp};
use crate::syntax::expr::builders::{
    CastSyntax, IndexFromEndExpr, PatternGuardExpr, RangeEndpoint, RangeExpr, SwitchArm, SwitchExpr,
};
use crate::syntax::expr::precedence::{
    assignment_operator, binary_precedence, can_start_unary_expression, unary_operator,
};

impl ExprParser {
    pub(super) fn parse_expression(&mut self) -> Result<ExprNode, ExprError> {
        self.parse_assignment()
    }

    pub(super) fn parse_assignment(&mut self) -> Result<ExprNode, ExprError> {
        let left = self.parse_conditional()?;
        if let Some(token) = self.peek().cloned()
            && let Some(op) = assignment_operator(&token)
        {
            self.advance();
            let right = self.parse_assignment()?;
            return Ok(ExprNode::Assign {
                target: Box::new(left),
                op,
                value: Box::new(right),
            });
        }
        Ok(left)
    }

    pub(super) fn parse_conditional(&mut self) -> Result<ExprNode, ExprError> {
        let mut condition = self.parse_range_expression()?;
        while let Some(token) = self.peek().cloned() {
            if !matches!(token.kind, TokenKind::Keyword(Keyword::Switch)) {
                break;
            }
            condition = self.parse_switch_expression(condition, Some(token.span))?;
        }
        let is_question = self.peek().is_some_and(|token| {
            matches!(token.kind, TokenKind::Punctuation('?'))
                || matches!(token.kind, TokenKind::Operator(op) if op == "?")
        });
        if !is_question {
            return Ok(condition);
        }

        let question = self.advance().ok_or_else(|| {
            ExprError::new(
                "expected `?` in conditional expression",
                self.peek().map(|token| token.span),
            )
        })?;
        let then_branch = self.parse_assignment()?;

        if !self.expect_punctuation(':') {
            let span = self.peek().map(|token| token.span).or(Some(question.span));
            return Err(ExprError::new(
                "expected ':' after `?` branch in conditional expression",
                span,
            ));
        }

        let else_branch = self.parse_assignment()?;
        Ok(ExprNode::Conditional {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        })
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn parse_range_expression(&mut self) -> Result<ExprNode, ExprError> {
        let start_index = self.index;
        if let Some(op_token) = self.peek().cloned()
            && matches!(op_token.kind, TokenKind::Operator(op) if op == ".." || op == "..=")
        {
            self.advance();
            if self.peek().is_some_and(
                |next| matches!(next.kind, TokenKind::Operator(op) if op == ".." || op == "..="),
            ) {
                return Err(ExprError::new(
                    "range expression cannot contain consecutive `..` operators",
                    Some(op_token.span),
                ));
            }
            let end_start = self.index;
            let end = if self.peek().is_some() && !self.peek_punctuation(')') {
                let end_expr = self.parse_binary(0)?;
                if matches!(end_expr, ExprNode::Range(_)) {
                    return Err(ExprError::new(
                        "range expression cannot contain multiple `..` operators",
                        Some(op_token.span),
                    ));
                }
                let span = self.span_for_range(end_start, self.index);
                Some(Box::new(RangeEndpoint::new(
                    end_expr.clone(),
                    matches!(end_expr, ExprNode::IndexFromEnd(_)),
                    span,
                )))
            } else {
                None
            };
            let span = self
                .span_for_range(start_index, self.index)
                .or(Some(op_token.span));
            return Ok(ExprNode::Range(RangeExpr {
                start: None,
                end,
                inclusive: matches!(op_token.kind, TokenKind::Operator(op) if op == "..="),
                span,
            }));
        }

        let left_start = self.index;
        let left = self.parse_binary(0)?;
        if let Some(op_token) = self.peek().cloned()
            && matches!(op_token.kind, TokenKind::Operator(op) if op == ".." || op == "..=")
        {
            self.advance();
            if self.peek().is_some_and(
                |next| matches!(next.kind, TokenKind::Operator(op) if op == ".." || op == "..="),
            ) {
                return Err(ExprError::new(
                    "range expression cannot contain multiple `..` operators",
                    Some(op_token.span),
                ));
            }
            let end_start = self.index;
            let end = if self.peek().is_some() && !self.peek_punctuation(')') {
                let end_expr = self.parse_binary(0)?;
                let span = self.span_for_range(end_start, self.index);
                Some(Box::new(RangeEndpoint::new(
                    end_expr.clone(),
                    matches!(end_expr, ExprNode::IndexFromEnd(_)),
                    span,
                )))
            } else {
                None
            };
            let start_span = self.span_for_range(left_start, self.index.saturating_sub(1));
            let range_span = self
                .span_for_range(left_start, self.index)
                .or(Some(op_token.span));
            if self.peek().is_some_and(
                |next| matches!(next.kind, TokenKind::Operator(op) if op == ".." || op == "..="),
            ) {
                return Err(ExprError::new(
                    "range expression cannot contain multiple `..` operators",
                    self.peek().map(|tok| tok.span),
                ));
            }
            let start = Some(Box::new(RangeEndpoint::new(
                left.clone(),
                matches!(left, ExprNode::IndexFromEnd(_)),
                start_span,
            )));
            let inclusive = matches!(op_token.kind, TokenKind::Operator(op) if op == "..=");
            return Ok(ExprNode::Range(RangeExpr {
                start,
                end,
                inclusive,
                span: range_span,
            }));
        }

        Ok(left)
    }

    #[allow(clippy::too_many_lines)]
    fn parse_switch_expression(
        &mut self,
        value: ExprNode,
        switch_span: Option<Span>,
    ) -> Result<ExprNode, ExprError> {
        let switch_token = self
            .advance()
            .ok_or_else(|| ExprError::new("expected `switch` keyword", switch_span))?;
        let Some(open_brace) = self.peek().cloned() else {
            return Err(ExprError::new(
                "expected `{` after `switch` expression",
                switch_span.or(Some(switch_token.span)),
            ));
        };
        if !self.expect_punctuation('{') {
            return Err(ExprError::new(
                "expected `{` after `switch` expression",
                Some(open_brace.span).or(switch_span),
            ));
        }
        if self.expect_punctuation('}') {
            return Err(ExprError::new(
                "switch expression requires at least one arm",
                switch_span.or(Some(switch_token.span)),
            ));
        }

        let mut arms = Vec::new();
        let braces_span;
        loop {
            let arm_start = self.index.saturating_sub(1);
            let pattern = self.parse_pattern_operand(switch_span)?;
            let guards = self.parse_pattern_guards(true, switch_span)?;
            let arrow_span = match self.peek() {
                Some(token) if matches!(token.kind, TokenKind::Operator(ref op) if *op == "=>") => {
                    let span = token.span;
                    self.advance();
                    Some(span)
                }
                Some(token) => {
                    return Err(ExprError::new(
                        "expected `=>` after switch arm pattern",
                        Some(token.span).or(switch_span),
                    ));
                }
                None => {
                    return Err(ExprError::new(
                        "expected `=>` after switch arm pattern",
                        switch_span,
                    ));
                }
            };

            let expression = self.parse_switch_arm_value(switch_span)?;
            let arm_end_index = self.index;
            let arm_span = self.span_for_range(arm_start, arm_end_index);
            arms.push(SwitchArm {
                pattern,
                guards,
                expression,
                span: arm_span,
                arrow_span,
            });

            let mut consumed_comma = false;
            if self.expect_punctuation(',') {
                consumed_comma = true;
            }

            if self.expect_punctuation('}') {
                braces_span = self
                    .tokens
                    .get(self.index.saturating_sub(1))
                    .map(|tok| Span::new(open_brace.span.start, tok.span.end));
                break;
            }

            if !consumed_comma {
                return Err(ExprError::new(
                    "expected `,` or `}` after switch arm",
                    self.peek().map(|tok| tok.span).or(switch_span),
                ));
            }
        }

        Ok(ExprNode::Switch(SwitchExpr {
            value: Box::new(value),
            arms,
            span: switch_span,
            switch_span: Some(switch_token.span),
            braces_span,
        }))
    }

    fn parse_switch_arm_value(&mut self, switch_span: Option<Span>) -> Result<ExprNode, ExprError> {
        let value_start = self.index;
        let value_end = self.find_switch_arm_end_index(value_start)?;
        if value_start == value_end {
            return Err(ExprError::new(
                "switch arm requires an expression",
                switch_span,
            ));
        }
        let expr_span = self.span_for_range(value_start, value_end);
        let expr_text = expr_span
            .and_then(|sp| self.source.get(sp.start..sp.end))
            .unwrap_or_default()
            .to_string();
        if expr_text.trim().is_empty() {
            return Err(ExprError::new(
                "switch arm requires an expression",
                expr_span.or(switch_span),
            ));
        }
        let expr = match crate::syntax::expr::parser::parse_expression(&expr_text) {
            Ok(node) => node,
            Err(mut err) => {
                if let (Some(base), Some(err_span)) = (expr_span, err.span) {
                    let start = base.start + err_span.start;
                    let end = base.start + err_span.end;
                    err.span = Some(Span::new(start, end));
                } else if err.span.is_none() {
                    err.span = expr_span.or(switch_span);
                }
                return Err(err);
            }
        };
        self.index = value_end;
        Ok(expr)
    }

    fn find_switch_arm_end_index(&self, start: usize) -> Result<usize, ExprError> {
        let mut index = start;
        let mut paren = 0i32;
        let mut brace = 0i32;
        let mut bracket = 0i32;
        while let Some(token) = self.tokens.get(index) {
            match token.kind {
                TokenKind::Punctuation('(') => paren += 1,
                TokenKind::Punctuation(')') => {
                    if paren == 0 {
                        break;
                    }
                    paren -= 1;
                }
                TokenKind::Punctuation('{') => {
                    brace += 1;
                }
                TokenKind::Punctuation('}') => {
                    if brace == 0 && paren == 0 && bracket == 0 {
                        break;
                    }
                    brace = (brace - 1).max(0);
                }
                TokenKind::Punctuation('[') => bracket += 1,
                TokenKind::Punctuation(']') => {
                    if bracket == 0 {
                        break;
                    }
                    bracket -= 1;
                }
                _ => {}
            }

            if paren == 0 && brace == 0 && bracket == 0 {
                match &token.kind {
                    TokenKind::Punctuation(',' | '}') => break,
                    _ => {}
                }
            }
            index += 1;
        }

        Ok(index)
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn parse_binary(&mut self, min_precedence: u8) -> Result<ExprNode, ExprError> {
        let mut left = self.parse_unary()?;
        loop {
            let Some(op_token) = self.peek().cloned() else {
                break;
            };

            if matches!(op_token.kind, TokenKind::Keyword(Keyword::Is)) {
                let precedence = 6;
                if precedence < min_precedence {
                    break;
                }
                self.advance();
                if let Some(next) = self.peek()
                    && matches!(
                        next.kind,
                        TokenKind::Keyword(Keyword::Ref | Keyword::In | Keyword::Out)
                    )
                {
                    let span = Some(next.span);
                    let message = format!(
                        "`{}` qualifier is only supported on parameters and receivers",
                        next.lexeme
                    );
                    return Err(ExprError::new(message, span));
                }
                let pattern = self.parse_pattern_operand(Some(op_token.span))?;
                let guards = self.parse_pattern_guards(true, Some(op_token.span))?;
                left = ExprNode::IsPattern {
                    value: Box::new(left),
                    pattern,
                    guards,
                };
                continue;
            }

            if matches!(op_token.kind, TokenKind::Keyword(Keyword::As)) {
                let precedence = 6;
                if precedence < min_precedence {
                    break;
                }
                self.advance();
                let start_index = self.index;
                let Some((ty_name, next_index)) = self.scan_type_name(start_index) else {
                    return Err(ExprError::new(
                        "expected type name after `as`",
                        Some(op_token.span),
                    ));
                };
                if ty_name.trim().is_empty() {
                    return Err(ExprError::new(
                        "expected non-empty type after `as`",
                        Some(op_token.span),
                    ));
                }
                self.index = next_index;
                left = ExprNode::Cast {
                    target: ty_name.trim().to_string(),
                    expr: Box::new(left),
                    syntax: CastSyntax::As,
                };
                continue;
            }

            let Some((precedence, bin_op)) = binary_precedence(&op_token) else {
                break;
            };

            if precedence < min_precedence {
                break;
            }

            self.advance();
            let rhs_precedence = if matches!(bin_op, BinOp::NullCoalesce) {
                precedence
            } else {
                precedence + 1
            };
            let right = self.parse_binary(rhs_precedence)?;
            left = ExprNode::Binary {
                op: bin_op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    #[allow(clippy::too_many_lines, clippy::collapsible_if)]
    pub(super) fn parse_unary(&mut self) -> Result<ExprNode, ExprError> {
        if let Some(lambda) = self.try_parse_lambda()? {
            return Ok(ExprNode::Lambda(lambda));
        }
        if let Some(token) = self.peek().cloned() {
            if matches!(token.kind, TokenKind::Operator(op) if op == "^") {
                self.advance();
                let expr = self.parse_unary()?;
                return Ok(ExprNode::IndexFromEnd(IndexFromEndExpr {
                    expr: Box::new(expr),
                    span: Some(token.span),
                }));
            }
            if matches!(token.kind, TokenKind::Operator(op) if op == "&") {
                self.advance();
                let op = if self
                    .peek()
                    .is_some_and(|next| matches!(next.kind, TokenKind::Keyword(Keyword::Mut)))
                {
                    self.advance();
                    UnOp::AddrOfMut
                } else {
                    UnOp::AddrOf
                };
                let expr = self.parse_unary()?;
                return Ok(ExprNode::Unary {
                    op,
                    expr: Box::new(expr),
                    postfix: false,
                });
            }
            if matches!(token.kind, TokenKind::Operator(op) if op == "*") {
                self.advance();
                let expr = self.parse_unary()?;
                return Ok(ExprNode::Unary {
                    op: UnOp::Deref,
                    expr: Box::new(expr),
                    postfix: false,
                });
            }
            if matches!(token.kind, TokenKind::Keyword(Keyword::Ref)) {
                self.advance();
                let readonly = self
                    .peek()
                    .is_some_and(|next| matches!(next.kind, TokenKind::Keyword(Keyword::Readonly)));
                if readonly {
                    self.advance();
                }
                let expr = self.parse_unary()?;
                return Ok(ExprNode::Ref {
                    expr: Box::new(expr),
                    readonly,
                });
            }
            if let Some(op) = unary_operator(&token) {
                self.advance();
                let expr = self.parse_unary()?;
                return Ok(ExprNode::Unary {
                    op,
                    expr: Box::new(expr),
                    postfix: false,
                });
            }
            if matches!(token.kind, TokenKind::Keyword(Keyword::Await)) {
                self.advance();
                let expr = self.parse_unary()?;
                return Ok(ExprNode::Await {
                    expr: Box::new(expr),
                });
            }
            if matches!(token.kind, TokenKind::Keyword(Keyword::Throw)) {
                self.advance();
                let expr = if let Some(next) = self.peek() {
                    if can_start_unary_expression(next) {
                        Some(Box::new(self.parse_unary()?))
                    } else {
                        None
                    }
                } else {
                    None
                };
                return Ok(ExprNode::Throw { expr });
            }
            if matches!(token.kind, TokenKind::Keyword(Keyword::Sizeof)) {
                let span = token.span;
                self.advance();
                return self.parse_sizeof_expr(span);
            }
            if matches!(token.kind, TokenKind::Keyword(Keyword::Alignof)) {
                let span = token.span;
                self.advance();
                return self.parse_alignof_expr(span);
            }
            if matches!(token.kind, TokenKind::Keyword(Keyword::Nameof)) {
                let span = token.span;
                self.advance();
                return self.parse_nameof_expr(span);
            }
            if matches!(token.kind, TokenKind::Identifier) && token.lexeme == "quote" {
                if self
                    .peek_n(1)
                    .is_some_and(|next| matches!(next.kind, TokenKind::Punctuation('(')))
                {
                    let span = token.span;
                    self.advance();
                    return self.parse_quote_expr(span);
                }
            }
            if self.peek_punctuation('(') {
                if let Some(cast) = self.try_parse_cast()? {
                    return Ok(cast);
                }
            }
        }
        self.parse_postfix()
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn skip_generic_arguments(&mut self) -> Result<(), ExprError> {
        let Some(token) = self.peek().cloned() else {
            return Ok(());
        };

        let is_generic_start = match &token.kind {
            TokenKind::Punctuation('<') => true,
            TokenKind::Operator(op) => op.chars().all(|c| c == '<'),
            _ => false,
        };
        if !is_generic_start {
            return Ok(());
        }

        let mut depth = 0usize;
        let error_span = token.span;

        loop {
            let Some(next) = self.advance() else {
                return Err(ExprError::new(
                    "unterminated generic argument list in `nameof` operand",
                    Some(error_span),
                ));
            };

            match &next.kind {
                TokenKind::Punctuation('<') => depth += 1,
                TokenKind::Punctuation('>') => {
                    if depth == 0 {
                        return Err(ExprError::new(
                            "unexpected `>` in `nameof` operand",
                            Some(next.span),
                        ));
                    }
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                TokenKind::Operator(op) if op.chars().all(|c| c == '<') => {
                    depth += op.len();
                }
                TokenKind::Operator(op) if op.chars().all(|c| c == '>') => {
                    let count = op.len();
                    if depth < count {
                        return Err(ExprError::new(
                            "unexpected `>` in `nameof` operand",
                            Some(next.span),
                        ));
                    }
                    depth -= count;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub(super) fn try_parse_cast(&mut self) -> Result<Option<ExprNode>, ExprError> {
        let checkpoint = self.index;
        let debug = std::env::var_os("CHIC_DEBUG_MIR_DIAGNOSTICS").is_some();
        let Some(paren) = self.peek() else {
            return Ok(None);
        };
        if !matches!(paren.kind, TokenKind::Punctuation('(')) {
            return Ok(None);
        }
        self.advance();

        let start_index = self.index;
        let Some((ty_name, next_index)) = self.scan_type_name(start_index) else {
            if debug {
                let tail: Vec<_> = self
                    .tokens
                    .iter()
                    .skip(start_index)
                    .take(6)
                    .map(|t| format!("{:?}:{}", t.kind, t.lexeme))
                    .collect();
                eprintln!(
                    "[chic-debug] try_parse_cast: failed to parse type starting at token {} ({tail:?})",
                    start_index
                );
            }
            self.index = checkpoint;
            return Ok(None);
        };

        let Some(close_token) = self.tokens.get(next_index) else {
            if debug {
                eprintln!(
                    "[chic-debug] try_parse_cast: missing closing `)` after cast type `{ty_name}`"
                );
            }
            self.index = checkpoint;
            return Ok(None);
        };
        if !matches!(close_token.kind, TokenKind::Punctuation(')')) {
            if debug {
                eprintln!("[chic-debug] try_parse_cast: token after type `{ty_name}` was not `)`");
            }
            self.index = checkpoint;
            return Ok(None);
        }

        let after_paren = next_index + 1;
        let next_token = self.tokens.get(after_paren);
        if !next_token.is_some_and(can_start_unary_expression) {
            if debug {
                eprintln!(
                    "[chic-debug] try_parse_cast: expression after cast type `{ty_name}` is missing"
                );
            }
            self.index = checkpoint;
            return Ok(None);
        }

        self.index = after_paren;
        let expr = self.parse_unary()?;
        Ok(Some(ExprNode::Cast {
            target: ty_name,
            expr: Box::new(expr),
            syntax: CastSyntax::Paren,
        }))
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn scan_type_name(&self, mut index: usize) -> Option<(String, usize)> {
        let (pointer_prefix, next_index) = self.collect_pointer_prefix(index);
        index = next_index;

        let mut text = String::new();
        let mut angle_depth = 0usize;
        let mut square_depth = 0usize;

        let token = self.tokens.get(index)?;
        if matches!(token.kind, TokenKind::Keyword(Keyword::Fn)) {
            let (fn_text, new_index) = self.scan_function_type(index)?;
            if !pointer_prefix.is_empty() {
                text.push_str(pointer_prefix.trim_end());
                text.push(' ');
            }
            text.push_str(&fn_text);
            return Some((text.trim().to_string(), new_index));
        }

        match &token.kind {
            TokenKind::Identifier => {
                text.push_str(&token.lexeme);
                index += 1;
            }
            TokenKind::Punctuation('(') => {
                let (tuple_text, new_index) = self.scan_tuple_type(index)?;
                text.push_str(&tuple_text);
                index = new_index;
            }
            _ => return None,
        }

        while let Some(token) = self.tokens.get(index) {
            match &token.kind {
                TokenKind::Punctuation('.') => {
                    let next = self.tokens.get(index + 1)?;
                    if !matches!(next.kind, TokenKind::Identifier) {
                        break;
                    }
                    text.push('.');
                    text.push_str(&next.lexeme);
                    index += 2;
                }
                TokenKind::Punctuation(':') => {
                    let next = self.tokens.get(index + 1)?;
                    if !matches!(next.kind, TokenKind::Punctuation(':')) {
                        break;
                    }
                    let next_ident = self.tokens.get(index + 2)?;
                    if !matches!(next_ident.kind, TokenKind::Identifier) {
                        break;
                    }
                    text.push_str("::");
                    text.push_str(&next_ident.lexeme);
                    index += 3;
                }
                TokenKind::Operator(op) if op == &"::" => {
                    let next = self.tokens.get(index + 1)?;
                    if !matches!(next.kind, TokenKind::Identifier) {
                        break;
                    }
                    text.push_str("::");
                    text.push_str(&next.lexeme);
                    index += 2;
                }
                TokenKind::Punctuation('<') | TokenKind::Operator("<") => {
                    angle_depth += 1;
                    text.push_str(&token.lexeme);
                    index += 1;
                }
                TokenKind::Punctuation('>') | TokenKind::Operator(">") => {
                    if angle_depth == 0 {
                        break;
                    }
                    angle_depth -= 1;
                    text.push_str(&token.lexeme);
                    index += 1;
                }
                TokenKind::Operator(">>") => {
                    if angle_depth < 2 {
                        break;
                    }
                    angle_depth -= 2;
                    text.push_str(">>");
                    index += 1;
                }
                TokenKind::Punctuation(',') => {
                    if angle_depth == 0 {
                        break;
                    }
                    text.push(',');
                    index += 1;
                }
                TokenKind::Punctuation('[') => {
                    square_depth += 1;
                    text.push('[');
                    index += 1;
                }
                TokenKind::Punctuation(']') => {
                    if square_depth == 0 {
                        break;
                    }
                    square_depth -= 1;
                    text.push(']');
                    index += 1;
                }
                TokenKind::Punctuation('?') => {
                    text.push('?');
                    index += 1;
                }
                TokenKind::Keyword(Keyword::Mut | Keyword::Const) => {
                    if text.chars().last().is_some_and(|ch| !ch.is_whitespace()) {
                        text.push(' ');
                    }
                    text.push_str(&token.lexeme);
                    index += 1;
                }
                TokenKind::Operator(op) if op == &"*" || op == &"&" => {
                    text.push_str(op);
                    index += 1;
                }
                TokenKind::Identifier => {
                    if angle_depth > 0 || square_depth > 0 {
                        text.push_str(&token.lexeme);
                        index += 1;
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }

        let mut result = String::new();
        if !pointer_prefix.is_empty() {
            result.push_str(pointer_prefix.trim_end());
            result.push(' ');
        }
        result.push_str(text.trim());
        Some((result.trim().to_string(), index))
    }

    pub(super) fn scan_function_type(&self, mut index: usize) -> Option<(String, usize)> {
        let mut text = String::new();
        let token = self.tokens.get(index)?;
        if !matches!(token.kind, TokenKind::Keyword(Keyword::Fn)) {
            return None;
        }
        text.push_str(&token.lexeme);
        index += 1;

        loop {
            let Some(token) = self.tokens.get(index) else {
                return None;
            };
            match &token.kind {
                TokenKind::Punctuation('@') => {
                    text.push(' ');
                    text.push('@');
                    index += 1;
                    if let Some(attr) = self.tokens.get(index) {
                        text.push_str(&attr.lexeme);
                        index += 1;
                        if let Some(next) = self.tokens.get(index) {
                            if matches!(next.kind, TokenKind::Punctuation('(')) {
                                let (args, next_index) = self.consume_parenthesised(index)?;
                                text.push_str(&args);
                                index = next_index;
                            }
                        }
                    }
                }
                TokenKind::Punctuation('(') => break,
                _ => break,
            }
        }

        let (params, mut index) = self.consume_parenthesised(index)?;
        text.push_str(&params);

        if let Some(token) = self.tokens.get(index) {
            if matches!(token.kind, TokenKind::Operator(op) if op == "->") {
                text.push(' ');
                text.push_str(&token.lexeme);
                index += 1;
                text.push(' ');
                let (ret, next_index) = self.scan_type_name(index)?;
                text.push_str(&ret);
                index = next_index;
            }
        }

        Some((text.trim().to_string(), index))
    }

    fn consume_parenthesised(&self, mut index: usize) -> Option<(String, usize)> {
        let mut depth = 0usize;
        let mut text = String::new();
        if !matches!(self.tokens.get(index)?.kind, TokenKind::Punctuation('(')) {
            return None;
        }
        loop {
            let token = self.tokens.get(index)?;
            match token.kind {
                TokenKind::Punctuation('(') => depth += 1,
                TokenKind::Punctuation(')') => {
                    depth = depth.checked_sub(1)?;
                }
                _ => {}
            }
            text.push_str(&token.lexeme);
            index += 1;
            if depth == 0 {
                break;
            }
        }
        Some((text, index))
    }

    pub(super) fn collect_pointer_prefix(&self, mut index: usize) -> (String, usize) {
        let mut prefix = String::new();
        while let Some(token) = self.tokens.get(index) {
            let is_star = match &token.kind {
                TokenKind::Operator(op) if op == &"*" => true,
                TokenKind::Punctuation('*') => true,
                _ => false,
            };
            if !is_star {
                break;
            }
            if !prefix.is_empty() {
                prefix.push(' ');
            }
            prefix.push('*');
            index += 1;
            if let Some((qualifier, next_index)) = self.pointer_qualifier(index) {
                prefix.push_str(qualifier);
                index = next_index;
            }
            let (modifiers, next_index) = self.pointer_modifiers(index);
            if !modifiers.is_empty() {
                prefix.push(' ');
                prefix.push_str(&modifiers);
            }
            index = next_index;
        }
        (prefix, index)
    }

    fn pointer_qualifier(&self, index: usize) -> Option<(&'static str, usize)> {
        self.tokens.get(index).and_then(|token| match token.kind {
            TokenKind::Keyword(Keyword::Mut) => Some(("mut", index + 1)),
            TokenKind::Keyword(Keyword::Const) => Some(("const", index + 1)),
            _ => None,
        })
    }

    fn pointer_modifiers(&self, mut index: usize) -> (String, usize) {
        let mut modifiers = String::new();
        loop {
            let Some(at) = self.tokens.get(index) else {
                break;
            };
            if !matches!(at.kind, TokenKind::Punctuation('@')) {
                break;
            }
            let Some(ident) = self.tokens.get(index + 1) else {
                break;
            };
            if !matches!(ident.kind, TokenKind::Identifier | TokenKind::Keyword(_)) {
                break;
            }

            if !modifiers.is_empty() {
                modifiers.push(' ');
            }
            modifiers.push('@');
            modifiers.push_str(&ident.lexeme);
            index += 2;

            if ident.lexeme.eq_ignore_ascii_case("aligned") {
                if let (Some(open), Some(value), Some(close)) = (
                    self.tokens.get(index),
                    self.tokens.get(index + 1),
                    self.tokens.get(index + 2),
                ) {
                    if matches!(open.kind, TokenKind::Punctuation('('))
                        && matches!(value.kind, TokenKind::NumberLiteral(_))
                        && matches!(close.kind, TokenKind::Punctuation(')'))
                    {
                        modifiers.push('(');
                        modifiers.push_str(&value.lexeme);
                        modifiers.push(')');
                        index += 3;
                        continue;
                    }
                }
            }
        }

        (modifiers, index)
    }

    pub(super) fn scan_tuple_type(&self, mut index: usize) -> Option<(String, usize)> {
        let mut depth = 0usize;
        let mut text = String::new();

        while let Some(token) = self.tokens.get(index) {
            match token.kind {
                TokenKind::Punctuation('(') => {
                    depth = depth.saturating_add(1);
                }
                TokenKind::Punctuation(')') => {
                    if depth == 0 {
                        return None;
                    }
                    depth -= 1;
                }
                _ => {}
            }
            text.push_str(&token.lexeme);
            index += 1;
            if depth == 0 {
                return Some((text, index));
            }
        }
        None
    }

    fn parse_pattern_guards(
        &mut self,
        allow_guards: bool,
        context_span: Option<Span>,
    ) -> Result<Vec<PatternGuardExpr>, ExprError> {
        let mut guards = Vec::new();
        loop {
            let Some(token) = self.peek().cloned() else {
                break;
            };
            if !matches!(token.kind, TokenKind::Keyword(Keyword::When)) {
                break;
            }
            if !allow_guards {
                let message = "`when` guards are only supported in switch expressions";
                return Err(ExprError::new(message, Some(token.span).or(context_span)));
            }
            self.advance();
            let depth = guards.len();
            let guard = self.parse_pattern_guard_clause(token.span, depth)?;
            guards.push(guard);
        }
        Ok(guards)
    }

    fn parse_pattern_guard_clause(
        &mut self,
        when_span: Span,
        depth: usize,
    ) -> Result<PatternGuardExpr, ExprError> {
        let guard_start = self.index;
        let guard_end = self.find_guard_end_index(guard_start);
        if guard_start == guard_end {
            return Err(ExprError::new(
                "`when` guard requires an expression",
                Some(when_span),
            ));
        }

        let guard_span = self.span_for_range(guard_start, guard_end);
        let guard_text = guard_span
            .and_then(|sp| self.source.get(sp.start..sp.end))
            .unwrap_or_default()
            .to_string();

        if guard_text.trim().is_empty() {
            return Err(ExprError::new(
                "`when` guard requires an expression",
                guard_span.or(Some(when_span)),
            ));
        }

        let expr = Self::parse_guard_expression_text(&guard_text, guard_span)?;
        self.index = guard_end;
        Ok(PatternGuardExpr {
            expr,
            span: guard_span,
            depth,
            keyword_span: Some(when_span),
        })
    }

    fn parse_guard_expression_text(text: &str, span: Option<Span>) -> Result<ExprNode, ExprError> {
        match crate::syntax::expr::parser::parse_expression(text) {
            Ok(node) => Ok(node),
            Err(mut err) => {
                if let (Some(base), Some(err_span)) = (span, err.span) {
                    let start = base.start + err_span.start;
                    let end = base.start + err_span.end;
                    err.span = Some(Span::new(start, end));
                } else if err.span.is_none() {
                    err.span = span;
                }
                Err(err)
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn find_guard_end_index(&self, start: usize) -> usize {
        let mut index = start;
        let mut paren = 0i32;
        let mut brace = 0i32;
        let mut bracket = 0i32;
        let mut conditional = 0i32;

        while let Some(token) = self.tokens.get(index) {
            match token.kind {
                TokenKind::Punctuation('(') => {
                    paren += 1;
                    index += 1;
                    continue;
                }
                TokenKind::Punctuation(')') => {
                    if paren == 0 {
                        break;
                    }
                    paren -= 1;
                    index += 1;
                    continue;
                }
                TokenKind::Punctuation('{') => {
                    brace += 1;
                    index += 1;
                    continue;
                }
                TokenKind::Punctuation('}') => {
                    if brace == 0 {
                        break;
                    }
                    brace -= 1;
                    index += 1;
                    continue;
                }
                TokenKind::Punctuation('[') => {
                    bracket += 1;
                    index += 1;
                    continue;
                }
                TokenKind::Punctuation(']') => {
                    if bracket == 0 {
                        break;
                    }
                    bracket -= 1;
                    index += 1;
                    continue;
                }
                _ => {}
            }

            if paren == 0 && brace == 0 && bracket == 0 {
                if let TokenKind::Operator(op) = &token.kind {
                    if *op == "=>" {
                        break;
                    }
                }
                match &token.kind {
                    TokenKind::Keyword(Keyword::When) => break,
                    TokenKind::Punctuation(',' | ';') => {
                        if conditional == 0 {
                            break;
                        }
                    }
                    TokenKind::Punctuation(':') => {
                        if conditional == 0 {
                            break;
                        }
                        conditional = conditional.saturating_sub(1);
                        index += 1;
                        continue;
                    }
                    TokenKind::Punctuation('?') => {
                        conditional = conditional.saturating_add(1);
                        index += 1;
                        continue;
                    }
                    TokenKind::Operator(op) if matches!(*op, "&&" | "||" | "??") => {
                        if conditional == 0 {
                            break;
                        }
                    }
                    TokenKind::Punctuation(')' | '}' | ']') => {
                        break;
                    }
                    _ => {}
                }
            }

            index += 1;
        }

        index
    }
}
