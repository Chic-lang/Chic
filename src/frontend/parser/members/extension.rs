use super::{methods::OperatorOwner, modifiers::MemberModifiers, *};
use crate::frontend::ast::ExtensionMethodDecl;

parser_impl! {
    pub(in crate::frontend::parser) fn parse_extension_member(&mut self) -> Option<ExtensionMember> {
        self.stash_leading_doc();
        let attrs = self.collect_attributes();
        self.stash_leading_doc();
        if self.check_punctuation('}') {
            return None;
        }

        let mut doc = self.take_pending_doc();
        let visibility = self.parse_visibility();
        let modifiers = MemberModifiers::new(self.consume_modifiers());
        let has_unsafe_modifier = modifiers.unsafe_modifier.is_some();
        let is_async = modifiers.async_modifier.is_some();
        let is_constexpr = modifiers.constexpr_modifier.is_some();
        let has_extern_modifier = modifiers.extern_modifier.is_some();
        let mut is_default_member = false;
        let mut default_span = None;
        if self.match_keyword(Keyword::Default) {
            is_default_member = true;
            default_span = self.last_span;
        }
        if modifiers.has_required() {
            self.push_error(
                "`required` modifier is not supported on extension members",
                modifiers.first_required_span(),
            );
        }

        if self.check_keyword(Keyword::Implicit) || self.check_keyword(Keyword::Explicit) {
            if is_default_member {
                self.push_error(
                    "`default` modifier is only supported on regular extension methods",
                    default_span,
                );
            }
            let mut function = self.parse_conversion_operator_member(
                visibility,
                is_async,
                doc.take(),
                modifiers.clone_remaining(),
                has_unsafe_modifier,
                OperatorOwner::Extension,
            )?;
            self.apply_method_attributes(attrs, has_extern_modifier, &mut function);
            return Some(ExtensionMember::Method(ExtensionMethodDecl {
                function,
                is_default: false,
            }));
        }

        let return_type = self.parse_type_expr()?;

        if self.check_keyword(Keyword::Operator) {
            if is_default_member {
                self.push_error(
                    "`default` modifier is only supported on regular extension methods",
                    default_span,
                );
            }
            let mut function = self.parse_symbol_operator_member(
                visibility,
                is_async,
                doc.take(),
                modifiers.clone_remaining(),
                return_type,
                has_unsafe_modifier,
                OperatorOwner::Extension,
            )?;
            self.apply_method_attributes(attrs, has_extern_modifier, &mut function);
            return Some(ExtensionMember::Method(ExtensionMethodDecl {
                function,
                is_default: false,
            }));
        }

        let name = self.consume_identifier("expected extension member name")?;
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
        let body = match self.parse_function_tail(true, returns_value)? {
            FunctionBodyKind::Block(block) => Some(block),
            FunctionBodyKind::Declaration => None,
        };

        let has_body = body.is_some();
        if is_default_member && !has_body {
            self.push_error(
                "`default` extension methods must provide a body",
                default_span,
            );
        }

        let method_modifiers = modifiers
            .clone_remaining()
            .into_iter()
            .map(|modifier| modifier.name)
            .collect();
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
        self.apply_method_attributes(attrs, has_extern_modifier, &mut function);
        Some(ExtensionMember::Method(ExtensionMethodDecl {
            function,
            is_default: is_default_member && has_body,
        }))
    }
}
