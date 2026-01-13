use super::{ExprError, ExprNode, ExprParser};
use crate::frontend::diagnostics::Span;
use crate::frontend::lexer::{Keyword, Token, TokenKind};
use crate::mir::UnOp;
use crate::syntax::expr::builders::{
    CallArgument, CallArgumentModifier, CallArgumentName, InlineBinding, InlineBindingKind,
};
use crate::syntax::expr::precedence::can_start_unary_expression;

impl ExprParser {
    #[allow(clippy::too_many_lines)]
    pub(super) fn parse_postfix(&mut self) -> Result<ExprNode, ExprError> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.peek_punctuation('?') {
                if let Some(next) = self.peek_n(1) {
                    if matches!(next.kind, TokenKind::Punctuation('.')) {
                        self.advance(); // '?'
                        self.advance(); // '.'
                        let member = self.advance().ok_or_else(|| {
                            ExprError::new("expected member name after `?.`", None)
                        })?;
                        if !matches!(member.kind, TokenKind::Identifier) {
                            return Err(ExprError::new(
                                "member access requires identifier after `?.`",
                                Some(member.span),
                            ));
                        }
                        expr = ExprNode::Member {
                            base: Box::new(expr),
                            member: member.lexeme,
                            null_conditional: true,
                        };
                        continue;
                    }
                    if matches!(next.kind, TokenKind::Punctuation('[')) {
                        self.advance(); // '?'
                        self.advance(); // '['
                        let mut indices = Vec::new();
                        loop {
                            if self.peek_punctuation(']') {
                                return Err(ExprError::new(
                                    "expected index expression before `]`",
                                    self.peek().map(|token| token.span),
                                ));
                            }
                            let index_expr = self.parse_expression()?;
                            indices.push(index_expr);
                            if self.peek_punctuation(',') {
                                self.advance();
                                continue;
                            }
                            break;
                        }
                        if !self.expect_punctuation(']') {
                            return Err(ExprError::new(
                                "expected `]` after index expression",
                                self.peek().map(|token| token.span),
                            ));
                        }
                        expr = ExprNode::Index {
                            base: Box::new(expr),
                            indices,
                            null_conditional: true,
                        };
                        continue;
                    }
                }
                let next_starts_expression = self.peek_n(1).is_some_and(can_start_unary_expression);
                if next_starts_expression {
                    break;
                }
                let question_span = self.peek().map(|token| token.span);
                self.advance();
                expr = ExprNode::TryPropagate {
                    expr: Box::new(expr),
                    question_span,
                };
                continue;
            }
            if let Some(token) = self.peek().cloned() {
                let is_scope_op = matches!(token.kind, TokenKind::Operator(op) if op == "::")
                    || (matches!(token.kind, TokenKind::Punctuation(':'))
                        && self.peek_punctuation_n(1, ':'));
                if is_scope_op {
                    self.advance();
                    if matches!(token.kind, TokenKind::Punctuation(':')) {
                        // consume second ':' when lexer splits the operator.
                        self.advance();
                    }
                    let member = self
                        .advance()
                        .ok_or_else(|| ExprError::new("expected member name after `::`", None))?;
                    if !matches!(member.kind, TokenKind::Identifier) {
                        return Err(ExprError::new(
                            "member access requires identifier after `::`",
                            Some(member.span),
                        ));
                    }
                    expr = ExprNode::Member {
                        base: Box::new(expr),
                        member: member.lexeme,
                        null_conditional: false,
                    };
                    continue;
                }
            }
            if self.peek_punctuation('.') {
                self.advance();
                let member = self
                    .advance()
                    .ok_or_else(|| ExprError::new("expected member name after '.'", None))?;
                if !matches!(member.kind, TokenKind::Identifier) {
                    return Err(ExprError::new(
                        "member access requires identifier after '.'",
                        Some(member.span),
                    ));
                }
                expr = ExprNode::Member {
                    base: Box::new(expr),
                    member: member.lexeme,
                    null_conditional: false,
                };
                continue;
            }
            if self.generic_call_follows() {
                let generics = self.parse_generic_argument_list()?;
                if !self.expect_punctuation('(') {
                    return Err(ExprError::new(
                        "expected `(` after generic argument list",
                        self.peek().map(|token| token.span),
                    ));
                }
                let args = self.parse_argument_list()?;
                expr = ExprNode::Call {
                    callee: Box::new(expr),
                    args,
                    generics: Some(generics),
                };
                continue;
            }
            if Self::expr_accepts_generic_suffix(&expr)
                && let Some((suffix, next_index)) = self.scan_type_generic_suffix(self.index)
            {
                self.index = next_index;
                expr = Self::append_generic_suffix(expr, &suffix);
                continue;
            }
            if self.peek_punctuation('(') {
                self.advance();
                let args = self.parse_argument_list()?;
                expr = ExprNode::Call {
                    callee: Box::new(expr),
                    args,
                    generics: None,
                };
                continue;
            }
            if self.peek_punctuation('[') {
                self.advance();
                let mut indices = Vec::new();
                if self.peek_punctuation(']') {
                    return Err(ExprError::new(
                        "expected index expression before `]`",
                        self.peek().map(|token| token.span),
                    ));
                }
                loop {
                    if self.peek_punctuation(']') {
                        return Err(ExprError::new(
                            "expected index expression before `]`",
                            self.peek().map(|token| token.span),
                        ));
                    }
                    let index_expr = self.parse_expression()?;
                    indices.push(index_expr);
                    if self.peek_punctuation(',') {
                        self.advance();
                        continue;
                    }
                    break;
                }
                if !self.expect_punctuation(']') {
                    return Err(ExprError::new(
                        "expected `]` after index expression",
                        self.peek().map(|token| token.span),
                    ));
                }
                expr = ExprNode::Index {
                    base: Box::new(expr),
                    indices,
                    null_conditional: false,
                };
                continue;
            }
            if let Some(token) = self.peek().cloned() {
                if let TokenKind::Operator(op) = token.kind {
                    if op == "++" || op == "--" {
                        self.advance();
                        let op = if op == "++" {
                            UnOp::Increment
                        } else {
                            UnOp::Decrement
                        };
                        expr = ExprNode::Unary {
                            op,
                            expr: Box::new(expr),
                            postfix: true,
                        };
                        continue;
                    }
                }
            }
            break;
        }
        Ok(expr)
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn parse_argument_list(&mut self) -> Result<Vec<CallArgument>, ExprError> {
        let mut args = Vec::new();
        if self.expect_punctuation(')') {
            return Ok(args);
        }
        loop {
            let arg_start = self.index;
            let arg = if self.is_named_argument() {
                let Some(name_token) = self.advance() else {
                    return Err(ExprError::new(
                        "expected identifier for named argument",
                        self.peek().map(|token| token.span),
                    ));
                };
                if !matches!(name_token.kind, TokenKind::Identifier) {
                    return Err(ExprError::new(
                        "expected identifier for named argument",
                        Some(name_token.span),
                    ));
                }
                if !self.expect_punctuation(':') {
                    return Err(ExprError::new(
                        "expected `:` after named argument",
                        self.peek().map(|token| token.span),
                    ));
                }
                let (modifier, modifier_span) = self.take_argument_modifier()?;
                let inline_binding = self.parse_inline_binding(modifier)?;
                let (value, value_span) = if let Some(binding) = inline_binding.clone() {
                    (
                        ExprNode::Identifier(binding.name.clone()),
                        binding.name_span,
                    )
                } else {
                    let value_start = self.index;
                    let value = self.parse_expression()?;
                    let value_span = self.span_for_range(value_start, self.index);
                    (value, value_span)
                };
                let span = self.span_for_range(arg_start, self.index);
                let mut argument = CallArgument::named(
                    CallArgumentName::new(name_token.lexeme, Some(name_token.span)),
                    value,
                    span,
                    value_span,
                );
                if let Some(modifier) = modifier {
                    argument = argument.with_modifier(modifier, modifier_span);
                }
                if let Some(binding) = inline_binding {
                    argument = argument.with_inline_binding(binding);
                }
                argument
            } else {
                let (modifier, modifier_span) = self.take_argument_modifier()?;
                let inline_binding = self.parse_inline_binding(modifier)?;
                let (value, value_span) = if let Some(binding) = inline_binding.clone() {
                    (
                        ExprNode::Identifier(binding.name.clone()),
                        binding.name_span,
                    )
                } else {
                    let value_start = self.index;
                    let value = self.parse_expression()?;
                    let value_span = self.span_for_range(value_start, self.index);
                    (value, value_span)
                };
                let span = self.span_for_range(arg_start, self.index);
                let mut argument = CallArgument::positional(value, span, value_span);
                if let Some(modifier) = modifier {
                    argument = argument.with_modifier(modifier, modifier_span);
                }
                if let Some(binding) = inline_binding {
                    argument = argument.with_inline_binding(binding);
                }
                argument
            };
            args.push(arg);
            if self.expect_punctuation(')') {
                break;
            }
            if !self.expect_punctuation(',') {
                return Err(ExprError::new(
                    "expected ',' or ')' in argument list",
                    self.peek().map(|token| token.span),
                ));
            }
        }
        Ok(args)
    }

    pub(super) fn is_named_argument(&self) -> bool {
        matches!(
            (self.peek(), self.peek_n(1)),
            (
                Some(Token {
                    kind: TokenKind::Identifier,
                    ..
                }),
                Some(Token {
                    kind: TokenKind::Punctuation(':'),
                    ..
                })
            )
        )
    }

    pub(super) fn take_argument_modifier(
        &mut self,
    ) -> Result<(Option<CallArgumentModifier>, Option<Span>), ExprError> {
        let Some((modifier, span)) = self.consume_argument_modifier() else {
            return Ok((None, None));
        };
        if let Some((_, dup_span)) = self.peek_argument_modifier() {
            return Err(ExprError::new(
                "duplicate argument modifier",
                Some(dup_span),
            ));
        }
        Ok((Some(modifier), Some(span)))
    }

    #[allow(clippy::too_many_lines, clippy::collapsible_if)]
    pub(super) fn parse_inline_binding(
        &mut self,
        modifier: Option<CallArgumentModifier>,
    ) -> Result<Option<InlineBinding>, ExprError> {
        if !matches!(modifier, Some(CallArgumentModifier::Out)) {
            return Ok(None);
        }
        let is_assignment_token = |token: &Token| match &token.kind {
            TokenKind::Punctuation('=') => true,
            TokenKind::Operator(op) if op.as_bytes() == b"=" => true,
            _ => false,
        };

        if let Some(keyword_token) = self.peek()
            && matches!(keyword_token.kind, TokenKind::Keyword(Keyword::Var))
        {
            let Some(var_token) = self.advance() else {
                return Err(ExprError::new(
                    "expected `var` keyword",
                    self.peek().map(|token| token.span),
                ));
            };
            let var_span = Some(var_token.span);

            let Some(name_token) = self.advance() else {
                return Err(ExprError::new(
                    "expected identifier after `var`",
                    self.peek().map(|token| token.span),
                ));
            };
            if !matches!(name_token.kind, TokenKind::Identifier) {
                return Err(ExprError::new(
                    "expected identifier after `var`",
                    Some(name_token.span),
                ));
            }

            let (initializer, initializer_span) = if let Some(token) = self.peek()
                && is_assignment_token(token)
            {
                self.advance();
                let init_start = self.index;
                let value = self.parse_assignment()?;
                let span = self.span_for_range(init_start, self.index);
                (Some(value), span)
            } else {
                (None, None)
            };

            return Ok(Some(InlineBinding {
                kind: InlineBindingKind::Var,
                name: name_token.lexeme.clone(),
                keyword_span: var_span,
                name_span: Some(name_token.span),
                initializer,
                initializer_span,
            }));
        }

        let checkpoint = self.index;
        if let Some((type_name, after_type)) = self.scan_type_name(self.index) {
            let type_span = self.span_for_range(self.index, after_type);
            let Some(name_token_peek) = self.tokens.get(after_type).cloned() else {
                self.index = checkpoint;
                return Ok(None);
            };
            if !matches!(name_token_peek.kind, TokenKind::Identifier) {
                self.index = checkpoint;
                return Ok(None);
            }

            self.index = after_type;
            let Some(name_token) = self.advance() else {
                self.index = checkpoint;
                return Err(ExprError::new(
                    "expected identifier after type name",
                    self.peek().map(|token| token.span),
                ));
            };

            let (initializer, initializer_span) = if let Some(token) = self.peek()
                && is_assignment_token(token)
            {
                self.advance();
                let init_start = self.index;
                let value = self.parse_assignment()?;
                let span = self.span_for_range(init_start, self.index);
                (Some(value), span)
            } else {
                (None, None)
            };

            return Ok(Some(InlineBinding {
                kind: InlineBindingKind::Typed {
                    type_name,
                    type_span,
                },
                name: name_token.lexeme.clone(),
                keyword_span: type_span,
                name_span: Some(name_token.span),
                initializer,
                initializer_span,
            }));
        }

        self.index = checkpoint;
        Ok(None)
    }

    fn consume_argument_modifier(&mut self) -> Option<(CallArgumentModifier, Span)> {
        let (modifier, span) = self.peek_argument_modifier()?;
        self.advance();
        Some((modifier, span))
    }

    fn peek_argument_modifier(&self) -> Option<(CallArgumentModifier, Span)> {
        let token = self.peek()?;
        match token.kind {
            TokenKind::Keyword(Keyword::In) => Some((CallArgumentModifier::In, token.span)),
            TokenKind::Keyword(Keyword::Ref) => Some((CallArgumentModifier::Ref, token.span)),
            TokenKind::Keyword(Keyword::Out) => Some((CallArgumentModifier::Out, token.span)),
            _ => None,
        }
    }

    #[allow(clippy::too_many_lines)]
    fn generic_call_follows(&self) -> bool {
        let mut index = self.index;
        let Some(token) = self.tokens.get(index) else {
            return false;
        };
        let mut depth = match &token.kind {
            TokenKind::Punctuation('<') => 1usize,
            TokenKind::Operator(op) if op.chars().all(|ch| ch == '<') => op.len(),
            _ => return false,
        };
        index += 1;
        while let Some(tok) = self.tokens.get(index) {
            match &tok.kind {
                TokenKind::Punctuation('<') => depth += 1,
                TokenKind::Operator(op) if op.chars().all(|ch| ch == '<') => depth += op.len(),
                TokenKind::Punctuation('>') => {
                    if depth == 0 {
                        return false;
                    }
                    depth -= 1;
                    index += 1;
                    if depth == 0 {
                        break;
                    }
                    continue;
                }
                TokenKind::Operator(op) if op.chars().all(|ch| ch == '>') => {
                    let count = op.len();
                    if depth < count {
                        return false;
                    }
                    depth -= count;
                    index += 1;
                    if depth == 0 {
                        break;
                    }
                    continue;
                }
                TokenKind::Punctuation('(') if depth == 0 => break,
                _ => {}
            }
            index += 1;
        }
        if depth != 0 {
            return false;
        }
        let mut lookahead = index;
        while let Some(tok) = self.tokens.get(lookahead) {
            match tok.kind {
                TokenKind::Punctuation('(') => return true,
                TokenKind::Punctuation(')') | TokenKind::Identifier | TokenKind::Keyword(_) => {
                    return false;
                }
                TokenKind::Operator("=>") => return false,
                _ => lookahead += 1,
            }
        }
        false
    }

    fn scan_type_generic_suffix(&self, start: usize) -> Option<(String, usize)> {
        let token = self.tokens.get(start)?;
        let (mut depth, mut text): (usize, String) = match &token.kind {
            TokenKind::Punctuation('<') => (1usize, token.lexeme.clone()),
            TokenKind::Operator(op) if op.chars().all(|ch| ch == '<') => {
                (op.len(), (*op).to_string())
            }
            _ => return None,
        };
        let mut index = start + 1;
        while depth > 0 {
            let tok = self.tokens.get(index)?;
            let lexeme = tok.lexeme.clone();
            match &tok.kind {
                TokenKind::Punctuation('<') => depth += 1,
                TokenKind::Operator(op) if op.chars().all(|ch| ch == '<') => depth += op.len(),
                TokenKind::Punctuation('>') => {
                    if depth == 0 {
                        return None;
                    }
                    depth -= 1;
                }
                TokenKind::Operator(op) if op.chars().all(|ch| ch == '>') => {
                    let count = op.len();
                    if depth < count {
                        return None;
                    }
                    depth -= count;
                }
                TokenKind::Punctuation(',' | '.' | '?' | '(' | ')' | '[' | ']' | ':')
                | TokenKind::Operator("::")
                | TokenKind::Identifier
                | TokenKind::Keyword(_)
                | TokenKind::NumberLiteral(_)
                | TokenKind::StringLiteral(_)
                | TokenKind::CharLiteral(_) => {}
                _ => return None,
            }
            text.push_str(&lexeme);
            index += 1;
            if depth == 0 {
                break;
            }
        }
        if depth != 0 {
            return None;
        }
        Some((text, index))
    }

    fn expr_accepts_generic_suffix(expr: &ExprNode) -> bool {
        match expr {
            ExprNode::Identifier(_) | ExprNode::Member { .. } => true,
            ExprNode::Parenthesized(inner) => Self::expr_accepts_generic_suffix(inner),
            _ => false,
        }
    }

    fn append_generic_suffix(expr: ExprNode, suffix: &str) -> ExprNode {
        match expr {
            ExprNode::Identifier(mut name) => {
                name.push_str(suffix);
                ExprNode::Identifier(name)
            }
            ExprNode::Member {
                base,
                mut member,
                null_conditional,
            } => {
                member.push_str(suffix);
                ExprNode::Member {
                    base,
                    member,
                    null_conditional,
                }
            }
            ExprNode::Parenthesized(inner) => {
                ExprNode::Parenthesized(Box::new(Self::append_generic_suffix(*inner, suffix)))
            }
            other => other,
        }
    }

    #[allow(clippy::too_many_lines)]
    fn parse_generic_argument_list(&mut self) -> Result<Vec<String>, ExprError> {
        let Some(first) = self.advance() else {
            return Err(ExprError::new(
                "unterminated generic argument list",
                self.peek().map(|token| token.span),
            ));
        };
        let mut depth = match &first.kind {
            TokenKind::Punctuation('<') => 1usize,
            TokenKind::Operator(op) if op.chars().all(|ch| ch == '<') => op.len(),
            _ => {
                return Err(ExprError::new(
                    "expected `<` to start generic argument list",
                    Some(first.span),
                ));
            }
        };
        let start = first.span.start;
        let mut end = first.span.end;
        while depth > 0 {
            let Some(token) = self.advance() else {
                return Err(ExprError::new(
                    "unterminated generic argument list",
                    Some(first.span),
                ));
            };
            end = token.span.end;
            match &token.kind {
                TokenKind::Punctuation('<') => depth += 1,
                TokenKind::Operator(op) if op.chars().all(|ch| ch == '<') => depth += op.len(),
                TokenKind::Punctuation('>') => {
                    if depth == 0 {
                        return Err(ExprError::new(
                            "unexpected `>` in generic argument list",
                            Some(token.span),
                        ));
                    }
                    depth -= 1;
                }
                TokenKind::Operator(op) if op.chars().all(|ch| ch == '>') => {
                    let count = op.len();
                    if depth < count {
                        return Err(ExprError::new(
                            "unexpected `>` in generic argument list",
                            Some(token.span),
                        ));
                    }
                    depth -= count;
                }
                _ => {}
            }
        }

        let raw = &self.source[start..end];
        if raw.len() < 2 {
            return Err(ExprError::new(
                "generic argument list is empty",
                Some(first.span),
            ));
        }
        let inner = &raw[1..raw.len() - 1];
        let arguments = Self::split_generic_arguments(inner, first.span)?;
        Ok(arguments)
    }

    fn split_generic_arguments(content: &str, span: Span) -> Result<Vec<String>, ExprError> {
        let mut args = Vec::new();
        let mut start = 0usize;
        let mut angle = 0isize;
        let mut paren = 0isize;
        let mut bracket = 0isize;
        let mut brace = 0isize;

        for (index, ch) in content.char_indices() {
            match ch {
                '<' => angle += 1,
                '>' => angle -= 1,
                '(' => paren += 1,
                ')' => paren -= 1,
                '[' => bracket += 1,
                ']' => bracket -= 1,
                '{' => brace += 1,
                '}' => brace -= 1,
                ',' if angle == 0 && paren == 0 && bracket == 0 && brace == 0 => {
                    let segment = content[start..index].trim();
                    if segment.is_empty() {
                        return Err(ExprError::new(
                            "generic argument list contains empty entry",
                            Some(span),
                        ));
                    }
                    args.push(segment.to_string());
                    start = index + ch.len_utf8();
                }
                _ => {}
            }
        }

        if angle != 0 || paren != 0 || bracket != 0 || brace != 0 {
            return Err(ExprError::new(
                "unterminated generic argument list",
                Some(span),
            ));
        }

        let tail = content[start..].trim();
        if tail.is_empty() {
            return Err(ExprError::new(
                "generic argument list contains empty entry",
                Some(span),
            ));
        }
        args.push(tail.to_string());
        Ok(args)
    }
}
