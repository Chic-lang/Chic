use super::*;

parser_impl! {
    pub(super) fn parse_function(
        &mut self,
        visibility: Visibility,
        is_async: bool,
        is_constexpr: bool,
        doc: Option<DocComment>,
    ) -> Option<FunctionDecl> {
        if self.check_keyword(Keyword::Implicit)
            || self.check_keyword(Keyword::Explicit)
            || self.check_keyword(Keyword::Operator)
        {
            let span = self.peek().map(|token| token.span);
            self.push_error(
                "operator overloads may only be declared inside classes or extensions",
                span,
                            );
            return None;
        }

        let return_type = self.parse_type_expr()?;
        let name = self.consume_identifier("expected function name")?;
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

        Some(FunctionDecl {
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
            doc,
            modifiers: Vec::new(),
            is_unsafe: false,
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
        })
    }
}

parser_impl! {
    pub(super) fn parse_throws_clause(&mut self) -> Option<ThrowsClause> {
        if !self.match_keyword(Keyword::Throws) {
            return None;
        }

        let start_span = self.last_span;
        let mut effects = Vec::new();

        loop {
            let checkpoint = self.index;
            match self.parse_type_expr() {
                Some(ty) => effects.push(ty),
                None => {
                    if self.index == checkpoint {
                        if let Some(token) = self.peek() {
                            self.push_error(
                                "expected exception type after `throws`",
                                Some(token.span),
                            );
                            self.advance();
                        } else {
                            self.push_error(
                                "expected exception type after `throws`",
                                start_span,
                            );
                        }
                    }
                    break;
                }
            }

            if !self.consume_punctuation(',') {
                break;
            }
        }

        if effects.is_empty() {
            self.push_error("`throws` clause requires at least one exception type", start_span);
        }

        let end_span = self.last_span;
        let span = match (start_span, end_span) {
            (Some(start), Some(end)) if end.end >= start.start => Some(Span::in_file(
                start.file_id,
                start.start,
                end.end,
            )),
            _ => start_span,
        };

        Some(ThrowsClause::new(effects, span))
    }
}
