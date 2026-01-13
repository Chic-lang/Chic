use super::*;

parser_impl! {
    pub(super) fn parse_item_by_keyword(
        &mut self,
        attrs: CollectedAttributes,
        ctx: &mut ItemDispatch,
    ) -> Option<Item> {
        self.try_parse_namespace_item(attrs.clone(), ctx)
            .or_else(|| self.try_parse_import_item(attrs.clone(), ctx))
            .or_else(|| self.try_parse_static_item(attrs.clone(), ctx))
            .or_else(|| self.try_parse_const_item(attrs.clone(), ctx))
            .or_else(|| self.try_parse_typealias_item(ctx, attrs.clone()))
            .or_else(|| self.try_parse_struct_item(ctx, attrs.clone()))
            .or_else(|| self.try_parse_union_item(ctx, attrs.clone()))
            .or_else(|| self.try_parse_enum_item(ctx, attrs.clone()))
            .or_else(|| self.try_parse_class_item(ctx, attrs.clone()))
            .or_else(|| self.try_parse_error_item(ctx, attrs.clone()))
            .or_else(|| self.try_parse_interface_item(ctx, attrs.clone()))
            .or_else(|| self.try_parse_delegate_item(ctx, attrs.clone()))
            .or_else(|| self.try_parse_trait_item(ctx, attrs.clone()))
            .or_else(|| self.try_parse_extension_item(attrs.clone(), ctx))
            .or_else(|| self.try_parse_impl_item(ctx, attrs.clone()))
            .or_else(|| self.try_parse_testcase_item(attrs, ctx))
    }

    fn try_parse_namespace_item(
        &mut self,
        attrs: CollectedAttributes,
        ctx: &mut ItemDispatch,
    ) -> Option<Item> {
        let directive_start = self.index;
        if !self.match_keyword(Keyword::Namespace) {
            return None;
        }

        let doc = ctx.doc.take();
        self.parse_namespace(
            ctx.visibility,
            ctx.modifiers.as_slice(),
            doc,
            attrs,
            directive_start,
        )
    }

    fn try_parse_import_item(
        &mut self,
        attrs: CollectedAttributes,
        ctx: &mut ItemDispatch,
    ) -> Option<Item> {
        let directive_start = self.index;
        let (is_global, keyword_span, using_keyword) = self.consume_import_keyword();
        if keyword_span.is_none() && !is_global {
            return None;
        }
        if using_keyword {
            self.emit_using_import_error(keyword_span);
        }

        self.ensure_attributes_absent(attrs, "attributes are not supported on import directives");
        self.parse_import(ctx.doc.take(), is_global, directive_start)
    }

    fn try_parse_static_item(
        &mut self,
        mut attrs: CollectedAttributes,
        ctx: &mut ItemDispatch,
    ) -> Option<Item> {
        let mut static_attrs = attrs.take_static_attributes();
        let static_modifier_index = ctx
            .modifiers
            .iter()
            .position(|m| m.name.eq_ignore_ascii_case("static"));
        let starts_with_keyword = self.check_keyword(Keyword::Static);
        if !starts_with_keyword && static_modifier_index.is_none() {
            return None;
        }
        // LL1_ALLOW: static declarations require a single-token lookahead to distinguish `static const`/`static mut` from other items.
        let next_is_const = if starts_with_keyword {
            // LL1_ALLOW: lookahead for `const` after `static`.
            self.peek_keyword_n(1, Keyword::Const)
        } else {
            self.check_keyword(Keyword::Const)
        };
        // LL1_ALLOW: see above — lookahead keeps the grammar LL(1) for `static mut`.
        let next_is_mut = if starts_with_keyword {
            // LL1_ALLOW: `static mut` lookahead
            self.peek_keyword_n(1, Keyword::Mut)
        } else {
            self.check_keyword(Keyword::Mut)
        };
        if !next_is_const && !next_is_mut {
            return None;
        }

        if let Some(index) = static_modifier_index {
            ctx.modifiers.remove(index);
        }

        if ctx.is_async {
            let span = self.last_span;
            self.push_error("`async` modifier is not supported on static declarations", span);
        }
        if !ctx.modifiers.is_empty() {
            for modifier in ctx.modifiers.drain(..) {
                self.push_error(
                    format!("modifier `{}` is not supported on static declarations", modifier.name),
                    Some(modifier.span),
                );
            }
        }

        let attrs_for_misuse = attrs.clone();
        let attributes = attrs.take_list();
        if !attrs_for_misuse.is_empty() {
            self.report_attribute_misuse(
                attrs_for_misuse,
                "unsupported attributes on static declarations",
            );
        }

        let start = self.peek().map(|token| token.span.start);
        if starts_with_keyword {
            self.match_keyword(Keyword::Static);
        }
        let mutability = if self.match_keyword(Keyword::Mut) {
            StaticMutability::Mutable
        } else {
            self.match_keyword(Keyword::Const);
            StaticMutability::Const
        };

        let extern_span = static_attrs.extern_span;
        let extern_spec = static_attrs.extern_spec.take();
        let mut extern_abi = None;
        let mut extern_options = None;
        let mut link_library = None;
        if ctx.is_extern {
            let abi = extern_spec
                .as_ref()
                .and_then(|spec| spec.convention.clone())
                .unwrap_or_else(|| "C".to_string());
            let canonical = if abi.eq_ignore_ascii_case("c") {
                "C".to_string()
            } else {
                abi.to_ascii_lowercase()
            };
            extern_abi = Some(canonical.clone());
            extern_options = Some(self.build_extern_options(extern_spec, &canonical, extern_span));
            if let Some(library) = static_attrs.link_library.take() {
                link_library = Some(library);
            }
        } else {
            if static_attrs.mark_extern || extern_spec.is_some() {
                self.push_error(
                    "`@extern` is only valid on `extern static` declarations",
                    extern_span.or(self.last_span),
                );
            }
            if static_attrs.link_library.is_some() {
                self.push_error(
                    "`@link(...)` is only valid on `extern static` declarations",
                    extern_span.or(self.last_span),
                );
            }
        }

        let doc = ctx.doc.take();
        let mut is_weak_import = attributes.iter().any(|attr| {
            let mut name = attr.name.to_ascii_lowercase();
            name.retain(|ch| ch != '_' && ch != '-');
            name == "weakimport"
        });
        if is_weak_import && !ctx.is_extern {
            self.push_error(
                "`@weak_import` is only valid on `extern static` declarations",
                extern_span.or(self.last_span),
            );
            is_weak_import = false;
        }
        if is_weak_import
            && attributes.iter().any(|attr| {
                let mut name = attr.name.to_ascii_lowercase();
                name.retain(|ch| ch != '_' && ch != '-');
                name == "weak"
            })
        {
            self.push_error(
                "[MIRL0450] `@weak` and `@weak_import` cannot be combined on the same declaration",
                extern_span.or(self.last_span),
            );
            is_weak_import = false;
        }

        let mut declaration = self.parse_static_declaration(doc, attributes, mutability)?;
        declaration.is_extern = ctx.is_extern;
        declaration.extern_abi = extern_abi;
        declaration.extern_options = extern_options;
        declaration.link_library = link_library;
        declaration.is_weak_import = is_weak_import;
        if declaration.is_extern {
            let has_initializer = declaration
                .declarators
                .iter()
                .any(|decl| decl.initializer.is_some());
            let missing_initializer = declaration
                .declarators
                .iter()
                .any(|decl| decl.initializer.is_none());
            if has_initializer && missing_initializer {
                self.push_error(
                    "extern static declarations must consistently either import (no initializer) or export (initializer present)",
                    extern_span.or(self.last_span),
                );
            }
            if declaration.is_weak_import && has_initializer {
                self.push_error(
                    "[MIRL0452] `@weak_import` declarations must not provide an initializer",
                    extern_span.or(self.last_span),
                );
                declaration.is_weak_import = false;
            }
            if declaration.declarators.len() > 1
                && declaration
                    .extern_options
                    .as_ref()
                    .and_then(|opts| opts.alias.as_ref())
                    .is_some()
            {
                self.push_error(
                    "`@extern(alias = ...)` requires exactly one static declarator",
                    extern_span.or(self.last_span),
                );
            }
        }
        if !self.expect_punctuation(';') {
            return None;
        }
        declaration.span = self.make_span(start);
        Some(Item::Static(StaticItemDecl {
            visibility: ctx.visibility,
            declaration,
        }))
    }

    fn try_parse_const_item(
        &mut self,
        attrs: CollectedAttributes,
        ctx: &mut ItemDispatch,
    ) -> Option<Item> {
        if !self.check_keyword(Keyword::Const) {
            return None;
        }

        self.ensure_attributes_absent(attrs, "attributes are not supported on const declarations");
        if ctx.is_async {
            let span = self.last_span;
            self.push_error(
                "`async` modifier is not supported on const declarations",
                span,
                            );
        }
        if !ctx.modifiers.is_empty() {
            for modifier in ctx.modifiers.drain(..) {
                self.push_error(
                    format!(
                        "modifier `{}` is not supported on const declarations",
                        modifier.name
                    ),
                    Some(modifier.span),
                );
            }
        }

        let start = self.peek().map(|token| token.span.start);
        self.match_keyword(Keyword::Const);
        let doc = ctx.doc.take();
        let mut declaration = self.parse_const_declaration_body(doc, ';')?;
        if !self.expect_punctuation(';') {
            return None;
        }
        declaration.span = self.make_span(start);
        Some(Item::Const(ConstItemDecl {
            visibility: ctx.visibility,
            declaration,
        }))
    }

    fn try_parse_typealias_item(
        &mut self,
        ctx: &mut ItemDispatch,
        attrs: CollectedAttributes,
    ) -> Option<Item> {
        if !self.match_keyword(Keyword::Typealias) {
            return None;
        }
        if ctx.is_async {
            self.push_error(
                "`async` modifier is not supported on type aliases",
                self.last_span,
            );
        }
        if ctx.is_extern {
            self.push_error(
                "`extern` modifier is not supported on type aliases",
                self.last_span,
            );
        }
        let doc = ctx.doc.take();
        self.parse_type_alias(ctx.visibility, doc, attrs, ctx.modifiers.as_slice())
    }

    fn parse_static_declaration(
        &mut self,
        doc: Option<DocComment>,
        attributes: Vec<Attribute>,
        mutability: StaticMutability,
    ) -> Option<StaticDeclaration> {
        let ty = self.parse_type_expr()?;
        let mut declarators = Vec::new();

        loop {
            let name = self.consume_identifier("expected static name")?;
            let name_span = self.last_span;
            let initializer = if self.consume_operator("=") {
                Some(self.collect_expression_until(&[',', ';']))
            } else {
                None
            };
            let span = match (name_span, initializer.as_ref().and_then(|expr| expr.span)) {
                (Some(name_span), Some(expr_span)) if expr_span.end >= name_span.start => {
                    Some(Span::in_file(
                        name_span.file_id,
                        name_span.start,
                        expr_span.end,
                    ))
                }
                _ => name_span,
            };
            declarators.push(StaticDeclarator {
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

        Some(StaticDeclaration {
            mutability,
            ty,
            declarators,
            attributes,
            is_extern: false,
            extern_abi: None,
            extern_options: None,
            link_library: None,
            is_weak_import: false,
            doc,
            span: None,
        })
    }

    fn try_parse_struct_item(
        &mut self,
        ctx: &mut ItemDispatch,
        attrs: CollectedAttributes,
    ) -> Option<Item> {
        let mut is_record = false;
        // LL1_ALLOW: `record struct` uses contextual sugar that relies on a single-token lookahead to avoid adding a dedicated keyword (docs/compiler/parser.md#ll1-allowances).
        if self.peek_identifier("record") {
            is_record = true;
            self.advance();
            let _ = self.match_keyword(Keyword::Struct);
        } else if !self.match_keyword(Keyword::Struct) {
            return None;
        }
        let doc = ctx.doc.take();
        self.parse_struct(
            ctx.visibility,
            doc,
            attrs,
            ctx.modifiers.as_slice(),
            is_record,
        )
    }

    fn try_parse_union_item(
        &mut self,
        ctx: &mut ItemDispatch,
        attrs: CollectedAttributes,
    ) -> Option<Item> {
        if !self.match_keyword(Keyword::Union) {
            return None;
        }
        let doc = ctx.doc.take();
        self.parse_union(ctx.visibility, doc, attrs)
    }

    fn try_parse_enum_item(
        &mut self,
        ctx: &mut ItemDispatch,
        attrs: CollectedAttributes,
    ) -> Option<Item> {
        if !self.match_keyword(Keyword::Enum) {
            return None;
        }
        let doc = ctx.doc.take();
        self.parse_enum(ctx.visibility, doc, attrs)
    }

    fn try_parse_class_item(
        &mut self,
        ctx: &mut ItemDispatch,
        attrs: CollectedAttributes,
    ) -> Option<Item> {
        if !self.match_keyword(Keyword::Class) {
            return None;
        }
        self.reject_async_class(ctx.is_async);
        self.reject_class_modifiers(ctx.modifiers.as_slice());
        let doc = ctx.doc.take();
        self.parse_class(ctx.visibility, ctx.modifiers.as_slice(), doc, attrs)
    }

    fn try_parse_error_item(
        &mut self,
        ctx: &mut ItemDispatch,
        attrs: CollectedAttributes,
    ) -> Option<Item> {
        let is_error = if self.match_keyword(Keyword::Error) {
            true
        } else if self.peek_identifier("error") {
            self.advance();
            true
        } else {
            false
        };
        if !is_error {
            return None;
        }
        self.reject_async_class(ctx.is_async);
        self.reject_class_modifiers(ctx.modifiers.as_slice());
        let doc = ctx.doc.take();
        self.parse_error(ctx.visibility, ctx.modifiers.as_slice(), doc, attrs)
    }

    fn try_parse_interface_item(
        &mut self,
        ctx: &mut ItemDispatch,
        attrs: CollectedAttributes,
    ) -> Option<Item> {
        if !self.match_keyword(Keyword::Interface) {
            return None;
        }
        let doc = ctx.doc.take();
        self.parse_interface(ctx.visibility, doc, attrs)
    }

    fn try_parse_delegate_item(
        &mut self,
        ctx: &mut ItemDispatch,
        attrs: CollectedAttributes,
    ) -> Option<Item> {
        if !self.match_keyword(Keyword::Delegate) {
            return None;
        }

        let doc = ctx.doc.take();
        self.parse_delegate(ctx.visibility, doc, attrs, ctx.modifiers.as_slice())
    }

    fn try_parse_trait_item(
        &mut self,
        ctx: &mut ItemDispatch,
        attrs: CollectedAttributes,
    ) -> Option<Item> {
        if !self.match_keyword(Keyword::Trait) {
            return None;
        }
        self.reject_async_trait(ctx.is_async);
        self.reject_trait_modifiers(ctx.modifiers.as_slice());
        let doc = ctx.doc.take();
        self.parse_trait(ctx.visibility, doc, attrs)
    }

    fn try_parse_extension_item(
        &mut self,
        attrs: CollectedAttributes,
        ctx: &mut ItemDispatch,
    ) -> Option<Item> {
        if !self.match_keyword(Keyword::Extension) {
            return None;
        }

        self.ensure_attributes_absent(
            attrs,
            "attributes are not supported on extension declarations",
        );
        let doc = ctx.doc.take();
        self.parse_extension(ctx.visibility, doc)
    }

    fn try_parse_impl_item(
        &mut self,
        ctx: &mut ItemDispatch,
        attrs: CollectedAttributes,
    ) -> Option<Item> {
        if !self.check_keyword(Keyword::Impl) {
            return None;
        }
        let mut cursor = self.index;
        while let Some(token) = self.tokens.get(cursor) {
            match token.kind {
                TokenKind::Whitespace | TokenKind::Comment => {
                    cursor += 1;
                }
                TokenKind::Keyword(Keyword::Impl) => {
                    cursor += 1;
                }
                TokenKind::Punctuation('(') => {
                    // `impl Trait Func(...)` – treat as a function rather than an `impl` block.
                    return None;
                }
                TokenKind::Punctuation('{') => break,
                _ => cursor += 1,
            }
        }
        self.advance(); // consume `impl`
        self.reject_async_impl(ctx.is_async);
        self.reject_impl_modifiers(ctx.modifiers.as_slice());
        let doc = ctx.doc.take();
        self.parse_impl(ctx.visibility, doc, attrs)
    }

    fn try_parse_testcase_item(
        &mut self,
        mut attrs: CollectedAttributes,
        ctx: &mut ItemDispatch,
    ) -> Option<Item> {
        if !self.match_keyword(Keyword::Testcase) {
            return None;
        }

        let attributes = attrs.take_list();
        let doc = ctx.doc.take();
        self.parse_testcase(ctx.is_async, doc, attributes)
    }

    fn ensure_attributes_absent(&mut self, attrs: CollectedAttributes, message: &str) {
        if !attrs.is_empty() {
            self.report_attribute_misuse(attrs, message);
        }
    }

    fn reject_async_class(&mut self, is_async: bool) {
        if !is_async {
            return;
        }
        let span = self.last_span;
        self.push_error(
            "`async` modifier is not supported on class declarations",
            span,
                    );
    }

    fn reject_class_modifiers(&mut self, modifiers: &[Modifier]) {
        if modifiers.is_empty() {
            return;
        }

        const ALLOWED: &[&str] = &["abstract", "sealed", "partial", "static"];
        let unsupported: Vec<&str> = modifiers
            .iter()
            .map(|modifier| modifier.name.as_str())
            .filter(|name| !ALLOWED.contains(name))
            .collect();

        if unsupported.is_empty() {
            return;
        }

        for modifier in modifiers {
            if !ALLOWED.contains(&modifier.name.as_str()) {
                self.push_error(
                    format!(
                        "class declarations do not support modifier `{}`",
                        modifier.name
                    ),
                    Some(modifier.span),
                );
            }
        }
    }

    fn reject_async_trait(&mut self, is_async: bool) {
        if !is_async {
            return;
        }
        let span = self.last_span;
        self.push_error(
            "`async` modifier is not supported on trait declarations",
            span,
        );
    }

    fn reject_trait_modifiers(&mut self, modifiers: &[Modifier]) {
        for modifier in modifiers {
            self.push_error(
                format!(
                    "trait declarations do not support modifier `{}`",
                    modifier.name
                ),
                Some(modifier.span),
            );
        }
    }

    fn reject_async_impl(&mut self, is_async: bool) {
        if !is_async {
            return;
        }
        let span = self.last_span;
        self.push_error(
            "`async` modifier is not supported on impl blocks",
            span,
        );
    }

    fn reject_impl_modifiers(&mut self, modifiers: &[Modifier]) {
        for modifier in modifiers {
            self.push_error(
                format!("impl blocks do not support modifier `{}`", modifier.name),
                Some(modifier.span),
            );
        }
    }
}
