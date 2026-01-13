//! Overload grouping metadata derived from AST items.

use super::items::{
    ClassDecl, ClassMember, ConstructorDecl, ExtensionDecl, ExtensionMember, FunctionDecl,
    GenericParams, ImplDecl, ImplMember, InterfaceMember, Item, Parameter, StructDecl, TraitMember,
    Visibility,
};
use super::{Attribute, BindingModifier, DiInjectAttr, ThrowsClause};
use crate::frontend::ast::types::TypeExpr;
use crate::frontend::diagnostics::Span;
use std::collections::HashMap;

/// Classifies the kind of overload set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverloadKind {
    /// Free/namespace function.
    Function,
    /// Method associated with a type (class/struct/impl/extension).
    Method,
    /// Constructor overload for a type.
    Constructor,
}

/// Lookup key for an overload set.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OverloadKey {
    pub owner: Option<String>,
    pub name: String,
    pub kind: OverloadKind,
}

impl OverloadKey {
    #[must_use]
    pub fn function(owner: Option<String>, name: impl Into<String>) -> Self {
        Self {
            owner,
            name: name.into(),
            kind: OverloadKind::Function,
        }
    }

    #[must_use]
    pub fn method(owner: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            owner: Some(owner.into()),
            name: name.into(),
            kind: OverloadKind::Method,
        }
    }

    #[must_use]
    pub fn constructor(owner: impl Into<String>) -> Self {
        Self {
            owner: Some(owner.into()),
            name: "init".into(),
            kind: OverloadKind::Constructor,
        }
    }
}

/// Canonical collection of overload sets for a module.
#[derive(Debug, Clone, Default)]
pub struct OverloadCatalog {
    sets: Vec<OverloadSet>,
    index: HashMap<OverloadKey, usize>,
}

impl OverloadCatalog {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn sets(&self) -> &[OverloadSet] {
        &self.sets
    }

    #[must_use]
    pub fn get(&self, key: &OverloadKey) -> Option<&OverloadSet> {
        self.index.get(key).map(|&idx| &self.sets[idx])
    }

    #[must_use]
    pub fn iter(&self) -> impl Iterator<Item = &OverloadSet> {
        self.sets.iter()
    }

    pub(crate) fn from_module(namespace: Option<&str>, items: &[Item]) -> Self {
        let mut catalog = Self::new();
        catalog.collect_items(namespace.map(str::to_string), items);
        catalog
    }

    fn collect_items(&mut self, namespace: Option<String>, items: &[Item]) {
        let namespace_ref = namespace.as_deref();
        for (index, item) in items.iter().enumerate() {
            match item {
                Item::Function(func) => {
                    let owner = namespace.clone();
                    let qualified = qualify(owner.as_deref(), &func.name);
                    self.insert_entry(
                        OverloadKey::function(
                            owner.clone().map(|s| canonical_owner(&s)),
                            func.name.clone(),
                        ),
                        OverloadEntry::from_function(func, qualified, index),
                    );
                }
                Item::Struct(strct) => {
                    let owner = qualify(namespace_ref, &strct.name);
                    self.collect_struct(strct, &owner, index);
                }
                Item::Class(class) => {
                    let owner = qualify(namespace_ref, &class.name);
                    self.collect_class(class, &owner, index);
                }
                Item::Impl(impl_decl) => {
                    self.collect_impl(namespace_ref, impl_decl, index);
                }
                Item::Extension(ext) => {
                    self.collect_extension(namespace_ref, ext, index);
                }
                Item::Namespace(ns_decl) => {
                    let child = qualify(namespace_ref, &ns_decl.name);
                    self.collect_items(Some(child), &ns_decl.items);
                }
                Item::Trait(trait_decl) => {
                    let owner = qualify(namespace_ref, &trait_decl.name);
                    self.collect_trait_methods(
                        &owner,
                        &trait_decl.generics,
                        &trait_decl.members,
                        index,
                    );
                }
                Item::Interface(iface) => {
                    let owner = qualify(namespace_ref, &iface.name);
                    self.collect_interface_methods(&owner, &iface.generics, &iface.members, index);
                }
                Item::TestCase(_)
                | Item::Import(_)
                | Item::Enum(_)
                | Item::Union(_)
                | Item::Delegate(_)
                | Item::Const(_)
                | Item::Static(_)
                | Item::TypeAlias(_) => {}
            }
        }
    }

    fn collect_struct(&mut self, strct: &StructDecl, owner: &str, item_index: usize) {
        for (method_index, method) in strct.methods.iter().enumerate() {
            let qualified = format!("{owner}::{}", method.name);
            self.insert_entry(
                OverloadKey::method(canonical_owner(owner), method.name.clone()),
                OverloadEntry::from_method(
                    method,
                    qualified,
                    OverloadDeclaration::StructMethod {
                        item_index,
                        method_index,
                    },
                ),
            );
        }
        for (ctor_index, ctor) in strct.constructors.iter().enumerate() {
            let qualified = format!("{owner}::init#{ctor_index}");
            self.insert_entry(
                OverloadKey::constructor(canonical_owner(owner)),
                OverloadEntry::from_constructor(
                    ctor,
                    qualified,
                    owner.to_string(),
                    OverloadDeclaration::StructConstructor {
                        item_index,
                        ctor_index,
                    },
                ),
            );
        }
        if !strct.nested_types.is_empty() {
            self.collect_items(Some(owner.to_string()), &strct.nested_types);
        }
    }

    fn collect_class(&mut self, class: &ClassDecl, owner: &str, item_index: usize) {
        for (member_index, member) in class.members.iter().enumerate() {
            match member {
                ClassMember::Method(method) => {
                    let qualified = format!("{owner}::{}", method.name);
                    self.insert_entry(
                        OverloadKey::method(canonical_owner(owner), method.name.clone()),
                        OverloadEntry::from_method(
                            method,
                            qualified,
                            OverloadDeclaration::ClassMethod {
                                item_index,
                                member_index,
                            },
                        ),
                    );
                }
                ClassMember::Constructor(ctor) => {
                    let qualified = format!("{owner}::init#{member_index}");
                    self.insert_entry(
                        OverloadKey::constructor(canonical_owner(owner)),
                        OverloadEntry::from_constructor(
                            ctor,
                            qualified,
                            owner.to_string(),
                            OverloadDeclaration::ClassConstructor {
                                item_index,
                                member_index,
                            },
                        ),
                    );
                }
                ClassMember::Field(_) | ClassMember::Property(_) | ClassMember::Const(_) => {}
            }
        }
    }

    fn collect_impl(&mut self, namespace: Option<&str>, impl_decl: &ImplDecl, item_index: usize) {
        let target_name = type_expr_name(&impl_decl.target);
        let owner = if target_name.contains("::") {
            target_name
        } else {
            qualify(namespace, &target_name)
        };
        for (member_index, member) in impl_decl.members.iter().enumerate() {
            if let ImplMember::Method(method) = member {
                let qualified = format!("{owner}::{}", method.name);
                self.insert_entry(
                    OverloadKey::method(canonical_owner(&owner), method.name.clone()),
                    OverloadEntry::from_method(
                        method,
                        qualified,
                        OverloadDeclaration::ImplMethod {
                            item_index,
                            member_index,
                        },
                    ),
                );
            }
        }
    }

    fn collect_extension(
        &mut self,
        namespace: Option<&str>,
        ext: &ExtensionDecl,
        item_index: usize,
    ) {
        let target_name = type_expr_name(&ext.target);
        let owner = if target_name.contains("::") {
            target_name
        } else {
            qualify(namespace, &target_name)
        };
        for (member_index, member) in ext.members.iter().enumerate() {
            match member {
                ExtensionMember::Method(method) => {
                    let qualified = format!("{owner}::{}", method.function.name);
                    self.insert_entry(
                        OverloadKey::method(canonical_owner(&owner), method.function.name.clone()),
                        OverloadEntry::from_method(
                            &method.function,
                            qualified,
                            OverloadDeclaration::ExtensionMethod {
                                item_index,
                                member_index,
                            },
                        ),
                    );
                }
            }
        }
    }

    fn collect_trait_methods(
        &mut self,
        owner: &str,
        generics: &Option<GenericParams>,
        members: &[TraitMember],
        item_index: usize,
    ) {
        for (member_index, member) in members.iter().enumerate() {
            if let crate::frontend::ast::items::TraitMember::Method(method) = member {
                let qualified = format!("{owner}::{}", method.name);
                self.insert_entry(
                    OverloadKey::method(canonical_owner(owner), method.name.clone()),
                    OverloadEntry::from_trait_method(
                        method,
                        qualified,
                        generics.clone(),
                        OverloadDeclaration::ImplMethod {
                            item_index,
                            member_index,
                        },
                    ),
                );
            }
        }
    }

    fn collect_interface_methods(
        &mut self,
        owner: &str,
        generics: &Option<GenericParams>,
        members: &[InterfaceMember],
        item_index: usize,
    ) {
        for (member_index, member) in members.iter().enumerate() {
            if let crate::frontend::ast::items::InterfaceMember::Method(method) = member {
                let qualified = format!("{owner}::{}", method.name);
                self.insert_entry(
                    OverloadKey::method(canonical_owner(owner), method.name.clone()),
                    OverloadEntry::from_trait_method(
                        method,
                        qualified,
                        generics.clone(),
                        OverloadDeclaration::ImplMethod {
                            item_index,
                            member_index,
                        },
                    ),
                );
            }
        }
    }

    fn insert_entry(&mut self, key: OverloadKey, entry: OverloadEntry) {
        if let Some(index) = self.index.get(&key).copied() {
            self.sets[index].entries.push(entry);
        } else {
            let index = self.sets.len();
            self.sets.push(OverloadSet {
                key: key.clone(),
                entries: vec![entry],
            });
            self.index.insert(key, index);
        }
    }
}

/// All overloads for a single key.
#[derive(Debug, Clone)]
pub struct OverloadSet {
    pub key: OverloadKey,
    pub entries: Vec<OverloadEntry>,
}

/// Specific overload entry metadata.
#[derive(Debug, Clone)]
pub struct OverloadEntry {
    pub qualified: String,
    pub span: Option<Span>,
    pub signature: OverloadSignatureSummary,
    pub declaration: OverloadDeclaration,
}

impl OverloadEntry {
    fn from_function(func: &FunctionDecl, qualified: String, item_index: usize) -> Self {
        Self {
            span: func.body.as_ref().and_then(|body| body.span),
            signature: OverloadSignatureSummary::from_function(func),
            declaration: OverloadDeclaration::Function { item_index },
            qualified,
        }
    }

    fn from_method(
        method: &FunctionDecl,
        qualified: String,
        declaration: OverloadDeclaration,
    ) -> Self {
        Self {
            span: method.body.as_ref().and_then(|body| body.span),
            signature: OverloadSignatureSummary::from_method(method),
            declaration,
            qualified,
        }
    }

    fn from_trait_method(
        method: &FunctionDecl,
        qualified: String,
        container_generics: Option<GenericParams>,
        declaration: OverloadDeclaration,
    ) -> Self {
        let mut signature = OverloadSignatureSummary::from_method(method);
        if signature.generics.is_none() {
            signature.generics = container_generics;
        }
        Self {
            span: method.body.as_ref().and_then(|body| body.span),
            signature,
            declaration,
            qualified,
        }
    }

    fn from_constructor(
        ctor: &ConstructorDecl,
        qualified: String,
        owner: String,
        declaration: OverloadDeclaration,
    ) -> Self {
        Self {
            span: ctor.span,
            signature: OverloadSignatureSummary::from_constructor(ctor, owner),
            declaration,
            qualified,
        }
    }
}

/// Kind of declaration backing an overload entry.
#[derive(Debug, Clone)]
pub enum OverloadDeclaration {
    Function {
        item_index: usize,
    },
    StructMethod {
        item_index: usize,
        method_index: usize,
    },
    StructConstructor {
        item_index: usize,
        ctor_index: usize,
    },
    ClassMethod {
        item_index: usize,
        member_index: usize,
    },
    ClassConstructor {
        item_index: usize,
        member_index: usize,
    },
    ImplMethod {
        item_index: usize,
        member_index: usize,
    },
    ExtensionMethod {
        item_index: usize,
        member_index: usize,
    },
}

/// Signature summary captured for overload analysis.
#[derive(Debug, Clone)]
pub struct OverloadSignatureSummary {
    pub visibility: Visibility,
    pub is_async: bool,
    pub is_constexpr: bool,
    pub is_extern: bool,
    pub extern_abi: Option<String>,
    pub is_unsafe: bool,
    pub is_static: bool,
    pub is_operator: bool,
    pub parameters: Vec<ParameterSummary>,
    pub return_type: Option<TypeExpr>,
    pub throws: Option<ThrowsClause>,
    pub generics: Option<GenericParams>,
    pub attribute_names: Vec<String>,
    pub modifier_names: Vec<String>,
}

impl OverloadSignatureSummary {
    fn from_function(func: &FunctionDecl) -> Self {
        Self {
            visibility: func.visibility,
            is_async: func.is_async,
            is_constexpr: func.is_constexpr,
            is_extern: func.is_extern,
            extern_abi: func.extern_abi.clone(),
            is_unsafe: func.is_unsafe,
            is_static: func
                .modifiers
                .iter()
                .any(|modifier| modifier.eq_ignore_ascii_case("static")),
            is_operator: func.operator.is_some(),
            parameters: func
                .signature
                .parameters
                .iter()
                .map(ParameterSummary::from_parameter)
                .collect(),
            return_type: Some(func.signature.return_type.clone()),
            throws: func.signature.throws.clone(),
            generics: func.generics.clone(),
            attribute_names: attribute_names(&func.attributes),
            modifier_names: func.modifiers.clone(),
        }
    }

    fn from_method(method: &FunctionDecl) -> Self {
        let mut summary = Self::from_function(method);
        summary.is_static = summary
            .modifier_names
            .iter()
            .any(|modifier| modifier.eq_ignore_ascii_case("static"));
        summary
    }

    fn from_constructor(ctor: &ConstructorDecl, owner: String) -> Self {
        Self {
            visibility: ctor.visibility,
            is_async: false,
            is_constexpr: false,
            is_extern: false,
            extern_abi: None,
            is_unsafe: false,
            is_static: true,
            is_operator: false,
            parameters: ctor
                .parameters
                .iter()
                .map(ParameterSummary::from_parameter)
                .collect(),
            return_type: Some(TypeExpr::simple(owner)),
            throws: None,
            generics: None,
            attribute_names: attribute_names(&ctor.attributes),
            modifier_names: Vec::new(),
        }
    }
}

/// Summary of a single parameter within an overload signature.
#[derive(Debug, Clone)]
pub struct ParameterSummary {
    pub binding: BindingModifier,
    pub binding_nullable: bool,
    pub name: String,
    pub ty: TypeExpr,
    pub has_default: bool,
    pub attribute_names: Vec<String>,
    pub di_inject: Option<DiInjectAttr>,
}

impl ParameterSummary {
    fn from_parameter(param: &Parameter) -> Self {
        Self {
            binding: param.binding,
            binding_nullable: param.binding_nullable,
            name: param.name.clone(),
            ty: param.ty.clone(),
            has_default: param.default.is_some(),
            attribute_names: attribute_names(&param.attributes),
            di_inject: param.di_inject.clone(),
        }
    }
}

fn attribute_names(attrs: &[Attribute]) -> Vec<String> {
    attrs.iter().map(|attr| attr.name.clone()).collect()
}

fn type_expr_name(expr: &TypeExpr) -> String {
    if expr.base.is_empty() {
        expr.name.replace('.', "::")
    } else {
        let mut parts = expr.base.clone();
        let canonical = expr.name.replace('.', "::");
        if parts
            .last()
            .is_some_and(|segment| segment.replace('.', "::") == canonical)
        {
            parts.join("::")
        } else {
            parts.push(canonical);
            parts.join("::")
        }
    }
}

fn qualify(namespace: Option<&str>, name: &str) -> String {
    match namespace {
        Some(prefix) if !prefix.is_empty() => {
            let mut prefix_parts: Vec<String> = prefix
                .replace("::", ".")
                .split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect();
            let mut name_parts: Vec<String> = name
                .replace("::", ".")
                .split('.')
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect();
            if name_parts.is_empty() {
                prefix_parts.join("::")
            } else if !prefix_parts.is_empty()
                && name_parts.len() >= prefix_parts.len()
                && name_parts[..prefix_parts.len()] == prefix_parts[..]
            {
                name_parts.join("::")
            } else {
                prefix_parts.append(&mut name_parts);
                prefix_parts.join("::")
            }
        }
        _ => name.to_string(),
    }
}

fn canonical_owner(owner: &str) -> String {
    strip_generic_arguments(owner)
}

fn strip_generic_arguments(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut depth = 0i32;
    for ch in name.chars() {
        match ch {
            '<' => depth += 1,
            '>' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            _ if depth == 0 => result.push(ch),
            _ => {}
        }
    }
    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::MemberDispatch;
    use crate::frontend::ast::Module;
    use crate::frontend::ast::expressions::{Block, Expression};
    use crate::frontend::ast::items::{
        Attribute, AttributeKind, ClassDecl, ClassKind, ClassMember, ConstructorDecl,
        ConstructorKind, ExtensionDecl, ExtensionMember, ExtensionMethodDecl, FunctionDecl, Item,
        Parameter, Signature, StructDecl,
    };

    fn simple_function(name: &str) -> FunctionDecl {
        FunctionDecl {
            visibility: Visibility::Public,
            name: name.to_string(),
            name_span: None,
            signature: Signature {
                parameters: Vec::new(),
                return_type: TypeExpr::simple("void"),
                lends_to_return: None,
                variadic: false,
                throws: None,
            },
            body: Some(Block {
                statements: Vec::new(),
                span: None,
            }),
            is_async: false,
            is_constexpr: false,
            doc: None,
            modifiers: Vec::new(),
            is_unsafe: false,
            attributes: Vec::new(),
            is_extern: false,
            extern_abi: None,
            extern_options: None,
            link_name: None,
            link_library: None,
            operator: None,
            generics: None,
            vectorize_hint: None,
            dispatch: MemberDispatch::default(),
        }
    }

    #[test]
    fn collects_free_function_overloads() {
        let mut module = Module::new(Some("Root.Math".into()));
        module.items = vec![
            Item::Function(simple_function("Add")),
            Item::Function(simple_function("Add")),
        ];
        module.rebuild_overloads();
        let catalog = module.overloads();
        assert_eq!(catalog.sets().len(), 1);
        let set = &catalog.sets()[0];
        assert_eq!(set.entries.len(), 2);
        assert_eq!(set.key.kind, OverloadKind::Function);
        assert_eq!(set.key.name, "Add");
        assert_eq!(set.key.owner.as_deref(), Some("Root.Math"));
    }

    #[test]
    fn records_method_and_constructor_metadata() {
        let mut method = simple_function("Compute");
        method.modifiers = vec!["static".into()];
        method.signature.parameters = vec![
            Parameter {
                binding: BindingModifier::Ref,
                binding_nullable: false,
                name: "source".into(),
                name_span: None,
                ty: TypeExpr::simple("Span<byte>"),
                attributes: vec![],
                di_inject: None,
                default: None,
                default_span: None,

                lends: None,
                is_extension_this: false,
            },
            Parameter {
                binding: BindingModifier::Value,
                binding_nullable: false,
                name: "offset".into(),
                name_span: None,
                ty: TypeExpr::simple("int"),
                attributes: vec![Attribute::new(
                    "inline_hint",
                    Vec::new(),
                    None,
                    None,
                    AttributeKind::Builtin,
                )],
                di_inject: None,
                default: Some(Expression::new("42", None)),
                default_span: None,

                lends: None,
                is_extension_this: false,
            },
        ];
        method.attributes.push(Attribute::new(
            "inline",
            Vec::new(),
            None,
            None,
            AttributeKind::Builtin,
        ));

        let constructor = ConstructorDecl {
            visibility: Visibility::Public,
            kind: ConstructorKind::Designated,
            parameters: vec![Parameter {
                binding: BindingModifier::In,
                binding_nullable: false,
                name: "allocator".into(),
                name_span: None,
                ty: TypeExpr::simple("IAllocator"),
                attributes: vec![Attribute::new(
                    "inject",
                    Vec::new(),
                    None,
                    None,
                    AttributeKind::Builtin,
                )],
                di_inject: None,
                default: Some(Expression::new("DefaultAllocator.Instance", None)),
                default_span: None,

                lends: None,
                is_extension_this: false,
            }],
            body: None,
            initializer: None,
            doc: None,
            span: None,
            attributes: vec![Attribute::new(
                "service_ctor",
                Vec::new(),
                None,
                None,
                AttributeKind::Builtin,
            )],
            di_inject: None,
        };

        let class = ClassDecl {
            visibility: Visibility::Public,
            kind: ClassKind::Class,
            name: "Widget".into(),
            bases: Vec::new(),
            members: vec![
                ClassMember::Method(method.clone()),
                ClassMember::Constructor(constructor.clone()),
            ],
            nested_types: Vec::new(),
            thread_safe_override: None,
            shareable_override: None,
            copy_override: None,
            doc: None,
            generics: None,
            attributes: Vec::new(),
            di_service: None,
            di_module: false,
            is_static: false,
            is_abstract: false,
            is_sealed: false,
        };

        let mut module = Module::with_items(Some("Models".into()), vec![Item::Class(class)]);
        module.rebuild_overloads();
        let catalog = module.overloads();

        let method_key = OverloadKey::method("Models::Widget", "Compute");
        let method_set = catalog
            .get(&method_key)
            .expect("missing method overload set");
        assert_eq!(method_set.entries.len(), 1);
        let method_entry = &method_set.entries[0];
        assert_eq!(method_entry.qualified, "Models::Widget::Compute");
        assert!(method_entry.signature.is_static);
        assert_eq!(method_entry.signature.attribute_names, vec!["inline"]);
        assert_eq!(method_entry.signature.parameters.len(), 2);
        assert!(!method_entry.signature.parameters[0].has_default);
        assert!(method_entry.signature.parameters[1].has_default);
        assert_eq!(
            method_entry.signature.parameters[1].attribute_names,
            vec!["inline_hint"]
        );

        let ctor_key = OverloadKey::constructor("Models::Widget");
        let ctor_set = catalog
            .get(&ctor_key)
            .expect("missing constructor overload set");
        assert_eq!(ctor_set.entries.len(), 1);
        let ctor_entry = &ctor_set.entries[0];
        assert!(ctor_entry.signature.is_static);
        assert_eq!(ctor_entry.signature.attribute_names, vec!["service_ctor"]);
        assert_eq!(ctor_entry.signature.parameters.len(), 1);
        let ctor_param = &ctor_entry.signature.parameters[0];
        assert!(ctor_param.has_default);
        assert_eq!(ctor_param.attribute_names, vec!["inject"]);
    }

    #[test]
    fn groups_overloads_from_impls_and_extensions() {
        let mut module = Module::new(Some("Samples".into()));
        module.items.push(Item::Struct(StructDecl {
            visibility: Visibility::Public,
            name: "Vec2".into(),
            fields: Vec::new(),
            properties: Vec::new(),
            constructors: Vec::new(),
            consts: Vec::new(),
            methods: vec![simple_function("Length")],
            nested_types: Vec::new(),
            bases: Vec::new(),
            thread_safe_override: None,
            shareable_override: None,
            copy_override: None,
            mmio: None,
            doc: None,
            generics: None,
            attributes: Vec::new(),
            is_readonly: false,
            layout: None,
            is_intrinsic: false,
            inline_attr: None,
            is_record: false,
            record_positional_fields: Vec::new(),
        }));
        module.items.push(Item::Extension(ExtensionDecl {
            visibility: Visibility::Public,
            target: TypeExpr::simple("Vec2"),
            generics: None,
            members: vec![ExtensionMember::Method(ExtensionMethodDecl {
                function: simple_function("Length"),
                is_default: false,
            })],
            doc: None,
            attributes: Vec::new(),
            conditions: Vec::new(),
        }));
        module.rebuild_overloads();
        let catalog = module.overloads();
        let method_set = catalog
            .sets()
            .iter()
            .find(|set| set.key.kind == OverloadKind::Method && set.key.name == "Length")
            .expect("expected Vec2::Length overloads");
        assert_eq!(method_set.entries.len(), 2);
        assert!(
            method_set
                .entries
                .iter()
                .any(|entry| matches!(entry.declaration, OverloadDeclaration::StructMethod { .. }))
        );
        assert!(method_set.entries.iter().any(|entry| matches!(
            entry.declaration,
            OverloadDeclaration::ExtensionMethod { .. }
        )));
    }
}
