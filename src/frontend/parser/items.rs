use super::attributes::{FunctionAttributeSet, ParsedExternSpec};
use super::*;
use crate::frontend::ast::{ExternBinding, ExternOptions, PackageImport};
use crate::frontend::diagnostics::{Diagnostic, DiagnosticCode};

parser_impl! {
    pub(super) fn parse_module_item(&mut self, module: &mut Module) {
        if self.consume_misplaced_crate_attribute() {
            return;
        }
        self.stash_leading_doc();
        let mut attrs = self.collect_attributes();
        self.stash_leading_doc();

        for (prefix, span) in attrs.take_friend_namespaces() {
            module
                .friend_declarations
                .push(FriendDirective { prefix, span });
        }

        for (name, span) in attrs.take_package_imports() {
            module.package_imports.push(PackageImport { name, span });
        }

        for (header, span) in attrs.take_c_imports() {
            if header.is_empty() {
                self.push_error("`@cimport` attribute requires a header name", span);
                continue;
            }
            if self.module_import_block_closed {
                self.push_error(
                    "import directives must appear before other declarations at file scope",
                    span,
                );
            }
            module.push_item(Item::Import(ImportDirective {
                doc: None,
                is_global: false,
                span,
                kind: ImportKind::CImport { header },
            }));
        }

        if self.handle_end_of_file_attributes(attrs.clone()) {
            return;
        }

        if self.try_parse_file_namespace(module, attrs.clone()) {
            return;
        }

        if self.handle_unexpected_closing_brace() {
            return;
        }

        if self.import_directive_ahead() {
            // LL1_ALLOW: one-token lookahead distinguishes `global import` from other keywords at file scope.
            let is_global_import = {
                self.check_keyword(Keyword::Global)
                    && (
                        // LL1_ALLOW: `global import` lookahead
                        self.peek_keyword_n(1, Keyword::Import)
                            // LL1_ALLOW: `global using` lookahead
                            || self.peek_keyword_n(1, Keyword::Using)
                    )
            };
            let ordering_violation =
                self.module_import_block_closed || (is_global_import && module.namespace.is_some());
            if ordering_violation {
                let span = self.peek().map(|token| token.span);
                let message = if is_global_import {
                    "global import directives must appear at the top of the file, before any namespace or type declarations"
                } else {
                    "import directives must appear before other declarations at file scope"
                };
                self.diagnostics.push(
                    Diagnostic::error(message, span)
                        .with_code(DiagnosticCode::new("E0G01", Some("import".to_string()))),
                );
            }
        }

        match self.parse_item(attrs) {
            Some(item) => {
                if !matches!(item, Item::Import(_)) {
                    self.module_import_block_closed = true;
                }
                module.push_item(item);
            }
            None => self.synchronize_item(),
        }
    }

    fn handle_end_of_file_attributes(&mut self, attrs: CollectedAttributes) -> bool {
        if !self.is_at_end() {
            return false;
        }

        if !attrs.is_empty() {
            self.report_attribute_misuse(
                attrs,
                "attributes are not supported at the end of a file",
            );
        }
        true
    }

    fn try_parse_file_namespace(
        &mut self,
        module: &mut Module,
        attrs: CollectedAttributes,
    ) -> bool {
        if module.namespace.is_some() || !self.check_keyword(Keyword::Namespace) {
            return false;
        }

        let doc = self.take_pending_doc();
        let mut attrs = attrs;
        let attributes = attrs.take_list();
        if !attrs.is_empty() {
            self.report_attribute_misuse(
                attrs,
                "attributes are not supported on file-scoped namespaces",
            );
        }
        if !attributes.is_empty() {
            module.namespace_attributes.extend(attributes);
        }

        if let Some((ns, span)) = self.consume_file_scoped_namespace() {
            self.set_file_namespace(Some(&ns));
            module.namespace = Some(ns);
            module.namespace_span = Some(span);
            if doc.as_ref().is_some_and(|comment| !comment.is_empty()) {
                self.diagnostics.push_warning(
                    "documentation comments on file-scoped namespaces are ignored",
                    self.last_span,
                );
            }
            return true;
        }

        if let Some(doc) = doc {
            self.pending_doc = Some(doc);
        }
        false
    }

    fn handle_unexpected_closing_brace(&mut self) -> bool {
        if !self.check_punctuation('}') {
            return false;
        }

        let span = self.peek().map(|token| token.span);
        self.push_error("unexpected closing brace at namespace scope", span);
        self.advance();
        true
    }

    pub(super) fn parse_item(&mut self, mut attrs: CollectedAttributes) -> Option<Item> {
        self.stash_leading_doc();
        let mut doc = self.take_pending_doc();
        let visibility = self.parse_visibility();
        let mut modifiers = self.consume_modifiers();
        let is_async = Self::take_modifier(&mut modifiers, "async").is_some();
        let is_constexpr = Self::take_modifier(&mut modifiers, "constexpr").is_some();
        let is_extern_modifier = Self::take_modifier(&mut modifiers, "extern").is_some();
        let is_unsafe = Self::take_modifier(&mut modifiers, "unsafe").is_some();
        while let Some(modifier) = Self::take_modifier(&mut modifiers, "required") {
            self.push_error(
                "`required` modifier is not supported on top-level declarations",
                Some(modifier.span),
            );
        }

        // LL1_ALLOW: `const fn` requires one-token lookahead to disambiguate from `const` items.
        if self.check_keyword(Keyword::Const) && self.peek_keyword_n(1, Keyword::Fn) {
            let mut attrs = attrs;
            let function_attrs = attrs.take_function_attributes();
            let surface_attributes = attrs.take_list();
            if !attrs.is_empty() {
                self.report_attribute_misuse(
                    attrs,
                    "attributes are not supported on this declaration",
                );
            }

            let start_index = self.index;
            self.match_keyword(Keyword::Const);
            self.match_keyword(Keyword::Fn);

            let name = self.consume_identifier("expected function name after `const fn`")?;
            let name_span = self.last_span;
            let mut generics = self.parse_generic_parameter_list();

            if !self.expect_punctuation('(') {
                return None;
            }
            let (parameters, variadic) = self.parse_parameters();
            if !self.expect_punctuation(')') {
                return None;
            }

            let return_type = if self.consume_operator("->") {
                self.parse_type_expr()?
            } else {
                TypeExpr::simple("void")
            };

            self.parse_where_clauses(&mut generics);
            let throws = self.parse_throws_clause();
            let lends_to_return = self.parse_lends_clause();
            let returns_value = self.type_returns_value(&return_type);
            let body = match self.parse_function_tail(true, returns_value)? {
                FunctionBodyKind::Block(block) => Some(block),
                FunctionBodyKind::Declaration => None,
            };

            let span = self.span_from_indices(start_index, self.index);
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
                is_constexpr: true,
                doc: doc.take(),
                modifiers: modifiers
                    .iter()
                    .map(|modifier| modifier.name.clone())
                    .collect(),
                is_unsafe,
                attributes: surface_attributes,
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

            self.apply_function_attributes(&mut function, is_extern_modifier, function_attrs);
            if function.body.is_none() {
                self.push_error("`const fn` requires a body", span);
            }
            return Some(Item::Function(function));
        }

        let mut dispatch = ItemDispatch {
            visibility,
            doc: &mut doc,
            modifiers: &mut modifiers,
            is_async,
            is_extern: is_extern_modifier,
        };

        if self.peek().is_some_and(|token| {
            matches!(token.kind, TokenKind::Keyword(Keyword::Trait))
                || matches!(token.kind, TokenKind::Keyword(Keyword::Impl))
                || (matches!(token.kind, TokenKind::Identifier)
                    && (token.lexeme == "trait" || token.lexeme == "impl"))
        }) {
            let span = self.peek().map(|token| token.span);
            let keyword = self.peek().map(|token| token.lexeme.clone()).unwrap_or_default();
            self.advance();
            self.push_error(
                format!(
                    "`{}` is no longer supported; use `interface` and interface implementations instead",
                    keyword
                ),
                span,
            );
            self.synchronize_item();
            return None;
        }

        if let Some(item) = self.parse_item_by_keyword(attrs.clone(), &mut dispatch) {
            return Some(item);
        }

        if !modifiers.is_empty() {
            // Currently modifiers other than async/extern are ignored for free functions but preserved for future work.
        }

        let function_attrs = attrs.take_function_attributes();
        let surface_attributes = attrs.take_list();
        if !attrs.is_empty() {
            self.report_attribute_misuse(
                attrs,
                "attributes are not supported on this declaration",
            );
        }

        let mut function = self.parse_function(visibility, is_async, is_constexpr, doc.take())?;
        function.modifiers = modifiers
            .into_iter()
            .map(|modifier| modifier.name)
            .collect();
        function.is_unsafe = is_unsafe;
        function.attributes = surface_attributes;
        self.apply_function_attributes(&mut function, is_extern_modifier, function_attrs);
        Some(Item::Function(function))
    }

    pub(super) fn apply_function_attributes(
        &mut self,
        function: &mut FunctionDecl,
        is_extern_modifier: bool,
        mut attrs: FunctionAttributeSet,
    ) {
        let is_import = is_extern_modifier;
        let extern_span = attrs.extern_span;
        if let Some(library) = attrs.link_library.take() {
            if is_import {
                function.link_library = Some(library);
            } else {
                self.push_error(
                    "`@link(...)` is only valid on `extern` function declarations",
                    extern_span.or(self.last_span),
                );
            }
        }

        let extern_spec = attrs.extern_spec.take();

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
                    "extern functions may not provide a body",
                    extern_span.or(self.last_span),
                );
                function.body = None;
            }
            function.is_extern = true;
            function.extern_abi = Some(canonical.clone());
            function.extern_options =
                Some(self.build_extern_options(extern_spec, &canonical, extern_span));
        } else if attrs.mark_extern || extern_spec.is_some() {
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
            function.vectorize_hint = attrs.vectorize_hint.take();
        } else {
            attrs.vectorize_hint = None;
        }
    }

    pub(super) fn build_extern_options(
        &mut self,
        spec: Option<ParsedExternSpec>,
        canonical: &str,
        extern_span: Option<Span>,
    ) -> ExternOptions {
        match spec {
            Some(mut spec) => {
                let has_library = spec.library.is_some();
                let binding = if has_library {
                    let requested = spec.binding.unwrap_or(ExternBinding::Lazy);
                    if matches!(requested, ExternBinding::Static) {
                        self.push_error(
                            "`binding=\"static\"` may only be used without a `library`; remove the library argument or choose `lazy`/`eager`",
                            spec.span.or(extern_span),
                        );
                        ExternBinding::Lazy
                    } else {
                        requested
                    }
                } else {
                    if spec.binding.is_some() {
                        self.push_error(
                            "`binding` argument requires a `library` in `@extern` attribute",
                            spec.span.or(extern_span),
                        );
                    }
                    ExternBinding::Static
                };

                if spec.optional.is_some() && !has_library {
                    self.push_error(
                        "`optional` argument requires a `library` in `@extern` attribute",
                        spec.span.or(extern_span),
                    );
                }

                ExternOptions::new(
                    canonical.to_string(),
                    spec.library.take(),
                    spec.alias.take(),
                    binding,
                    spec.optional.unwrap_or(false),
                    spec.charset.take(),
                    spec.span.or(extern_span),
                )
            }
            None => ExternOptions::new(
                canonical.to_string(),
                None,
                None,
                ExternBinding::Static,
                false,
                None,
                extern_span,
            ),
        }
    }
}
