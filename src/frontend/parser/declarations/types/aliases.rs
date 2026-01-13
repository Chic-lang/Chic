use super::helpers::reject_flags_for_non_enum;
use crate::frontend::ast::TypeAliasDecl;
use crate::frontend::parser::*;

parser_impl! {
    pub(crate) fn parse_type_alias(
        &mut self,
        visibility: Visibility,
        doc: Option<DocComment>,
        mut attrs: CollectedAttributes,
        modifiers: &[Modifier],
    ) -> Option<Item> {
        reject_flags_for_non_enum(self, &mut attrs);
        if !modifiers.is_empty() {
            for modifier in modifiers {
                self.push_error(
                    format!(
                        "modifier `{}` is not supported on type aliases",
                        modifier.name
                    ),
                    Some(modifier.span),
                );
            }
        }

        let attributes = attrs.take_list();
        if !attrs.is_empty() {
            self.report_attribute_misuse(
                attrs,
                "unsupported attributes on type aliases",
            );
        }

        let start = self.peek().map(|token| token.span.start);
        let name = self.consume_identifier("expected type alias name")?;
        let mut generics = self.parse_generic_parameter_list();
        self.parse_where_clauses(&mut generics);
        if !self.consume_operator("=") {
            self.push_error("expected `=` after type alias name", self.last_span);
            return None;
        }
        let target = self.parse_type_expr()?;
        if !self.expect_punctuation(';') {
            return None;
        }

        Some(Item::TypeAlias(TypeAliasDecl {
            visibility,
            name,
            target,
            generics,
            attributes,
            doc,
            span: self.make_span(start),
        }))
    }
}
