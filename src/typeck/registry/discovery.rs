use super::super::arena::{
    ImplInfo, ObjectSafetyViolation, ObjectSafetyViolationKind, TraitAssociatedTypeInfo, TraitInfo,
    TraitMethodInfo, TraitObjectSafety, TypeChecker, TypeInfo, TypeKind,
};
use super::diagnostics::codes;
use super::hooks::RegisteredItemKind;
use super::{qualify, returns_self_value, signature_from};
use crate::frontend::ast::{ImplDecl, ImplMember, TraitDecl, TraitMember};
use crate::mir::AutoTraitOverride;
use std::collections::HashSet;

impl<'a> TypeChecker<'a> {
    pub(super) fn register_trait(&mut self, trait_decl: &'a TraitDecl, namespace: Option<&str>) {
        let trait_name = qualify(namespace, &trait_decl.name);
        let arity = trait_decl.generics.as_ref().map_or(0, |g| g.params.len());
        if !self.reserve_type_slot(
            &trait_name,
            RegisteredItemKind::Trait,
            arity,
            trait_decl.span,
        ) {
            return;
        }
        self.validate_generics(&trait_name, trait_decl.generics.as_ref());
        self.push_pending_generics(&trait_name, trait_decl.generics.as_ref());
        let mut object_safety = TraitObjectSafety::default();
        let auto_trait_overrides = AutoTraitOverride {
            thread_safe: trait_decl.thread_safe_override,
            shareable: trait_decl.shareable_override,
            copy: trait_decl.copy_override,
        };
        self.insert_type_info(
            trait_name.clone(),
            TypeInfo {
                kind: TypeKind::Trait,
                generics: trait_decl.generics.clone(),
                repr_c: false,
                packing: None,
                align: None,
                is_readonly: false,
                is_intrinsic: false,
                visibility: trait_decl.visibility,
            },
        );
        for bound in &trait_decl.super_traits {
            self.ensure_type_expr(bound, namespace, Some(trait_name.as_str()), trait_decl.span);
        }

        let mut methods = Vec::new();
        let mut associated_types = Vec::new();
        let mut consts = Vec::new();

        for member in &trait_decl.members {
            match member {
                TraitMember::Method(method) => {
                    let method_name = format!("{trait_name}::{}", method.name);
                    self.validate_generics(&method_name, method.generics.as_ref());
                    if method
                        .generics
                        .as_ref()
                        .is_some_and(|params| !params.params.is_empty())
                    {
                        object_safety.record(ObjectSafetyViolation {
                            kind: ObjectSafetyViolationKind::GenericMethod,
                            member: method_name.clone(),
                            span: trait_decl.span,
                        });
                    }
                    self.ensure_unique_parameter_names(
                        &method.signature.parameters,
                        &method_name,
                        None,
                    );
                    self.validate_parameter_defaults(
                        &method_name,
                        &method.signature.parameters,
                        namespace,
                        Some(trait_name.as_str()),
                    );
                    let return_span = method.body.as_ref().and_then(|body| body.span);
                    self.ensure_type_expr(
                        &method.signature.return_type,
                        namespace,
                        Some(trait_name.as_str()),
                        return_span,
                    );
                    if returns_self_value(&method.signature.return_type) {
                        object_safety.record(ObjectSafetyViolation {
                            kind: ObjectSafetyViolationKind::ReturnsSelf,
                            member: method_name.clone(),
                            span: trait_decl.span,
                        });
                    }
                    for param in &method.signature.parameters {
                        self.ensure_type_expr(
                            &param.ty,
                            namespace,
                            Some(trait_name.as_str()),
                            None,
                        );
                    }
                    let sig = signature_from(&method.signature, method_name.clone(), None);
                    let sig_id = self.allocate_signature(sig);
                    let clause_span = method
                        .signature
                        .throws
                        .as_ref()
                        .and_then(|clause| clause.span);
                    self.record_declared_effects(
                        &method_name,
                        &method.signature,
                        namespace,
                        Some(trait_name.as_str()),
                        clause_span,
                    );
                    self.validate_const_function(
                        &method_name,
                        method,
                        namespace,
                        Some(trait_name.as_str()),
                    );
                    methods.push(TraitMethodInfo {
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
                            Some(trait_name.as_str()),
                            return_span,
                        ) {
                            self.async_signatures.insert(sig_id, result_ty);
                        }
                    }
                    if let Some(body) = method.body.as_ref() {
                        self.queue_body_validation(
                            &method_name,
                            body,
                            namespace,
                            Some(trait_name.as_str()),
                        );
                    }
                }
                TraitMember::AssociatedType(assoc) => {
                    if let Some(default) = &assoc.default {
                        self.ensure_type_expr(
                            default,
                            namespace,
                            Some(trait_name.as_str()),
                            assoc.span,
                        );
                    } else {
                        object_safety.record(ObjectSafetyViolation {
                            kind: ObjectSafetyViolationKind::MissingAssociatedTypeDefault,
                            member: format!("{trait_name}::{}", assoc.name),
                            span: assoc.span.or(trait_decl.span),
                        });
                    }
                    associated_types.push(TraitAssociatedTypeInfo {
                        name: assoc.name.clone(),
                        generics: assoc.generics.clone(),
                        default: assoc.default.clone(),
                    });
                }
                TraitMember::Const(const_member) => {
                    self.ensure_type_expr(
                        &const_member.declaration.ty,
                        namespace,
                        Some(trait_name.as_str()),
                        const_member.declaration.span,
                    );
                    consts.push(const_member.clone());
                    self.validate_const_declaration(
                        namespace,
                        Some(trait_name.as_str()),
                        &const_member.declaration,
                    );
                }
            }
        }

        self.insert_trait_info(
            trait_name.clone(),
            TraitInfo {
                methods,
                associated_types,
                consts,
                generics: trait_decl.generics.clone(),
                super_traits: trait_decl.super_traits.clone(),
                object_safety,
                auto_trait_overrides,
                span: trait_decl.span,
            },
        );
    }

    pub(super) fn register_impl(&mut self, impl_decl: &'a ImplDecl, namespace: Option<&str>) {
        if let Some(trait_ref) = &impl_decl.trait_ref {
            self.ensure_type_expr(trait_ref, namespace, None, impl_decl.span);
        }
        self.ensure_type_expr(&impl_decl.target, namespace, None, impl_decl.span);

        let qualified_trait = impl_decl
            .trait_ref
            .as_ref()
            .map(|ty| qualify(namespace, &ty.name));
        let trait_label = qualified_trait
            .as_deref()
            .and_then(|name| name.rsplit("::").next())
            .unwrap_or("impl");
        let impl_name = format!("impl {}", impl_decl.target.name);
        for member in &impl_decl.members {
            match member {
                ImplMember::Method(method) => {
                    let method_name = format!("{impl_name}::{}", method.name);
                    let lowered_name = format!(
                        "{}::{trait_label}::{}",
                        qualify(namespace, &impl_decl.target.name),
                        method.name
                    );
                    self.validate_generics(&method_name, method.generics.as_ref());
                    self.ensure_unique_parameter_names(
                        &method.signature.parameters,
                        &method_name,
                        None,
                    );
                    let return_span = method.body.as_ref().and_then(|body| body.span);
                    self.ensure_type_expr(
                        &method.signature.return_type,
                        namespace,
                        None,
                        return_span,
                    );
                    for param in &method.signature.parameters {
                        self.ensure_type_expr(&param.ty, namespace, None, None);
                    }
                    let clause_span = method
                        .signature
                        .throws
                        .as_ref()
                        .and_then(|clause| clause.span);
                    self.record_declared_effects(
                        &lowered_name,
                        &method.signature,
                        namespace,
                        None,
                        clause_span,
                    );
                    let signature = signature_from(&method.signature, lowered_name.clone(), None);
                    let sig_id = self.allocate_signature(signature);
                    self.record_signature_generics(sig_id, method.generics.as_ref());
                    self.functions
                        .entry(lowered_name.clone())
                        .or_default()
                        .push(sig_id);
                    if method.is_async {
                        if let Some(result_ty) = self.validate_async_return_type(
                            &lowered_name,
                            &method.signature,
                            namespace,
                            None,
                            return_span,
                        ) {
                            self.async_signatures.insert(sig_id, result_ty);
                        }
                    }
                    if let Some(body) = method.body.as_ref() {
                        self.queue_body_validation(&lowered_name, body, namespace, None);
                    }
                }
                ImplMember::AssociatedType(assoc) => {
                    if let Some(default) = &assoc.default {
                        self.ensure_type_expr(default, namespace, None, assoc.span);
                    }
                }
                ImplMember::Const(const_member) => {
                    self.ensure_type_expr(
                        &const_member.declaration.ty,
                        namespace,
                        None,
                        const_member.declaration.span,
                    );
                    self.validate_const_declaration(namespace, None, &const_member.declaration);
                }
            }
        }

        let qualified_target = qualify(namespace, &impl_decl.target.name);

        self.impls.push(ImplInfo {
            trait_name: qualified_trait.clone(),
            target: impl_decl.target.clone(),
            generics: impl_decl.generics.clone(),
            span: impl_decl.span,
        });

        let Some(trait_name) = qualified_trait else {
            self.emit_error(
                codes::TRAIT_FEATURE_UNAVAILABLE,
                impl_decl.span,
                "inherent `impl` blocks are not supported yet",
            );
            return;
        };

        let Some(trait_info) = self.traits.get(&trait_name).cloned() else {
            self.emit_error(
                codes::TRAIT_NOT_IMPLEMENTED,
                impl_decl.span,
                format!("trait `{trait_name}` is not defined in this module"),
            );
            return;
        };

        if impl_decl
            .generics
            .as_ref()
            .is_some_and(|params| !params.params.is_empty())
        {
            self.emit_error(
                codes::TRAIT_IMPL_SPECIALIZATION_FORBIDDEN,
                impl_decl.span,
                "blanket trait implementations are not supported (no specialization or negative reasoning)",
            );
        }

        let mut method_impls = HashSet::new();
        let mut assoc_impls = HashSet::new();

        for member in &impl_decl.members {
            match member {
                ImplMember::Method(method) => {
                    method_impls.insert(method.name.clone());
                }
                ImplMember::AssociatedType(assoc) => {
                    assoc_impls.insert(assoc.name.clone());
                }
                ImplMember::Const(_const_member) => {}
            }
        }

        for method in &trait_info.methods {
            if method.has_body {
                continue;
            }
            if !method_impls.contains(&method.name) {
                self.emit_error(
                    codes::TRAIT_MEMBER_MISMATCH,
                    impl_decl.span,
                    format!(
                        "impl of `{trait_name}` for `{qualified_target}` is missing method `{}`",
                        method.name
                    ),
                );
            }
        }

        for assoc in &trait_info.associated_types {
            if assoc.default.is_some() {
                continue;
            }
            if !assoc_impls.contains(&assoc.name) {
                self.emit_error(
                    codes::TRAIT_MEMBER_MISMATCH,
                    impl_decl.span,
                    format!(
                        "impl of `{trait_name}` for `{qualified_target}` is missing associated type `{}`",
                        assoc.name
                    ),
                );
            }
        }
    }
}
