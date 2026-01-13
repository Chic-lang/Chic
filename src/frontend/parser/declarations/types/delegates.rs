use crate::frontend::parser::*;

parser_impl! {
    pub(crate) fn parse_delegate(
        &mut self,
        visibility: Visibility,
        doc: Option<DocComment>,
        mut attrs: CollectedAttributes,
        modifiers: &[Modifier],
    ) -> Option<Item> {
        let start_pos = self.last_span.map(|span| span.start).or_else(|| {
            self.peek().and_then(|token| Some(token.span.start))
        });

        let mut is_unsafe = false;
        for modifier in modifiers {
            if modifier.name.eq_ignore_ascii_case("unsafe") {
                is_unsafe = true;
                continue;
            }
            self.push_error(
                format!(
                    "modifier `{}` is not supported on delegate declarations",
                    modifier.name
                ),
                Some(modifier.span),
            );
        }

        let return_type = self.parse_type_expr()?;
        let name = self.consume_identifier("expected delegate name")?;
        let mut generics = self.parse_generic_parameter_list_allowing_variance();

        if !self.expect_punctuation('(') {
            return None;
        }
        let (parameters, variadic) = self.parse_parameters();
        if !self.expect_punctuation(')') {
            return None;
        }

        self.parse_where_clauses(&mut generics);

        let attributes = attrs.take_list();

        if !self.expect_punctuation(';') {
            return None;
        }

        let span = self.make_span(start_pos);
        Some(Item::Delegate(DelegateDecl {
            visibility,
            name,
            signature: Signature {
                parameters,
                return_type,
                lends_to_return: None,
                variadic,
                throws: None,
            },
            generics,
            attributes,
            doc,
            is_unsafe,
            modifiers: modifiers.iter().map(|modifier| modifier.name.clone()).collect(),
            span,
        }))
    }
}
