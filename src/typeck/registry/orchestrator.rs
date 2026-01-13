use super::super::arena::{
    ObjectSafetyViolation, ObjectSafetyViolationKind, TraitAssociatedTypeInfo, TraitInfo,
    TraitMethodInfo, TraitObjectSafety,
};
use super::hooks::RegisteredItemKind;
use super::*;
use crate::frontend::ast::{
    ClassDecl, ClassMember, DelegateDecl, ExtensionDecl, FunctionDecl, InterfaceDecl,
    InterfaceMember, Item, UnionMember,
};
use crate::frontend::attributes::collect_layout_hints;
use crate::frontend::local_functions::local_function_symbol;
use crate::frontend::type_utils::extension_method_symbol;
use crate::mir::AutoTraitOverride;

impl<'a> TypeChecker<'a> {
    pub(crate) fn reserve_type_slot(
        &mut self,
        name: &str,
        kind: RegisteredItemKind,
        arity: usize,
        span: Option<Span>,
    ) -> bool {
        match self
            .registry_index
            .record(name, kind, arity, span, &mut self.registry_hooks)
        {
            Ok(()) => true,
            Err(conflict) => {
                self.emit_error(
                    codes::REGISTRY_CONFLICT,
                    span.or(conflict.first_span),
                    format!("{kind} `{name}` conflicts with a previous declaration"),
                );
                let note_message =
                    format!("previous declaration of `{}` ({})", name, conflict.kind);
                self.diagnostics
                    .push(diagnostics::note(note_message, conflict.first_span));
                false
            }
        }
    }
    pub(crate) fn visit_items_with_units(&mut self, items: &'a [Item], namespace: Option<&str>) {
        if let Some(units) = self.package_context.item_units.clone() {
            for (index, item) in items.iter().enumerate() {
                let unit: Option<usize> = units.get(index).copied();
                self.visit_item(item, namespace, unit);
            }
            #[cfg(test)]
            self.run_pending_body_validations();
            return;
        }
        self.visit_items(items, namespace);
    }

    pub(crate) fn visit_items(&mut self, items: &'a [Item], namespace: Option<&str>) {
        for item in items {
            self.visit_item(item, namespace, self.current_unit);
        }
        #[cfg(test)]
        self.run_pending_body_validations();
    }

    fn visit_item(&mut self, item: &'a Item, namespace: Option<&str>, unit: Option<usize>) {
        let previous_unit = self.current_unit;
        let previous_package = self.current_package.clone();
        let package = self
            .package_context
            .package_for_unit(unit)
            .map(str::to_string)
            .or(previous_package.clone());
        self.current_unit = unit.or(previous_unit);
        self.current_package = package;
        self.visit_item_body(item, namespace);
        self.current_unit = previous_unit;
        self.current_package = previous_package;
    }

    fn visit_item_body(&mut self, item: &'a Item, namespace: Option<&str>) {
        match item {
            Item::Function(func) => {
                let function_name = qualify(namespace, &func.name);
                self.record_function_package(&function_name);
                self.register_function(func, namespace);
                self.validate_const_function(&function_name, func, namespace, None);
            }
            Item::Delegate(delegate) => {
                self.register_delegate(delegate, namespace);
            }
            Item::Struct(strct) => {
                let type_name = qualify(namespace, &strct.name);
                self.record_type_package(&type_name);
                let arity = strct.generics.as_ref().map_or(0, |g| g.params.len());
                if !self.reserve_type_slot(&type_name, RegisteredItemKind::Struct, arity, None) {
                    return;
                }
                self.validate_generics(&type_name, strct.generics.as_ref());
                self.push_pending_generics(&type_name, strct.generics.as_ref());
                let (mut layout_hints, _) = collect_layout_hints(&strct.attributes);
                if let Some(struct_layout) = strct.layout {
                    layout_hints.repr_c = struct_layout.repr_c;
                    if struct_layout.packing.is_some() {
                        layout_hints.packing = struct_layout.packing;
                    }
                    if struct_layout.align.is_some() {
                        layout_hints.align = struct_layout.align;
                    }
                }
                let constructor_infos = strct
                    .constructors
                    .iter()
                    .map(|ctor| ConstructorInfo {
                        visibility: ctor.visibility,
                        param_count: ctor.parameters.len(),
                    })
                    .collect::<Vec<_>>();
                let bases = strct
                    .bases
                    .iter()
                    .filter_map(|base| {
                        self.ensure_type_expr(base, namespace, Some(type_name.as_str()), None);
                        let resolved =
                            match self.resolve_type_for_expr(base, namespace, Some(type_name.as_str()))
                            {
                                ImportResolution::Found(resolved) => {
                                    if let Some(info) = self.resolve_type_info(&resolved).cloned() {
                                        if !self.type_accessible_from_current(
                                            info.visibility,
                                            &resolved,
                                            None,
                                            namespace,
                                            Some(type_name.as_str()),
                                        ) {
                                            self.emit_error(
                                                codes::INACCESSIBLE_BASE,
                                                base.span,
                                                format!(
                                                    "type `{}` cannot inherit from `{}` because it is not accessible from this package",
                                                    type_name, resolved
                                                ),
                                            );
                                            return None;
                                        }
                                    }
                                    resolved
                                }
                                ImportResolution::Ambiguous(candidates) => {
                                    self.emit_error(
                                        codes::AMBIGUOUS_CLASS_BASE,
                                        base.span,
                                        format!(
                                            "base type `{}` resolves to multiple candidates: {}",
                                            base.name,
                                            candidates.join(", ")
                                        ),
                        );
                        return None;
                    }
                    ImportResolution::NotFound => {
                        self.emit_error(
                            codes::BASE_TYPE_NOT_FOUND,
                            base.span,
                            format!(
                                "base type `{}` for `{}` could not be resolved",
                                base.name, type_name
                            ),
                        );
                        return None;
                    }
                };
            Some(BaseTypeBinding::new(resolved, base.clone()))
        })
        .collect::<Vec<_>>();
                self.insert_type_info(
                    type_name.clone(),
                    TypeInfo {
                        kind: TypeKind::Struct {
                            constructors: constructor_infos,
                            is_record: strct.is_record,
                            bases,
                        },
                        generics: strct.generics.clone(),
                        repr_c: layout_hints.repr_c,
                        packing: layout_hints
                            .packing
                            .map(|hint| hint.value.unwrap_or(1).max(1)),
                        align: layout_hints.align.map(|hint| hint.value),
                        is_readonly: strct.is_readonly,
                        is_intrinsic: strct.is_intrinsic,
                        visibility: strct.visibility,
                    },
                );
                for field in &strct.fields {
                    self.ensure_type_expr(&field.ty, namespace, Some(type_name.as_str()), None);
                    let member_name = format!("{type_name}::{}", field.name);
                    self.validate_public_type_expr(
                        &member_name,
                        field.visibility,
                        &field.ty,
                        namespace,
                        Some(type_name.as_str()),
                    );
                    if let Some(initializer) = &field.initializer {
                        let field_name = format!("{type_name}::{}", field.name);
                        self.validate_expression(
                            &field_name,
                            initializer,
                            namespace,
                            Some(type_name.as_str()),
                        );
                        self.check_numeric_literal_expression(initializer, Some(&field.ty));
                    }
                }
                for const_member in &strct.consts {
                    self.ensure_type_expr(
                        &const_member.declaration.ty,
                        namespace,
                        Some(type_name.as_str()),
                        None,
                    );
                    self.validate_const_declaration(
                        namespace,
                        Some(type_name.as_str()),
                        &const_member.declaration,
                    );
                }
                for method in &strct.methods {
                    let method_name = format!("{type_name}::{}", method.name);
                    self.validate_generics(&method_name, method.generics.as_ref());
                    self.register_function_generics(&method_name, method.generics.as_ref());
                    self.push_pending_generics(&method_name, method.generics.as_ref());
                    self.ensure_unique_parameter_names(
                        &method.signature.parameters,
                        &method_name,
                        None,
                    );
                    for param in &method.signature.parameters {
                        self.ensure_type_expr(
                            &param.ty,
                            namespace,
                            Some(method_name.as_str()),
                            None,
                        );
                    }
                    self.validate_parameter_defaults(
                        &method_name,
                        &method.signature.parameters,
                        namespace,
                        Some(method_name.as_str()),
                    );
                    let return_span = method.body.as_ref().and_then(|body| body.span);
                    self.ensure_type_expr(
                        &method.signature.return_type,
                        namespace,
                        Some(method_name.as_str()),
                        return_span,
                    );
                    self.validate_public_signature(
                        &method_name,
                        method.visibility,
                        Some(&method.signature.return_type),
                        &method.signature.parameters,
                        namespace,
                        Some(method_name.as_str()),
                    );
                    let sig = signature_from(&method.signature, method_name.clone(), None);
                    let sig_id = self.allocate_signature(sig);
                    self.record_signature_generics(sig_id, method.generics.as_ref());
                    self.methods
                        .entry(type_name.clone())
                        .or_default()
                        .push(sig_id);
                    self.validate_const_function(
                        &method_name,
                        method,
                        namespace,
                        Some(type_name.as_str()),
                    );
                    if method.is_async {
                        if let Some(result_ty) = self.validate_async_return_type(
                            &method_name,
                            &method.signature,
                            namespace,
                            Some(type_name.as_str()),
                            return_span,
                        ) {
                            self.async_signatures.insert(sig_id, result_ty);
                        }
                    }
                    self.pop_pending_generics(&method_name);
                }
                for property in &strct.properties {
                    self.ensure_type_expr(
                        &property.ty,
                        namespace,
                        Some(type_name.as_str()),
                        property.span,
                    );
                    let member_name = format!("{type_name}::{}", property.name);
                    self.validate_public_type_expr(
                        &member_name,
                        property.visibility,
                        &property.ty,
                        namespace,
                        Some(type_name.as_str()),
                    );
                }
                for (index, ctor) in strct.constructors.iter().enumerate() {
                    let ctor_name = format!("{type_name}::init#{index}");
                    self.ensure_unique_parameter_names(
                        &ctor.parameters,
                        &format!("{type_name}::init"),
                        ctor.span,
                    );
                    self.validate_parameter_defaults(
                        &ctor_name,
                        &ctor.parameters,
                        namespace,
                        Some(type_name.as_str()),
                    );
                    self.validate_public_signature(
                        &ctor_name,
                        ctor.visibility,
                        None,
                        &ctor.parameters,
                        namespace,
                        Some(type_name.as_str()),
                    );
                    if let Some(initializer) = &ctor.initializer {
                        for argument in &initializer.arguments {
                            self.validate_expression(
                                &ctor_name,
                                argument,
                                namespace,
                                Some(type_name.as_str()),
                            );
                        }
                    }
                    if let Some(body) = ctor.body.as_ref() {
                        self.queue_body_validation(
                            &ctor_name,
                            body,
                            namespace,
                            Some(type_name.as_str()),
                        );
                    }
                }
                if !strct.nested_types.is_empty() {
                    self.enclosing_types.push(type_name.clone());
                    self.visit_items(&strct.nested_types, Some(type_name.as_str()));
                    self.enclosing_types.pop();
                }
            }
            Item::Union(union_def) => {
                let type_name = qualify(namespace, &union_def.name);
                self.record_type_package(&type_name);
                let arity = union_def.generics.as_ref().map_or(0, |g| g.params.len());
                if !self.reserve_type_slot(&type_name, RegisteredItemKind::Union, arity, None) {
                    return;
                }
                self.validate_generics(&type_name, union_def.generics.as_ref());
                self.push_pending_generics(&type_name, union_def.generics.as_ref());
                let (layout_hints, _) = collect_layout_hints(&union_def.attributes);
                self.insert_type_info(
                    type_name.clone(),
                    TypeInfo {
                        kind: TypeKind::Union,
                        generics: union_def.generics.clone(),
                        repr_c: layout_hints.repr_c,
                        packing: layout_hints
                            .packing
                            .map(|hint| hint.value.unwrap_or(1).max(1)),
                        align: layout_hints.align.map(|hint| hint.value),
                        is_readonly: false,
                        is_intrinsic: false,
                        visibility: union_def.visibility,
                    },
                );
                for member in &union_def.members {
                    if let UnionMember::Field(field) = member {
                        self.ensure_type_expr(&field.ty, namespace, Some(type_name.as_str()), None);
                    }
                }
            }
            Item::Enum(enm) => {
                let type_name = qualify(namespace, &enm.name);
                self.record_type_package(&type_name);
                let arity = enm.generics.as_ref().map_or(0, |g| g.params.len());
                if !self.reserve_type_slot(&type_name, RegisteredItemKind::Enum, arity, None) {
                    return;
                }
                self.validate_generics(&type_name, enm.generics.as_ref());
                self.push_pending_generics(&type_name, enm.generics.as_ref());
                let (layout_hints, _) = collect_layout_hints(&enm.attributes);
                self.insert_type_info(
                    type_name.clone(),
                    TypeInfo {
                        kind: TypeKind::Enum,
                        generics: enm.generics.clone(),
                        repr_c: layout_hints.repr_c,
                        packing: layout_hints
                            .packing
                            .map(|hint| hint.value.unwrap_or(1).max(1)),
                        align: layout_hints.align.map(|hint| hint.value),
                        is_readonly: false,
                        is_intrinsic: false,
                        visibility: enm.visibility,
                    },
                );
                for variant in &enm.variants {
                    for field in &variant.fields {
                        self.ensure_type_expr(&field.ty, namespace, Some(type_name.as_str()), None);
                    }
                }
            }
            Item::Class(class) => self.register_class(class, namespace),
            Item::Interface(iface) => self.register_interface(iface, namespace),
            Item::Trait(trait_decl) => self.register_trait(trait_decl, namespace),
            Item::Impl(impl_decl) => self.register_impl(impl_decl, namespace),
            Item::Const(const_item) => {
                self.ensure_type_expr(&const_item.declaration.ty, namespace, None, None);
                self.validate_const_declaration(namespace, None, &const_item.declaration);
            }
            Item::Extension(ext) => self.register_extension(ext, namespace),
            Item::Namespace(ns) => {
                let nested = qualify(namespace, &ns.name);
                self.visit_items(&ns.items, Some(&nested));
            }
            Item::Static(_) => {}
            Item::TypeAlias(_) => {}
            Item::TestCase(_) | Item::Import(_) => {}
        }
    }

    fn register_delegate(&mut self, delegate: &DelegateDecl, namespace: Option<&str>) {
        let type_name = qualify(namespace, &delegate.name);
        self.record_type_package(&type_name);
        let arity = delegate.generics.as_ref().map_or(0, |g| g.params.len());
        if !self.reserve_type_slot(
            &type_name,
            RegisteredItemKind::Delegate,
            arity,
            delegate.span,
        ) {
            return;
        }
        self.validate_generics(&type_name, delegate.generics.as_ref());
        self.push_pending_generics(&type_name, delegate.generics.as_ref());

        for param in &delegate.signature.parameters {
            self.ensure_type_expr(&param.ty, namespace, Some(type_name.as_str()), None);
        }
        self.ensure_type_expr(
            &delegate.signature.return_type,
            namespace,
            Some(type_name.as_str()),
            delegate.span,
        );
        self.validate_delegate_variance(&type_name, delegate);

        let sig = signature_from(
            &delegate.signature,
            format!("{type_name}::Invoke"),
            delegate.span,
        );
        let sig_id = self.allocate_signature(sig.clone());
        self.record_signature_generics(sig_id, delegate.generics.as_ref());

        self.insert_type_info(
            type_name.clone(),
            TypeInfo {
                kind: TypeKind::Delegate { _signature: sig },
                generics: delegate.generics.clone(),
                repr_c: false,
                packing: None,
                align: None,
                is_readonly: false,
                is_intrinsic: false,
                visibility: delegate.visibility,
            },
        );
    }

    pub(crate) fn register_function(&mut self, func: &'a FunctionDecl, namespace: Option<&str>) {
        let full_name = qualify(namespace, &func.name);
        self.validate_generics(&full_name, func.generics.as_ref());
        self.register_function_generics(&full_name, func.generics.as_ref());
        self.push_pending_generics(&full_name, func.generics.as_ref());
        if func
            .signature
            .parameters
            .iter()
            .any(|param| param.is_extension_this)
        {
            self.emit_error(
                codes::INVALID_EXTENSION_CONTEXT,
                None,
                format!(
                    "extension methods must be declared as static methods in a non-generic static class; `{full_name}` is a free function"
                ),
            );
        }
        self.ensure_unique_parameter_names(&func.signature.parameters, &full_name, None);
        for param in &func.signature.parameters {
            self.ensure_type_expr(&param.ty, namespace, Some(full_name.as_str()), None);
        }
        self.validate_parameter_defaults(&full_name, &func.signature.parameters, namespace, None);
        self.validate_lends_return_clause(&full_name, &func.signature);
        let return_span = func.body.as_ref().and_then(|body| body.span);
        self.ensure_type_expr(
            &func.signature.return_type,
            namespace,
            Some(full_name.as_str()),
            return_span,
        );
        let signature = signature_from(&func.signature, full_name.clone(), None);
        let id = self.allocate_signature(signature);
        self.record_signature_generics(id, func.generics.as_ref());
        if func.is_async {
            if let Some(result_ty) = self.validate_async_return_type(
                &full_name,
                &func.signature,
                namespace,
                Some(full_name.as_str()),
                return_span,
            ) {
                self.async_signatures.insert(id, result_ty);
            }
        }
        let clause_span = func
            .signature
            .throws
            .as_ref()
            .and_then(|clause| clause.span);
        self.record_declared_effects(&full_name, &func.signature, namespace, None, clause_span);
        self.functions
            .entry(full_name.clone())
            .or_default()
            .push(id);
        if let Some(body) = func.body.as_ref() {
            self.queue_body_validation(&full_name, body, namespace, Some(full_name.as_str()));
        }
        self.pop_pending_generics(&full_name);
    }

    pub(crate) fn allocate_local_function_symbol(&mut self, parent: &str, name: &str) -> String {
        let counter = self
            .local_function_ordinals
            .entry(parent.to_string())
            .or_insert(0);
        let ordinal = *counter;
        *counter += 1;
        local_function_symbol(parent, ordinal, name)
    }

    pub(crate) fn validate_local_function_decl(
        &mut self,
        symbol: &str,
        func: &'a FunctionDecl,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) {
        self.validate_generics(symbol, func.generics.as_ref());
        self.register_function_generics(symbol, func.generics.as_ref());
        self.push_pending_generics(symbol, func.generics.as_ref());
        if func
            .signature
            .parameters
            .iter()
            .any(|param| param.is_extension_this)
        {
            self.emit_error(
                codes::INVALID_EXTENSION_CONTEXT,
                None,
                format!(
                    "extension methods must be declared as static methods in a non-generic static class; `{symbol}` is a local function"
                ),
            );
        }
        self.ensure_unique_parameter_names(&func.signature.parameters, symbol, None);
        for param in &func.signature.parameters {
            self.ensure_type_expr(&param.ty, namespace, Some(symbol), None);
        }
        self.validate_parameter_defaults(
            symbol,
            &func.signature.parameters,
            namespace,
            context_type,
        );
        let return_span = func.body.as_ref().and_then(|body| body.span);
        self.ensure_type_expr(
            &func.signature.return_type,
            namespace,
            Some(symbol),
            return_span,
        );
        self.validate_public_signature(
            symbol,
            func.visibility,
            Some(&func.signature.return_type),
            &func.signature.parameters,
            namespace,
            context_type,
        );
        let signature = signature_from(&func.signature, symbol.to_string(), None);
        let id = self.allocate_signature(signature);
        self.record_signature_generics(id, func.generics.as_ref());
        if func.is_async {
            if let Some(result_ty) = self.validate_async_return_type(
                symbol,
                &func.signature,
                namespace,
                Some(symbol),
                return_span,
            ) {
                self.async_signatures.insert(id, result_ty);
            }
        }
        let clause_span = func
            .signature
            .throws
            .as_ref()
            .and_then(|clause| clause.span);
        self.record_declared_effects(
            symbol,
            &func.signature,
            namespace,
            context_type,
            clause_span,
        );
        self.functions
            .entry(symbol.to_string())
            .or_default()
            .push(id);
        if let Some(body) = func.body.as_ref() {
            self.queue_body_validation(symbol, body, namespace, Some(symbol));
        }
        self.pop_pending_generics(symbol);
    }

    pub(crate) fn register_class(&mut self, class: &'a ClassDecl, namespace: Option<&str>) {
        let full_name = qualify(namespace, &class.name);
        self.record_type_package(&full_name);
        let arity = class.generics.as_ref().map_or(0, |g| g.params.len());
        if !self.reserve_type_slot(&full_name, RegisteredItemKind::Class, arity, None) {
            return;
        }
        self.validate_generics(&full_name, class.generics.as_ref());
        self.push_pending_generics(&full_name, class.generics.as_ref());
        let mut methods = Vec::new();
        let mut properties = Vec::new();
        let mut constructors = Vec::new();
        for member in &class.members {
            match member {
                ClassMember::Method(method) => {
                    let method_name = format!("{full_name}::{}", method.name);
                    self.record_function_package(&method_name);
                    self.validate_generics(&method_name, method.generics.as_ref());
                    self.register_function_generics(&method_name, method.generics.as_ref());
                    self.push_pending_generics(&method_name, method.generics.as_ref());
                    let method_is_static = Self::method_is_static(method);
                    let has_extension_receiver = method_is_static
                        && method
                            .signature
                            .parameters
                            .iter()
                            .any(|param| param.is_extension_this);
                    if has_extension_receiver {
                        if !method
                            .signature
                            .parameters
                            .first()
                            .is_some_and(|param| param.is_extension_this)
                        {
                            self.emit_error(
                                codes::INVALID_EXTENSION_POSITION,
                                None,
                                format!(
                                    "extension receiver on `{method_name}` must be declared as the first parameter"
                                ),
                            );
                        }
                        if method
                            .signature
                            .parameters
                            .iter()
                            .skip(1)
                            .any(|param| param.is_extension_this)
                        {
                            self.emit_error(
                                codes::INVALID_EXTENSION_POSITION,
                                None,
                                format!(
                                    "only the first parameter of `{method_name}` may use the `this` modifier"
                                ),
                            );
                        }
                        let class_is_generic = class
                            .generics
                            .as_ref()
                            .is_some_and(|params| !params.params.is_empty());
                        if !method_is_static {
                            self.emit_error(
                                codes::INVALID_EXTENSION_CONTEXT,
                                None,
                                format!(
                                    "extension method `{method_name}` must be declared as a static method inside a non-generic static class"
                                ),
                            );
                        } else {
                            if !class.is_static || class_is_generic {
                                self.emit_error(
                                    codes::INVALID_EXTENSION_CONTEXT,
                                    None,
                                    format!(
                                        "extension method `{method_name}` must be declared as a static method inside a non-generic static class"
                                    ),
                                );
                            }
                            if !self.enclosing_types.is_empty() {
                                self.emit_error(
                                    codes::INVALID_EXTENSION_CONTEXT,
                                    None,
                                    format!(
                                        "extension method `{method_name}` cannot be declared inside a nested static class; use a non-nested static class"
                                    ),
                                );
                            }
                        }
                    }
                    self.ensure_unique_parameter_names(
                        &method.signature.parameters,
                        &method_name,
                        None,
                    );
                    for param in &method.signature.parameters {
                        self.ensure_type_expr(
                            &param.ty,
                            namespace,
                            Some(method_name.as_str()),
                            None,
                        );
                    }
                    self.validate_parameter_defaults(
                        &method_name,
                        &method.signature.parameters,
                        namespace,
                        Some(full_name.as_str()),
                    );
                    let return_span = method.body.as_ref().and_then(|body| body.span);
                    self.ensure_type_expr(
                        &method.signature.return_type,
                        namespace,
                        Some(method_name.as_str()),
                        return_span,
                    );
                    let sig = signature_from(&method.signature, method_name.clone(), None);
                    let sig_id = self.allocate_signature(sig);
                    self.record_signature_generics(sig_id, method.generics.as_ref());
                    let dispatch_info = MethodDispatchInfo {
                        dispatch: method.dispatch,
                        visibility: method.visibility,
                        is_static: Self::method_is_static(method),
                        has_body: method.body.is_some(),
                        span: method.body.as_ref().and_then(|body| body.span),
                    };
                    self.method_dispatch.insert(sig_id, dispatch_info);
                    self.methods
                        .entry(full_name.clone())
                        .or_default()
                        .push(sig_id);
                    methods.push(sig_id);
                    self.validate_operator(&full_name, method);
                    self.validate_const_function(
                        &method_name,
                        method,
                        namespace,
                        Some(full_name.as_str()),
                    );
                    if method.is_async {
                        if let Some(result_ty) = self.validate_async_return_type(
                            &method_name,
                            &method.signature,
                            namespace,
                            Some(method_name.as_str()),
                            return_span,
                        ) {
                            self.async_signatures.insert(sig_id, result_ty);
                        }
                    }
                    let clause_span = method
                        .signature
                        .throws
                        .as_ref()
                        .and_then(|clause| clause.span);
                    self.record_declared_effects(
                        &method_name,
                        &method.signature,
                        namespace,
                        Some(method_name.as_str()),
                        clause_span,
                    );
                    if let Some(body) = method.body.as_ref() {
                        self.queue_body_validation(
                            &method_name,
                            body,
                            namespace,
                            Some(method_name.as_str()),
                        );
                    }
                    self.pop_pending_generics(&method_name);
                }
                ClassMember::Property(property) => {
                    self.ensure_type_expr(
                        &property.ty,
                        namespace,
                        Some(full_name.as_str()),
                        property.span,
                    );
                    let member_name = format!("{full_name}::{}", property.name);
                    self.validate_public_type_expr(
                        &member_name,
                        property.visibility,
                        &property.ty,
                        namespace,
                        Some(full_name.as_str()),
                    );
                    properties.push(PropertyInfo::from_decl(property));
                }
                ClassMember::Constructor(ctor) => {
                    self.ensure_unique_parameter_names(
                        &ctor.parameters,
                        &format!("{full_name}::init"),
                        ctor.span,
                    );
                    for param in &ctor.parameters {
                        self.ensure_type_expr(&param.ty, namespace, Some(full_name.as_str()), None);
                    }
                    let ctor_name = format!("{full_name}::init");
                    self.validate_parameter_defaults(
                        &ctor_name,
                        &ctor.parameters,
                        namespace,
                        Some(full_name.as_str()),
                    );
                    self.validate_public_signature(
                        &ctor_name,
                        ctor.visibility,
                        None,
                        &ctor.parameters,
                        namespace,
                        Some(full_name.as_str()),
                    );
                    constructors.push(ConstructorInfo {
                        visibility: ctor.visibility,
                        param_count: ctor.parameters.len(),
                    });
                    if let Some(initializer) = &ctor.initializer {
                        for argument in &initializer.arguments {
                            self.validate_expression(
                                &ctor_name,
                                argument,
                                namespace,
                                Some(full_name.as_str()),
                            );
                        }
                    }
                    if let Some(body) = ctor.body.as_ref() {
                        self.queue_body_validation(
                            &ctor_name,
                            body,
                            namespace,
                            Some(full_name.as_str()),
                        );
                    }
                }
                ClassMember::Field(field) => {
                    self.ensure_type_expr(&field.ty, namespace, Some(full_name.as_str()), None);
                    let member_name = format!("{full_name}::{}", field.name);
                    self.validate_public_type_expr(
                        &member_name,
                        field.visibility,
                        &field.ty,
                        namespace,
                        Some(full_name.as_str()),
                    );
                    if let Some(initializer) = &field.initializer {
                        let field_name = format!("{full_name}::{}", field.name);
                        self.validate_expression(
                            &field_name,
                            initializer,
                            namespace,
                            Some(full_name.as_str()),
                        );
                        self.check_numeric_literal_expression(initializer, Some(&field.ty));
                    }
                }
                ClassMember::Const(const_member) => {
                    self.ensure_type_expr(
                        &const_member.declaration.ty,
                        namespace,
                        Some(full_name.as_str()),
                        None,
                    );
                    self.validate_const_declaration(
                        namespace,
                        Some(full_name.as_str()),
                        &const_member.declaration,
                    );
                }
            }
        }

        let bases = class
            .bases
            .iter()
            .filter_map(|base| {
                match self.resolve_type_for_expr(base, namespace, Some(full_name.as_str())) {
                    ImportResolution::Found(resolved) => {
                        if let Some(info) = self.resolve_type_info(&resolved).cloned() {
                            if !self.type_accessible_from_current(
                                info.visibility,
                                &resolved,
                                None,
                                namespace,
                                Some(full_name.as_str()),
                            ) {
                                self.emit_error(
                                    codes::INACCESSIBLE_BASE,
                                    base.span,
                                    format!(
                                        "type `{}` cannot inherit from `{}` because it is not accessible from this package",
                                        full_name, resolved
                                    ),
                                );
                                return None;
                            }
                        }
                        self.ensure_type_expr(base, namespace, Some(full_name.as_str()), None);
                        Some(BaseTypeBinding::new(resolved, base.clone()))
                    }
                    ImportResolution::Ambiguous(candidates) => {
                        self.emit_error(
                            codes::AMBIGUOUS_CLASS_BASE,
                            base.span,
                            format!(
                                "ambiguous base type `{}`; candidates: {}",
                                base.name,
                                candidates.join(", ")
                            ),
                        );
                        None
                    }
                    ImportResolution::NotFound => {
                        self.emit_error(
                            codes::BASE_TYPE_NOT_FOUND,
                            base.span,
                            format!(
                                "base type `{}` for `{}` could not be resolved",
                                base.name, full_name
                            ),
                        );
                        None
                    }
                }
            })
            .collect::<Vec<_>>();

        let (layout_hints, _) = collect_layout_hints(&class.attributes);
        let class_name = full_name.clone();
        self.insert_type_info(
            full_name,
            TypeInfo {
                kind: TypeKind::Class {
                    methods,
                    bases,
                    kind: class.kind,
                    properties,
                    constructors,
                    is_abstract: class.is_abstract,
                    is_sealed: class.is_sealed,
                    is_static: class.is_static,
                },
                generics: class.generics.clone(),
                repr_c: layout_hints.repr_c,
                packing: layout_hints
                    .packing
                    .map(|hint| hint.value.unwrap_or(1).max(1)),
                align: layout_hints.align.map(|hint| hint.value),
                is_readonly: false,
                is_intrinsic: false,
                visibility: class.visibility,
            },
        );
        if !class.nested_types.is_empty() {
            self.enclosing_types.push(class_name.clone());
            self.visit_items(&class.nested_types, Some(class_name.as_str()));
            self.enclosing_types.pop();
        }
    }

    pub(crate) fn register_interface(&mut self, iface: &'a InterfaceDecl, namespace: Option<&str>) {
        let full_name = qualify(namespace, &iface.name);
        self.record_type_package(&full_name);
        let arity = iface.generics.as_ref().map_or(0, |g| g.params.len());
        if !self.reserve_type_slot(&full_name, RegisteredItemKind::Interface, arity, None) {
            return;
        }
        self.validate_generics(&full_name, iface.generics.as_ref());
        self.push_pending_generics(&full_name, iface.generics.as_ref());
        self.validate_interface_variance(&full_name, iface);
        let mut methods = Vec::new();
        let mut trait_methods = Vec::new();
        let mut trait_consts = Vec::new();
        let mut associated_types = Vec::new();
        let mut object_safety = TraitObjectSafety::default();
        let mut properties = Vec::new();
        for member in &iface.members {
            match member {
                InterfaceMember::Method(method) => {
                    let method_name = format!("{full_name}::{}", method.name);
                    self.record_function_package(&method_name);
                    self.validate_generics(&method_name, method.generics.as_ref());
                    self.ensure_unique_parameter_names(
                        &method.signature.parameters,
                        &method_name,
                        None,
                    );
                    self.validate_parameter_defaults(
                        &method_name,
                        &method.signature.parameters,
                        namespace,
                        Some(full_name.as_str()),
                    );
                    self.ensure_type_expr(
                        &method.signature.return_type,
                        namespace,
                        Some(full_name.as_str()),
                        None,
                    );
                    if method
                        .generics
                        .as_ref()
                        .is_some_and(|params| !params.params.is_empty())
                    {
                        object_safety.record(ObjectSafetyViolation {
                            kind: ObjectSafetyViolationKind::GenericMethod,
                            member: method_name.clone(),
                            span: None,
                        });
                    }
                    if returns_self_value(&method.signature.return_type) {
                        object_safety.record(ObjectSafetyViolation {
                            kind: ObjectSafetyViolationKind::ReturnsSelf,
                            member: method_name.clone(),
                            span: None,
                        });
                    }
                    let sig = signature_from(&method.signature, method_name.clone(), None);
                    let sig_id = self.allocate_signature(sig);
                    self.record_signature_generics(sig_id, method.generics.as_ref());
                    methods.push(sig_id);
                    trait_methods.push(TraitMethodInfo {
                        name: method.name.clone(),
                        signature: sig_id,
                        has_body: method.body.is_some(),
                        is_async: method.is_async,
                    });
                    if method.is_async {
                        if let Some(result_ty) = self.validate_async_return_type(
                            &method_name,
                            &method.signature,
                            namespace,
                            Some(full_name.as_str()),
                            None,
                        ) {
                            self.async_signatures.insert(sig_id, result_ty);
                        }
                    }
                    let clause_span = method
                        .signature
                        .throws
                        .as_ref()
                        .and_then(|clause| clause.span);
                    self.record_declared_effects(
                        &method_name,
                        &method.signature,
                        namespace,
                        Some(full_name.as_str()),
                        clause_span,
                    );
                    self.validate_const_function(
                        &method_name,
                        method,
                        namespace,
                        Some(full_name.as_str()),
                    );
                    if let Some(body) = method.body.as_ref() {
                        self.queue_body_validation(
                            &method_name,
                            body,
                            namespace,
                            Some(full_name.as_str()),
                        );
                        self.register_interface_default_provider(
                            &full_name,
                            InterfaceDefaultProvider {
                                method: method.name.clone(),
                                symbol: method_name,
                                kind: InterfaceDefaultKind::Inline,
                                conditions: Vec::new(),
                                span: None,
                                origin: format!("{full_name} (inline)"),
                            },
                        );
                    }
                }
                InterfaceMember::Property(property) => {
                    properties.push(PropertyInfo::from_decl(property));
                }
                InterfaceMember::Const(const_member) => {
                    self.ensure_type_expr(
                        &const_member.declaration.ty,
                        namespace,
                        Some(full_name.as_str()),
                        None,
                    );
                    self.validate_const_declaration(
                        namespace,
                        Some(full_name.as_str()),
                        &const_member.declaration,
                    );
                    trait_consts.push(const_member.clone());
                }
                InterfaceMember::AssociatedType(assoc) => {
                    if let Some(default) = &assoc.default {
                        self.ensure_type_expr(
                            default,
                            namespace,
                            Some(full_name.as_str()),
                            assoc.span,
                        );
                    } else {
                        object_safety.record(ObjectSafetyViolation {
                            kind: ObjectSafetyViolationKind::MissingAssociatedTypeDefault,
                            member: format!("{full_name}::{}", assoc.name),
                            span: assoc.span,
                        });
                    }
                    associated_types.push(TraitAssociatedTypeInfo {
                        name: assoc.name.clone(),
                        generics: assoc.generics.clone(),
                        default: assoc.default.clone(),
                    });
                }
            }
        }
        let bases = iface
            .bases
            .iter()
            .filter_map(|base| {
                match self.resolve_type_for_expr(base, namespace, Some(full_name.as_str())) {
                    ImportResolution::Found(resolved) => {
                        if let Some(info) = self.resolve_type_info(&resolved).cloned() {
                            if !self.type_accessible_from_current(
                                info.visibility,
                                &resolved,
                                None,
                                namespace,
                                Some(full_name.as_str()),
                            ) {
                                self.emit_error(
                                    codes::INACCESSIBLE_BASE,
                                    base.span,
                                    format!(
                                        "type `{}` cannot inherit from `{}` because it is not accessible from this package",
                                        full_name, resolved
                                    ),
                                );
                                return None;
                            }
                        }
                        self.ensure_type_expr(base, namespace, Some(full_name.as_str()), None);
                        Some(BaseTypeBinding::new(resolved, base.clone()))
                    }
                    ImportResolution::Ambiguous(candidates) => {
                        self.emit_error(
                            codes::AMBIGUOUS_INTERFACE_BASE,
                            base.span,
                            format!(
                                "ambiguous interface base `{}`; candidates: {}",
                                base.name,
                                candidates.join(", ")
                            ),
                        );
                        None
                    }
                    ImportResolution::NotFound => {
                        self.emit_error(
                            codes::BASE_TYPE_NOT_FOUND,
                            base.span,
                            format!(
                                "base type `{}` for `{}` could not be resolved",
                                base.name, full_name
                            ),
                        );
                        None
                    }
                }
            })
            .collect::<Vec<_>>();
        let trait_super_bases: Vec<_> = bases.iter().map(|base| base.expr.clone()).collect();
        self.insert_type_info(
            full_name.clone(),
            TypeInfo {
                kind: TypeKind::Interface {
                    methods,
                    properties,
                    bases,
                },
                generics: iface.generics.clone(),
                repr_c: false,
                packing: None,
                align: None,
                is_readonly: false,
                is_intrinsic: false,
                visibility: iface.visibility,
            },
        );
        self.insert_trait_info(
            full_name.clone(),
            TraitInfo {
                methods: trait_methods,
                associated_types,
                consts: trait_consts,
                generics: iface.generics.clone(),
                super_traits: trait_super_bases,
                object_safety,
                auto_trait_overrides: AutoTraitOverride {
                    thread_safe: iface.thread_safe_override,
                    shareable: iface.shareable_override,
                    copy: iface.copy_override,
                },
                span: None,
            },
        );
    }

    pub(crate) fn register_extension(&mut self, ext: &'a ExtensionDecl, namespace: Option<&str>) {
        let resolution = self.resolve_type_for_expr(&ext.target, namespace, None);
        let target_resolution_ambiguous = matches!(resolution, ImportResolution::Ambiguous(_));
        let target_name = match resolution {
            ImportResolution::Found(resolved) => resolved,
            ImportResolution::Ambiguous(candidates) => {
                self.emit_error(
                    codes::AMBIGUOUS_EXTENSION_TARGET,
                    ext.target.span,
                    format!(
                        "extension target `{}` resolves to multiple candidates: {}",
                        ext.target.name,
                        candidates.join(", ")
                    ),
                );
                qualify(namespace, &ext.target.name)
            }
            ImportResolution::NotFound => {
                self.emit_error(
                    codes::UNKNOWN_EXTENSION_TARGET,
                    ext.target.span,
                    format!(
                        "extension target `{}` could not be resolved",
                        ext.target.name
                    ),
                );
                qualify(namespace, &ext.target.name)
            }
        };

        self.ensure_type_expr(&ext.target, namespace, Some(target_name.as_str()), None);
        let is_interface_target = self.is_interface(&target_name);
        let normalized_conditions = normalize_extension_conditions(self, ext, namespace);
        let allows_interface_target = ext.members.iter().any(|member| {
            matches!(
                member,
                crate::frontend::ast::ExtensionMember::Method(method) if method.is_default
            )
        });

        let owner_key = base_type_name(&target_name).to_string();
        let display_owner = target_name.clone();

        let is_builtin_extension_target = matches!(
            target_name.as_str(),
            "Std::String" | "Std::Str" | "string" | "str"
        );
        let type_known = self.symbol_index.type_names().contains(&owner_key)
            || self.symbol_index.type_names().contains(&target_name);
        let type_known = type_known || is_builtin_extension_target;
        if !target_resolution_ambiguous {
            if let Some(info) = self.resolve_type_info(&target_name) {
                match info.kind {
                    TypeKind::Struct { .. } | TypeKind::Class { .. } => {}
                    TypeKind::Interface { .. } if allows_interface_target => {}
                    _ => {
                        self.emit_error(
                            codes::INVALID_EXTENSION_TARGET_KIND,
                            None,
                            format!("extension target `{target_name}` must be a struct or class"),
                        );
                    }
                }
            } else if !type_known {
                self.emit_error(
                    codes::UNKNOWN_EXTENSION_TARGET,
                    None,
                    format!("unknown extension target `{target_name}`"),
                );
            }
        }

        self.validate_generics(&target_name, ext.generics.as_ref());

        if normalized_conditions.is_none() {
            return;
        }

        for crate::frontend::ast::ExtensionMember::Method(method) in &ext.members {
            let function = &method.function;
            let method_name = format!("{display_owner}::{}", function.name);
            self.record_function_package(&method_name);
            self.validate_generics(&method_name, function.generics.as_ref());
            self.register_function_generics(&method_name, function.generics.as_ref());
            self.push_pending_generics(&method_name, function.generics.as_ref());
            self.ensure_unique_parameter_names(&function.signature.parameters, &method_name, None);
            self.validate_parameter_defaults(
                &method_name,
                &function.signature.parameters,
                namespace,
                Some(method_name.as_str()),
            );
            let return_span = function.body.as_ref().and_then(|body| body.span);
            self.ensure_type_expr(
                &function.signature.return_type,
                namespace,
                Some(method_name.as_str()),
                return_span,
            );

            if !function
                .signature
                .parameters
                .first()
                .is_some_and(|param| param.is_extension_this)
            {
                self.emit_error(
                    codes::MISSING_EXTENSION_RECEIVER,
                    None,
                    format!(
                        "extension method `{}` must declare a leading `this` receiver parameter",
                        method_name
                    ),
                );
                self.pop_pending_generics(&method_name);
                continue;
            }

            let receiver = &function.signature.parameters[0].ty;
            let receiver_base = receiver.base.last();
            let target_base = ext.target.base.last();
            let is_self = receiver_base.is_some_and(|segment| segment == "Self");
            let matches_target = receiver_base
                .zip(target_base)
                .is_some_and(|(recv, target)| recv == target);
            if !is_self && !matches_target {
                self.emit_error(
                    codes::INVALID_EXTENSION_RECEIVER,
                    None,
                    format!(
                        "receiver parameter on extension method `{}` must be typed as `Self` or `{}`",
                        method_name,
                        ext.target.name
                    ),
                );
            }

            let substituted = signature_from_extension(
                &function.signature,
                method_name.clone(),
                None,
                &ext.target,
            );
            let sig_id = self.allocate_signature(substituted);
            self.record_signature_generics(sig_id, function.generics.as_ref());
            self.methods
                .entry(owner_key.clone())
                .or_default()
                .push(sig_id);
            self.validate_operator(&display_owner, function);
            if function.is_async {
                if let Some(result_ty) = self.validate_async_return_type(
                    &method_name,
                    &function.signature,
                    namespace,
                    Some(method_name.as_str()),
                    return_span,
                ) {
                    self.async_signatures.insert(sig_id, result_ty);
                }
            }
            let clause_span = function
                .signature
                .throws
                .as_ref()
                .and_then(|clause| clause.span);
            self.record_declared_effects(
                &method_name,
                &function.signature,
                namespace,
                Some(method_name.as_str()),
                clause_span,
            );
            self.validate_const_function(
                &method_name,
                function,
                namespace,
                Some(method_name.as_str()),
            );
            if let Some(body) = function.body.as_ref() {
                self.queue_body_validation(
                    &method_name,
                    body,
                    namespace,
                    Some(method_name.as_str()),
                );
            }
            if method.is_default {
                if !is_interface_target {
                    self.emit_error(
                        codes::DEFAULT_TARGET_INVALID,
                        return_span,
                        format!(
                            "default extension method `{}` must target an interface; `{target_name}` is not an interface",
                            method_name
                        ),
                    );
                } else if let Some(conditions) = normalized_conditions.clone() {
                    let symbol = extension_method_symbol(
                        target_name.as_str(),
                        namespace,
                        &function.name,
                        true,
                    );
                    self.register_interface_default_provider(
                        &target_name,
                        InterfaceDefaultProvider {
                            method: function.name.clone(),
                            symbol,
                            kind: InterfaceDefaultKind::Extension,
                            conditions,
                            span: return_span,
                            origin: format!(
                                "extension {}::{}",
                                namespace.unwrap_or("<root>"),
                                ext.target.name
                            ),
                        },
                    );
                }
            }

            self.pop_pending_generics(&method_name);
        }
    }
}
