use super::{ExprError, ExprNode, ExprParser};
use crate::frontend::diagnostics::Span;
use crate::frontend::lexer::TokenKind;
use crate::frontend::literals::StringLiteralContents;
use crate::syntax::expr::builders::{
    InlineAsmExpr, InlineAsmOperand, InlineAsmOperandMode, InlineAsmOptions, InlineAsmRegister,
    InlineAsmRegisterClass, InlineAsmTemplate, InlineAsmTemplateOperandRef, InlineAsmTemplatePiece,
};

impl ExprParser {
    #[allow(clippy::too_many_lines)]
    pub(super) fn parse_inline_asm(&mut self, asm_span: Span) -> Result<ExprNode, ExprError> {
        if !self.expect_inline_asm_bang() {
            return Err(ExprError::new(
                "expected `!` after `asm`",
                self.peek().map(|token| token.span),
            ));
        }
        if !self.expect_punctuation('(') {
            return Err(ExprError::new(
                "expected `(` to start inline assembly",
                self.peek().map(|token| token.span),
            ));
        }

        let expr_start = self.index.saturating_sub(2);
        let mut template = InlineAsmTemplate {
            pieces: Vec::new(),
            span: Some(asm_span),
        };
        let mut operands = Vec::new();
        let mut clobbers = Vec::new();
        let mut options = InlineAsmOptions::default();
        let mut parsed_templates = false;
        let mut allow_templates = true;
        let mut parsed_args = 0usize;

        while !self.peek_punctuation(')') {
            if parsed_args > 0 {
                if self.expect_punctuation(',') {
                    if self.peek_punctuation(')') {
                        break;
                    }
                } else {
                    return Err(ExprError::new(
                        "expected `,` or `)` in inline assembly",
                        self.peek().map(|token| token.span),
                    ));
                }
            }

            let Some(next) = self.peek() else {
                return Err(ExprError::new(
                    "expected inline assembly argument",
                    self.peek().map(|token| token.span),
                ));
            };

            if allow_templates && matches!(next.kind, TokenKind::StringLiteral(_)) {
                parsed_templates = true;
                let Some(token) = self.advance() else {
                    return Err(ExprError::new(
                        "expected inline assembly template string",
                        self.peek().map(|tok| tok.span),
                    ));
                };
                let TokenKind::StringLiteral(literal) = token.kind else {
                    unreachable!();
                };
                let pieces =
                    parse_inline_asm_template(&literal.contents, token.span, self.source.as_str())?;
                template.pieces.extend(pieces);
                allow_templates = self.peek_punctuation(',')
                    && self
                        .peek_n(1)
                        .is_some_and(|tok| matches!(tok.kind, TokenKind::StringLiteral(_)));
                parsed_args += 1;
                continue;
            }

            allow_templates = false;
            if !parsed_templates {
                return Err(ExprError::new(
                    "inline assembly requires a leading template string literal",
                    Some(asm_span),
                ));
            }

            let operand_start = self.index;
            let name = if let (Some(token), Some(next)) = (self.peek(), self.peek_n(1)) {
                if matches!(token.kind, TokenKind::Identifier)
                    && matches!(next.kind, TokenKind::Punctuation('='))
                {
                    let Some(name_token) = self.advance() else {
                        return Err(ExprError::new(
                            "expected inline assembly operand name",
                            self.peek().map(|token| token.span),
                        ));
                    };
                    self.advance();
                    Some(name_token.lexeme)
                } else {
                    None
                }
            } else {
                None
            };

            let Some(kind_token) = self.advance() else {
                return Err(ExprError::new(
                    "expected inline assembly operand or options",
                    self.peek().map(|token| token.span),
                ));
            };
            let kind_text = kind_token.lexeme.as_str();
            let span = self.span_for_range(operand_start, self.index);

            match kind_text {
                "in" => {
                    let reg_span = self.expect_inline_asm_paren_start("in")?;
                    let reg = self.parse_inline_asm_register()?;
                    if !self.expect_inline_asm_paren_end("in", reg_span) {
                        return Err(ExprError::new(
                            "expected `)` after `in(...)` register",
                            reg_span.or_else(|| self.peek().map(|token| token.span)),
                        ));
                    }
                    let expr_start = self.index;
                    let value = self.parse_expression()?;
                    let value_span = self.span_for_range(expr_start, self.index);
                    operands.push(InlineAsmOperand {
                        name,
                        reg,
                        mode: InlineAsmOperandMode::In { expr: value },
                        span: span.or(value_span),
                    });
                }
                "out" | "lateout" => {
                    let reg_span = self.expect_inline_asm_paren_start(kind_text)?;
                    let reg = self.parse_inline_asm_register()?;
                    if !self.expect_inline_asm_paren_end(kind_text, reg_span) {
                        return Err(ExprError::new(
                            "expected `)` after `out(...)` register",
                            reg_span.or_else(|| self.peek().map(|token| token.span)),
                        ));
                    }
                    let expr_start = self.index;
                    let place = self.parse_expression()?;
                    let place_span = self.span_for_range(expr_start, self.index);
                    operands.push(InlineAsmOperand {
                        name,
                        reg,
                        mode: InlineAsmOperandMode::Out {
                            expr: place,
                            late: kind_text == "lateout",
                        },
                        span: span.or(place_span),
                    });
                }
                "inout" | "inlateout" => {
                    let reg_span = self.expect_inline_asm_paren_start(kind_text)?;
                    let reg = self.parse_inline_asm_register()?;
                    if !self.expect_inline_asm_paren_end(kind_text, reg_span) {
                        return Err(ExprError::new(
                            "expected `)` after `inout(...)` register",
                            reg_span.or_else(|| self.peek().map(|token| token.span)),
                        ));
                    }
                    let input_start = self.index;
                    let input = self.parse_expression()?;
                    let mut output = None;
                    if self.expect_punctuation('=') {
                        if !self.expect_punctuation('>') {
                            return Err(ExprError::new(
                                "expected `>` after `=>` in `inout` operand",
                                self.peek().map(|token| token.span),
                            ));
                        }
                        output = Some(self.parse_expression()?);
                    }
                    let input_span = self.span_for_range(input_start, self.index);
                    operands.push(InlineAsmOperand {
                        name,
                        reg,
                        mode: InlineAsmOperandMode::InOut {
                            input,
                            output,
                            late: kind_text == "inlateout",
                        },
                        span: span.or(input_span),
                    });
                }
                "const" => {
                    let expr_start = self.index;
                    let expr = self.parse_expression()?;
                    let expr_span = self.span_for_range(expr_start, self.index);
                    operands.push(InlineAsmOperand {
                        name,
                        reg: InlineAsmRegister::Class(InlineAsmRegisterClass::Reg),
                        mode: InlineAsmOperandMode::Const { expr },
                        span: span.or(expr_span),
                    });
                }
                "sym" => {
                    let Some(sym_token) = self.advance() else {
                        return Err(ExprError::new(
                            "expected symbol path after `sym`",
                            self.peek().map(|token| token.span),
                        ));
                    };
                    if !matches!(sym_token.kind, TokenKind::Identifier) {
                        return Err(ExprError::new(
                            "symbol operand must be an identifier path",
                            Some(sym_token.span),
                        ));
                    }
                    operands.push(InlineAsmOperand {
                        name,
                        reg: InlineAsmRegister::Class(InlineAsmRegisterClass::Reg),
                        mode: InlineAsmOperandMode::Sym {
                            path: sym_token.lexeme,
                        },
                        span,
                    });
                }
                "clobber" => {
                    let reg_span = self.expect_inline_asm_paren_start(kind_text)?;
                    loop {
                        let reg = self.parse_inline_asm_register()?;
                        clobbers.push(reg);
                        if self.expect_inline_asm_paren_end(kind_text, reg_span) {
                            break;
                        }
                        if !self.expect_punctuation(',') {
                            return Err(ExprError::new(
                                "expected `,` between clobber registers",
                                self.peek().map(|token| token.span),
                            ));
                        }
                    }
                }
                "options" => {
                    let options_span = self.expect_inline_asm_paren_start(kind_text)?;
                    self.parse_inline_asm_options(&mut options, options_span)?;
                }
                _ => {
                    return Err(ExprError::new(
                        format!("unknown inline assembly operand kind `{kind_text}`"),
                        Some(kind_token.span),
                    ));
                }
            }

            parsed_args += 1;
        }

        if !self.expect_punctuation(')') {
            return Err(ExprError::new(
                "expected `)` to close inline assembly",
                self.peek().map(|token| token.span),
            ));
        }

        if !parsed_templates {
            return Err(ExprError::new(
                "inline assembly requires at least one template string literal",
                Some(asm_span),
            ));
        }

        let span = self.span_for_range(expr_start, self.index);
        Ok(ExprNode::InlineAsm(InlineAsmExpr {
            template,
            operands,
            clobbers,
            options,
            span,
        }))
    }

    pub(super) fn peek_inline_asm_bang(&self) -> bool {
        self.peek().is_some_and(|token| match &token.kind {
            TokenKind::Punctuation('!') => true,
            TokenKind::Operator(op) => *op == "!",
            _ => false,
        })
    }

    fn expect_inline_asm_bang(&mut self) -> bool {
        if self.peek_inline_asm_bang() {
            self.advance();
            return true;
        }
        false
    }

    #[allow(clippy::too_many_lines)]
    fn parse_inline_asm_options(
        &mut self,
        options: &mut InlineAsmOptions,
        options_span: Option<Span>,
    ) -> Result<(), ExprError> {
        let mut seen_any = false;
        loop {
            if self.expect_punctuation(')') {
                break;
            }
            let Some(token) = self.advance() else {
                return Err(ExprError::new(
                    "expected option inside `options(...)`",
                    options_span,
                ));
            };
            if !matches!(token.kind, TokenKind::Identifier | TokenKind::Keyword(_)) {
                return Err(ExprError::new(
                    "options must be identifiers (e.g., volatile, nostack)",
                    Some(token.span),
                ));
            }
            seen_any = true;
            match token.lexeme.as_str() {
                "volatile" => options.volatile = true,
                "alignstack" => options.alignstack = true,
                "intel" | "intel_syntax" => options.intel_syntax = true,
                "att_syntax" | "att" => options.intel_syntax = false,
                "nomem" => options.nomem = true,
                "nostack" => options.nostack = true,
                "preserves_flags" => options.preserves_flags = true,
                "pure" => options.pure = true,
                "readonly" => options.readonly = true,
                "noreturn" => options.noreturn = true,
                other => {
                    return Err(ExprError::new(
                        format!("unsupported inline assembly option `{other}`"),
                        Some(token.span),
                    ));
                }
            }

            if self.expect_punctuation(')') {
                break;
            }
            if !self.expect_punctuation(',') {
                return Err(ExprError::new(
                    "expected `,` or `)` after inline assembly option",
                    self.peek().map(|token| token.span),
                ));
            }
        }

        if !seen_any {
            return Err(ExprError::new(
                "inline assembly options cannot be empty",
                options_span,
            ));
        }
        Ok(())
    }

    fn expect_inline_asm_paren_start(&mut self, ctx: &str) -> Result<Option<Span>, ExprError> {
        let start = self.peek().map(|token| token.span);
        if !self.expect_punctuation('(') {
            return Err(ExprError::new(
                format!("expected `(` after `{ctx}`"),
                self.peek().map(|token| token.span),
            ));
        }
        Ok(start)
    }

    fn expect_inline_asm_paren_end(&mut self, _ctx: &str, _start_span: Option<Span>) -> bool {
        self.expect_punctuation(')')
    }

    fn parse_inline_asm_register(&mut self) -> Result<InlineAsmRegister, ExprError> {
        let Some(token) = self.advance() else {
            return Err(ExprError::new(
                "expected register class or name in inline assembly operand",
                self.peek().map(|token| token.span),
            ));
        };
        match token.kind {
            TokenKind::Identifier => {
                let name = token.lexeme.to_ascii_lowercase();
                let reg = match name.as_str() {
                    "reg" => InlineAsmRegister::Class(InlineAsmRegisterClass::Reg),
                    "reg8" => InlineAsmRegister::Class(InlineAsmRegisterClass::Reg8),
                    "reg16" => InlineAsmRegister::Class(InlineAsmRegisterClass::Reg16),
                    "reg32" => InlineAsmRegister::Class(InlineAsmRegisterClass::Reg32),
                    "reg64" => InlineAsmRegister::Class(InlineAsmRegisterClass::Reg64),
                    "xmm" => InlineAsmRegister::Class(InlineAsmRegisterClass::Xmm),
                    "ymm" => InlineAsmRegister::Class(InlineAsmRegisterClass::Ymm),
                    "zmm" => InlineAsmRegister::Class(InlineAsmRegisterClass::Zmm),
                    "vreg" => InlineAsmRegister::Class(InlineAsmRegisterClass::Vreg),
                    "kreg" => InlineAsmRegister::Class(InlineAsmRegisterClass::Kreg),
                    other => InlineAsmRegister::Explicit(other.to_string()),
                };
                Ok(reg)
            }
            TokenKind::StringLiteral(literal) => match literal.contents {
                StringLiteralContents::Simple(text) => {
                    Ok(InlineAsmRegister::Explicit(text.to_ascii_lowercase()))
                }
                StringLiteralContents::Interpolated(_) => Err(ExprError::new(
                    "register strings cannot be interpolated",
                    Some(token.span),
                )),
            },
            _ => Err(ExprError::new(
                "expected register class or name",
                Some(token.span),
            )),
        }
    }
}

#[allow(clippy::too_many_lines)]
fn parse_inline_asm_template(
    literal: &StringLiteralContents,
    span: Span,
    _source: &str,
) -> Result<Vec<InlineAsmTemplatePiece>, ExprError> {
    let contents = match literal {
        StringLiteralContents::Simple(text) => text.clone(),
        StringLiteralContents::Interpolated(_) => {
            return Err(ExprError::new(
                "inline assembly templates do not support string interpolation",
                Some(span),
            ));
        }
    };
    let mut pieces = Vec::new();
    let mut current = String::new();
    let mut chars = contents.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '{' => {
                if chars.peek() == Some(&'{') {
                    chars.next();
                    current.push('{');
                    continue;
                }
                if !current.is_empty() {
                    pieces.push(InlineAsmTemplatePiece::Literal(std::mem::take(
                        &mut current,
                    )));
                }
                let mut name = String::new();
                let mut modifier = String::new();
                let mut in_modifier = false;
                loop {
                    let Some(next) = chars.next() else {
                        return Err(ExprError::new(
                            "unclosed `{` in inline assembly template",
                            Some(span),
                        ));
                    };
                    if next == '}' {
                        break;
                    }
                    if next == ':' && !in_modifier {
                        in_modifier = true;
                        continue;
                    }
                    if in_modifier {
                        modifier.push(next);
                    } else {
                        name.push(next);
                    }
                }
                if name.is_empty() {
                    return Err(ExprError::new(
                        "inline assembly placeholder must reference an operand",
                        Some(span),
                    ));
                }
                let operand = if let Ok(index) = name.parse::<usize>() {
                    InlineAsmTemplateOperandRef::Position(index)
                } else {
                    InlineAsmTemplateOperandRef::Named(name)
                };
                pieces.push(InlineAsmTemplatePiece::Placeholder {
                    operand,
                    modifier: if modifier.is_empty() {
                        None
                    } else {
                        Some(modifier)
                    },
                    span: Some(span),
                });
            }
            '}' => {
                if chars.peek() == Some(&'}') {
                    chars.next();
                    current.push('}');
                    continue;
                }
                return Err(ExprError::new(
                    "unmatched `}` in inline assembly template",
                    Some(span),
                ));
            }
            other => current.push(other),
        }
    }
    if !current.is_empty() {
        pieces.push(InlineAsmTemplatePiece::Literal(current));
    }
    Ok(pieces)
}
