use super::helpers::{
    CommonTypeAttributes, reject_flags_for_non_enum, reject_pin_for_type,
    take_common_type_attributes,
};
use crate::frontend::parser::*;

parser_impl! {
    pub(crate) fn parse_enum(
        &mut self,
        visibility: Visibility,
        doc: Option<DocComment>,
        mut attrs: CollectedAttributes,
    ) -> Option<Item> {
        reject_pin_for_type(self, &mut attrs);
        let name = self.consume_identifier("expected enum name")?;
        let mut generics = self.parse_generic_parameter_list();

        let underlying_type = if self.consume_punctuation(':') {
            self.parse_type_expr()
        } else {
            None
        };

        self.parse_where_clauses(&mut generics);

        if !self.expect_punctuation('{') {
            return None;
        }

        let variants = self.collect_enum_variants();

        if !self.expect_punctuation('}') {
            return None;
        }

        let is_flags = attrs.builtin.flags;
        let CommonTypeAttributes {
            thread_safe_override,
            shareable_override,
            copy_override,
            attributes,
        } = take_common_type_attributes(&mut attrs);

        Some(Item::Enum(EnumDecl {
            visibility,
            name,
            underlying_type,
            variants,
            thread_safe_override,
            shareable_override,
            copy_override,
            is_flags,
            doc,
            generics,
            attributes,
        }))
    }

    pub(crate) fn parse_union(
        &mut self,
        visibility: Visibility,
        doc: Option<DocComment>,
        mut attrs: CollectedAttributes,
    ) -> Option<Item> {
        reject_pin_for_type(self, &mut attrs);
        reject_flags_for_non_enum(self, &mut attrs);
        let name = self.consume_identifier("expected union name")?;
        let mut generics = self.parse_generic_parameter_list();

        self.parse_where_clauses(&mut generics);

        if !self.expect_punctuation('{') {
            return None;
        }

        let members = self.collect_union_members();

        if !self.expect_punctuation('}') {
            return None;
        }

        let CommonTypeAttributes {
            thread_safe_override,
            shareable_override,
            copy_override,
            attributes,
        } = take_common_type_attributes(&mut attrs);

        Some(Item::Union(UnionDecl {
            visibility,
            name,
            members,
            thread_safe_override,
            shareable_override,
            copy_override,
            doc,
            generics,
            attributes,
        }))
    }

    fn collect_enum_variants(&mut self) -> Vec<EnumVariant> {
        let mut variants = Vec::new();
        while !self.is_at_end() && !self.check_punctuation('}') {
            self.stash_leading_doc();
            self.skip_attributes();
            self.stash_leading_doc();
            if self.check_punctuation('}') {
                break;
            }

            if let Some(variant) = self.parse_enum_variant() {
                variants.push(variant);
            } else {
                self.synchronize_variant();
                continue;
            }

            if self.consume_punctuation(',') {
                continue;
            }

            if self.check_punctuation('}') {
                break;
            }

            self.report_missing_enum_separator();
            self.synchronize_variant();
        }
        variants
    }

    fn parse_enum_variant(&mut self) -> Option<EnumVariant> {
        let variant_doc = self.take_pending_doc();
        let name = self.consume_identifier("expected enum variant name")?;

        let fields = if self.consume_punctuation('{') {
            let parsed = self.parse_variant_fields();
            if !self.expect_punctuation('}') {
                return None;
            }
            parsed
        } else {
            Vec::new()
        };

        let discriminant = if self.check_operator("=") {
            if !fields.is_empty() {
                self.push_error(
                    "data-carrying enum variants cannot specify explicit discriminants",
                    self.peek().map(|token| token.span),
                );
                None
            } else {
                self.advance();
                Some(self.collect_expression_until(&[',', '}']))
            }
        } else {
            None
        };

        Some(EnumVariant {
            name,
            fields,
            discriminant,
            doc: variant_doc,
        })
    }

    fn report_missing_enum_separator(&mut self) {
        let span = self.peek().map(|token| token.span);
        self.push_error("expected ',' or '}' after enum variant", span);
    }

    fn parse_variant_fields(&mut self) -> Vec<FieldDecl> {
        let mut fields = Vec::new();
        while !self.is_at_end() && !self.check_punctuation('}') {
            self.stash_leading_doc();
            self.skip_attributes();
            self.stash_leading_doc();
            if self.check_punctuation('}') {
                break;
            }
            let field_doc = self.take_pending_doc();
            let field_visibility = self.parse_visibility();
            let Some(field_type) = self.parse_type_expr() else {
                self.synchronize_field();
                continue;
            };

            let Some(field_name) = self.consume_identifier("expected field name") else {
                self.synchronize_field();
                continue;
            };

            if !self.expect_punctuation(';') {
                self.synchronize_field();
                continue;
            }

            fields.push(FieldDecl {
                visibility: field_visibility,
                name: field_name,
                ty: field_type,
                initializer: None,
                doc: field_doc,
                attributes: Vec::new(),
                mmio: None,
                is_required: false,
                display_name: None,
                is_readonly: false,
                is_static: false,
                view_of: None,
            });
        }
        fields
    }
}
