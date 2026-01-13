use super::{modifiers::MemberModifiers, *};
use crate::frontend::ast::{ImplMember, TraitAssociatedType, TraitMember};

enum AssociatedTypeContext {
    Trait,
    Impl,
}

parser_impl! {
    pub(in crate::frontend::parser) fn parse_trait_member(&mut self) -> Option<TraitMember> {
        self.stash_leading_doc();
        self.skip_attributes();
        self.stash_leading_doc();
        if self.check_punctuation('}') {
            return None;
        }

        let mut doc = self.take_pending_doc();
        let visibility = self.parse_visibility();
        let modifiers = MemberModifiers::new(self.consume_modifiers());
        let is_async = modifiers.async_modifier.is_some();
        if modifiers.constexpr_modifier.is_some() {
            self.push_error("`constexpr` modifier is not supported on trait members", self.last_span);
        }
        if modifiers.has_required() {
            self.push_error(
                "`required` modifier is not supported on trait members",
                modifiers.first_required_span(),
            );
        }

        if self.check_keyword(Keyword::Const) {
            return self
                .parse_trait_const_member(visibility, modifiers.clone_remaining(), doc.take())
                .map(TraitMember::Const);
        }

        if self.match_keyword(Keyword::Type) {
            return self
                .parse_associated_type(AssociatedTypeContext::Trait, doc.take())
                .map(TraitMember::AssociatedType);
        }

        if self.check_keyword(Keyword::Implicit) || self.check_keyword(Keyword::Explicit) {
            let span = self.peek().map(|token| token.span);
            self.push_error("traits may not declare conversion operators", span);
            return None;
        }
        if self.check_keyword(Keyword::Operator) {
            let span = self.peek().map(|token| token.span);
            self.push_error("traits may not declare operator overloads", span);
            return None;
        }

        let return_type = self.parse_type_expr()?;
        let name = self.consume_identifier("expected trait member name")?;
        let name_span = self.last_span;
        let mut generics = self.parse_generic_parameter_list();

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

        let returns_value = self.type_returns_value(&return_type);
        match self.parse_function_tail(true, returns_value) {
            Some(FunctionBodyKind::Declaration) => {
                let function = FunctionDecl {
                    visibility,
                    name,
                    name_span,
                    signature: Signature {
                        parameters,
                        return_type,
                        lends_to_return: lends_to_return.clone(),
                        variadic,
                        throws: throws.clone(),
                    },
                    body: None,
                    is_async,
                    is_constexpr: false,
                    doc,
                    modifiers: modifiers
                        .clone_remaining()
                        .into_iter()
                        .map(|modifier| modifier.name)
                        .collect(),
                    is_unsafe: modifiers.unsafe_modifier.is_some(),
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
                Some(TraitMember::Method(function))
            }
            Some(FunctionBodyKind::Block(block)) => {
                let function = FunctionDecl {
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
                    body: Some(block),
                    is_async,
                    is_constexpr: false,
                    doc,
                    modifiers: modifiers
                        .clone_remaining()
                        .into_iter()
                        .map(|modifier| modifier.name)
                        .collect(),
                    is_unsafe: modifiers.unsafe_modifier.is_some(),
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
                Some(TraitMember::Method(function))
            }
            None => None,
        }
    }

    pub(in crate::frontend::parser) fn parse_impl_member(&mut self) -> Option<ImplMember> {
        self.stash_leading_doc();
        self.skip_attributes();
        self.stash_leading_doc();
        if self.check_punctuation('}') {
            return None;
        }

        let mut doc = self.take_pending_doc();
        let visibility = self.parse_visibility();
        let modifiers = MemberModifiers::new(self.consume_modifiers());
        let is_async = modifiers.async_modifier.is_some();
        if modifiers.constexpr_modifier.is_some() {
            self.push_error("`constexpr` modifier is not supported on impl members", self.last_span);
        }

        if self.check_keyword(Keyword::Const) {
            return self
                .parse_trait_const_member(visibility, modifiers.clone_remaining(), doc.take())
                .map(ImplMember::Const);
        }

        if self.match_keyword(Keyword::Type) {
            return self
                .parse_associated_type(AssociatedTypeContext::Impl, doc.take())
                .map(ImplMember::AssociatedType);
        }

        if self.check_keyword(Keyword::Implicit) || self.check_keyword(Keyword::Explicit) {
            let span = self.peek().map(|token| token.span);
            self.push_error("impl blocks may not declare conversion operators", span);
            return None;
        }
        if self.check_keyword(Keyword::Operator) {
            let span = self.peek().map(|token| token.span);
            self.push_error("impl blocks may not declare operator overloads", span);
            return None;
        }

        let return_type = self.parse_type_expr()?;
        let name = self.consume_identifier("expected impl member name")?;
        let name_span = self.last_span;
        let mut generics = self.parse_generic_parameter_list();

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

        let returns_value = self.type_returns_value(&return_type);
        match self.parse_function_tail(true, returns_value) {
            Some(FunctionBodyKind::Block(block)) => {
                let function = FunctionDecl {
                    visibility,
                    name,
                    name_span,
                    signature: Signature {
                        parameters,
                        return_type,
                        lends_to_return,
                        throws: throws.clone(),
                        variadic,
                    },
                    body: Some(block),
                    is_async,
                    is_constexpr: false,
                    doc,
                    modifiers: modifiers
                        .clone_remaining()
                        .into_iter()
                        .map(|modifier| modifier.name)
                        .collect(),
                    is_unsafe: modifiers.unsafe_modifier.is_some(),
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
                Some(ImplMember::Method(function))
            }
            Some(FunctionBodyKind::Declaration) => {
                self.push_error("impl methods must provide a body", self.last_span);
                None
            }
            None => None,
        }
    }
}

impl Parser<'_> {
    fn parse_trait_const_member(
        &mut self,
        visibility: Visibility,
        modifiers: Vec<Modifier>,
        mut doc: Option<DocComment>,
    ) -> Option<ConstMemberDecl> {
        let start = self.peek().map(|token| token.span.start);
        self.match_keyword(Keyword::Const);
        let mut declaration = self.parse_const_declaration_body(doc.take(), ';')?;
        if !self.expect_punctuation(';') {
            return None;
        }
        declaration.span = self.make_span(start);
        let modifier_names = modifiers
            .into_iter()
            .map(|modifier| modifier.name)
            .collect();
        Some(ConstMemberDecl {
            visibility,
            modifiers: modifier_names,
            declaration,
        })
    }

    fn parse_associated_type(
        &mut self,
        context: AssociatedTypeContext,
        doc: Option<DocComment>,
    ) -> Option<TraitAssociatedType> {
        let start = self.last_span.map(|span| span.start);
        let name = self.consume_identifier("expected associated type name")?;
        let mut generics = self.parse_generic_parameter_list();
        self.parse_where_clauses(&mut generics);

        let default = if self.consume_operator("=") {
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        if matches!(context, AssociatedTypeContext::Impl) && default.is_none() {
            self.push_error(
                "impl associated types must assign a concrete type",
                self.peek().map(|token| token.span),
            );
        }

        if !self.expect_punctuation(';') {
            return None;
        }

        Some(TraitAssociatedType {
            name,
            generics,
            default,
            doc,
            span: self.make_span(start),
        })
    }
}
