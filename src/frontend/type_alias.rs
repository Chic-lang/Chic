use crate::frontend::ast::items::TypeAliasDecl;
use crate::frontend::ast::{Attribute, DocComment, GenericParams, Item, Module, Visibility};
use crate::frontend::diagnostics::Span;
use crate::frontend::type_utils::qualify_name;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TypeAlias {
    pub name: String,
    pub namespace: Option<String>,
    pub span: Option<Span>,
    pub visibility: Visibility,
    pub target: crate::frontend::ast::TypeExpr,
    pub generics: Option<GenericParams>,
    pub attributes: Vec<Attribute>,
    pub doc: Option<DocComment>,
}

#[derive(Debug, Clone, Default)]
pub struct TypeAliasRegistry {
    aliases: HashMap<String, TypeAlias>,
    duplicates: HashMap<String, Vec<TypeAlias>>,
}

impl TypeAliasRegistry {
    #[must_use]
    pub fn collect(module: &Module) -> Self {
        let mut registry = Self::default();
        registry.collect_items(&module.items, module.namespace.as_deref());
        registry
    }

    fn collect_items(&mut self, items: &[Item], namespace: Option<&str>) {
        for item in items {
            match item {
                Item::Namespace(ns) => {
                    let nested = qualify_name(namespace, &ns.name);
                    self.collect_items(&ns.items, Some(&nested));
                }
                Item::Struct(strct) => {
                    let nested = qualify_name(namespace, &strct.name);
                    self.collect_items(&strct.nested_types, Some(&nested));
                }
                Item::Class(class) => {
                    let nested = qualify_name(namespace, &class.name);
                    self.collect_items(&class.nested_types, Some(&nested));
                }
                Item::TypeAlias(alias) => {
                    self.record_alias(alias, namespace);
                }
                _ => {}
            }
        }
    }

    fn record_alias(&mut self, alias: &TypeAliasDecl, namespace: Option<&str>) {
        let qualified = qualify_name(namespace, &alias.name);
        let entry = TypeAlias {
            name: qualified.clone(),
            namespace: namespace.map(str::to_string),
            span: alias.span,
            visibility: alias.visibility,
            target: alias.target.clone(),
            generics: alias.generics.clone(),
            attributes: alias.attributes.clone(),
            doc: alias.doc.clone(),
        };
        if let Some(existing) = self.aliases.get(&qualified) {
            self.duplicates
                .entry(qualified)
                .or_default()
                .push(existing.clone());
            self.duplicates
                .entry(entry.name.clone())
                .or_default()
                .push(entry);
        } else {
            self.aliases.insert(entry.name.clone(), entry);
        }
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&TypeAlias> {
        self.aliases.get(name)
    }

    #[must_use]
    pub fn iter(&self) -> impl Iterator<Item = (&String, &TypeAlias)> {
        self.aliases.iter()
    }

    #[must_use]
    pub fn duplicates(&self) -> &HashMap<String, Vec<TypeAlias>> {
        &self.duplicates
    }
}
