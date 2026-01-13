use super::{modifiers::MemberModifiers, *};

parser_impl! {
    pub(super) fn parse_class_property_member(
        &mut self,
        visibility: Visibility,
        name: String,
        name_token_index: usize,
        return_type: TypeExpr,
        parameters: Vec<Parameter>,
        doc: &mut Option<DocComment>,
        is_async: bool,
        has_generics: bool,
        generics_span: Option<Span>,
        is_constexpr: bool,
        modifiers: &MemberModifiers,
        attrs: &mut CollectedAttributes,
        class_is_static: bool,
        explicit_interface: Option<String>,
        is_indexer: bool,
    ) -> Option<ClassMember> {
        if !attrs.is_empty() && !attrs.builtin.is_empty() {
            self.report_attribute_misuse(
                attrs.clone(),
                "unsupported built-in attribute on properties",
            );
        }
        if let Some(span) = modifiers.unsafe_modifier.as_ref().map(|modifier| modifier.span) {
            self.push_error(
                "`unsafe` modifier is not supported on properties",
                Some(span),
            );
        }
        if let Some(span) = modifiers.extern_modifier.as_ref().map(|modifier| modifier.span) {
            self.push_error(
                "`extern` modifier is not supported on properties",
                Some(span),
            );
        }
        if has_generics {
            self.push_error(
                "properties cannot declare generic parameter lists",
                generics_span,
            );
        }
        if is_constexpr {
            self.push_error(
                "`constexpr` modifier is not supported on properties",
                self.last_span,
            );
        }

        let mut property = self.parse_class_property(
            visibility,
            modifiers.clone_remaining(),
            name,
            name_token_index,
            return_type,
            parameters,
            doc.take(),
            is_async,
            modifiers.has_required(),
            modifiers.first_required_span(),
            modifiers.dispatch_modifiers(),
            class_is_static,
            explicit_interface,
            is_indexer,
        )?;
        let attributes = attrs.take_list();
        property.attributes = attributes;
        property.di_inject = None;
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
                return Some(ClassMember::Property(property));
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
        Some(ClassMember::Property(property))
    }
}
