use super::*;

parser_impl! {
    pub(in crate::frontend::parser) fn parse_parameters(&mut self) -> (Vec<Parameter>, bool) {
        let mut parameters = Vec::new();
        let mut variadic = false;
        if self.check_punctuation(')') {
            return (parameters, variadic);
        }

        loop {
            if self.check_operator("...") {
                let span = self.peek().map(|token| token.span);
                self.advance();
                if parameters.is_empty() {
                    self.push_error(
                        "variadic parameter list requires at least one named parameter",
                        span,
                    );
                }
                variadic = true;
                if self.consume_punctuation(',') {
                    self.push_error("`...` must be the final parameter", span);
                    self.synchronize_parameter();
                }
                while !self.check_punctuation(')') {
                    if self.peek().is_none() {
                        break;
                    }
                    let trailing = self.peek().map(|token| token.span);
                    self.push_error("expected ')' after `...`", trailing);
                    self.advance();
                }
                break;
            }
            self.stash_leading_doc();
            let mut attrs = self.collect_attributes();
            self.stash_leading_doc();
            if !attrs.is_empty() && !attrs.builtin.is_empty() {
                self.report_attribute_misuse(
                    attrs.clone(),
                    "unsupported built-in attribute on parameters",
                );
            }
            let attributes = attrs.take_list();
            let (binding, binding_nullable) = self.parse_binding_modifier();

            if self.peek_identifier("this") {
                let _ = self.advance();
                if let Some(attr) = attributes.first() {
                    self.push_error(
                        "attributes are not supported on the implicit `this` parameter",
                        attr.span,
                    );
                }

                // C#-style extension receiver: `this Type name`
                if !self.check_punctuation(')') && !self.check_punctuation(',') {
                    let Some(ty) = self.parse_type_expr() else {
                        self.synchronize_parameter();
                        break;
                    };

                    if let Some(token) = self.peek() {
                        match &token.kind {
                            TokenKind::Punctuation('?') => {
                                self.push_error(
                                    "type annotation accepts at most one `?`",
                                    Some(token.span),
                                );
                                self.advance();
                            }
                            TokenKind::Operator(op) if *op == "??" => {
                                self.push_error(
                                    "type annotation accepts at most one `?`",
                                    Some(token.span),
                                );
                                self.advance();
                            }
                            _ => {}
                        }
                    }

                    let name_span = self.peek().map(|token| token.span);
                    let Some(name) = self.consume_identifier("expected parameter name") else {
                        self.synchronize_parameter();
                        break;
                    };

                    let lends = self.parse_lends_clause();
                    let default = if self.consume_operator("=") {
                        self.parse_parameter_default_expression(&[',', ')'])
                    } else {
                        None
                    };
                    let default_span = default.as_ref().and_then(|expr| expr.span);

                    parameters.push(Parameter {
                        binding,
                        binding_nullable,
                        name,
                        name_span,
                        ty: ty.clone(),
                        attributes,
                        di_inject: None,
                        default,
                        default_span,
                        lends,
                        is_extension_this: true,
                    });

                    if self.check_punctuation(')') {
                        break;
                    }

                    if self.consume_punctuation(',') {
                        continue;
                    }

                    let span = self.peek().map(|token| token.span);
                    self.push_error("expected ',' or ')' after parameter", span);
                    self.synchronize_parameter();
                    break;
                }

                parameters.push(Parameter {
                    binding,
                    binding_nullable,
                    name: "this".to_string(),
                    name_span: None,
                    ty: TypeExpr::self_type(),
                    attributes,
                    di_inject: None,
                    default: None,
                    default_span: None,
                    lends: None,
                    is_extension_this: true,
                });

                if self.check_punctuation(')') {
                    break;
                }

                if self.consume_punctuation(',') {
                    continue;
                }

                let span = self.peek().map(|token| token.span);
                self.push_error("expected ',' or ')' after parameter", span);
                self.synchronize_parameter();
                break;
            }

            let Some(ty) = self.parse_type_expr() else {
                self.synchronize_parameter();
                break;
            };

            if let Some(token) = self.peek() {
                match &token.kind {
                    TokenKind::Punctuation('?') => {
                        self.push_error(
                            "type annotation accepts at most one `?`",
                            Some(token.span),
                        );
                        self.advance();
                    }
                    TokenKind::Operator(op) if *op == "??" => {
                        self.push_error(
                            "type annotation accepts at most one `?`",
                            Some(token.span),
                        );
                        self.advance();
                    }
                    _ => {}
                }
            }

            let name_span = self.peek().map(|token| token.span);
            let Some(name) = self.consume_identifier("expected parameter name") else {
                self.synchronize_parameter();
                break;
            };

            let lends = self.parse_lends_clause();

            let default = if self.consume_operator("=") {
                self.parse_parameter_default_expression(&[',', ')'])
            } else {
                None
            };
            let default_span = default.as_ref().and_then(|expr| expr.span);

            parameters.push(Parameter {
                binding,
                binding_nullable,
                name,
                name_span,
                ty,
                attributes,
                di_inject: None,
                default,
                default_span,
                lends,
                is_extension_this: false,
            });

            if self.check_punctuation(')') {
                break;
            }

            if self.consume_punctuation(',') {
                continue;
            }

            let span = self.peek().map(|token| token.span);
            self.push_error("expected ',' or ')' after parameter", span);
            self.synchronize_parameter();
            break;
        }

        (parameters, variadic)
    }

    pub(in crate::frontend::parser) fn parse_binding_modifier(&mut self) -> (BindingModifier, bool) {
        let mut modifier = BindingModifier::Value;
        let mut binding_nullable = false;
        if let Some(token) = self.peek()
            && let TokenKind::Keyword(keyword) = token.kind
        {
            match keyword {
                Keyword::In => {
                    self.advance();
                    modifier = BindingModifier::In;
                }
                Keyword::Ref => {
                    self.advance();
                    modifier = BindingModifier::Ref;
                }
                Keyword::Out => {
                    self.advance();
                    modifier = BindingModifier::Out;
                }
                _ => {}
            }
        }
        if let Some(token) = self.peek() {
            match &token.kind {
                TokenKind::Punctuation('?') => {
                    let question_span = token.span;
                    self.advance();
                    match modifier {
                        BindingModifier::Ref | BindingModifier::Out => {
                            binding_nullable = true;
                        }
                        BindingModifier::In | BindingModifier::Value => {
                            self.push_error(
                                "`?` after parameter modifier is only valid with `ref` or `out`",
                                Some(question_span),
                            );
                        }
                    }
                    if let Some(next) = self.peek()
                        && matches!(next.kind, TokenKind::Punctuation('?'))
                    {
                        self.push_error(
                            "parameter modifier accepts at most one `?`",
                            Some(next.span),
                        );
                        self.advance();
                    }
                }
                TokenKind::Operator(op) if *op == "??" => {
                    let span = token.span;
                    self.advance();
                    match modifier {
                        BindingModifier::Ref | BindingModifier::Out => {
                            binding_nullable = true;
                        }
                        BindingModifier::In | BindingModifier::Value => {
                            self.push_error(
                                "`?` after parameter modifier is only valid with `ref` or `out`",
                                Some(span),
                            );
                        }
                    }
                    self.push_error(
                        "parameter modifier accepts at most one `?`",
                        Some(span),
                    );
                }
                _ => {}
            }
        }
        (modifier, binding_nullable)
    }

    fn parse_parameter_default_expression(&mut self, terminators: &[char]) -> Option<Expression> {
        let expr = self.collect_expression_until(terminators);
        if expr.span.is_none() && expr.text.trim().is_empty() {
            let span = self.peek().map(|token| token.span);
            self.push_error("expected default expression after '='", span);
            None
        } else {
            Some(expr)
        }
    }

    pub(in crate::frontend::parser) fn parse_visibility(&mut self) -> Visibility {
        let Some(Token {
            kind: TokenKind::Keyword(keyword),
            ..
        }) = self.peek()
        else {
            return Visibility::default();
        };

        match keyword {
            Keyword::Public => {
                self.advance();
                Visibility::Public
            }
            Keyword::Private => {
                self.advance();
                if self.match_keyword(Keyword::Protected) {
                    Visibility::PrivateProtected
                } else {
                    Visibility::Private
                }
            }
            Keyword::Internal => {
                self.advance();
                if self.match_keyword(Keyword::Protected) {
                    Visibility::ProtectedInternal
                } else {
                    Visibility::Internal
                }
            }
            Keyword::Protected => {
                self.advance();
                if self.match_keyword(Keyword::Internal) {
                    Visibility::ProtectedInternal
                } else {
                    Visibility::Protected
                }
            }
            _ => Visibility::default(),
        }
    }

    pub(in crate::frontend::parser) fn detect_local_declaration(
        &mut self,
    ) -> Option<LocalDeclStart> {
        while self.consume_borrow_qualifier_misuse(true) {}

        if self
            .peek()
            .is_some_and(|token| token.kind == TokenKind::Keyword(Keyword::Let))
        {
            return Some(LocalDeclStart::Let);
        }

        if self
            .peek()
            .is_some_and(|token| token.kind == TokenKind::Keyword(Keyword::Var))
        {
            return Some(LocalDeclStart::Var);
        }

        if self
            .peek()
            .is_some_and(|token| token.kind == TokenKind::Keyword(Keyword::Const))
        {
            return Some(LocalDeclStart::Const);
        }

        if self
            .peek()
            .is_some_and(|token| token.kind == TokenKind::Keyword(Keyword::Await))
        {
            return None;
        }

        // LL1_ALLOW: Typed locals reuse expression-looking syntax, so we speculatively parse a type to keep user code concise (docs/compiler/parser.md#ll1-allowances).
        if let Some((ty, next_index)) = self.try_type_expr_from(self.index)
            && self
                .tokens
                .get(next_index)
                .is_some_and(|token| matches!(token.kind, TokenKind::Identifier))
        {
            let after_name = next_index + 1;
            match self.tokens.get(after_name) {
                Some(token)
                    if matches!(
                        token.kind,
                        TokenKind::Operator("=") | TokenKind::Punctuation(';' | ',' | ')')
                    ) =>
                {
                    return Some(LocalDeclStart::Typed {
                        ty,
                        ty_start: self.index,
                        name_index: next_index,
                    });
                }
                None => {
                    return Some(LocalDeclStart::Typed {
                        ty,
                        ty_start: self.index,
                        name_index: next_index,
                    })
                }
                _ => {}
            }
        }

        None
    }

    pub(in crate::frontend::parser) fn parse_variable_declarators(
        &mut self,
        modifier: VariableModifier,
        type_annotation: Option<TypeExpr>,
        terminator: char,
        require_initializer: bool,
    ) -> Option<VariableDeclaration> {
        let mut type_annotation = type_annotation;
        let mut declarators = Vec::new();

        loop {
            let name = self.consume_identifier("expected variable name")?;
            let name_span = self.last_span;

            if type_annotation.is_none() && self.consume_punctuation(':') {
                type_annotation = Some(self.parse_type_expr()?);
            }

            let initializer = if self.consume_operator("=") {
                Some(self.collect_expression_until(&[',', terminator]))
            } else {
                None
            };

            if require_initializer && initializer.is_none() {
                self.push_error("initializer required for this declaration", name_span);
            }

            declarators.push(VariableDeclarator { name, initializer });

            if self.check_punctuation(',') {
                self.advance();
                continue;
            }
            break;
        }

        Some(VariableDeclaration {
            modifier,
            type_annotation,
            declarators,
            is_pinned: false,
        })
    }

    pub(in crate::frontend::parser) fn parse_const_declaration_body(
        &mut self,
        doc: Option<DocComment>,
        terminator: char,
    ) -> Option<ConstDeclaration> {
        let ty = self.parse_type_expr()?;
        let mut declarators = Vec::new();

        loop {
            let name = self.consume_identifier("expected constant name")?;
            let name_span = self.last_span;
            let initializer = if self.consume_operator("=") {
                self.collect_expression_until(&[',', terminator])
            } else {
                self.push_error(
                    "constant declarations require an initializer",
                    name_span,
                );
                Expression::new(String::new(), None)
            };

            let span = match (name_span, initializer.span) {
                (Some(name_span), Some(expr_span)) if expr_span.end >= name_span.start => {
                    Some(Span::in_file(
                        name_span.file_id,
                        name_span.start,
                        expr_span.end,
                    ))
                }
                _ => name_span,
            };

            declarators.push(ConstDeclarator {
                name,
                initializer,
                span,
                            });

            if self.check_punctuation(',') {
                self.advance();
                continue;
            }
            break;
        }

        Some(ConstDeclaration {
            ty,
            declarators,
            doc,
            span: None,
                    })
    }

    pub(in crate::frontend::parser) fn parse_rejected_typed_local(
        &mut self,
        ty: TypeExpr,
        ty_start: usize,
        name_index: usize,
        terminator: char,
        require_initializer: bool,
    ) -> Option<VariableDeclaration> {
        let type_span = self.span_from_indices(ty_start, name_index);
        let name_span = self.tokens.get(name_index).map(|token| token.span);
        if name_index > 0 {
            self.last_span = Some(self.tokens[name_index - 1].span);
        }
        self.index = name_index;
        self.consume_all_borrow_qualifier_misuse(false);
        let decl = self.parse_variable_declarators(
            VariableModifier::Var,
            Some(ty),
            terminator,
            require_initializer,
        );
        if !self.expect_punctuation(terminator) {
            self.emit_typed_local_error(type_span, name_span, decl.as_ref());
            return None;
        }
        self.emit_typed_local_error(type_span, name_span, decl.as_ref());
        decl
    }

    pub(in crate::frontend::parser) fn emit_typed_local_error(
        &mut self,
        type_span: Option<Span>,
        name_span: Option<Span>,
        decl: Option<&VariableDeclaration>,
    ) {
        let primary_span = type_span.or(name_span);
        let mut diagnostic = Diagnostic::error(
            "local variables must be declared with `let` (immutable) or `var` (mutable); typed locals `Type name = expr` are not allowed",
            primary_span,
        )
        .with_code(DiagnosticCode::new("LCL0001", Some("parse".into())));

        if let Some(span) = primary_span {
            diagnostic.primary_label =
                Some(Label::primary(span, "explicit typed locals are not allowed in block scope"));
        }
        if let Some(span) = name_span {
            if let Some(name) = decl
                .and_then(|d| d.declarators.first())
                .map(|d| d.name.as_str())
            {
                diagnostic = diagnostic.with_secondary(Label::secondary(
                    span,
                    format!("local variable `{name}` declared with a type"),
                ));
            } else {
                diagnostic = diagnostic
                    .with_secondary(Label::secondary(span, "local variable declared with a type"));
            }
        }

        if let Some(message) = Self::typed_local_suggestion_text(decl) {
            diagnostic.add_suggestion(Suggestion::new(message, None, None));
        }

        self.diagnostics.push(diagnostic);
    }

    fn typed_local_suggestion_text(decl: Option<&VariableDeclaration>) -> Option<String> {
        let decl = decl?;
        if decl.declarators.is_empty() {
            return None;
        }
        if decl.declarators.len() > 1 {
            let sample = decl
                .declarators
                .iter()
                .map(|d| {
                    let init = d
                        .initializer
                        .as_ref()
                        .and_then(|expr| {
                            let text = expr.text.trim();
                            (!text.is_empty()).then_some(text)
                        })
                        .unwrap_or("...");
                    if d.initializer.is_some() {
                        format!("let {} = {};", d.name, init)
                    } else {
                        format!("let {};", d.name)
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            return Some(format!(
                "split this into separate `let` or `var` bindings, e.g.: {sample}"
            ));
        }

        let first = &decl.declarators[0];
        let init = first
            .initializer
            .as_ref()
            .and_then(|expr| {
                let text = expr.text.trim();
                (!text.is_empty()).then_some(text)
            })
            .unwrap_or("...");
        let binding = if first.initializer.is_some() {
            format!("let {} = {};", first.name, init)
        } else {
            format!("let {};", first.name)
        };
        Some(format!(
            "write this as `{binding}` or use `var` if you need to mutate it later"
        ))
    }

    #[expect(
        clippy::too_many_lines,
        reason = "Handles multiple declaration forms (let/var/const); split when parser refactor lands."
    )]
    pub(in crate::frontend::parser) fn parse_variable_declaration_with_kind(
        &mut self,
        start_kind: LocalDeclStart,
        terminator: char,
        require_initializer: bool,
    ) -> Option<VariableDeclaration> {
        match start_kind {
            LocalDeclStart::Let => {
                self.match_keyword(Keyword::Let);
                self.consume_all_borrow_qualifier_misuse(false);
                let type_annotation =
                    // LL1_ALLOW: `let` declarations optionally name an explicit type, so we peek ahead to keep identifier-leading form usable (docs/compiler/parser.md#ll1-allowances).
                    if let Some((ty, next_index)) = self.try_type_expr_from(self.index) {
                        if let Some(Token {
                            kind: TokenKind::Identifier,
                            ..
                        }) = self.tokens.get(next_index)
                        {
                            if next_index > 0 {
                                self.last_span = Some(self.tokens[next_index - 1].span);
                            }
                            self.index = next_index;
                            Some(ty)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                self.parse_variable_declarators(
                    VariableModifier::Let,
                    type_annotation,
                    terminator,
                    require_initializer,
                )
            }
            LocalDeclStart::Var => {
                self.match_keyword(Keyword::Var);
                self.consume_all_borrow_qualifier_misuse(false);
                let type_annotation =
                    // LL1_ALLOW: `var` declarations share the same optional explicit type grammar and require the same speculative lookahead (docs/compiler/parser.md#ll1-allowances).
                    if let Some((ty, next_index)) = self.try_type_expr_from(self.index) {
                        if let Some(Token {
                            kind: TokenKind::Identifier,
                            ..
                        }) = self.tokens.get(next_index)
                        {
                            if next_index > 0 {
                                self.last_span = Some(self.tokens[next_index - 1].span);
                            }
                            self.index = next_index;
                            Some(ty)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                self.parse_variable_declarators(
                    VariableModifier::Var,
                    type_annotation,
                    terminator,
                    require_initializer,
                )
            }
            LocalDeclStart::Const => unreachable!("const declarations handled separately"),
            LocalDeclStart::Typed { .. } => unreachable!("typed locals handled separately"),
        }
    }
}
