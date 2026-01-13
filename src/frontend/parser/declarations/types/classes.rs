use super::helpers::{CommonTypeAttributes, reject_pin_for_type, take_common_type_attributes};
use crate::frontend::parser::*;

parser_impl! {
    pub(crate) fn parse_class(
        &mut self,
        visibility: Visibility,
        modifiers: &[Modifier],
        doc: Option<DocComment>,
        attrs: CollectedAttributes,
    ) -> Option<Item> {
        self.parse_class_with_kind(
            visibility,
            modifiers,
            doc,
            attrs,
            ClassKind::Class,
        )
    }

    pub(crate) fn parse_error(
        &mut self,
        visibility: Visibility,
        modifiers: &[Modifier],
        doc: Option<DocComment>,
        attrs: CollectedAttributes,
    ) -> Option<Item> {
        self.parse_class_with_kind(
            visibility,
            modifiers,
            doc,
            attrs,
            ClassKind::Error,
        )
    }

    fn parse_class_with_kind(
        &mut self,
        visibility: Visibility,
        modifiers: &[Modifier],
        doc: Option<DocComment>,
        mut attrs: CollectedAttributes,
        kind: ClassKind,
    ) -> Option<Item> {
        reject_pin_for_type(self, &mut attrs);
        let class_is_static = modifiers
            .iter()
            .any(|modifier| modifier.name.eq_ignore_ascii_case("static"));
        let is_abstract = modifiers
            .iter()
            .any(|modifier| modifier.name.eq_ignore_ascii_case("abstract"));
        let is_sealed = modifiers
            .iter()
            .any(|modifier| modifier.name.eq_ignore_ascii_case("sealed"));

        let name = self.consume_identifier("expected class name")?;
        let mut generics = self.parse_generic_parameter_list();

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
        let mut nested_types = Vec::new();
        while !self.is_at_end() && !self.check_punctuation('}') {
            self.stash_leading_doc();
            let nested_before = nested_types.len();
            if let Some(member) =
                self.parse_class_member(&name, class_is_static, &mut nested_types)
            {
                members.push(member);
            } else if nested_types.len() == nested_before {
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
        if std::env::var("CHIC_DEBUG_NESTED_TYPES").is_ok() {
            let nested_names = nested_types
                .iter()
                .map(|item| match item {
                    Item::Class(class) => class.name.clone(),
                    Item::Struct(strct) => strct.name.clone(),
                    Item::Enum(enm) => enm.name.clone(),
                    Item::Interface(iface) => iface.name.clone(),
                    Item::Trait(trait_decl) => trait_decl.name.clone(),
                    Item::Delegate(delegate) => delegate.name.clone(),
                    Item::TypeAlias(alias) => alias.name.clone(),
                    Item::Extension(_) => "<extension>".to_string(),
                    Item::Impl(_) => "<impl>".to_string(),
                    Item::Const(_) => "<const>".to_string(),
                    Item::Function(func) => func.name.clone(),
                    Item::Static(_) => "<static>".to_string(),
                    Item::TestCase(test) => test.name.clone(),
                    Item::Namespace(ns) => ns.name.clone(),
                    Item::Union(union_decl) => union_decl.name.clone(),
                    Item::Import(_) => "<import>".to_string(),
                })
                .collect::<Vec<_>>();
            eprintln!(
                "[chic-debug] parsed class `{name}` nested types: {nested_names:?}"
            );
        }
        Some(Item::Class(ClassDecl {
            visibility,
            kind,
            name,
            bases,
            members,
            nested_types,
            thread_safe_override,
            shareable_override,
            copy_override,
            doc,
            generics,
            attributes,
            di_service: None,
            di_module: false,
            is_static: class_is_static,
            is_abstract,
            is_sealed,
        }))
    }
}
