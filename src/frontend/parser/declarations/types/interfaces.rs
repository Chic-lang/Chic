use super::helpers::{CommonTypeAttributes, reject_pin_for_type, take_common_type_attributes};
use crate::frontend::parser::*;

parser_impl! {
    pub(crate) fn parse_interface(
        &mut self,
        visibility: Visibility,
        doc: Option<DocComment>,
        mut attrs: CollectedAttributes,
    ) -> Option<Item> {
        reject_pin_for_type(self, &mut attrs);
        let name = self.consume_identifier("expected interface name")?;
        let mut generics = self.parse_generic_parameter_list_allowing_variance();

        let bases = if self.consume_punctuation(':') {
            self.parse_type_list()
        } else {
            Vec::new()
        };

        self.parse_where_clauses(&mut generics);

        if !self.expect_punctuation('{') {
            return None;
        }

        let mut members = Vec::new();
        while !self.is_at_end() && !self.check_punctuation('}') {
            self.stash_leading_doc();
            if let Some(member) = self.parse_interface_member() {
                members.push(member);
            } else {
                self.synchronize_class_member();
            }
        }

        if !self.expect_punctuation('}') {
            return None;
        }

        let CommonTypeAttributes {
            thread_safe_override,
            shareable_override,
            copy_override,
            attributes,
        } = take_common_type_attributes(&mut attrs);

        Some(Item::Interface(InterfaceDecl {
            visibility,
            name,
            bases,
            members,
            thread_safe_override,
            shareable_override,
            copy_override,
            doc,
            generics,
            attributes,
        }))
    }
}
