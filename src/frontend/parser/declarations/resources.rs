use super::super::*;
use crate::frontend::ast::ExtensionCondition;

parser_impl! {
    pub(crate) fn parse_extension(
        &mut self,
        visibility: Visibility,
        doc: Option<DocComment>,
    ) -> Option<Item> {
        let mut generics = self.parse_generic_parameter_list();
        let target = self.parse_type_expr()?;

        self.parse_where_clauses(&mut generics);

        let conditions = if self.match_keyword(Keyword::When) {
            self.parse_extension_conditions()?
        } else {
            Vec::new()
        };

        if !self.expect_punctuation('{') {
            return None;
        }

        let mut members = Vec::new();
        while !self.is_at_end() && !self.check_punctuation('}') {
            self.stash_leading_doc();
            if let Some(member) = self.parse_extension_member() {
                members.push(member);
            } else {
                self.synchronize_class_member();
            }
        }

        if !self.expect_punctuation('}') {
            return None;
        }

        Some(Item::Extension(ExtensionDecl {
            visibility,
            target,
            generics,
            members,
            doc,
            attributes: Vec::new(),
            conditions,
        }))
    }

    fn parse_extension_conditions(&mut self) -> Option<Vec<ExtensionCondition>> {
        let mut conditions = Vec::new();
        loop {
            let start = self.peek().map(|token| token.span.start);
            let target = self.parse_type_expr()?;
            if !self.expect_punctuation(':') {
                return None;
            }

            let constraint = self.parse_type_expr()?;
            conditions.push(ExtensionCondition {
                target,
                constraint,
                span: self.make_span(start),
            });

            if !self.consume_punctuation(',') {
                break;
            }
        }
        Some(conditions)
    }
}
