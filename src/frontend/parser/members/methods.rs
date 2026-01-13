use super::{
    operators::{
        OperatorTokenKind, canonical_conversion_name, canonical_operator_name, operator_token_kind,
    },
    *,
};

#[derive(Clone, Copy)]
pub(crate) enum OperatorOwner {
    Class,
    Extension,
    Struct,
    Interface,
}

parser_impl! {
    pub(crate) fn apply_method_attributes(
        &mut self,
        attrs: CollectedAttributes,
        is_extern_modifier: bool,
        function: &mut FunctionDecl,
    ) {
        let mut attrs = attrs;
        let mut function_attrs = attrs.take_function_attributes();
        let is_import = is_extern_modifier;

        if let Some(library) = function_attrs.link_library.take() {
            if is_import {
                function.link_library = Some(library);
            } else {
                self.push_error(
                    "`@link(...)` is only valid on `extern` method declarations",
                    function_attrs.extern_span.or(self.last_span),
                );
            }
        }

        let extern_span = function_attrs.extern_span;
        let extern_spec = function_attrs.extern_spec.take();

        let abi = extern_spec
            .as_ref()
            .and_then(|spec| spec.convention.clone())
            .unwrap_or_else(|| "C".to_string());
        let canonical = if abi.eq_ignore_ascii_case("c") {
            "C".to_string()
        } else {
            abi.to_ascii_lowercase()
        };

        if is_import {
            if function.body.is_some() {
                self.push_error(
                    "extern methods may not provide a body",
                    extern_span.or(self.last_span),
                );
                function.body = None;
            }
            function.is_extern = true;
            function.extern_abi = Some(canonical.clone());
            function.extern_options =
                Some(self.build_extern_options(extern_spec, &canonical, extern_span));
        } else if function_attrs.mark_extern || extern_spec.is_some() {
            if let Some(spec) = extern_spec.as_ref() {
                if spec.library.is_some()
                    || spec.alias.is_some()
                    || spec.binding.is_some()
                    || spec.optional.is_some()
                    || spec.charset.is_some()
                {
                    self.push_error(
                        "`@extern` metadata (library/alias/binding/optional/charset) is only valid on `extern` declarations; definitions may only specify the convention",
                        spec.span.or(extern_span).or(self.last_span),
                    );
                }
            }
            function.extern_abi = Some(canonical);
        }

        if function.vectorize_hint.is_none() {
            function.vectorize_hint = function_attrs.vectorize_hint.take();
        } else {
            function_attrs.vectorize_hint = None;
        }

        let surface_attributes = attrs.take_list();
        if !attrs.is_empty() {
            self.report_attribute_misuse(
                attrs,
                "unsupported attributes applied to class methods",
            );
        }
        function.attributes = surface_attributes;

        self.validate_deterministic_destructor_hook(function);
    }

    fn validate_deterministic_destructor_hook(&mut self, function: &FunctionDecl) {
        if function.name == "deinit" {
            let mut diagnostic = Diagnostic::error(
                "`deinit` is forbidden; use `dispose`",
                function.name_span,
            )
            .with_code(DiagnosticCode::new(
                "DISPOSE0001",
                Some("lint".into()),
            ))
            .with_primary_label("rename this destructor hook to `dispose`");
            diagnostic.add_suggestion(Suggestion::new(
                "replace `deinit` with `dispose`",
                function.name_span,
                Some("dispose".into()),
            ));
            self.diagnostics.push(diagnostic);
            return;
        }

        if function.name != "dispose" {
            return;
        }

        let mut valid = true;

        if function
            .modifiers
            .iter()
            .any(|modifier| modifier.eq_ignore_ascii_case("static"))
        {
            valid = false;
        }
        if function.is_extern || function.extern_abi.is_some() {
            valid = false;
        }
        if function.signature.variadic {
            valid = false;
        }

        let is_void_return = function
            .signature
            .return_type
            .name
            .eq_ignore_ascii_case("void");
        if !is_void_return {
            valid = false;
        }

        let receiver_ok = function.signature.parameters.len() == 1
            && {
                let param = &function.signature.parameters[0];
                matches!(param.binding, BindingModifier::Ref)
                    && (param.is_extension_this
                        || param.name.eq_ignore_ascii_case("this")
                        || param.name.eq_ignore_ascii_case("self"))
            };
        if !receiver_ok {
            valid = false;
        }

        if valid {
            return;
        }

        let mut diagnostic = Diagnostic::error(
            "invalid `dispose` signature; expected `dispose(ref this)` returning void",
            function.name_span,
        )
        .with_code(DiagnosticCode::new(
            "DISPOSE0002",
            Some("typeck".into()),
        ))
        .with_primary_label("`dispose` is a deterministic destructor hook");
        diagnostic.add_suggestion(Suggestion::new(
            "change this declaration to `dispose(ref this)` returning void",
            function.name_span,
            None,
        ));
        self.diagnostics.push(diagnostic);
    }

    pub(crate) fn parse_conversion_operator_member(
        &mut self,
        visibility: Visibility,
        is_async: bool,
        doc: Option<DocComment>,
        mut modifiers: Vec<Modifier>,
        has_unsafe_modifier: bool,
        _owner: OperatorOwner,
    ) -> Option<FunctionDecl> {
        let keyword = self.advance()?;
        let span = Some(keyword.span);
        let kind = match keyword.kind {
            TokenKind::Keyword(Keyword::Implicit) => ConversionKind::Implicit,
            TokenKind::Keyword(Keyword::Explicit) => ConversionKind::Explicit,
            _ => {
                self.push_error("expected `implicit` or `explicit` conversion modifier", span);
                return None;
            }
        };

        self.enforce_operator_modifiers(&mut modifiers, span);
        self.report_operator_async(is_async, span);

        if !self.match_keyword(Keyword::Operator) {
            let error_span = self.peek().map(|token| token.span).or(span);
            self.push_error("expected `operator` keyword after conversion modifier", error_span);
            return None;
        }

        let target_type = self.parse_type_expr()?;
        let mut generics = self.parse_generic_parameter_list();
        let has_generics = generics
            .as_ref()
            .is_some_and(|params| !params.is_empty());
        if has_generics {
            self.push_error(
                "operator overloads cannot declare generic parameters",
                span,
                            );
        }

        if !self.expect_punctuation('(') {
            return None;
        }
        let (parameters, variadic) = self.parse_parameters();
        if !self.expect_punctuation(')') {
            return None;
        }

        if parameters.len() != 1 {
            self.push_error(
                "conversion operators must declare exactly one parameter",
                span,
                            );
        }
        if variadic {
            self.push_error("conversion operators cannot be variadic", span);
        }

        self.parse_where_clauses(&mut generics);
        let throws = self.parse_throws_clause();
        let lends_to_return = self.parse_lends_clause();

        let returns_value = self.type_returns_value(&target_type);
        let body = match self.parse_function_tail(true, returns_value)? {
            FunctionBodyKind::Block(block) => Some(block),
            FunctionBodyKind::Declaration => None,
        };

        let signature = Signature {
            parameters,
            return_type: target_type.clone(),
            lends_to_return,
            throws,
            variadic,
        };
        let canonical_name =
            canonical_conversion_name(kind, &target_type.name);

            Some(FunctionDecl {
                visibility,
                name: canonical_name,
                name_span: span,
                signature,
                body,
                is_async,
                is_constexpr: false,
                doc,
                modifiers: modifiers
                    .iter()
                    .map(|modifier| modifier.name.clone())
                    .collect(),
                is_unsafe: has_unsafe_modifier,
                attributes: Vec::new(),
                is_extern: false,
                extern_abi: None,
                extern_options: None,
            link_name: None,
            link_library: None,
            operator: Some(OperatorDecl {
                kind: OperatorKind::Conversion(kind),
                span,
                            }),
            generics,
            vectorize_hint: None,
            dispatch: MemberDispatch::default(),
        })
    }

    pub(crate) fn parse_symbol_operator_member(
        &mut self,
        visibility: Visibility,
        is_async: bool,
        doc: Option<DocComment>,
        mut modifiers: Vec<Modifier>,
        return_type: TypeExpr,
        has_unsafe_modifier: bool,
        _owner: OperatorOwner,
    ) -> Option<FunctionDecl> {
        let operator_keyword = self.advance()?;
        let span = Some(operator_keyword.span);

        let Some(symbol_token) = self.advance() else {
            self.push_error("expected operator symbol after `operator`", span);
            return None;
        };
        let operator_kind_token = match operator_token_kind(&symbol_token) {
            Some(kind) => kind,
            None => {
                self.push_error(
                    format!("`{}` is not a supported overloadable operator", symbol_token.lexeme),
                    Some(symbol_token.span),
                );
                return None;
            }
        };

        self.enforce_operator_modifiers(&mut modifiers, span);
        self.report_operator_async(is_async, span);

        let mut generics = self.parse_generic_parameter_list();
        let has_generics = generics
            .as_ref()
            .is_some_and(|params| !params.is_empty());
        if has_generics {
            self.push_error(
                "operator overloads cannot declare generic parameters",
                span,
                            );
        }

        if !self.expect_punctuation('(') {
            return None;
        }
        let (parameters, variadic) = self.parse_parameters();
        if !self.expect_punctuation(')') {
            return None;
        }
        if variadic {
            self.push_error("operator overloads cannot be variadic", span);
        }

        self.parse_where_clauses(&mut generics);
        let throws = self.parse_throws_clause();
        let lends_to_return = self.parse_lends_clause();

        let operator_kind = match (operator_kind_token, parameters.len()) {
            (OperatorTokenKind::Unary(op), 1) => OperatorKind::Unary(op),
            (OperatorTokenKind::Binary(op), 2) => OperatorKind::Binary(op),
            (OperatorTokenKind::UnaryOrBinary { unary, .. }, 1) => OperatorKind::Unary(unary),
            (OperatorTokenKind::UnaryOrBinary { binary, .. }, 2) => OperatorKind::Binary(binary),
            (OperatorTokenKind::Unary(_), _) => {
                self.push_error(
                    "unary operator overloads must declare exactly one parameter",
                    span,
                                    );
                return None;
            }
            (OperatorTokenKind::Binary(_), _) => {
                self.push_error(
                    "binary operator overloads must declare exactly two parameters",
                    span,
                                    );
                return None;
            }
            (OperatorTokenKind::UnaryOrBinary { .. }, _) => {
                self.push_error(
                    "this operator requires one or two parameters to determine its arity",
                    span,
                                    );
                return None;
            }
        };

        let returns_value = self.type_returns_value(&return_type);
        let body = match self.parse_function_tail(true, returns_value)? {
            FunctionBodyKind::Block(block) => Some(block),
            FunctionBodyKind::Declaration => None,
        };

        let canonical_name = canonical_operator_name(&operator_kind, &return_type);
            Some(FunctionDecl {
                visibility,
                name: canonical_name,
                name_span: span,
                signature: Signature {
                    parameters,
                    return_type,
                    lends_to_return,
                    throws,
                    variadic,
                },
                body,
                is_async,
                is_constexpr: false,
                doc,
                modifiers: modifiers
                    .iter()
                    .map(|modifier| modifier.name.clone())
                    .collect(),
                is_unsafe: has_unsafe_modifier,
                attributes: Vec::new(),
                is_extern: false,
                extern_abi: None,
                extern_options: None,
            link_name: None,
            link_library: None,
            operator: Some(OperatorDecl {
                kind: operator_kind,
                span,
                            }),
            generics,
            vectorize_hint: None,
            dispatch: MemberDispatch::default(),
        })
    }

    pub(in crate::frontend::parser) fn parse_function_tail(
        &mut self,
        allow_body: bool,
        expression_returns_value: bool,
    ) -> Option<FunctionBodyKind> {
        if self.consume_punctuation(';') {
            return Some(FunctionBodyKind::Declaration);
        }

        if allow_body {
            if self.check_punctuation('{') {
                return self.parse_block().map(FunctionBodyKind::Block);
            }
            if self.check_operator("=>") {
                let arrow_index = self.index;
                let arrow_span = self.peek().map(|token| token.span);
                self.advance();
                let expression = self.collect_expression_until(&[';']);
                if expression.span.is_none() && expression.text.trim().is_empty() {
                    self.push_error(
                        "expression-bodied members require an expression before ';'",
                        arrow_span,
                    );
                    if !self.expect_punctuation(';') {
                        return None;
                    }
                    return None;
                }
                if !self.expect_punctuation(';') {
                    return None;
                }
                let statement = if expression_returns_value {
                    Statement::new(
                        expression.span,
                        StatementKind::Return {
                            expression: Some(expression),
                        },
                    )
                } else {
                    Statement::new(expression.span, StatementKind::Expression(expression))
                };
                let span = self
                    .span_from_indices(arrow_index, self.index)
                    .or(arrow_span);
                let block = Block {
                    statements: vec![statement],
                    span,
                };
                return Some(FunctionBodyKind::Block(block));
            }
            let span = self.peek().map(|token| token.span);
            self.push_error(
                "expected function body block `{ ... }` or ';' after parameter list",
                span,
                            );
            None
        } else {
            let span = self.peek().map(|token| token.span);
            self.push_error("expected ';' after declaration", span);
            None
        }
    }

    fn enforce_operator_modifiers(&mut self, modifiers: &mut Vec<Modifier>, span: Option<Span>) {
        let mut has_static = false;
        for modifier in modifiers.iter() {
            if modifier.name.eq_ignore_ascii_case("static") {
                has_static = true;
                break;
            }
        }
        if !has_static {
            self.push_error("operator overloads must be declared `static`", span);
        }

        let mut kept = Vec::new();
        for modifier in modifiers.drain(..) {
            if modifier.name.eq_ignore_ascii_case("static") {
                kept.push(modifier);
                continue;
            }
            self.push_error(
                format!(
                    "modifier `{}` is not supported on operator overloads",
                    modifier.name
                ),
                Some(modifier.span),
            );
        }
        *modifiers = kept;
    }

    fn report_operator_async(&mut self, is_async: bool, span: Option<Span>) {
        if is_async {
            self.push_error("operator overloads cannot be marked `async`", span);
        }
    }
}
