use super::super::*;
use crate::frontend::ast::{Attribute, ImplDecl, TestCaseDecl};

parser_impl! {
    pub(crate) fn parse_impl(
        &mut self,
        visibility: Visibility,
        doc: Option<DocComment>,
        mut attrs: CollectedAttributes,
    ) -> Option<Item> {
        let start = self.last_span.map(|span| span.start);
        let mut generics = self.parse_generic_parameter_list();
        let mut trait_ref = None;

        let candidate = self.parse_type_expr()?;
        let target;
        if self.match_keyword(Keyword::For) {
            trait_ref = Some(candidate);
            target = self.parse_type_expr()?;
        } else {
            target = candidate;
        }

        self.parse_where_clauses(&mut generics);

        if !self.expect_punctuation('{') {
            return None;
        }

        let mut members = Vec::new();
        while !self.is_at_end() && !self.check_punctuation('}') {
            self.stash_leading_doc();
            if let Some(member) = self.parse_impl_member() {
                members.push(member);
            } else {
                self.synchronize_class_member();
            }
        }

        if !self.expect_punctuation('}') {
            return None;
        }

        Some(Item::Impl(ImplDecl {
            visibility,
            trait_ref,
            target,
            generics,
            members,
            doc,
            attributes: attrs.take_list(),
            span: self.make_span(start),
        }))
    }

    pub(crate) fn parse_testcase(
        &mut self,
        is_async: bool,
        doc: Option<DocComment>,
        attributes: Vec<Attribute>,
    ) -> Option<Item> {
        let name = self.consume_identifier("expected test case name")?;

        let mut parameters = Vec::new();
        let mut variadic = false;
        if self.check_punctuation('(') {
            self.advance();
            let parsed = self.parse_parameters();
            parameters = parsed.0;
            variadic = parsed.1;
            if !self.expect_punctuation(')') {
                return None;
            }
        }

        if !self.check_punctuation('{') {
            let span = self.peek().map(|token| token.span);
            self.push_error("expected '{' to start test body", span);
            return None;
        }

        let body = self.parse_block()?;

        let return_type = if is_async {
            TypeExpr::simple("Task")
        } else {
            TypeExpr::simple("void")
        };

        let signature = Signature {
            parameters,
            return_type,
            lends_to_return: None,
            variadic,
            throws: None,
        };

        Some(Item::TestCase(TestCaseDecl {
            name,
            signature: Some(signature),
            body,
            is_async,
            doc,
            attributes,
        }))
    }
}
