use super::helpers::{
    CommonTypeAttributes, reject_flags_for_non_enum, reject_pin_for_type,
    take_common_type_attributes,
};
use crate::frontend::ast::items::RecordPositionalField;
use crate::frontend::ast::{Block, Expression, Statement, StatementKind};
use crate::frontend::parser::members::MemberModifiers;
use crate::frontend::parser::members::OperatorOwner;
use crate::frontend::parser::properties::PropertyContext;
use crate::frontend::parser::*;

parser_impl! {
    pub(crate) fn parse_struct(
        &mut self,
        visibility: Visibility,
        doc: Option<DocComment>,
        mut attrs: CollectedAttributes,
        modifiers: &[Modifier],
        is_record: bool,
    ) -> Option<Item> {
        reject_pin_for_type(self, &mut attrs);
        reject_flags_for_non_enum(self, &mut attrs);
        let mut is_readonly = is_record;
        for modifier in modifiers {
            if modifier.name.eq_ignore_ascii_case("readonly") {
                if is_readonly {
                    self.push_error(
                        "duplicate `readonly` modifier on struct declarations",
                        Some(modifier.span),
                    );
                }
                is_readonly = true;
            } else {
                self.push_error(
                    format!(
                        "modifier `{}` is not supported on struct declarations",
                        modifier.name
                    ),
                    Some(modifier.span),
                );
            }
        }
        let name = self.consume_identifier("expected struct name")?;
        let mut generics = self.parse_generic_parameter_list();

        let mut record_params = Vec::new();
        if is_record && self.check_punctuation('(') {
            self.advance();
            let parsed = self.parse_parameters();
            record_params = parsed.0;
            if parsed.1 {
                self.push_error(
                    "record primary constructors cannot be variadic",
                    self.last_span,
                );
            }
            if !self.expect_punctuation(')') {
                return None;
            }
        }

        let bases = if self.consume_punctuation(':') {
            self.parse_type_list()
        } else {
            Vec::new()
        };

        self.parse_where_clauses(&mut generics);

        let mut record_positional_fields = Vec::new();
        let mut fields = Vec::new();
        if is_record {
            for param in &record_params {
                if param.is_extension_this {
                    self.push_error(
                        "`this` parameter is not valid in record primary constructors",
                        param
                            .attributes
                            .first()
                            .and_then(|attr| attr.span)
                            .or(self.last_span),
                    );
                }
                if !matches!(param.binding, BindingModifier::Value | BindingModifier::In) {
                    self.push_error(
                        "record primary constructor parameters must use value or `in` binding",
                        param
                            .attributes
                            .first()
                            .and_then(|attr| attr.span)
                            .or(self.last_span),
                    );
                }
                record_positional_fields.push(RecordPositionalField {
                    name: param.name.clone(),
                    span: param
                        .attributes
                        .first()
                        .and_then(|attr| attr.span)
                        .or(self.last_span),
                });
                fields.push(FieldDecl {
                    visibility,
                    name: param.name.clone(),
                    ty: param.ty.clone(),
                    initializer: None,
                    mmio: None,
                    doc: None,
                    is_required: true,
                    display_name: None,
                    attributes: param.attributes.clone(),
                    is_readonly: true,
                    is_static: false,
                    view_of: None,
                });
            }
        }

        let mut properties = Vec::new();
        let mut constructors = Vec::new();
        let mut consts = Vec::new();
        let mut methods = Vec::new();
        let mut nested_types = Vec::new();
        let struct_mmio_attr = attrs.builtin.mmio_struct.clone();

        let finalize_record = |fields: &mut Vec<FieldDecl>,
                               constructors: &mut Vec<ConstructorDecl>| {
            if !is_record {
                return;
            }
            for field in fields.iter_mut() {
                if !field.is_static {
                    field.is_readonly = true;
                }
            }
            if constructors.is_empty() && !record_params.is_empty() {
                let mut statements = Vec::new();
                for param in &record_params {
                    let text = format!("self.{} = {}", param.name, param.name);
                    statements.push(Statement::new(
                        None,
                        StatementKind::Expression(Expression::new(text, None)),
                    ));
                }
                constructors.push(ConstructorDecl {
                    visibility,
                    kind: ConstructorKind::Designated,
                    parameters: record_params.clone(),
                    body: Some(Block {
                        statements,
                        span: None,
                    }),
                    initializer: None,
                    doc: None,
                    span: None,
                    attributes: Vec::new(),
                    di_inject: None,
                });
            }
        };

        if is_record && self.consume_punctuation(';') {
            finalize_record(&mut fields, &mut constructors);
            let layout_hints = attrs.builtin.struct_layout.clone();
            let is_intrinsic = attrs.builtin.intrinsic;
            let inline_attr = attrs.builtin.inline_attr;
            let CommonTypeAttributes {
                thread_safe_override,
                shareable_override,
                copy_override,
                attributes,
            } = take_common_type_attributes(&mut attrs);

            return Some(Item::Struct(StructDecl {
                visibility,
                name,
                fields,
                properties,
                constructors,
                consts,
                methods,
                nested_types,
                bases,
                thread_safe_override,
                shareable_override,
                copy_override,
                mmio: attrs.builtin.mmio_struct.clone(),
                doc,
                generics,
                attributes,
                is_readonly,
                layout: layout_hints,
                is_intrinsic,
                inline_attr,
                is_record,
                record_positional_fields,
            }));
        }

        if !self.expect_punctuation('{') {
            return None;
        }
        while !self.is_at_end() && !self.check_punctuation('}') {
            self.stash_leading_doc();
            if self.check_punctuation('}') {
                break;
            }

            let mut member_attrs = self.collect_attributes();
            self.stash_leading_doc();
            if self.check_punctuation('}') {
                if !member_attrs.is_empty() {
                    self.report_attribute_misuse(
                        member_attrs,
                        "attributes are not supported here",
                    );
                }
                break;
            }

            let mut member_doc = self.take_pending_doc();
            let member_visibility = self.parse_visibility();
            let modifiers = MemberModifiers::new(self.consume_modifiers());
            let required_span = modifiers.first_required_span();
            let is_required = modifiers.has_required();
            let is_async_member = modifiers.async_modifier.is_some();
            let is_constexpr_member = modifiers.constexpr_modifier.is_some();
            let delegate_modifier = if self.check_keyword(Keyword::Delegate) {
                let span = self.peek().map(|token| token.span);
                self.advance();
                span
            } else {
                None
            };

            let is_record_struct =
                // LL1_ALLOW: Nested `record struct` declarations are gated by the same contextual lookahead used at the item level (docs/compiler/parser.md#ll1-allowances).
                self.peek_identifier("record") && self.peek_keyword_n(1, Keyword::Struct);
            if is_record_struct || self.check_keyword(Keyword::Struct) {
                if let Some(span) = delegate_modifier {
                    self.push_error(
                        "modifier `delegate` is not supported on nested struct declarations",
                        Some(span),
                    );
                }
                if is_required {
                    self.push_error(
                        "`required` modifier is not supported on nested struct declarations",
                        required_span,
                    );
                }
                if let Some(modifier) = modifiers.async_modifier.as_ref() {
                    self.push_error(
                        "`async` modifier is not supported on nested struct declarations",
                        Some(modifier.span),
                    );
                }
                if let Some(modifier) = modifiers.constexpr_modifier.as_ref() {
                    self.push_error(
                        "`constexpr` modifier is not supported on nested struct declarations",
                        Some(modifier.span),
                    );
                }
                if let Some(modifier) = modifiers.extern_modifier.as_ref() {
                    self.push_error(
                        "`extern` modifier is not supported on nested struct declarations",
                        Some(modifier.span),
                    );
                }
                if let Some(modifier) = modifiers.unsafe_modifier.as_ref() {
                    self.push_error(
                        "`unsafe` modifier is not supported on nested struct declarations",
                        Some(modifier.span),
                    );
                }
                for modifier in modifiers.remaining() {
                    if modifier.name.eq_ignore_ascii_case("readonly") {
                        continue;
                    }
                    self.push_error(
                        format!(
                            "modifier `{}` is not supported on nested struct declarations",
                            modifier.name
                        ),
                        Some(modifier.span),
                    );
                }

                if is_record_struct {
                    self.advance();
                    self.advance();
                } else {
                    self.match_keyword(Keyword::Struct);
                }

                match self.parse_struct(
                    member_visibility,
                    member_doc,
                    member_attrs,
                    modifiers.remaining(),
                    is_record_struct,
                ) {
                    Some(Item::Struct(def)) => nested_types.push(Item::Struct(def)),
                    Some(item) => nested_types.push(item),
                    None => self.synchronize_field(),
                }
                continue;
            }

            // LL1_ALLOW: Struct constructors reuse the type name or the `init` keyword, so we peek for `(` to disambiguate from fields (docs/compiler/parser.md#ll1-allowances).
            let is_named_ctor = self.peek_identifier(&name) && self.peek_punctuation_n(1, '(');
            // LL1_ALLOW: same constructor disambiguation for `init(...)`.
            let is_init_ctor = self.peek_keyword_n(0, Keyword::Init) && self.peek_punctuation_n(1, '(');
            if is_named_ctor || is_init_ctor {
                if is_required {
                    self.push_error(
                        "`required` modifier is not supported on struct constructors",
                        required_span,
                    );
                }
                if let Some(modifier) = modifiers.async_modifier.as_ref() {
                    self.push_error(
                        "struct constructors cannot be marked `async`",
                        Some(modifier.span),
                    );
                }
                if let Some(modifier) = modifiers.constexpr_modifier.as_ref() {
                    self.push_error(
                        "`constexpr` modifier is not supported on struct constructors",
                        Some(modifier.span),
                    );
                }
                if let Some(modifier) = modifiers.extern_modifier.as_ref() {
                    self.push_error(
                        "`extern` modifier is not supported on struct constructors",
                        Some(modifier.span),
                    );
                }
                if let Some(modifier) = modifiers.unsafe_modifier.as_ref() {
                    self.push_error(
                        "`unsafe` modifier is not supported on struct constructors",
                        Some(modifier.span),
                    );
                }
                if modifiers.required_modifiers.len() > 1 {
                    self.push_error(
                        "duplicate `required` modifier",
                        modifiers.duplicate_required_span(),
                    );
                }
                if !member_attrs.is_empty() && !member_attrs.builtin.is_empty() {
                    self.report_attribute_misuse(
                        member_attrs.clone(),
                        "unsupported built-in attribute on struct constructors",
                    );
                }

                let mut constructor = match self.parse_struct_constructor(
                    &name,
                    member_visibility,
                    modifiers.clone_remaining(),
                    member_doc.take(),
                ) {
                    Some(constructor) => constructor,
                    None => {
                        self.synchronize_field();
                        continue;
                    }
                };
                constructor.attributes = member_attrs.take_list();
                constructor.di_inject = None;
                constructors.push(constructor);
                continue;
            }

            if self.check_keyword(Keyword::Const) {
                if is_required {
                    self.push_error(
                        "`required` modifier is not supported on struct constants",
                        required_span,
                    );
                }
                if let Some(modifier) = modifiers.async_modifier.as_ref() {
                    self.push_error(
                        "`async` modifier is not supported on struct constants",
                        Some(modifier.span),
                    );
                }
                if let Some(modifier) = modifiers.constexpr_modifier.as_ref() {
                    self.push_error(
                        "`constexpr` modifier is not supported on struct constants",
                        Some(modifier.span),
                    );
                }
                if let Some(modifier) = modifiers.extern_modifier.as_ref() {
                    self.push_error(
                        "`extern` modifier is not supported on struct constants",
                        Some(modifier.span),
                    );
                }
                if let Some(modifier) = modifiers.unsafe_modifier.as_ref() {
                    self.push_error(
                        "`unsafe` modifier is not supported on struct constants",
                        Some(modifier.span),
                    );
                }
                if let Some(span) = delegate_modifier {
                    self.push_error(
                        "modifier `delegate` is not supported on struct constants",
                        Some(span),
                    );
                }
                for modifier in modifiers.remaining() {
                    self.push_error(
                        format!(
                            "modifier `{}` is not supported on struct constants",
                            modifier.name
                        ),
                        Some(modifier.span),
                    );
                }

                let field_attrs = self.attributes_to_field_attributes(&mut member_attrs);
                if field_attrs.builtin.mmio.is_some() {
                    self.push_error(
                        "`@register` attribute is only supported on fields inside `@mmio` structs",
                        field_attrs.builtin.mmio_span,
                    );
                }
                let start = self.peek().map(|token| token.span.start);
                self.match_keyword(Keyword::Const);
                let Some(mut declaration) =
                    self.parse_const_declaration_body(member_doc, ';')
                else {
                    self.synchronize_field();
                    continue;
                };
                if !self.expect_punctuation(';') {
                    self.synchronize_field();
                    continue;
                }
                declaration.span = self.make_span(start);
                consts.push(ConstMemberDecl {
                    visibility: member_visibility,
                    modifiers: Vec::new(),
                    declaration,
                });
                continue;
            }

            if self.check_keyword(Keyword::Implicit) || self.check_keyword(Keyword::Explicit) {
                if is_required {
                    self.push_error(
                        "`required` modifier is not supported on struct methods",
                        required_span,
                    );
                }
                if let Some(span) = delegate_modifier {
                    self.push_error(
                        "modifier `delegate` is not supported on struct methods",
                        Some(span),
                    );
                }
                if let Some(modifier) = modifiers.constexpr_modifier.as_ref() {
                    self.push_error(
                        "`constexpr` modifier is not supported on struct methods",
                        Some(modifier.span),
                    );
                }
                if let Some(modifier) = modifiers.extern_modifier.as_ref() {
                    self.push_error(
                        "`extern` modifier is not supported on struct methods",
                        Some(modifier.span),
                    );
                }
                let mut function = match self.parse_conversion_operator_member(
                    member_visibility,
                    is_async_member,
                    member_doc.take(),
                    modifiers.clone_remaining(),
                    modifiers.unsafe_modifier.is_some(),
                    OperatorOwner::Struct,
                ) {
                    Some(function) => function,
                    None => {
                        self.synchronize_field();
                        continue;
                    }
                };
                if modifiers.unsafe_modifier.is_some() {
                    self.push_error(
                        "operator overloads cannot be marked `unsafe`",
                        modifiers.unsafe_modifier.map(|modifier| modifier.span),
                    );
                }
                self.apply_method_attributes(member_attrs, false, &mut function);
                methods.push(function);
                continue;
            }

            self.consume_all_borrow_qualifier_misuse(true);
            let Some(field_type) = self.parse_type_expr() else {
                self.synchronize_field();
                continue;
            };

            if self.check_keyword(Keyword::Operator) {
                if is_required {
                    self.push_error(
                        "`required` modifier is not supported on struct methods",
                        required_span,
                    );
                }
                if let Some(span) = delegate_modifier {
                    self.push_error(
                        "modifier `delegate` is not supported on struct methods",
                        Some(span),
                    );
                }
                if let Some(modifier) = modifiers.constexpr_modifier.as_ref() {
                    self.push_error(
                        "`constexpr` modifier is not supported on struct methods",
                        Some(modifier.span),
                    );
                }
                if let Some(modifier) = modifiers.extern_modifier.as_ref() {
                    self.push_error(
                        "`extern` modifier is not supported on struct methods",
                        Some(modifier.span),
                    );
                }
                let mut function = match self.parse_symbol_operator_member(
                    member_visibility,
                    is_async_member,
                    member_doc.take(),
                    modifiers.clone_remaining(),
                    field_type,
                    modifiers.unsafe_modifier.is_some(),
                    OperatorOwner::Struct,
                ) {
                    Some(function) => function,
                    None => {
                        self.synchronize_field();
                        continue;
                    }
                };
                if modifiers.unsafe_modifier.is_some() {
                    self.push_error(
                        "operator overloads cannot be marked `unsafe`",
                        modifiers.unsafe_modifier.map(|modifier| modifier.span),
                    );
                }
                self.apply_method_attributes(member_attrs, false, &mut function);
                methods.push(function);
                continue;
            }

            let mut explicit_interface: Option<String> = None;
            let (field_name, name_token_index) =
                if self.peek_identifier("this") {
                    let _ = self.advance();
                    ("this".to_string(), self.index.saturating_sub(1))
                } else {
                    let ident = match self.consume_identifier("expected field name") {
                        Some(name) => name,
                        None => {
                            self.synchronize_field();
                            continue;
                        }
                    };
                    let mut token_index = self.index.saturating_sub(1);
                    if self.consume_punctuation('.') {
                        explicit_interface = Some(ident);
                        if self.peek_identifier("this") {
                            let _ = self.advance();
                            ("this".to_string(), self.index.saturating_sub(1))
                        } else {
                            let member = match self.consume_identifier(
                                "expected member name after interface qualifier",
                            ) {
                                Some(name) => name,
                                None => {
                                    self.synchronize_field();
                                    continue;
                                }
                            };
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
            let mut view_of = None;
            if self.match_keyword(Keyword::Of) {
                if !field_type.is_view {
                    self.push_error(
                        "`of` clause requires a `view` field type",
                        self.last_span,
                    );
                }
                view_of = self.consume_identifier("expected owning field name after `of`");
            }
            let mut indexer_parameters = Vec::new();
            let mut is_indexer = false;
            if self.check_punctuation('[') {
                indexer_parameters = self.parse_indexer_parameters();
                is_indexer = true;
                if !field_name.eq_ignore_ascii_case("this") {
                    self.push_error(
                        "indexer must be declared as 'this'",
                        self.tokens
                            .get(name_token_index)
                            .map(|token| token.span),
                    );
                }
            }
            if self.check_operator("=>") || self.check_punctuation('{') {
                if !member_attrs.is_empty() && !member_attrs.builtin.is_empty() {
                    self.report_attribute_misuse(
                        member_attrs.clone(),
                        "unsupported built-in attribute on properties",
                    );
                }
                if let Some(modifier) = modifiers.async_modifier.as_ref() {
                    self.push_error(
                        "properties cannot be marked `async`",
                        Some(modifier.span),
                    );
                }
                if let Some(modifier) = modifiers.constexpr_modifier.as_ref() {
                    self.push_error(
                        "`constexpr` modifier is not supported on properties",
                        Some(modifier.span),
                    );
                }
                if let Some(modifier) = modifiers.extern_modifier.as_ref() {
                    self.push_error(
                        "`extern` modifier is not supported on properties",
                        Some(modifier.span),
                    );
                }
                if let Some(modifier) = modifiers.unsafe_modifier.as_ref() {
                    self.push_error(
                        "`unsafe` modifier is not supported on properties",
                        Some(modifier.span),
                    );
                }
                if has_generics {
                    if let Some(params) = generics {
                        self.push_error(
                            "properties cannot declare generic parameter lists",
                            params.span,
                        );
                    }
                }
                let dispatch_markers = modifiers.dispatch_modifiers();
                if let Some(span) = dispatch_markers.abstract_span {
                    self.push_error(
                        "`abstract` modifier is not supported on properties",
                        Some(span),
                    );
                }
                let mut property = match self.parse_property(
                    PropertyContext::Struct,
                    member_visibility,
                    modifiers.clone_remaining(),
                    field_name,
                    name_token_index,
                    field_type,
                    indexer_parameters,
                    member_doc.take(),
                    is_async_member,
                    is_required,
                    required_span,
                    dispatch_markers,
                    false,
                    explicit_interface,
                    is_indexer,
                ) {
                    Some(property) => property,
                    None => {
                        self.synchronize_field();
                        continue;
                    }
                };
                if modifiers.required_modifiers.len() > 1 {
                    self.push_error(
                        "duplicate `required` modifier",
                        modifiers.duplicate_required_span(),
                    );
                }
                if self.check_operator("=") {
                    self.advance();
                    let initializer = self.collect_expression_until(&[';']);
                    if !self.expect_punctuation(';') {
                        self.synchronize_field();
                        continue;
                    }
                    if !property.is_auto() {
                        self.push_error(
                            "property initializers are only supported on auto-implemented properties",
                            initializer.span.or(property.span),
                        );
                    }
                    property.initializer = Some(initializer);
                } else {
                    let _ = self.consume_punctuation(';');
                }
                property.attributes = member_attrs.take_list();
                properties.push(property);
                continue;
            }

            if self.check_punctuation('(') {
                if is_required {
                    self.push_error(
                        "`required` modifier is not supported on struct methods",
                        required_span,
                    );
                }
                if modifiers.required_modifiers.len() > 1 {
                    self.push_error(
                        "duplicate `required` modifier on field",
                        modifiers.duplicate_required_span(),
                    );
                }
                if let Some(modifier) = modifiers.async_modifier.as_ref() {
                    self.push_error(
                        "`async` modifier is not supported on struct methods",
                        Some(modifier.span),
                    );
                }
                if is_constexpr_member {
                    if let Some(modifier) = modifiers.constexpr_modifier.as_ref() {
                        self.push_error(
                            "`constexpr` modifier is not supported on struct methods",
                            Some(modifier.span),
                        );
                    }
                }
                if let Some(modifier) = modifiers.extern_modifier.as_ref() {
                    self.push_error(
                        "`extern` modifier is not supported on struct methods",
                        Some(modifier.span),
                    );
                }
                if let Some(span) = delegate_modifier {
                    self.push_error(
                        "modifier `delegate` is not supported on struct methods",
                        Some(span),
                    );
                }
                for modifier in modifiers.remaining() {
                    if modifier.name.eq_ignore_ascii_case("static") {
                        continue;
                    }
                    self.push_error(
                        format!(
                            "modifier `{}` is not supported on struct methods",
                            modifier.name
                        ),
                        Some(modifier.span),
                    );
                }
                if !self.expect_punctuation('(') {
                    return None;
                }
                let (parameters, variadic) = self.parse_parameters();
                if !self.expect_punctuation(')') {
                    return None;
                }

                self.parse_where_clauses(&mut generics);
                let throws = self.parse_throws_clause();
                let lends_to_return = self.parse_lends_clause();

                let method_generics = generics.take();

                let returns_value = self.type_returns_value(&field_type);
                let body = match self.parse_function_tail(true, returns_value)? {
                    FunctionBodyKind::Block(block) => Some(block),
                    FunctionBodyKind::Declaration => None,
                };

                let method_modifiers: Vec<String> = modifiers
                    .clone_remaining()
                    .into_iter()
                    .map(|modifier| modifier.name)
                    .collect();
                let is_static_method = method_modifiers
                    .iter()
                    .any(|modifier| modifier.eq_ignore_ascii_case("static"));
                let type_named_method = field_name == name;

                let name_span = self
                    .tokens
                    .get(name_token_index)
                    .map(|token| token.span)
                    .or(self.last_span);
                let mut function = FunctionDecl {
                    visibility: member_visibility,
                    name: field_name,
                    name_span,
                    signature: Signature {
                        parameters,
                        return_type: field_type,
                        lends_to_return,
                        throws,
                        variadic,
                    },
                    body,
                    is_async: false,
                    is_constexpr: false,
                    doc: member_doc.clone(),
                    modifiers: method_modifiers,
                    is_unsafe: modifiers.unsafe_modifier.is_some(),
                    attributes: Vec::new(),
                    is_extern: false,
                    extern_abi: None,
                    extern_options: None,
                    link_name: None,
                    link_library: None,
                    operator: None,
                    generics: method_generics,
                    vectorize_hint: None,
                    dispatch: MemberDispatch::default(),
                };
                if type_named_method {
                    let mut diagnostic = Diagnostic::error(
                        format!(
                            "methods cannot use the containing struct name `{name}`; declare constructors with `init`"
                        ),
                        name_span,
                    )
                    .with_code(DiagnosticCode::new("E0C02", Some("constructor".into())))
                    .with_primary_label("type-named members are reserved for `init` constructors");
                    diagnostic.add_suggestion(Suggestion::new(
                        "rename this member or convert it to `init(...)`",
                        name_span,
                        None,
                    ));
                    self.diagnostics.push(diagnostic);
                }
                if is_static_method {
                    function.dispatch.is_virtual = false;
                }
                self.apply_method_attributes(member_attrs, false, &mut function);
                methods.push(function);
                continue;
            }

            if has_generics {
                if let Some(params) = generics {
                    if !params.is_empty() {
                        self.push_error(
                            "generic parameter list is only supported on struct methods",
                            params.span,
                        );
                    }
                }
            }

            if modifiers.required_modifiers.len() > 1 {
                self.push_error(
                    "duplicate `required` modifier on field",
                    modifiers.duplicate_required_span(),
                );
            }
            let member_has_static_modifier = modifiers
                .remaining()
                .iter()
                .any(|modifier| modifier.name.eq_ignore_ascii_case("static"));
            let readonly_spans: Vec<Span> = modifiers
                .remaining()
                .iter()
                .filter(|modifier| modifier.name.eq_ignore_ascii_case("readonly"))
                .map(|modifier| modifier.span)
                .collect();
            if readonly_spans.len() > 1 {
                self.push_error(
                    "duplicate `readonly` modifier on field",
                    readonly_spans.get(1).copied(),
                );
            }
            let is_field_readonly = !readonly_spans.is_empty();
            if modifiers.has_required() && member_has_static_modifier {
                self.push_error(
                    "`required` modifier is not supported on struct fields",
                    modifiers.first_required_span(),
                );
            }
            if let Some(modifier) = modifiers.async_modifier.as_ref() {
                self.push_error(
                    "`async` modifier is not supported on struct fields",
                    Some(modifier.span),
                );
            }
            if is_constexpr_member {
                if let Some(modifier) = modifiers.constexpr_modifier.as_ref() {
                    self.push_error(
                        "`constexpr` modifier is not supported on struct fields",
                        Some(modifier.span),
                    );
                }
            }
            if let Some(modifier) = modifiers.extern_modifier.as_ref() {
                self.push_error(
                    "`extern` modifier is not supported on struct fields",
                    Some(modifier.span),
                );
            }
            if let Some(modifier) = modifiers.unsafe_modifier.as_ref() {
                self.push_error(
                    "`unsafe` modifier is not supported on struct fields",
                    Some(modifier.span),
                );
            }
            for modifier in modifiers.remaining() {
                if modifier.name.eq_ignore_ascii_case("readonly")
                    || modifier.name.eq_ignore_ascii_case("static")
                {
                    continue;
                }
                self.push_error(
                    format!(
                        "modifier `{}` is not supported on struct fields",
                        modifier.name
                    ),
                    Some(modifier.span),
                );
            }

            let mut field_attrs = self.attributes_to_field_attributes(&mut member_attrs);

            if field_attrs.builtin.mmio.is_some() && struct_mmio_attr.is_none() {
                self.push_error(
                    "`@register` attribute is only supported inside `@mmio` structs",
                    field_attrs.builtin.mmio_span,
                );
            }

            if !self.expect_punctuation(';') {
                self.synchronize_field();
                continue;
            }

            fields.push(FieldDecl {
                visibility: member_visibility,
                name: field_name,
                ty: field_type,
                initializer: None,
                mmio: field_attrs.builtin.mmio.clone(),
                doc: member_doc,
                is_required,
                display_name: None,
                attributes: field_attrs.take_list(),
                is_readonly: is_field_readonly,
                is_static: member_has_static_modifier,
                view_of,
            });
        }

        if !self.expect_punctuation('}') {
            return None;
        }

        finalize_record(&mut fields, &mut constructors);

        let layout_hints = attrs.builtin.struct_layout.clone();
        let is_intrinsic = attrs.builtin.intrinsic;
        let inline_attr = attrs.builtin.inline_attr;
        let CommonTypeAttributes {
            thread_safe_override,
            shareable_override,
            copy_override,
            attributes,
        } = take_common_type_attributes(&mut attrs);

        Some(Item::Struct(StructDecl {
            visibility,
            name,
            fields,
            properties,
            constructors,
            consts,
            methods,
            nested_types,
            bases,
            thread_safe_override,
            shareable_override,
            copy_override,
            mmio: struct_mmio_attr,
            doc,
            generics,
            attributes,
            is_readonly,
            layout: layout_hints,
            is_intrinsic,
            inline_attr,
            is_record,
            record_positional_fields,
        }))
    }
}
