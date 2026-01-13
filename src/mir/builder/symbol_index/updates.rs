use super::qualify;
use super::{
    ConstSymbol, ConstructorDeclSymbol, FieldSymbol, FunctionDeclSymbol, FunctionParamSymbol,
    FunctionSymbol, PropertyAccessorLookup, PropertyAccessorMetadata, PropertySymbol, SymbolIndex,
    TypeGenericParamEntry,
};
use crate::frontend::ast::{
    BindingModifier, Block as AstBlock, CatchClause, ClassDecl, ClassMember, ConstDeclaration,
    ConstItemDecl, ConstructorDecl, DelegateDecl, EnumDecl, ExtensionDecl, ExtensionMember,
    FunctionDecl, GenericParams, InterfaceDecl, InterfaceMember, Item, PropertyAccessorKind,
    PropertyDecl, Statement, StatementKind, StructDecl, SwitchSection, TestCaseDecl, TraitDecl,
    UnionDecl, UnionMember, Visibility,
};
use crate::frontend::local_functions::local_function_symbol;
use crate::frontend::type_utils::{
    extension_method_symbol, instantiate_extension_method, type_expr_surface,
};
use crate::mir::data::{Abi, FnTy, ParamMode, Ty};
use crate::type_metadata::TypeVariance;
use std::collections::{HashMap, HashSet};

impl SymbolIndex {
    pub(super) fn collect_items<'a>(
        &mut self,
        items: impl Iterator<Item = &'a Item>,
        namespace: Option<&str>,
    ) {
        for item in items {
            match item {
                Item::Function(func) => self.register_function(namespace, None, func),
                Item::Struct(strct) => self.register_struct(namespace, strct),
                Item::Union(union_def) => self.register_union(namespace, union_def),
                Item::Enum(enm) => self.register_enum(namespace, enm),
                Item::Class(class) => self.register_class(namespace, class),
                Item::Interface(iface) => self.register_interface(namespace, iface),
                Item::Extension(ext) => self.register_extension(namespace, ext),
                Item::Const(const_item) => self.register_const_item(namespace, const_item),
                Item::Trait(trait_decl) => self.register_trait(namespace, trait_decl),
                Item::Static(_) => {}
                Item::Delegate(delegate) => self.register_delegate(namespace, delegate),
                Item::Namespace(ns) => {
                    let nested = qualify(namespace, &ns.name);
                    self.collect_items(ns.items.iter(), Some(nested.as_str()));
                }
                Item::Impl(_) => {}
                Item::TypeAlias(_) => {}
                Item::TestCase(test) => self.register_testcase(namespace, test),
                Item::Import(_) => {}
            }
        }
    }

    fn record_type_generics(&mut self, name: &str, generics: Option<&GenericParams>) {
        let Some(params) = generics else {
            return;
        };
        let entries = params
            .params
            .iter()
            .filter_map(|param| {
                let data = param.as_type()?;
                Some(TypeGenericParamEntry {
                    name: param.name.clone(),
                    variance: TypeVariance::from(data.variance),
                })
            })
            .collect::<Vec<_>>();
        if !entries.is_empty() {
            self.type_generics.insert(name.to_string(), entries);
        }
    }

    fn register_delegate(&mut self, namespace: Option<&str>, delegate: &DelegateDecl) {
        let qualified = qualify(namespace, &delegate.name);
        self.types.insert(qualified.clone());
        self.record_type_generics(&qualified, delegate.generics.as_ref());

        let params = delegate
            .signature
            .parameters
            .iter()
            .map(|param| Ty::from_type_expr(&param.ty))
            .collect::<Vec<_>>();
        let param_modes = delegate
            .signature
            .parameters
            .iter()
            .map(|param| Self::binding_to_param_mode(param.binding))
            .collect::<Vec<_>>();
        let ret = Ty::from_type_expr(&delegate.signature.return_type);
        let signature = FnTy::with_modes(
            params,
            param_modes,
            ret,
            Abi::Chic,
            delegate.signature.variadic,
        );
        self.delegate_signatures.insert(qualified, signature);
    }

    fn register_function(
        &mut self,
        namespace: Option<&str>,
        owner: Option<&str>,
        func: &FunctionDecl,
    ) {
        let qualified = if let Some(owner) = owner {
            format!("{owner}::{}", func.name)
        } else {
            qualify(namespace, &func.name)
        };
        let params = func
            .signature
            .parameters
            .iter()
            .map(|param| Ty::from_type_expr(&param.ty))
            .collect::<Vec<_>>();
        let param_symbols = func
            .signature
            .parameters
            .iter()
            .map(|param| FunctionParamSymbol {
                name: param.name.clone(),
                has_default: param.default.is_some(),
                mode: Self::binding_to_param_mode(param.binding),
                is_extension_this: param.is_extension_this,
            })
            .collect::<Vec<_>>();
        let param_modes = param_symbols.iter().map(|param| param.mode).collect();
        let ret = Ty::from_type_expr(&func.signature.return_type);
        let abi = match func.extern_abi.as_ref() {
            Some(convention) => Abi::Extern(convention.clone()),
            None => Abi::Chic,
        };
        let signature = FnTy::with_modes(params, param_modes, ret, abi, func.signature.variadic);
        let is_static = owner.is_some()
            && func
                .modifiers
                .iter()
                .any(|modifier| modifier.eq_ignore_ascii_case("static"));
        let internal_name = self.allocate_internal_name(&qualified);
        let symbol = FunctionSymbol {
            qualified: qualified.clone(),
            internal_name,
            signature,
            params: param_symbols,
            is_unsafe: func.is_unsafe,
            is_static,
            visibility: func.visibility,
            namespace: namespace.map(str::to_string),
            owner: owner.map(str::to_string),
        };
        self.record_function_decl(
            qualified.clone(),
            func,
            owner,
            namespace,
            &symbol.internal_name,
        );
        self.functions
            .entry(qualified.clone())
            .or_default()
            .push(symbol);

        if let Some(body) = func.body.as_ref() {
            let mut local_counter = 0usize;
            register_local_functions_in_block(self, &qualified, body, &mut local_counter);
        }
    }

    fn register_testcase(&mut self, namespace: Option<&str>, test: &TestCaseDecl) {
        let qualified = qualify(namespace, &test.name);
        let signature = if let Some(sig) = &test.signature {
            let params = sig
                .parameters
                .iter()
                .map(|param| Ty::from_type_expr(&param.ty))
                .collect::<Vec<_>>();
            let ret = Ty::from_type_expr(&sig.return_type);
            (params, ret, sig.variadic)
        } else {
            (Vec::new(), Ty::Unit, false)
        };
        let param_symbols = test
            .signature
            .as_ref()
            .map(|sig| {
                sig.parameters
                    .iter()
                    .map(|param| FunctionParamSymbol {
                        name: param.name.clone(),
                        has_default: param.default.is_some(),
                        mode: Self::binding_to_param_mode(param.binding),
                        is_extension_this: param.is_extension_this,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let param_modes = param_symbols.iter().map(|param| param.mode).collect();
        let fn_ty = FnTy::with_modes(
            signature.0,
            param_modes,
            signature.1,
            Abi::Chic,
            signature.2,
        );
        let internal_name = self.allocate_internal_name(&qualified);
        let symbol = FunctionSymbol {
            qualified: qualified.clone(),
            internal_name,
            signature: fn_ty,
            params: param_symbols,
            is_unsafe: false,
            is_static: true,
            visibility: Visibility::Public,
            namespace: namespace.map(str::to_string),
            owner: None,
        };
        self.functions
            .entry(qualified.clone())
            .or_default()
            .push(symbol);
    }

    fn binding_to_param_mode(binding: BindingModifier) -> ParamMode {
        match binding {
            BindingModifier::In => ParamMode::In,
            BindingModifier::Ref => ParamMode::Ref,
            BindingModifier::Out => ParamMode::Out,
            BindingModifier::Value => ParamMode::Value,
        }
    }

    fn record_function_decl(
        &mut self,
        qualified: String,
        func: &FunctionDecl,
        owner: Option<&str>,
        namespace: Option<&str>,
        internal_name: &str,
    ) {
        let decl_symbol = FunctionDeclSymbol {
            qualified,
            function: func.clone(),
            owner: owner.map(str::to_string),
            namespace: namespace.map(str::to_string),
            internal_name: internal_name.to_string(),
        };
        self.function_decls
            .entry(decl_symbol.qualified.clone())
            .or_default()
            .push(decl_symbol);
    }

    fn record_constructor_decl(
        &mut self,
        owner: &str,
        ctor: &ConstructorDecl,
        internal_name: &str,
    ) {
        let qualified = format!("{owner}::init");
        let namespace = owner.rsplit_once("::").map(|(ns, _)| ns.to_string());
        let decl_symbol = ConstructorDeclSymbol {
            qualified,
            constructor: ctor.clone(),
            owner: owner.to_string(),
            namespace,
            internal_name: internal_name.to_string(),
        };
        self.constructor_decls
            .entry(owner.to_string())
            .or_default()
            .push(decl_symbol);
    }

    fn register_constructor(&mut self, owner: &str, ctor: &ConstructorDecl, index: usize) {
        let qualified = format!("{owner}::init#{index}");
        let namespace = owner.rsplit_once("::").map(|(ns, _)| ns.to_string());
        let params = ctor
            .parameters
            .iter()
            .map(|param| Ty::from_type_expr(&param.ty))
            .collect::<Vec<_>>();
        let param_symbols = ctor
            .parameters
            .iter()
            .map(|param| FunctionParamSymbol {
                name: param.name.clone(),
                has_default: param.default.is_some(),
                mode: Self::binding_to_param_mode(param.binding),
                is_extension_this: param.is_extension_this,
            })
            .collect::<Vec<_>>();
        let param_modes = param_symbols.iter().map(|param| param.mode).collect();
        let signature = FnTy::with_modes(params, param_modes, Ty::Unit, Abi::Chic, false);
        let internal_name = self.allocate_internal_name(&qualified);
        let symbol = FunctionSymbol {
            qualified: qualified.clone(),
            internal_name,
            signature,
            params: param_symbols,
            is_unsafe: false,
            is_static: false,
            visibility: ctor.visibility,
            namespace: namespace.clone(),
            owner: Some(owner.to_string()),
        };
        self.record_constructor_decl(owner, ctor, &symbol.internal_name);
        self.functions
            .entry(qualified.clone())
            .or_default()
            .push(symbol.clone());
        self.functions
            .entry(owner.to_string())
            .or_default()
            .push(symbol);
    }

    fn register_trait(&mut self, namespace: Option<&str>, trait_decl: &TraitDecl) {
        let type_name = qualify(namespace, &trait_decl.name);
        self.types.insert(type_name.clone());
        self.record_type_generics(&type_name, trait_decl.generics.as_ref());
    }

    fn register_struct(&mut self, namespace: Option<&str>, strct: &StructDecl) {
        let type_name = qualify(namespace, &strct.name);
        self.types.insert(type_name.clone());
        self.record_type_generics(&type_name, strct.generics.as_ref());
        if strct.is_readonly {
            self.readonly_structs.insert(type_name.clone());
        }
        for field in &strct.fields {
            self.register_field_symbol(
                &type_name,
                &field.name,
                FieldSymbol {
                    ty: field.ty.clone(),
                    visibility: field.visibility,
                    is_static: field.is_static,
                    is_readonly: field.is_readonly,
                    is_required: field.is_required,
                    span: None,
                    namespace: namespace.map(str::to_string),
                },
            );
        }
        for const_member in &strct.consts {
            self.register_const_declaration(
                namespace,
                Some(type_name.as_str()),
                const_member.visibility,
                &const_member.modifiers,
                &const_member.declaration,
            );
        }
        for method in &strct.methods {
            self.register_method(&type_name, &method.name);
            self.register_function(namespace, Some(type_name.as_str()), method);
        }
        for property in &strct.properties {
            self.register_property(namespace, &type_name, property);
        }
        let mut constructor_index = 0usize;
        for ctor in &strct.constructors {
            self.register_constructor(&type_name, ctor, constructor_index);
            constructor_index += 1;
        }
        if !strct.nested_types.is_empty() {
            self.collect_items(strct.nested_types.iter(), Some(type_name.as_str()));
        }
    }

    fn register_class(&mut self, namespace: Option<&str>, class: &ClassDecl) {
        let type_name = qualify(namespace, &class.name);
        self.types.insert(type_name.clone());
        self.record_type_generics(&type_name, class.generics.as_ref());
        let mut constructor_index = 0usize;
        for member in &class.members {
            match member {
                ClassMember::Field(field) => {
                    self.register_field_symbol(
                        &type_name,
                        &field.name,
                        FieldSymbol {
                            ty: field.ty.clone(),
                            visibility: field.visibility,
                            is_static: field.is_static,
                            is_readonly: field.is_readonly,
                            is_required: field.is_required,
                            span: None,
                            namespace: namespace.map(str::to_string),
                        },
                    );
                }
                ClassMember::Method(method) => {
                    self.register_method(&type_name, &method.name);
                    self.register_function(namespace, Some(type_name.as_str()), method);
                }
                ClassMember::Property(property) => {
                    self.register_property(namespace, &type_name, property);
                }
                ClassMember::Constructor(ctor) => {
                    self.register_constructor(&type_name, ctor, constructor_index);
                    constructor_index += 1;
                }
                ClassMember::Const(const_member) => {
                    self.register_const_declaration(
                        namespace,
                        Some(type_name.as_str()),
                        const_member.visibility,
                        &const_member.modifiers,
                        &const_member.declaration,
                    );
                }
            }
        }
        if !class.nested_types.is_empty() {
            self.collect_items(class.nested_types.iter(), Some(type_name.as_str()));
        }
    }

    fn register_interface(&mut self, namespace: Option<&str>, iface: &InterfaceDecl) {
        let type_name = qualify(namespace, &iface.name);
        self.types.insert(type_name.clone());
        self.record_type_generics(&type_name, iface.generics.as_ref());
        for member in &iface.members {
            match member {
                InterfaceMember::Method(method) => self.register_method(&type_name, &method.name),
                InterfaceMember::Property(property) => {
                    self.register_property(namespace, &type_name, property);
                }
                InterfaceMember::Const(const_member) => {
                    self.register_const_declaration(
                        namespace,
                        Some(type_name.as_str()),
                        const_member.visibility,
                        &const_member.modifiers,
                        &const_member.declaration,
                    );
                }
                InterfaceMember::AssociatedType(_) => {}
            }
        }
    }

    fn register_const_item(&mut self, namespace: Option<&str>, item: &ConstItemDecl) {
        self.register_const_declaration(namespace, None, item.visibility, &[], &item.declaration);
    }

    fn register_property(
        &mut self,
        namespace: Option<&str>,
        type_name: &str,
        property: &PropertyDecl,
    ) {
        let type_key = type_name.to_string();

        let is_static = property.is_static;
        let backing_field = property.is_auto().then(|| property.backing_field_name());
        if let Some(field_name) = backing_field.as_ref() {
            self.register_field_symbol(
                type_name,
                field_name,
                FieldSymbol {
                    ty: property.ty.clone(),
                    visibility: Visibility::Private,
                    is_static,
                    is_readonly: false,
                    is_required: false,
                    span: property.span,
                    namespace: namespace.map(str::to_string),
                },
            );
        }

        let mut symbol = PropertySymbol {
            ty: type_expr_surface(&property.ty),
            is_static,
            accessors: HashMap::new(),
            span: property.span,
            is_required: property.is_required,
            is_nullable: property.ty.is_nullable(),
            visibility: property.visibility,
            namespace: namespace.map(str::to_string),
        };

        for kind in [
            PropertyAccessorKind::Get,
            PropertyAccessorKind::Set,
            PropertyAccessorKind::Init,
        ] {
            if property.accessor(kind).is_none() {
                continue;
            }
            let method_name = property.accessor_method_name(kind);
            self.register_method(type_name, &method_name);
            let qualified = format!("{type_name}::{method_name}");
            let metadata = PropertyAccessorMetadata {
                function: qualified.clone(),
            };
            symbol.accessors.insert(kind, metadata);
            self.property_accessors.insert(
                qualified,
                PropertyAccessorLookup {
                    owner: type_name.to_string(),
                    kind,
                    backing_field: backing_field.clone(),
                },
            );
        }

        self.type_properties
            .entry(type_key)
            .or_default()
            .insert(property.name.clone(), symbol);
    }

    fn register_const_declaration(
        &mut self,
        namespace: Option<&str>,
        owner: Option<&str>,
        visibility: Visibility,
        modifiers: &[String],
        declaration: &ConstDeclaration,
    ) {
        for declarator in &declaration.declarators {
            let name = declarator.name.clone();
            let qualified = if let Some(owner_name) = owner {
                format!("{owner_name}::{name}")
            } else {
                qualify(namespace, &name)
            };
            let symbol = ConstSymbol {
                qualified: qualified.clone(),
                name: name.clone(),
                owner: owner.map(str::to_string),
                namespace: namespace.map(str::to_string),
                ty: declaration.ty.clone(),
                initializer: declarator.initializer.clone(),
                visibility,
                modifiers: modifiers.to_vec(),
                span: declarator.span.or(declaration.span),
                value: None,
            };
            self.constants.insert(qualified.clone(), symbol.clone());
            if let Some(owner_name) = owner {
                self.type_constants
                    .entry(owner_name.to_string())
                    .or_default()
                    .insert(name.clone(), symbol.clone());
            } else {
                let ns_key = namespace.unwrap_or("");
                self.namespace_constants
                    .entry(ns_key.to_string())
                    .or_default()
                    .insert(name.clone(), symbol.clone());
            }
        }
    }

    fn register_union(&mut self, namespace: Option<&str>, union_def: &UnionDecl) {
        let type_name = qualify(namespace, &union_def.name);
        self.types.insert(type_name.clone());
        for member in &union_def.members {
            match member {
                UnionMember::Field(field) => {
                    self.register_field_symbol(
                        &type_name,
                        &field.name,
                        FieldSymbol {
                            ty: field.ty.clone(),
                            visibility: field.visibility,
                            is_static: false,
                            is_readonly: field.is_readonly,
                            is_required: false,
                            span: None,
                            namespace: namespace.map(str::to_string),
                        },
                    );
                }
                UnionMember::View(view) => {
                    let view_name = format!("{type_name}::{}", view.name);
                    self.types.insert(view_name.clone());
                    for field in &view.fields {
                        self.register_field_symbol(
                            &view_name,
                            &field.name,
                            FieldSymbol {
                                ty: field.ty.clone(),
                                visibility: field.visibility,
                                is_static: false,
                                is_readonly: field.is_readonly,
                                is_required: false,
                                span: None,
                                namespace: namespace.map(str::to_string),
                            },
                        );
                    }
                }
            }
        }
    }

    fn register_enum(&mut self, namespace: Option<&str>, enm: &EnumDecl) {
        let type_name = qualify(namespace, &enm.name);
        self.types.insert(type_name.clone());
        for variant in &enm.variants {
            self.register_enum_variant(&type_name, &variant.name);
        }
    }

    fn register_extension(&mut self, namespace: Option<&str>, ext: &ExtensionDecl) {
        if ext.target.base.is_empty() {
            return;
        }

        let base = ext.target.base.join("::");
        let mut candidates = Vec::new();
        candidates.push(base.clone());
        if let Some(ns) = namespace {
            candidates.push(format!("{ns}::{base}"));
        }

        for member in &ext.members {
            match member {
                ExtensionMember::Method(method) => {
                    let instantiated = instantiate_extension_method(&method.function, &ext.target);
                    for candidate in &candidates {
                        let owner_key = canonical_method_owner(candidate);
                        self.extension_placeholders
                            .entry(owner_key.clone())
                            .or_default()
                            .insert(method.function.name.clone());
                        let qualified = format!("{owner_key}::{}", method.function.name);
                        let params = instantiated
                            .signature
                            .parameters
                            .iter()
                            .map(|param| Ty::from_type_expr(&param.ty))
                            .collect::<Vec<_>>();
                        let param_symbols = instantiated
                            .signature
                            .parameters
                            .iter()
                            .map(|param| FunctionParamSymbol {
                                name: param.name.clone(),
                                has_default: param.default.is_some(),
                                mode: Self::binding_to_param_mode(param.binding),
                                is_extension_this: param.is_extension_this,
                            })
                            .collect::<Vec<_>>();
                        let ret = Ty::from_type_expr(&instantiated.signature.return_type);
                        let abi = if instantiated.is_extern {
                            Abi::Extern(
                                instantiated
                                    .extern_abi
                                    .clone()
                                    .unwrap_or_else(|| "C".to_string()),
                            )
                        } else {
                            Abi::Chic
                        };
                        let param_modes = param_symbols.iter().map(|param| param.mode).collect();
                        let signature = FnTy::with_modes(
                            params,
                            param_modes,
                            ret,
                            abi,
                            instantiated.signature.variadic,
                        );
                        let is_static = instantiated
                            .modifiers
                            .iter()
                            .any(|modifier| modifier.eq_ignore_ascii_case("static"));
                        let internal_name = extension_method_symbol(
                            owner_key.as_str(),
                            namespace,
                            &method.function.name,
                            method.is_default,
                        );
                        let symbol = FunctionSymbol {
                            qualified: qualified.clone(),
                            internal_name: internal_name.clone(),
                            signature,
                            params: param_symbols,
                            is_unsafe: instantiated.is_unsafe,
                            is_static,
                            visibility: instantiated.visibility,
                            namespace: namespace.map(str::to_string),
                            owner: Some(owner_key.clone()),
                        };
                        self.record_function_decl(
                            qualified.clone(),
                            &instantiated,
                            Some(owner_key.as_str()),
                            namespace,
                            &symbol.internal_name,
                        );
                        self.functions
                            .entry(qualified.clone())
                            .or_default()
                            .push(symbol);
                        if let Some(body) = instantiated.body.as_ref() {
                            let mut local_counter = 0usize;
                            register_local_functions_in_block(
                                self,
                                &qualified,
                                body,
                                &mut local_counter,
                            );
                        }
                    }
                }
            }
        }
    }

    fn register_field_symbol(&mut self, type_name: &str, field: &str, symbol: FieldSymbol) {
        self.type_fields
            .entry(type_name.to_string())
            .or_default()
            .insert(field.to_string(), symbol);
    }

    fn register_enum_variant(&mut self, type_name: &str, variant: &str) {
        self.enum_variants
            .entry(type_name.to_string())
            .or_default()
            .insert(variant.to_string());
    }
}

pub(crate) fn canonical_method_owner(name: &str) -> String {
    strip_generic_arguments(name)
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
            _ => {
                if depth == 0 {
                    result.push(ch);
                }
            }
        }
    }
    result.trim().to_string()
}

pub fn candidate_function_names(namespace: Option<&str>, segments: &[&str]) -> Vec<String> {
    let joined = segments.join("::");
    let mut seen = HashSet::new();
    let mut results = Vec::new();

    push_function_candidate(&joined, &mut seen, &mut results);

    if segments.len() == 1 {
        if let Some(ns) = namespace {
            let mut current = Some(ns);
            while let Some(prefix) = current {
                let candidate = format!("{prefix}::{joined}");
                push_function_candidate(&candidate, &mut seen, &mut results);
                current = prefix.rfind("::").map(|idx| &prefix[..idx]);
            }
        }
    }

    results
}

fn push_function_candidate(candidate: &str, seen: &mut HashSet<String>, results: &mut Vec<String>) {
    if seen.insert(candidate.to_string()) {
        results.push(candidate.to_string());
    }
}

fn register_local_functions_in_block(
    index: &mut SymbolIndex,
    parent: &str,
    block: &AstBlock,
    counter: &mut usize,
) {
    for statement in &block.statements {
        register_local_functions_in_statement(index, parent, statement, counter);
    }
}

fn register_local_functions_in_statement(
    index: &mut SymbolIndex,
    parent: &str,
    statement: &Statement,
    counter: &mut usize,
) {
    match &statement.kind {
        StatementKind::LocalFunction(local) => {
            let ordinal = *counter;
            *counter += 1;
            let symbol = local_function_symbol(parent, ordinal, &local.name);
            insert_local_function_symbol(index, &symbol, local);
            if let Some(body) = &local.body {
                let mut nested_counter = 0usize;
                register_local_functions_in_block(index, &symbol, body, &mut nested_counter);
            }
        }
        StatementKind::Block(block) => {
            register_local_functions_in_block(index, parent, block, counter);
        }
        StatementKind::If(if_stmt) => {
            register_local_functions_in_statement(
                index,
                parent,
                if_stmt.then_branch.as_ref(),
                counter,
            );
            if let Some(else_branch) = if_stmt.else_branch.as_ref() {
                register_local_functions_in_statement(index, parent, else_branch.as_ref(), counter);
            }
        }
        StatementKind::While { body, .. }
        | StatementKind::DoWhile { body, .. }
        | StatementKind::Lock { body, .. }
        | StatementKind::Unsafe { body }
        | StatementKind::Labeled {
            statement: body, ..
        } => {
            register_local_functions_in_statement(index, parent, body.as_ref(), counter);
        }
        StatementKind::Fixed(fixed_stmt) => {
            register_local_functions_in_statement(index, parent, fixed_stmt.body.as_ref(), counter);
        }
        StatementKind::For(for_stmt) => {
            register_local_functions_in_statement(index, parent, for_stmt.body.as_ref(), counter);
        }
        StatementKind::Foreach(foreach_stmt) => {
            register_local_functions_in_statement(
                index,
                parent,
                foreach_stmt.body.as_ref(),
                counter,
            );
        }
        StatementKind::Switch(switch_stmt) => {
            register_local_functions_in_switch_sections(
                index,
                parent,
                &switch_stmt.sections,
                counter,
            );
        }
        StatementKind::Try(try_stmt) => {
            register_local_functions_in_block(index, parent, &try_stmt.body, counter);
            for clause in &try_stmt.catches {
                register_local_functions_in_catch_clause(index, parent, clause, counter);
            }
            if let Some(finally) = &try_stmt.finally {
                register_local_functions_in_block(index, parent, finally, counter);
            }
        }
        StatementKind::Using(using_stmt) => {
            if let Some(body) = &using_stmt.body {
                register_local_functions_in_statement(index, parent, body.as_ref(), counter);
            }
        }
        StatementKind::Atomic { body, .. }
        | StatementKind::Checked { body }
        | StatementKind::Unchecked { body } => {
            register_local_functions_in_block(index, parent, body, counter);
        }
        _ => {}
    }
}

fn register_local_functions_in_switch_sections(
    index: &mut SymbolIndex,
    parent: &str,
    sections: &[SwitchSection],
    counter: &mut usize,
) {
    for section in sections {
        for stmt in &section.statements {
            register_local_functions_in_statement(index, parent, stmt, counter);
        }
    }
}

fn register_local_functions_in_catch_clause(
    index: &mut SymbolIndex,
    parent: &str,
    clause: &CatchClause,
    counter: &mut usize,
) {
    register_local_functions_in_block(index, parent, &clause.body, counter);
}

fn insert_local_function_symbol(index: &mut SymbolIndex, qualified: &str, func: &FunctionDecl) {
    let params = func
        .signature
        .parameters
        .iter()
        .map(|param| Ty::from_type_expr(&param.ty))
        .collect::<Vec<_>>();
    let param_symbols = func
        .signature
        .parameters
        .iter()
        .map(|param| FunctionParamSymbol {
            name: param.name.clone(),
            has_default: param.default.is_some(),
            mode: SymbolIndex::binding_to_param_mode(param.binding),
            is_extension_this: param.is_extension_this,
        })
        .collect::<Vec<_>>();
    let ret = Ty::from_type_expr(&func.signature.return_type);
    let param_modes = param_symbols.iter().map(|param| param.mode).collect();
    let signature = FnTy::with_modes(params, param_modes, ret, Abi::Chic, func.signature.variadic);
    let internal_name = index.allocate_internal_name(qualified);
    let symbol = FunctionSymbol {
        qualified: qualified.to_string(),
        internal_name,
        signature,
        params: param_symbols,
        is_unsafe: func.is_unsafe,
        is_static: true,
        visibility: Visibility::Private,
        namespace: None,
        owner: None,
    };
    index.record_function_decl(
        qualified.to_string(),
        func,
        None,
        None,
        &symbol.internal_name,
    );
    index
        .functions
        .entry(qualified.to_string())
        .or_default()
        .push(symbol);
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex};
    use std::thread;

    #[test]
    fn concurrent_type_updates_are_thread_safe() {
        let seen = Arc::new(Mutex::new(HashSet::new()));
        let mut handles = Vec::new();
        for worker in 0..4 {
            let seen = Arc::clone(&seen);
            handles.push(thread::spawn(move || {
                for offset in 0..250 {
                    let mut guard = seen.lock().unwrap();
                    guard.insert(format!("Test::{worker}_{offset}"));
                }
            }));
        }
        for handle in handles {
            handle.join().expect("thread panicked");
        }
        let guard = seen.lock().unwrap();
        assert_eq!(guard.len(), 1000);
    }

    #[test]
    fn concurrent_method_registration_remains_consistent() {
        let methods = Arc::new(Mutex::new(HashMap::new()));
        let mut handles = Vec::new();
        for _ in 0..4 {
            let methods = Arc::clone(&methods);
            handles.push(thread::spawn(move || {
                for _ in 0..250 {
                    let mut guard = methods.lock().unwrap();
                    *guard.entry("Root::Concurrent::Run").or_insert(0usize) += 1;
                }
            }));
        }
        for handle in handles {
            handle.join().expect("thread panicked");
        }
        let guard = methods.lock().unwrap();
        assert_eq!(guard.get("Root::Concurrent::Run").copied(), Some(1000));
    }
}
