use super::{methods::OperatorOwner, modifiers::MemberModifiers, *};

parser_impl! {
    pub(in crate::frontend::parser) fn parse_class_member(
        &mut self,
        class_name: &str,
        is_static_type: bool,
        nested_types: &mut Vec<Item>,
    ) -> Option<ClassMember> {
        self.stash_leading_doc();
        let mut attrs = self.collect_attributes();
        self.stash_leading_doc();
        if self.check_punctuation('}') {
            return None;
        }

        let mut doc = self.take_pending_doc();
        let visibility = self.parse_visibility();
        let modifiers = MemberModifiers::new(self.consume_modifiers());
        let is_async = modifiers.async_modifier.is_some();
        let is_constexpr = modifiers.constexpr_modifier.is_some();
        let has_extern_modifier = modifiers.extern_modifier.is_some();
        let extern_span = modifiers.extern_modifier.as_ref().map(|modifier| modifier.span);
        let has_unsafe_modifier = modifiers.unsafe_modifier.is_some();
        let unsafe_span = modifiers.unsafe_modifier.as_ref().map(|modifier| modifier.span);
        let has_required_modifier = modifiers.has_required();

        if self.check_keyword(Keyword::Class) {
            self.match_keyword(Keyword::Class);
            if let Some(item) =
                self.parse_class(visibility, modifiers.remaining(), doc.take(), attrs)
            {
                nested_types.push(item);
            }
            return None;
        }

        if self.check_keyword(Keyword::Struct) {
            self.match_keyword(Keyword::Struct);
            if let Some(item) = self.parse_struct(
                visibility,
                doc.take(),
                attrs,
                modifiers.remaining(),
                false,
            ) {
                nested_types.push(item);
            }
            return None;
        }

        if self.peek_identifier("record") {
            self.advance();
            let _ = self.match_keyword(Keyword::Struct);
            if let Some(item) = self.parse_struct(
                visibility,
                doc.take(),
                attrs,
                modifiers.remaining(),
                true,
            ) {
                nested_types.push(item);
            }
            return None;
        }

        if self.check_keyword(Keyword::Enum) {
            self.match_keyword(Keyword::Enum);
            if let Some(item) = self.parse_enum(visibility, doc.take(), attrs) {
                nested_types.push(item);
            }
            return None;
        }

        if self.check_keyword(Keyword::Interface) {
            self.match_keyword(Keyword::Interface);
            if let Some(item) = self.parse_interface(visibility, doc.take(), attrs) {
                nested_types.push(item);
            }
            return None;
        }

        if self.check_keyword(Keyword::Trait) {
            self.match_keyword(Keyword::Trait);
            if let Some(item) = self.parse_trait(visibility, doc.take(), attrs) {
                nested_types.push(item);
            }
            return None;
        }

        if self.check_keyword(Keyword::Delegate) {
            self.match_keyword(Keyword::Delegate);
            if modifiers.has_required() {
                self.push_error(
                    "`required` modifier is not supported on delegate declarations",
                    modifiers.first_required_span(),
                );
            }
            let remaining = modifiers.clone_remaining();
            if let Some(item) = self.parse_delegate(
                visibility,
                doc.take(),
                attrs,
                remaining.as_slice(),
            ) {
                nested_types.push(item);
            }
            return None;
        }

        if let Some(member) =
            self.parse_const_member(visibility, &mut doc, &attrs, &modifiers)
        {
            return Some(member);
        }

        if self.check_keyword(Keyword::Convenience) || self.check_keyword(Keyword::Init) {
            if !attrs.is_empty() && !attrs.builtin.is_empty() {
                self.report_attribute_misuse(
                    attrs.clone(),
                    "unsupported built-in attribute on constructors",
                );
            }
            if let Some(span) = unsafe_span {
                self.push_error(
                    "`unsafe` modifier is not supported on constructors",
                    Some(span),
                );
            }
            if has_required_modifier {
                self.push_error(
                    "`required` modifier is not supported on constructors",
                    modifiers.first_required_span(),
                );
            }
            if is_constexpr {
                self.push_error(
                    "constructors cannot be marked `constexpr`",
                    self.last_span,
                );
            }
            if let Some(span) = extern_span {
                self.push_error(
                    "`extern` modifier is not supported on constructors",
                    Some(span),
                );
            }
            let mut constructor = self.parse_constructor(
                visibility,
                modifiers.clone_remaining(),
                is_async,
                doc.take(),
            )?;
            let attributes = attrs.take_list();
            constructor.attributes = attributes;
            constructor.di_inject = None;
            return Some(ClassMember::Constructor(constructor));
        }

        if self.check_keyword(Keyword::Implicit) || self.check_keyword(Keyword::Explicit) {
            if has_required_modifier {
                self.push_error(
                    "`required` modifier is not supported on methods",
                    modifiers.first_required_span(),
                );
            }
            self.reject_dispatch_modifiers(&modifiers, "conversion operators");
            let mut function = self.parse_conversion_operator_member(
                visibility,
                is_async,
                doc.take(),
                modifiers.clone_remaining(),
                has_unsafe_modifier,
                OperatorOwner::Class,
            )?;
            self.apply_method_attributes(attrs, has_extern_modifier, &mut function);
            if has_unsafe_modifier {
                self.push_error(
                    "operator overloads cannot be marked `unsafe`",
                    unsafe_span,
                );
            }
            return Some(ClassMember::Method(function));
        }

        // LL1_ALLOW: Constructors named after the enclosing type require a one-token lookahead to distinguish them from fields (docs/compiler/parser.md#ll1-allowances).
        if self.peek_identifier(class_name) && self.peek_punctuation_n(1, '(') {
            self.reject_dispatch_modifiers(&modifiers, "constructors");
            return self.parse_named_constructor(
                class_name,
                visibility,
                modifiers.clone_remaining(),
                doc.take(),
                &mut attrs,
                is_async,
                is_constexpr,
                extern_span,
                unsafe_span,
                has_required_modifier,
            );
        }

        let return_type = self.parse_type_expr()?;

        if self.check_keyword(Keyword::Operator) {
            if has_required_modifier {
                self.push_error(
                    "`required` modifier is not supported on methods",
                    modifiers.first_required_span(),
                );
            }
            self.reject_dispatch_modifiers(&modifiers, "operator overloads");
            let mut function = self.parse_symbol_operator_member(
                visibility,
                is_async,
                doc.take(),
                modifiers.clone_remaining(),
                return_type,
                has_unsafe_modifier,
                OperatorOwner::Class,
            )?;
            self.apply_method_attributes(attrs, has_extern_modifier, &mut function);
            if has_unsafe_modifier {
                self.push_error(
                    "operator overloads cannot be marked `unsafe`",
                    unsafe_span,
                );
            }
            return Some(ClassMember::Method(function));
        }

        let mut explicit_interface: Option<String> = None;
        let (name, name_token_index) = if self.peek_identifier("this") {
            let _ = self.advance();
            ("this".to_string(), self.index.saturating_sub(1))
        } else {
            let ident = self.consume_identifier("expected member name")?;
            let mut token_index = self.index.saturating_sub(1);
            if self.consume_punctuation('.') {
                explicit_interface = Some(ident);
                if self.peek_identifier("this") {
                    let _ = self.advance();
                    ("this".to_string(), self.index.saturating_sub(1))
                } else {
                    let member = self.consume_identifier(
                        "expected member name after interface qualifier",
                    )?;
                    token_index = self.index.saturating_sub(1);
                    (member, token_index)
                }
            } else {
                (ident, token_index)
            }
        };

        let mut generics = self.parse_generic_parameter_list();
        let has_generics = generics
            .as_ref()
            .is_some_and(|params| !params.is_empty());

        let mut indexer_parameters = Vec::new();
        let mut is_indexer = false;
        if self.check_punctuation('[') {
            indexer_parameters = self.parse_indexer_parameters();
            is_indexer = true;
            if !name.eq_ignore_ascii_case("this") {
                self.push_error(
                    "indexer must be declared as 'this'",
                    self.tokens
                        .get(name_token_index)
                        .map(|token| token.span),
                );
            }
        }

        if !is_indexer && (has_generics || self.check_punctuation('(')) {
            if !self.expect_punctuation('(') {
                return None;
            }
            let (parameters, variadic) = self.parse_parameters();
            if !self.expect_punctuation(')') {
                return None;
            }

        if self.check_punctuation('{') {
            let mut idx = self.index + 1;
            let mut looks_like_accessor = false;
            while let Some(token) = self.tokens.get(idx) {
                if matches!(
                    token.kind,
                    TokenKind::Keyword(Keyword::Get | Keyword::Set | Keyword::Init)
                ) {
                    looks_like_accessor = true;
                    break;
                }
                if matches!(
                    token.kind,
                    TokenKind::Keyword(
                        Keyword::Public
                            | Keyword::Private
                            | Keyword::Protected
                            | Keyword::Internal
                    )
                ) || token
                    .lexeme
                    .eq_ignore_ascii_case("virtual")
                    || token.lexeme.eq_ignore_ascii_case("override")
                    || token.lexeme.eq_ignore_ascii_case("sealed")
                    || token.lexeme.eq_ignore_ascii_case("abstract")
                {
                    idx += 1;
                    continue;
                }
                break;
            }
            if looks_like_accessor {
                self.push_error(
                    "properties cannot declare parameters; use indexers declared as `this[...]`",
                    self.tokens
                        .get(name_token_index)
                        .map(|token| token.span)
                        .or(self.last_span),
                );
            }
        }

        self.parse_where_clauses(&mut generics);
        let throws = self.parse_throws_clause();
        let lends_to_return = self.parse_lends_clause();

            let returns_value = self.type_returns_value(&return_type);
            let body = match self.parse_function_tail(true, returns_value)? {
                FunctionBodyKind::Block(block) => Some(block),
                FunctionBodyKind::Declaration => None,
            };

            if has_required_modifier {
                self.push_error(
                    "`required` modifier is not supported on methods",
                    modifiers.first_required_span(),
                );
            }

            let method_modifiers: Vec<String> = modifiers
                .clone_remaining()
                .into_iter()
                .map(|modifier| modifier.name)
                .collect();
            let is_static_method = method_modifiers
                .iter()
                .any(|modifier| modifier.eq_ignore_ascii_case("static"));
            let type_named_method = name == class_name;
            let name_span = self
                .tokens
                .get(name_token_index)
                .map(|token| token.span)
                .or(self.last_span);
            let mut function = FunctionDecl {
                visibility,
                name,
                name_span,
                signature: Signature {
                parameters,
                return_type,
                lends_to_return,
                throws,
                variadic,
            },
                body,
                is_async,
                is_constexpr,
                doc: doc.take(),
                modifiers: method_modifiers,
                is_unsafe: has_unsafe_modifier,
                attributes: Vec::new(),
                is_extern: false,
                extern_abi: None,
                extern_options: None,
                link_name: None,
                link_library: None,
                operator: None,
                generics,
                vectorize_hint: None,
                dispatch: MemberDispatch::default(),
            };
            if type_named_method {
                let mut diagnostic = Diagnostic::error(
                    format!(
                        "methods cannot use the containing class name `{class_name}`; declare constructors with `init`"
                    ),
                    name_span,
                )
                .with_code(DiagnosticCode::new("E0C02", Some("constructor".into())))
                .with_primary_label("`init` is the only valid constructor name");
                diagnostic.add_suggestion(Suggestion::new(
                    "rename this member or declare it as `init(...)`",
                    name_span,
                    None,
                ));
                self.diagnostics.push(diagnostic);
            }
            function.dispatch = self.build_method_dispatch(
                &modifiers,
                "methods",
                is_static_method,
                is_static_type,
            );
            self.apply_method_attributes(attrs, has_extern_modifier, &mut function);
            return Some(ClassMember::Method(function));
        }

        if self.check_operator("=>") || self.check_punctuation('{') {
            return self.parse_class_property_member(
                visibility,
                name,
                name_token_index,
                return_type,
                indexer_parameters,
                &mut doc,
                is_async,
                has_generics,
                generics.as_ref().and_then(|params| params.span),
                is_constexpr,
                &modifiers,
                &mut attrs,
                is_static_type,
                explicit_interface,
                is_indexer,
            );
        }

        let initializer = if self.check_operator("=") {
            self.advance();
            let expr = self.collect_expression_until(&[';']);
            if expr.span.is_none() && expr.text.is_empty() {
                self.synchronize_field();
                return None;
            }
            Some(expr)
        } else {
            None
        };

        self.finalize_field_member(
            visibility,
            name,
            return_type,
            initializer,
            &mut doc,
            &attrs,
            &modifiers,
            is_static_type,
        )
    }

    fn parse_named_constructor(
        &mut self,
        class_name: &str,
        visibility: Visibility,
        mut modifiers: Vec<Modifier>,
        mut doc: Option<DocComment>,
        attrs: &mut CollectedAttributes,
        is_async: bool,
        is_constexpr: bool,
        extern_span: Option<Span>,
        unsafe_span: Option<Span>,
        has_required_modifier: bool,
    ) -> Option<ClassMember> {
        if is_async {
            self.push_error("constructors cannot be marked `async`", self.last_span);
        }
        if is_constexpr {
            self.push_error(
                "constructors cannot be marked `constexpr`",
                self.last_span,
            );
        }
        if let Some(span) = extern_span {
            self.push_error(
                "`extern` modifier is not supported on constructors",
                Some(span),
            );
        }
        if let Some(span) = unsafe_span {
            self.push_error(
                "`unsafe` modifier is not supported on constructors",
                Some(span),
            );
        }
        if has_required_modifier {
            self.push_error(
                "`required` modifier is not supported on constructors",
                self.last_span,
            );
        }
        if !attrs.is_empty() && !attrs.builtin.is_empty() {
            self.report_attribute_misuse(
                attrs.clone(),
                "unsupported built-in attribute on constructors",
            );
        }
        for modifier in modifiers.drain(..) {
            self.push_error(
                format!("modifier `{}` is not supported on constructors", modifier.name),
                Some(modifier.span),
            );
        }

        let start_offset = self.peek().map(|token| token.span.start);
        let constructor_name = self.consume_identifier("expected constructor name")?;
        let constructor_span = self.last_span.or_else(|| {
            self.tokens
                .get(self.index.saturating_sub(1))
                .map(|tok| tok.span)
        });
        let mut diagnostic = Diagnostic::error(
            format!("constructors must be declared with `init` on `{class_name}`"),
            constructor_span,
        )
        .with_code(DiagnosticCode::new("E0C01", Some("constructor".into())))
        .with_primary_label("type-named constructors are not allowed");
        diagnostic.add_note(format!(
            "replace `{constructor_name}(...)` with `init(...)` inside `{class_name}`"
        ));
        diagnostic.add_suggestion(Suggestion::new(
            "use `init` instead of repeating the type name",
            constructor_span,
            Some("init".to_string()),
        ));
        self.diagnostics.push(diagnostic);
        if constructor_name != class_name {
            self.push_error(
                format!(
                    "constructor name `{constructor_name}` must match enclosing class `{class_name}`"
                ),
                self.last_span,
            );
        }

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
            _ => self.last_span,
        };

        let mut constructor = ConstructorDecl {
            visibility,
            kind: ConstructorKind::Designated,
            parameters,
            body,
            initializer,
            doc: doc.take(),
            span,
            attributes: Vec::new(),
            di_inject: None,
        };
        constructor.attributes = attrs.take_list();
        constructor.di_inject = None;
        Some(ClassMember::Constructor(constructor))
    }
}
