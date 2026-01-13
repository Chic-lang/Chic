use super::*;

parser_impl! {
    pub(super) fn parse_constructor(
        &mut self,
        visibility: Visibility,
        mut modifiers: Vec<Modifier>,
        is_async: bool,
        doc: Option<DocComment>,
    ) -> Option<ConstructorDecl> {
        if is_async {
            self.push_error("constructors cannot be marked `async`", self.last_span);
        }

        let convenience_applied = self.match_keyword(Keyword::Convenience);
        let mut kind = if convenience_applied {
            ConstructorKind::Convenience
        } else {
            ConstructorKind::Designated
        };

        if !self.match_keyword(Keyword::Init) {
            let span = self.peek().map(|token| token.span);
            self.push_error("expected `init` keyword for constructor declaration", span);
            self.synchronize_class_member();
            return None;
        }

        let init_span = self.last_span;

        for modifier in modifiers.drain(..) {
            self.push_error(
                format!("modifier `{}` is not supported on constructors", modifier.name),
                Some(modifier.span),
            );
        }

        let start_offset = init_span.map(|span| span.start);

        if !self.expect_punctuation('(') {
            self.synchronize_class_member();
            return None;
        }
        let (parameters, variadic) = self.parse_parameters();
        if !self.expect_punctuation(')') {
            self.synchronize_class_member();
            return None;
        }
        if variadic {
            self.push_error("constructors cannot be variadic", self.last_span);
        }

        let initializer = match self.parse_constructor_initializer() {
            Some(initializer) => initializer,
            None => {
                self.synchronize_class_member();
                return None;
            }
        };

        if convenience_applied {
            if let Some(init) = &initializer {
                if init.target != ConstructorInitTarget::SelfType {
                    self.push_error(
                        "`convenience init` must delegate to `self(...)`",
                        init.span,
                    );
                    kind = ConstructorKind::Convenience;
                }
            } else {
                self.push_error(
                    "`convenience init` must delegate to another initializer via `: self(...)`",
                    init_span,
                );
            }
        }

        let body = match self.parse_function_tail(true, false)? {
            FunctionBodyKind::Block(block) => Some(block),
            FunctionBodyKind::Declaration => {
                self.push_error("constructors must declare a body", self.last_span);
                None
            }
        };

        let end_offset = self
            .last_span
            .map(|span| span.end)
            .or_else(|| body.as_ref().and_then(|block| block.span.map(|span| span.end)));
        let span = match (start_offset, end_offset) {
            (Some(start), Some(end)) if end >= start => {
                Some(Span::in_file(self.file_id, start, end))
            }
            _ => init_span,
        };

        Some(ConstructorDecl {
            visibility,
            kind,
            parameters,
            body,
            initializer,
            doc,
            span,
            attributes: Vec::new(),
            di_inject: None,
        })
    }

    pub(super) fn parse_struct_constructor(
        &mut self,
        struct_name: &str,
        visibility: Visibility,
        mut modifiers: Vec<Modifier>,
        doc: Option<DocComment>,
    ) -> Option<ConstructorDecl> {
        let start_span = self.peek().map(|token| token.span.start);
        let name_span = self.peek().map(|token| token.span);
        if !self.match_keyword(Keyword::Init) {
            let constructor_name = self.consume_identifier("expected constructor name")?;
            let mut diagnostic = Diagnostic::error(
                format!("constructors must be declared with `init` on `{struct_name}`"),
                self.last_span.or(name_span),
            )
            .with_code(DiagnosticCode::new("E0C01", Some("constructor".into())))
            .with_primary_label("type-named constructors are not allowed");
            diagnostic.add_note(format!(
                "replace `{constructor_name}(...)` with `init(...)` inside `{struct_name}`"
            ));
            diagnostic.add_suggestion(Suggestion::new(
                "use `init` instead of repeating the type name",
                self.last_span.or(name_span),
                Some("init".to_string()),
            ));
            self.diagnostics.push(diagnostic);
        }

        for modifier in modifiers.drain(..) {
            self.push_error(
                format!(
                    "modifier `{}` is not supported on struct constructors",
                    modifier.name
                ),
                Some(modifier.span),
            );
        }

        if !self.expect_punctuation('(') {
            self.synchronize_field();
            return None;
        }
        let (parameters, variadic) = self.parse_parameters();
        if !self.expect_punctuation(')') {
            self.synchronize_field();
            return None;
        }
        if variadic {
            self.push_error("constructors cannot be variadic", self.last_span);
        }

        let initializer = match self.parse_constructor_initializer() {
            Some(initializer) => {
                if let Some(init) = &initializer {
                    if init.target != ConstructorInitTarget::SelfType {
                        self.push_error(
                            "struct constructors can only delegate to `self(...)`",
                            init.span,
                        );
                    }
                }
                initializer
            }
            None => {
                self.synchronize_field();
                return None;
            }
        };

        let body = match self.parse_function_tail(true, false)? {
            FunctionBodyKind::Block(block) => Some(block),
            FunctionBodyKind::Declaration => {
                self.push_error("struct constructors must declare a body", self.last_span);
                None
            }
        };

        let end_offset = self
            .last_span
            .map(|span| span.end)
            .or_else(|| body.as_ref().and_then(|block| block.span.map(|span| span.end)));
        let span = match (start_span, end_offset) {
            (Some(start), Some(end)) if end >= start => {
                Some(Span::in_file(self.file_id, start, end))
            }
            _ => self.last_span,
        };

        Some(ConstructorDecl {
            visibility,
            kind: ConstructorKind::Designated,
            parameters,
            body,
            initializer,
            doc,
            span,
            attributes: Vec::new(),
            di_inject: None,
        })
    }

    pub(super) fn parse_constructor_initializer(&mut self) -> Option<Option<ConstructorInitializer>> {
        if !self.consume_punctuation(':') {
            return Some(None);
        }

        let target_token = self.advance();
        let Some(token) = target_token else {
            self.push_error(
                "expected `self`, `base`, or `super` after `:` in constructor initializer",
                None,
            );
            return None;
        };

        let target = match token.kind {
            TokenKind::Identifier if token.lexeme == "self" => ConstructorInitTarget::SelfType,
            TokenKind::Identifier if token.lexeme == "super" => ConstructorInitTarget::Super,
            TokenKind::Identifier if token.lexeme == "base" => ConstructorInitTarget::Super,
            _ => {
                self.push_error(
                    "expected `self`, `base`, or `super` after `:` in constructor initializer",
                    Some(token.span),
                );
                return None;
            }
        };

        let start_index = self.index - 1;

        if !self.expect_punctuation('(') {
            self.push_error(
                "expected `(` to start constructor initializer arguments",
                self.peek().map(|tok| tok.span),
            );
            return None;
        }

        let arguments = self.collect_expression_list_until(')');
        if !self.expect_punctuation(')') {
            self.push_error(
                "expected `)` to close constructor initializer",
                self.peek().map(|tok| tok.span),
            );
            return None;
        }

        let span = self
            .span_from_indices(start_index, self.index)
            .or(Some(token.span));

        Some(Some(ConstructorInitializer {
            target,
            arguments,
            span,
                    }))
    }
}
