use super::helpers::reject_pin_for_type;
use crate::frontend::ast::TraitDecl;
use crate::frontend::parser::*;

parser_impl! {
    pub(crate) fn parse_trait(
        &mut self,
        visibility: Visibility,
        doc: Option<DocComment>,
        mut attrs: CollectedAttributes,
    ) -> Option<Item> {
        reject_pin_for_type(self, &mut attrs);
        let flags = &mut attrs.builtin;
        let start = self.last_span.map(|span| span.start);
        let name = self.consume_identifier("expected trait name")?;
        let mut generics = self.parse_generic_parameter_list();

        let super_traits = if self.consume_punctuation(':') {
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
            if let Some(member) = self.parse_trait_member() {
                members.push(member);
            } else {
                self.synchronize_class_member();
            }
        }

        if !self.expect_punctuation('}') {
            return None;
        }

        let thread_safe_override = match flags.thread_safe {
            Some(true) => Some(true),
            Some(false) => {
                self.push_error(
                    "`@not_thread_safe` attribute is not supported on trait declarations",
                    flags.thread_safe_span,
                );
                None
            }
            None => None,
        };
        let shareable_override = match flags.shareable {
            Some(true) => Some(true),
            Some(false) => {
                self.push_error(
                    "`@not_shareable` attribute is not supported on trait declarations",
                    flags.shareable_span,
                );
                None
            }
            None => None,
        };
        let copy_override = match flags.copy {
            Some(true) => Some(true),
            Some(false) => {
                self.push_error(
                    "`@copy(false)` attribute is not supported on trait declarations",
                    flags.copy_span,
                );
                None
            }
            None => None,
        };
        let attributes = attrs.take_list();

        Some(Item::Trait(TraitDecl {
            visibility,
            name,
            super_traits,
            members,
            thread_safe_override,
            shareable_override,
            copy_override,
            doc,
            generics,
            attributes,
            span: self.make_span(start),
        }))
    }
}
