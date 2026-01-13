use super::{ExprError, ExprNode, ExprParser, interpolation};
use crate::decimal::Decimal128;
use crate::frontend::diagnostics::Span;
use crate::frontend::lexer::{Keyword, NumericLiteral, NumericLiteralKind, Token, TokenKind};
use crate::frontend::literals::StringLiteralContents;
use crate::mir::{ConstValue, FloatValue, FloatWidth};
use crate::syntax::expr::builders::{
    ArrayLiteralExpr, LiteralConst, NameOfOperand, NewExpr, NewInitializer, ObjectInitializerField,
    QuoteInterpolation, QuoteLiteral, QuoteSourceSpan, SizeOfOperand,
};
use crate::syntax::expr::parse_expression;
use crate::syntax::numeric::{self, IntegerWidth, NumericLiteralType};

impl ExprParser {
    #[allow(clippy::too_many_lines)]
    pub(super) fn parse_primary(&mut self) -> Result<ExprNode, ExprError> {
        let token = self
            .advance()
            .ok_or_else(|| ExprError::new("expected expression", None))?;

        if let Some(array_literal) = self.try_parse_typed_array_literal(token.clone())? {
            return Ok(array_literal);
        }

        let literal_span = token.span;
        let content_start = string_content_start(&token);
        match token.kind {
            TokenKind::NumberLiteral(ref literal) => {
                Ok(ExprNode::Literal(evaluate_numeric_literal(&token, literal)))
            }
            TokenKind::StringLiteral(literal) => match literal.contents {
                StringLiteralContents::Simple(text) => Ok(ExprNode::Literal(
                    LiteralConst::without_numeric(ConstValue::RawStr(text)),
                )),
                StringLiteralContents::Interpolated(segments) => {
                    interpolation::parse_interpolated_string(segments, literal_span, content_start)
                }
            },
            TokenKind::CharLiteral(literal) => Ok(ExprNode::Literal(
                LiteralConst::without_numeric(ConstValue::Char(literal.value)),
            )),
            TokenKind::Punctuation('[') => self.parse_array_literal(None, token.span),
            TokenKind::Identifier => {
                if token.lexeme == "true" {
                    Ok(ExprNode::Literal(LiteralConst::without_numeric(
                        ConstValue::Bool(true),
                    )))
                } else if token.lexeme == "false" {
                    Ok(ExprNode::Literal(LiteralConst::without_numeric(
                        ConstValue::Bool(false),
                    )))
                } else if token.lexeme == "null" {
                    Ok(ExprNode::Literal(LiteralConst::without_numeric(
                        ConstValue::Null,
                    )))
                } else if token.lexeme == "asm" && self.peek_inline_asm_bang() {
                    self.parse_inline_asm(token.span)
                } else {
                    Ok(ExprNode::Identifier(token.lexeme))
                }
            }
            TokenKind::Keyword(keyword) => match keyword {
                Keyword::In
                | Keyword::Ref
                | Keyword::Out
                | Keyword::Var
                | Keyword::Let
                | Keyword::Const => Ok(ExprNode::Identifier(token.lexeme)),
                Keyword::New => self.parse_new_expr(token.span),
                Keyword::Type => Ok(ExprNode::Identifier(token.lexeme)),
                _ => Err(ExprError::new(
                    format!("keyword `{}` cannot be used in expression", token.lexeme),
                    Some(token.span),
                )),
            },
            TokenKind::Punctuation('(') => {
                if self.expect_punctuation(')') {
                    return Ok(ExprNode::Literal(LiteralConst::without_numeric(
                        ConstValue::Unit,
                    )));
                }

                let first = self.parse_expression()?;

                if self.expect_punctuation(',') {
                    let mut elements = vec![first];
                    loop {
                        if self.peek_punctuation(')') {
                            return Err(ExprError::new(
                                "tuple element expected before `)`",
                                Some(token.span),
                            ));
                        }

                        let element = self.parse_expression()?;
                        elements.push(element);

                        if self.expect_punctuation(')') {
                            break;
                        }

                        if !self.expect_punctuation(',') {
                            return Err(ExprError::new(
                                "expected `,` or `)` after tuple element",
                                Some(token.span),
                            ));
                        }
                    }
                    return Ok(ExprNode::Tuple(elements));
                }

                if !self.expect_punctuation(')') {
                    return Err(ExprError::new(
                        "expected `)` to close expression",
                        Some(token.span),
                    ));
                }
                Ok(ExprNode::Parenthesized(Box::new(first)))
            }
            _ => Err(ExprError::new(
                format!("unexpected token `{}` in expression", token.lexeme),
                Some(token.span),
            )),
        }
    }

    fn parse_array_literal(
        &mut self,
        explicit_type: Option<(String, Option<Span>)>,
        open_span: Span,
    ) -> Result<ExprNode, ExprError> {
        let mut elements = Vec::new();
        let mut element_spans = Vec::new();
        let mut trailing_comma = false;
        let open_index = self.index.saturating_sub(1);
        let close_span;

        if self.expect_punctuation(']') {
            close_span = Some(open_span);
        } else {
            loop {
                if self.peek_punctuation(']') {
                    return Err(ExprError::new(
                        "array element expected before `]`",
                        self.peek().map(|tok| tok.span).or(Some(open_span)),
                    ));
                }
                let elem_start = self.index;
                let element = self.parse_expression()?;
                let elem_span = self.span_for_range(elem_start, self.index);
                elements.push(element);
                element_spans.push(elem_span);

                if self.expect_punctuation(']') {
                    close_span = self
                        .tokens
                        .get(self.index.saturating_sub(1))
                        .map(|t| t.span);
                    break;
                }
                if self.expect_punctuation(',') {
                    if self.expect_punctuation(']') {
                        trailing_comma = true;
                        close_span = self
                            .tokens
                            .get(self.index.saturating_sub(1))
                            .map(|t| t.span);
                        break;
                    }
                    continue;
                }
                return Err(ExprError::new(
                    "expected `,` or `]` after array element",
                    self.peek().map(|tok| tok.span).or(Some(open_span)),
                ));
            }
        }

        let literal_span = self.span_for_range(open_index, self.index);
        Ok(ExprNode::ArrayLiteral(ArrayLiteralExpr {
            explicit_type: explicit_type.as_ref().map(|(name, _)| name.clone()),
            explicit_type_span: explicit_type.and_then(|(_, span)| span),
            elements,
            element_spans,
            open_span: Some(open_span),
            close_span,
            trailing_comma,
            span: literal_span,
        }))
    }

    fn try_parse_typed_array_literal(
        &mut self,
        _first_token: Token,
    ) -> Result<Option<ExprNode>, ExprError> {
        let saved_index = self.index;
        let start_index = saved_index.saturating_sub(1);
        let Some((mut type_name, mut next_index)) = self.scan_type_name_without_arrays(start_index)
        else {
            return Ok(None);
        };

        let mut saw_array_rank = false;
        while let (Some(open), Some(close)) =
            (self.tokens.get(next_index), self.tokens.get(next_index + 1))
        {
            if matches!(open.kind, TokenKind::Punctuation('['))
                && matches!(close.kind, TokenKind::Punctuation(']'))
            {
                type_name.push_str("[]");
                next_index += 2;
                saw_array_rank = true;
            } else {
                break;
            }
        }

        if !saw_array_rank {
            return Ok(None);
        }

        let looks_like_type = saw_array_rank
            || type_name.contains('<')
            || type_name.contains("::")
            || type_name.contains('.')
            || type_name.contains('*')
            || type_name.ends_with('?');
        if !looks_like_type {
            return Ok(None);
        }

        let Some(open_token) = self.tokens.get(next_index) else {
            return Ok(None);
        };
        if !matches!(open_token.kind, TokenKind::Punctuation('[')) {
            return Ok(None);
        }

        let type_span = self.span_for_range(start_index, next_index);
        self.index = next_index + 1;
        let literal = self.parse_array_literal(Some((type_name, type_span)), open_token.span)?;
        Ok(Some(literal))
    }

    fn parse_new_expr(&mut self, new_span: Span) -> Result<ExprNode, ExprError> {
        let expr_start = self.index.saturating_sub(1);
        let type_start = self.index;
        let (mut type_name, next_index) =
            if let Some((type_name, next_index)) = self.scan_type_name_without_arrays(type_start) {
                (type_name, next_index)
            } else {
                (String::new(), type_start)
            };
        self.index = next_index;

        let mut array_lengths = None;
        if self.peek_punctuation('[') {
            let open = self
                .advance()
                .ok_or_else(|| ExprError::new("expected `[` to start array length list", None))?;
            let (lengths, rank_suffix) = self.parse_new_array_dimensions(open.span)?;
            type_name.push_str(&rank_suffix);
            array_lengths = Some(lengths);
        }
        let type_span = self.span_for_range(type_start, self.index);

        let mut arguments_span = None;
        let args = if self.peek_punctuation('(') {
            let args_start = self.index;
            self.advance();
            let parsed = self.parse_argument_list()?;
            arguments_span = self.span_for_range(args_start, self.index);
            parsed
        } else {
            Vec::new()
        };

        let initializer = if self.peek_punctuation('{') {
            let open = self
                .advance()
                .ok_or_else(|| ExprError::new("expected `{` to start initializer", None))?;
            Some(self.parse_new_initializer(open.span)?)
        } else {
            None
        };

        let span = self.span_for_range(expr_start, self.index);
        Ok(ExprNode::New(NewExpr {
            type_name,
            type_span,
            keyword_span: Some(new_span),
            array_lengths,
            args,
            arguments_span,
            initializer,
            span,
        }))
    }

    fn scan_type_name_without_arrays(&self, mut index: usize) -> Option<(String, usize)> {
        let (pointer_prefix, next_index) = self.collect_pointer_prefix(index);
        index = next_index;

        let mut text = String::new();
        let mut angle_depth = 0usize;

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
                TokenKind::Punctuation('[') => break,
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
                    if angle_depth > 0 {
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

    fn parse_new_array_dimensions(
        &mut self,
        open_span: Span,
    ) -> Result<(Vec<ExprNode>, String), ExprError> {
        let mut lengths = Vec::new();
        let mut dimensions = 1usize;
        loop {
            if self.expect_punctuation(']') {
                break;
            }
            if self.expect_punctuation(',') {
                dimensions += 1;
                if self.expect_punctuation(']') {
                    break;
                }
                continue;
            }
            let expr = self.parse_expression()?;
            lengths.push(expr);
            if self.expect_punctuation(']') {
                break;
            }
            if self.expect_punctuation(',') {
                dimensions += 1;
                if self.expect_punctuation(']') {
                    break;
                }
                continue;
            }
            return Err(ExprError::new(
                "expected `,` or `]` after array length expression",
                Some(open_span),
            ));
        }
        let rank_suffix = if dimensions <= 1 {
            "[]".to_string()
        } else {
            format!("[{}]", ",".repeat(dimensions - 1))
        };
        Ok((lengths, rank_suffix))
    }

    #[allow(clippy::too_many_lines)]
    fn parse_new_initializer(&mut self, open_span: Span) -> Result<NewInitializer, ExprError> {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum InitializerMode {
            Object,
            Collection,
        }

        let initializer_start = self.index.saturating_sub(1);
        let mut mode: Option<InitializerMode> = None;
        let mut fields = Vec::new();
        let mut elements = Vec::new();

        loop {
            if self.expect_punctuation('}') {
                break;
            }

            let entry_start = self.index;
            if let Some(field) = self.try_parse_object_initializer_field()? {
                match mode {
                    Some(InitializerMode::Collection) => {
                        return Err(ExprError::new(
                            "object initializer entries cannot be mixed with collection elements",
                            field.span.or(Some(open_span)),
                        ));
                    }
                    None => mode = Some(InitializerMode::Object),
                    Some(InitializerMode::Object) => {}
                }
                fields.push(field);
            } else {
                let element = self.parse_expression()?;
                match mode {
                    Some(InitializerMode::Object) => {
                        return Err(ExprError::new(
                            "collection elements cannot be mixed with object member assignments",
                            self.span_for_range(entry_start, self.index)
                                .or(Some(open_span)),
                        ));
                    }
                    None => mode = Some(InitializerMode::Collection),
                    Some(InitializerMode::Collection) => {}
                }
                elements.push(element);
            }

            if self.expect_punctuation('}') {
                break;
            }
            if self.expect_punctuation(',') {
                if self.expect_punctuation('}') {
                    break;
                }
                continue;
            }
            if self.peek_punctuation('}') {
                self.advance();
                break;
            }
            return Err(ExprError::new(
                "expected `,` or `}` after initializer entry",
                self.peek().map(|token| token.span),
            ));
        }

        let span = self.span_for_range(initializer_start, self.index);
        match mode {
            Some(InitializerMode::Collection) => Ok(NewInitializer::Collection { elements, span }),
            _ => Ok(NewInitializer::Object { fields, span }),
        }
    }

    fn try_parse_object_initializer_field(
        &mut self,
    ) -> Result<Option<ObjectInitializerField>, ExprError> {
        let checkpoint = self.index;
        let name_token = match self.peek() {
            Some(token) if matches!(token.kind, TokenKind::Identifier) => token.clone(),
            _ => return Ok(None),
        };
        let eq_token = self.peek_n(1);
        let is_assignment = matches!(
            eq_token.map(|tok| &tok.kind),
            Some(TokenKind::Punctuation('=') | TokenKind::Operator("="))
        );
        if !is_assignment {
            return Ok(None);
        }
        let name_span = Some(name_token.span);
        let name = name_token.lexeme.clone();
        self.advance();
        self.advance();

        let value_start = self.index;
        let value = self.parse_expression()?;
        let value_span = self.span_for_range(value_start, self.index);
        let span = self.span_for_range(checkpoint, self.index).or_else(|| {
            self.span_for_range(value_start, self.index)
                .or(name_span)
                .or(Some(name_token.span))
        });
        Ok(Some(ObjectInitializerField {
            name,
            name_span,
            value,
            value_span,
            span,
        }))
    }

    pub(super) fn parse_sizeof_expr(&mut self, keyword_span: Span) -> Result<ExprNode, ExprError> {
        if let Some(token) = self.peek()
            && matches!(token.kind, TokenKind::Punctuation('('))
        {
            let open = self
                .advance()
                .ok_or_else(|| ExprError::new("expected `(` after `sizeof`", Some(keyword_span)))?;

            let start_index = self.index;
            if let Some((ty_name, next_index)) = self.scan_type_name(start_index)
                && let Some(close_token) = self.tokens.get(next_index)
                && matches!(close_token.kind, TokenKind::Punctuation(')'))
            {
                self.index = next_index + 1;
                return Ok(ExprNode::SizeOf(SizeOfOperand::Type(
                    ty_name.trim().to_string(),
                )));
            }

            let expr = self.parse_expression()?;
            if !self.expect_punctuation(')') {
                return Err(ExprError::new(
                    "expected `)` to close `sizeof` operand",
                    Some(open.span),
                ));
            }
            return Ok(ExprNode::SizeOf(SizeOfOperand::Value(Box::new(expr))));
        }

        let operand = self.parse_unary()?;
        Ok(ExprNode::SizeOf(SizeOfOperand::Value(Box::new(operand))))
    }

    pub(super) fn parse_alignof_expr(&mut self, keyword_span: Span) -> Result<ExprNode, ExprError> {
        if let Some(token) = self.peek()
            && matches!(token.kind, TokenKind::Punctuation('('))
        {
            let open = self.advance().ok_or_else(|| {
                ExprError::new("expected `(` after `alignof`", Some(keyword_span))
            })?;

            let start_index = self.index;
            if let Some((ty_name, next_index)) = self.scan_type_name(start_index)
                && let Some(close_token) = self.tokens.get(next_index)
                && matches!(close_token.kind, TokenKind::Punctuation(')'))
            {
                self.index = next_index + 1;
                return Ok(ExprNode::AlignOf(SizeOfOperand::Type(
                    ty_name.trim().to_string(),
                )));
            }

            let expr = self.parse_expression()?;
            if !self.expect_punctuation(')') {
                return Err(ExprError::new(
                    "expected `)` to close `alignof` operand",
                    Some(open.span),
                ));
            }
            return Ok(ExprNode::AlignOf(SizeOfOperand::Value(Box::new(expr))));
        }

        let operand = self.parse_unary()?;
        Ok(ExprNode::AlignOf(SizeOfOperand::Value(Box::new(operand))))
    }

    pub(super) fn parse_nameof_expr(&mut self, keyword_span: Span) -> Result<ExprNode, ExprError> {
        let open = self
            .advance()
            .ok_or_else(|| ExprError::new("expected `(` after `nameof`", Some(keyword_span)))?;
        if !matches!(open.kind, TokenKind::Punctuation('(')) {
            return Err(ExprError::new(
                "expected `(` after `nameof`",
                Some(open.span),
            ));
        }

        let operand = self.parse_nameof_operand(Some(open.span))?;

        if !self.expect_punctuation(')') {
            return Err(ExprError::new(
                "expected `)` to close `nameof` operand",
                Some(open.span),
            ));
        }

        Ok(ExprNode::NameOf(operand))
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn parse_nameof_operand(
        &mut self,
        open_span: Option<Span>,
    ) -> Result<NameOfOperand, ExprError> {
        let mut segments: Vec<String> = Vec::new();
        let mut operand_start: Option<usize> = None;

        loop {
            let token = self.peek().cloned().ok_or_else(|| {
                ExprError::new("expected identifier in `nameof` operand", open_span)
            })?;

            let ident_token = match &token.kind {
                TokenKind::Identifier
                | TokenKind::Keyword(
                    Keyword::In
                    | Keyword::Ref
                    | Keyword::Out
                    | Keyword::Var
                    | Keyword::Let
                    | Keyword::Const,
                ) => {
                    self.advance();
                    token
                }
                _ => {
                    return Err(ExprError::new(
                        "expected identifier in `nameof` operand",
                        Some(token.span),
                    ));
                }
            };

            if operand_start.is_none() {
                operand_start = Some(ident_token.span.start);
            }
            segments.push(ident_token.lexeme);

            self.skip_generic_arguments()?;

            let Some(next) = self.peek() else {
                return Err(ExprError::new("unterminated `nameof` operand", open_span));
            };

            match &next.kind {
                TokenKind::Punctuation('.') => {
                    self.advance();
                }
                TokenKind::Operator(op) if op == &"::" => {
                    self.advance();
                }
                TokenKind::Punctuation(')') => break,
                _ => {
                    return Err(ExprError::new(
                        "unexpected token in `nameof` operand",
                        Some(next.span),
                    ));
                }
            }
        }

        if segments.is_empty() {
            return Err(ExprError::new("`nameof` requires an operand", open_span));
        }

        let span = operand_start
            .and_then(|start| {
                let end = self
                    .index
                    .checked_sub(1)
                    .and_then(|idx| self.tokens.get(idx))
                    .map_or(start, |token| token.span.end);
                (end > start).then_some(Span::new(start, end))
            })
            .or(open_span);

        let text = span
            .and_then(|sp| self.source.get(sp.start..sp.end))
            .map_or_else(|| segments.join("."), ToString::to_string);

        Ok(NameOfOperand {
            segments,
            text,
            span,
        })
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn parse_quote_expr(&mut self, keyword_span: Span) -> Result<ExprNode, ExprError> {
        let open = self
            .advance()
            .ok_or_else(|| ExprError::new("expected `(` after `quote`", Some(keyword_span)))?;
        if !matches!(open.kind, TokenKind::Punctuation('(')) {
            return Err(ExprError::new(
                "expected `(` after `quote`",
                Some(open.span),
            ));
        }

        let mut depth = 1usize;
        let mut close_index = None;
        while let Some(token) = self.advance() {
            match token.kind {
                TokenKind::Punctuation('(') => depth += 1,
                TokenKind::Punctuation(')') => {
                    depth -= 1;
                    if depth == 0 {
                        close_index = Some(self.index.saturating_sub(1));
                        break;
                    }
                }
                _ => {}
            }
        }

        let close_idx = close_index.ok_or_else(|| {
            ExprError::new("unterminated `quote(expr)` expression", Some(keyword_span))
        })?;
        let close = self.tokens.get(close_idx).cloned().ok_or_else(|| {
            ExprError::new("unterminated `quote(expr)` expression", Some(keyword_span))
        })?;

        let content_span = QuoteSourceSpan {
            start: open.span.end,
            end: close.span.start,
        };
        if content_span.start >= content_span.end {
            return Err(ExprError::new(
                "`quote(expr)` requires an expression between the parentheses",
                Some(content_span.to_span()),
            ));
        }

        let body_text = self.source[content_span.start..content_span.end].to_string();
        if body_text.trim().is_empty() {
            return Err(ExprError::new(
                "`quote(expr)` requires a non-empty expression",
                Some(content_span.to_span()),
            ));
        }

        let parsed_body = Self::parse_quote_body(&body_text, content_span.start)?;
        let parsed_expr = match parse_expression(&parsed_body.sanitized) {
            Ok(expr) => expr,
            Err(err) => return Err(Self::map_quote_parse_error(err, content_span)),
        };

        let literal = QuoteLiteral {
            expression: Box::new(parsed_expr),
            source: body_text,
            sanitized: parsed_body.sanitized,
            content_span: Some(content_span),
            interpolations: parsed_body.interpolations,
            hygiene_anchor: keyword_span.start,
        };
        Ok(ExprNode::Quote(Box::new(literal)))
    }

    #[allow(clippy::too_many_lines)]
    fn parse_quote_body(body: &str, base_offset: usize) -> Result<ParsedQuoteBody, ExprError> {
        let mut sanitized = String::with_capacity(body.len());
        let mut interpolations = Vec::new();
        let bytes = body.as_bytes();
        let mut slot = 0usize;
        let mut i = 0usize;
        let mut last_literal = 0usize;

        while i < bytes.len() {
            if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                sanitized.push_str(&body[last_literal..i]);
                let expr_start = i + 2;
                let mut depth = 1usize;
                let mut j = expr_start;
                while j < bytes.len() {
                    match bytes[j] {
                        b'{' => depth += 1,
                        b'}' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
                if depth != 0 {
                    return Err(ExprError::new(
                        "unterminated `${...}` in `quote(expr)` expression",
                        Some(Span::new(
                            base_offset + i,
                            base_offset + i.saturating_add(2),
                        )),
                    ));
                }
                let expr_slice = &body[expr_start..j];
                let trimmed = expr_slice.trim();
                if trimmed.is_empty() {
                    return Err(ExprError::new(
                        "`quote(expr)` interpolation `${...}` requires an expression",
                        Some(Span::new(base_offset + i, base_offset + j + 1)),
                    ));
                }

                let placeholder = format!("__chic_quote_slot{slot}");
                slot += 1;
                sanitized.push_str(&placeholder);

                let offset = slice_offset(expr_slice, trimmed);
                let span = QuoteSourceSpan {
                    start: base_offset + expr_start + offset,
                    end: base_offset + expr_start + offset + trimmed.len(),
                };
                let expr_node = parse_expression(trimmed)
                    .map_err(|err| Self::map_interpolation_error(err, span))?;

                interpolations.push(QuoteInterpolation {
                    placeholder,
                    expression: expr_node,
                    expression_text: trimmed.to_string(),
                    span: Some(span),
                });
                i = j + 1;
                last_literal = i;
                continue;
            }

            let Some(ch) = body[i..].chars().next() else {
                return Err(ExprError::new(
                    "invalid character boundary in `quote` body",
                    Some(Span::new(base_offset + i, base_offset + i)),
                ));
            };
            i += ch.len_utf8();
        }

        sanitized.push_str(&body[last_literal..]);
        Ok(ParsedQuoteBody {
            sanitized,
            interpolations,
        })
    }

    #[allow(clippy::needless_pass_by_value)]
    fn map_quote_parse_error(err: ExprError, content_span: QuoteSourceSpan) -> ExprError {
        let message = format!("failed to parse `quote(expr)` body: {}", err.message);
        ExprError::new(message, Some(content_span.to_span()))
    }

    #[allow(clippy::needless_pass_by_value)]
    fn map_interpolation_error(err: ExprError, span: QuoteSourceSpan) -> ExprError {
        let span = err.span.map_or(span, |inner| QuoteSourceSpan {
            start: span.start + inner.start,
            end: span.start + inner.end,
        });
        let message = format!("failed to parse `quote` interpolation: {}", err.message);
        ExprError::new(message, Some(span.to_span()))
    }
}

struct ParsedQuoteBody {
    sanitized: String,
    interpolations: Vec<QuoteInterpolation>,
}

fn slice_offset(parent: &str, child: &str) -> usize {
    child.as_ptr() as usize - parent.as_ptr() as usize
}

fn string_content_start(token: &Token) -> usize {
    let prefix = token.lexeme.find('"').map_or(0, |idx| idx + '"'.len_utf8());
    token.span.start.saturating_add(prefix)
}

fn evaluate_numeric_literal(token: &Token, literal: &NumericLiteral) -> LiteralConst {
    if literal.has_errors() {
        return LiteralConst::without_numeric(ConstValue::Unknown);
    }

    let metadata = numeric::numeric_literal_metadata(literal);

    let value = match literal.kind {
        NumericLiteralKind::Decimal => {
            let mut text = literal.integer.clone();
            if let Some(fraction) = &literal.fraction {
                text.push('.');
                text.push_str(fraction);
            }
            match Decimal128::parse_literal(&text) {
                Ok(value) => ConstValue::Decimal(value),
                Err(_) => ConstValue::Unknown,
            }
        }
        NumericLiteralKind::Float => {
            let parsed = literal.normalized_float_text().parse::<f64>();
            match parsed {
                Ok(value) => {
                    let width = match metadata.as_ref().map(|meta| meta.literal_type) {
                        Some(NumericLiteralType::Float16) => FloatWidth::F16,
                        Some(NumericLiteralType::Float32) => FloatWidth::F32,
                        Some(NumericLiteralType::Float64) => FloatWidth::F64,
                        Some(NumericLiteralType::Float128) => FloatWidth::F128,
                        _ => FloatWidth::F64,
                    };
                    let float_value = FloatValue::from_f64_as(value, width);
                    ConstValue::Float(float_value)
                }
                Err(_) => ConstValue::Unknown,
            }
        }
        NumericLiteralKind::Integer => {
            if let Some(integer) = numeric::parse_integer_literal(literal) {
                if integer.is_unsigned || integer.value > i128::MAX as u128 {
                    ConstValue::UInt(integer.value)
                } else {
                    match i128::try_from(integer.value) {
                        Ok(signed) => {
                            if matches!(integer.width, Some(IntegerWidth::W32)) {
                                ConstValue::Int32(signed)
                            } else {
                                ConstValue::Int(signed)
                            }
                        }
                        Err(_) => ConstValue::Unknown,
                    }
                }
            } else if let Ok(value) = token.lexeme.trim().parse::<i128>() {
                ConstValue::Int(value)
            } else if let Ok(value) = token.lexeme.trim().parse::<u128>() {
                ConstValue::UInt(value)
            } else {
                ConstValue::Unknown
            }
        }
    };

    LiteralConst::new(value, metadata)
}
