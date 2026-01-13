use super::{modifiers::MemberModifiers, *};
use crate::frontend::ast::TraitAssociatedType;

parser_impl! {
    pub(in crate::frontend::parser) fn parse_interface_member(&mut self) -> Option<InterfaceMember> {
        self.stash_leading_doc();
        self.skip_attributes();
        self.stash_leading_doc();
        if self.check_punctuation('}') {
            return None;
        }

        let mut doc = self.take_pending_doc();
        let visibility = self.parse_visibility();
        let modifiers = MemberModifiers::new(self.consume_modifiers());
        let mut dispatch_markers = modifiers.dispatch_modifiers();
        dispatch_markers.abstract_span = None;
        self.reject_dispatch_markers(dispatch_markers, "interface members");
        let unsafe_span = modifiers
            .unsafe_modifier
            .as_ref()
            .map(|modifier| modifier.span);
        let is_async = modifiers.async_modifier.is_some();
        let is_constexpr = modifiers.constexpr_modifier.is_some();
        if let Some(span) = unsafe_span {
            self.push_error(
                "`unsafe` modifier is not supported on interface members",
                Some(span),
            );
        }
        if is_constexpr {
            self.push_error(
                "`constexpr` modifier is not supported on interface members",
                self.last_span,
            );
        }
        if modifiers.has_required() {
            self.push_error(
                "`required` modifier is not supported on interface members",
                modifiers.first_required_span(),
            );
        }

        if self.check_keyword(Keyword::Const) {
            let start = self.peek().map(|token| token.span.start);
            self.match_keyword(Keyword::Const);
            let mut declaration = self.parse_const_declaration_body(doc.take(), ';')?;
            if !self.expect_punctuation(';') {
                return None;
            }
            declaration.span = self.make_span(start);
            let modifier_names = modifiers
                .remaining()
                .iter()
                .map(|modifier| modifier.name.clone())
                .collect();
            return Some(InterfaceMember::Const(ConstMemberDecl {
                visibility,
                modifiers: modifier_names,
                declaration,
            }));
        }

        if self.match_keyword(Keyword::Type) {
            let assoc = self.parse_interface_associated_type(doc.take())?;
            return Some(InterfaceMember::AssociatedType(assoc));
        }

        if self.check_keyword(Keyword::Implicit) || self.check_keyword(Keyword::Explicit) {
            let mut function = self.parse_conversion_operator_member(
                visibility,
                is_async,
                doc.take(),
                modifiers.clone_remaining(),
                modifiers.unsafe_modifier.is_some(),
                OperatorOwner::Interface,
            )?;
            if function.body.is_some() {
                self.push_error(
                    "interface conversion operators cannot provide a body",
                    self.last_span,
                );
                function.body = None;
            }
            return Some(InterfaceMember::Method(function));
        }

        let return_type = self.parse_type_expr()?;

        if self.check_keyword(Keyword::Operator) {
            let mut function = self.parse_symbol_operator_member(
                visibility,
                is_async,
                doc.take(),
                modifiers.clone_remaining(),
                return_type.clone(),
                modifiers.unsafe_modifier.is_some(),
                OperatorOwner::Interface,
            )?;
            if function.body.is_some() {
                self.push_error(
                    "interface operator overloads cannot provide a body",
                    self.last_span,
                );
                function.body = None;
            }
            return Some(InterfaceMember::Method(function));
        }

        let (name, name_token_index) = if self.peek_identifier("this") {
            let _ = self.advance();
            ("this".to_string(), self.index.saturating_sub(1))
        } else {
            let ident = self.consume_identifier("expected interface member name")?;
            (ident, self.index.saturating_sub(1))
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

        if self.check_operator("=>") || self.check_punctuation('{') {
            if has_generics {
                self.push_error(
                    "properties cannot declare generic parameter lists",
                    generics.as_ref().and_then(|params| params.span),
                );
            }
            let property = self.parse_interface_property(
                visibility,
                modifiers.clone_remaining(),
                name,
                name_token_index,
                return_type,
                indexer_parameters,
                doc.take(),
                is_async,
                false,
                None,
                None,
                is_indexer,
            )?;
            return Some(InterfaceMember::Property(property));
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

        let returns_value = self.type_returns_value(&return_type);
        let body = match self.parse_function_tail(true, returns_value)? {
            FunctionBodyKind::Declaration => None,
            FunctionBodyKind::Block(block) => Some(block),
        };
        let name_span = self
            .tokens
            .get(name_token_index)
            .map(|token| token.span)
            .or(self.last_span);
        Some(InterfaceMember::Method(FunctionDecl {
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
            is_constexpr: false,
            doc: doc.take(),
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
        }))
    }

    fn parse_interface_associated_type(
        &mut self,
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
