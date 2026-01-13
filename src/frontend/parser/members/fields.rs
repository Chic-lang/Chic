use super::{modifiers::MemberModifiers, *};

parser_impl! {
    pub(super) fn parse_const_member(
        &mut self,
        visibility: Visibility,
        doc: &mut Option<DocComment>,
        attrs: &CollectedAttributes,
        modifiers: &MemberModifiers,
    ) -> Option<ClassMember> {
        if !self.check_keyword(Keyword::Const) {
            return None;
        }

        self.reject_dispatch_modifiers(modifiers, "const declarations");

        if !attrs.is_empty() {
            self.report_attribute_misuse(
                attrs.clone(),
                "attributes are not supported on const declarations",
            );
        }
        if let Some(span) = modifiers.unsafe_modifier.as_ref().map(|modifier| modifier.span) {
            self.push_error(
                "`unsafe` modifier is not supported on const declarations",
                Some(span),
            );
        }
        if modifiers.has_required() {
            self.push_error(
                "`required` modifier is not supported on const declarations",
                modifiers.first_required_span(),
            );
        }
        if modifiers.constexpr_modifier.is_some() {
            self.push_error(
                "`constexpr` modifier is not supported on const declarations",
                self.last_span,
            );
        }
        if let Some(span) = modifiers.extern_modifier.as_ref().map(|modifier| modifier.span) {
            self.push_error(
                "`extern` modifier is not supported on const declarations",
                Some(span),
            );
        }

        let start = self.peek().map(|token| token.span.start);
        self.match_keyword(Keyword::Const);
        let mut declaration = self.parse_const_declaration_body(doc.take(), ';')?;
        if !self.expect_punctuation(';') {
            return None;
        }
        declaration.span = self.make_span(start);
        if modifiers.required_modifiers.len() > 1 {
            self.push_error(
                "duplicate `required` modifier",
                modifiers.duplicate_required_span(),
            );
        }
        let modifier_names = modifiers
            .remaining()
            .iter()
            .map(|modifier| modifier.name.clone())
            .collect();
        Some(ClassMember::Const(ConstMemberDecl {
            visibility,
            modifiers: modifier_names,
            declaration,
        }))
    }

    pub(super) fn finalize_field_member(
        &mut self,
        visibility: Visibility,
        name: String,
        ty: TypeExpr,
        initializer: Option<Expression>,
        doc: &mut Option<DocComment>,
        attrs: &CollectedAttributes,
        modifiers: &MemberModifiers,
        enclosing_is_static: bool,
    ) -> Option<ClassMember> {

        self.reject_dispatch_modifiers(modifiers, "fields");
        if !self.expect_punctuation(';') {
            return None;
        }

        if !attrs.is_empty() {
            self.report_attribute_misuse(
                attrs.clone(),
                "attributes are not supported on fields",
            );
        }

        let member_has_static_modifier = modifiers
            .remaining()
            .iter()
            .any(|modifier| modifier.name.eq_ignore_ascii_case("static"));
        if modifiers.has_required() && (member_has_static_modifier || enclosing_is_static) {
            self.push_error(
                "`required` modifier is not supported on static fields",
                modifiers.first_required_span(),
            );
        }
        if modifiers.constexpr_modifier.is_some() {
            self.push_error(
                "`constexpr` modifier is not supported on fields",
                self.last_span,
            );
        }
        if let Some(span) = modifiers.unsafe_modifier.as_ref().map(|modifier| modifier.span) {
            self.push_error(
                "`unsafe` modifier is not supported on fields",
                Some(span),
            );
        }
        if let Some(span) = modifiers.extern_modifier.as_ref().map(|modifier| modifier.span) {
            self.push_error(
                "`extern` modifier is not supported on fields",
                Some(span),
            );
        }
        if modifiers.required_modifiers.len() > 1 {
            self.push_error(
                "duplicate `required` modifier",
                modifiers.duplicate_required_span(),
            );
        }
        let readonly_spans: Vec<Span> = modifiers
            .remaining()
            .iter()
            .filter(|modifier| modifier.name.eq_ignore_ascii_case("readonly"))
            .map(|modifier| modifier.span)
            .collect();
        if readonly_spans.len() > 1 {
            self.push_error(
                "duplicate `readonly` modifier",
                readonly_spans.get(1).copied(),
            );
        }
        let is_readonly = !readonly_spans.is_empty();
        let is_static = enclosing_is_static || member_has_static_modifier;

        Some(ClassMember::Field(FieldDecl {
            visibility,
            name,
            ty,
            initializer,
            doc: doc.take(),
            attributes: Vec::new(),
            mmio: None,
            is_required: modifiers.has_required(),
            display_name: None,
            is_readonly,
             is_static,
            view_of: None,
        }))
    }
}
