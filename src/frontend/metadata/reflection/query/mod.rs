//! Query helpers for reflection descriptor construction.

use super::{ReflectionTables, TypeAliasDescriptor};
use crate::frontend::ast::items::{
    ClassDecl, ConstItemDecl, DelegateDecl, EnumDecl, ExtensionDecl, FunctionDecl, ImplDecl,
    InterfaceDecl, Item, NamespaceDecl, StaticItemDecl, StructDecl, TraitDecl, TypeAliasDecl,
    UnionDecl,
};
use crate::frontend::ast::{Module, Visibility};
use crate::frontend::type_utils::type_expr_surface;

mod consts;
mod functions;
mod helpers;
mod impls;
mod members;
mod statics;
mod types;

use consts::const_descriptors;
use functions::push_function;
use helpers::{attribute_descriptors, extend_scope, namespace_for_scope, qualify, type_handle};
use impls::{extension_descriptor, impl_descriptor};
use statics::static_descriptors;
use types::{
    class_descriptor, delegate_descriptor, enum_descriptor, interface_descriptor,
    struct_descriptor, trait_descriptor, union_descriptor,
};

pub(crate) struct DescriptorQuery;

impl DescriptorQuery {
    pub(crate) fn collect(module: &Module) -> ReflectionTables {
        let mut collector = Collector::new();
        collector.collect_module(module);
        collector.tables
    }
}

struct Collector {
    tables: ReflectionTables,
}

impl Collector {
    fn new() -> Self {
        Self {
            tables: ReflectionTables::default(),
        }
    }

    fn collect_module(&mut self, module: &Module) {
        let mut scope = Vec::new();
        if let Some(namespace) = &module.namespace {
            extend_scope(&mut scope, namespace);
        }
        self.collect_items(&module.items, &mut scope);
    }

    fn collect_items(&mut self, items: &[Item], scope: &mut Vec<String>) {
        for item in items {
            match item {
                Item::Function(func) => self.push_function(func, scope),
                Item::Struct(def) => self.push_struct(def, scope),
                Item::Union(def) => self.push_union(def, scope),
                Item::Enum(def) => self.push_enum(def, scope),
                Item::Class(def) => self.push_class(def, scope),
                Item::Interface(def) => self.push_interface(def, scope),
                Item::Delegate(def) => self.push_delegate(def, scope),
                Item::Trait(def) => self.push_trait(def, scope),
                Item::Impl(def) => self.push_impl(def, scope),
                Item::Extension(def) => self.push_extension(def, scope),
                Item::Namespace(ns) => self.push_namespace(ns, scope),
                Item::Const(decl) => self.push_const(decl, scope),
                Item::Static(decl) => self.push_static(decl, scope),
                Item::TypeAlias(alias) => self.push_type_alias(alias, scope),
                Item::TestCase(_) | Item::Import(_) => {}
            }
        }
    }

    fn push_namespace(&mut self, ns: &NamespaceDecl, scope: &mut Vec<String>) {
        let added = extend_scope(scope, &ns.name);
        self.collect_items(&ns.items, scope);
        scope.truncate(scope.len().saturating_sub(added));
    }

    fn push_function(&mut self, func: &FunctionDecl, scope: &[String]) {
        push_function(&mut self.tables, func, scope);
    }

    fn push_struct(&mut self, decl: &StructDecl, scope: &mut Vec<String>) {
        if let Some(descriptor) = struct_descriptor(decl, scope) {
            self.tables.types.push(descriptor);
        }
        let added = extend_scope(scope, &decl.name);
        self.collect_items(&decl.nested_types, scope);
        scope.truncate(scope.len().saturating_sub(added));
    }

    fn push_union(&mut self, decl: &UnionDecl, scope: &[String]) {
        if let Some(descriptor) = union_descriptor(decl, scope) {
            self.tables.types.push(descriptor);
        }
    }

    fn push_enum(&mut self, decl: &EnumDecl, scope: &[String]) {
        if let Some(descriptor) = enum_descriptor(decl, scope) {
            self.tables.types.push(descriptor);
        }
    }

    fn push_class(&mut self, decl: &ClassDecl, scope: &[String]) {
        if let Some(descriptor) = class_descriptor(decl, scope) {
            self.tables.types.push(descriptor);
        }
    }

    fn push_interface(&mut self, decl: &InterfaceDecl, scope: &[String]) {
        if let Some(descriptor) = interface_descriptor(decl, scope) {
            self.tables.types.push(descriptor);
        }
    }

    fn push_type_alias(&mut self, alias: &TypeAliasDecl, scope: &[String]) {
        if !matches!(alias.visibility, Visibility::Public | Visibility::Internal) {
            return;
        }
        let full_name = qualify(scope, &alias.name);
        let namespace = namespace_for_scope(scope);
        let target = type_expr_surface(&alias.target);
        let attributes = attribute_descriptors(&alias.attributes);
        let generic_params = alias
            .generics
            .as_ref()
            .map(|params| {
                params
                    .params
                    .iter()
                    .map(|param| param.name.clone())
                    .collect()
            })
            .unwrap_or_default();
        self.tables.aliases.push(TypeAliasDescriptor {
            namespace,
            name: alias.name.clone(),
            full_name,
            target: type_handle(&target),
            visibility: alias.visibility.into(),
            attributes,
            generic_params,
        });
    }

    fn push_delegate(&mut self, decl: &DelegateDecl, scope: &[String]) {
        if let Some(descriptor) = delegate_descriptor(decl, scope) {
            self.tables.types.push(descriptor);
        }
    }

    fn push_trait(&mut self, decl: &TraitDecl, scope: &[String]) {
        if let Some(descriptor) = trait_descriptor(decl, scope) {
            self.tables.types.push(descriptor);
        }
    }

    fn push_impl(&mut self, decl: &ImplDecl, scope: &[String]) {
        if let Some(descriptor) = impl_descriptor(decl, scope) {
            self.tables.types.push(descriptor);
        }
    }

    fn push_extension(&mut self, decl: &ExtensionDecl, scope: &[String]) {
        if let Some(descriptor) = extension_descriptor(decl, scope) {
            self.tables.types.push(descriptor);
        }
    }

    fn push_const(&mut self, decl: &ConstItemDecl, scope: &[String]) {
        self.tables.types.extend(const_descriptors(decl, scope));
    }

    fn push_static(&mut self, decl: &StaticItemDecl, scope: &[String]) {
        self.tables.types.extend(static_descriptors(decl, scope));
    }
}

#[cfg(test)]
mod tests;
