use super::arena::{BaseTypeBinding, FunctionSignature, TypeChecker, TypeInfo, TypeKind};
use super::coercions::parse_type_text;
use super::diagnostics;
use super::diagnostics::codes;
use super::helpers::{base_type_name, strip_receiver, type_names_equivalent};
use super::{AutoTraitConstraintOrigin, AutoTraitKind};
use crate::di::{DiDependency, DiInjectionSite, DiService};
use crate::frontend::ast::{
    AutoTraitConstraint, ClassKind, DiLifetime, GenericConstraintKind, GenericParam,
    GenericParamKind, TypeExpr, Variance, Visibility,
};
use crate::mir::AutoTraitStatus;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Clone, Copy)]
pub(super) struct AutoTraitCheck<'a> {
    pub function: &'a str,
    pub target: &'a str,
    pub ty: &'a str,
    pub kind: AutoTraitKind,
    pub origin: AutoTraitConstraintOrigin,
    pub span: Option<Span>,
}

use crate::frontend::diagnostics::Span;

#[allow(dead_code)]
impl<'a> TypeChecker<'a> {
    pub(super) fn validate_type_hierarchy(&mut self) {
        let type_names: Vec<String> = self.types.keys().cloned().collect();
        for name in &type_names {
            let Some(infos) = self.types.get(name).cloned() else {
                continue;
            };
            for info in infos {
                match &info.kind {
                    TypeKind::Class {
                        bases,
                        is_abstract,
                        is_sealed,
                        ..
                    } => {
                        self.validate_class_flags(name, *is_abstract, *is_sealed);
                        self.validate_class_bases(name, info.visibility, bases);
                    }
                    TypeKind::Interface { bases, .. } => {
                        self.validate_interface_bases(name, bases);
                    }
                    TypeKind::Struct { bases, .. } => {
                        self.validate_struct_bases(name, bases);
                    }
                    _ => {}
                }
            }
        }
        self.detect_interface_cycles();
    }

    fn validate_class_flags(&mut self, name: &str, is_abstract: bool, is_sealed: bool) {
        if is_abstract && is_sealed {
            self.emit_error(
                codes::ABSTRACT_SEALED_CLASS,
                None,
                format!("class `{name}` cannot be both abstract and sealed"),
            );
        }
    }

    fn validate_class_bases(
        &mut self,
        class_name: &str,
        visibility: Visibility,
        bases: &[BaseTypeBinding],
    ) {
        let mut base_class: Option<String> = None;
        for base in bases {
            let Some(base_info) = self.resolve_type_info(&base.name).cloned() else {
                continue;
            };
            match &base_info.kind {
                TypeKind::Class {
                    is_sealed,
                    is_static,
                    ..
                } => {
                    if let Some(existing) = &base_class {
                        self.emit_error(
                            codes::MULTIPLE_BASE_CLASSES,
                            None,
                            format!(
                                "class `{class_name}` cannot inherit from multiple base classes (`{existing}` and `{}`)",
                                base.name
                            ),
                        );
                    } else {
                        base_class = Some(base.name.clone());
                    }
                    if *is_sealed {
                        self.emit_error(
                            codes::SEALED_BASE_INHERITANCE,
                            None,
                            format!(
                                "class `{class_name}` cannot derive from sealed class `{}`",
                                base.name
                            ),
                        );
                    }
                    if *is_static {
                        self.emit_error(
                            codes::STATIC_BASE_INHERITANCE,
                            None,
                            format!(
                                "class `{class_name}` cannot derive from static class `{}`",
                                base.name
                            ),
                        );
                    }
                    self.validate_accessibility(
                        class_name,
                        visibility,
                        base_info.visibility,
                        &base.name,
                    );
                }
                TypeKind::Interface { .. } => {
                    self.validate_accessibility(
                        class_name,
                        visibility,
                        base_info.visibility,
                        &base.name,
                    );
                }
                _ => {
                    self.emit_error(
                        codes::INVALID_BASE_TYPE,
                        None,
                        format!(
                            "type `{class_name}` cannot inherit from non-class/interface type `{}`",
                            base.name
                        ),
                    );
                }
            }
        }
    }

    fn validate_interface_bases(&mut self, iface_name: &str, bases: &[BaseTypeBinding]) {
        for base in bases {
            let Some(base_info) = self.resolve_type_info(&base.name) else {
                continue;
            };
            if !matches!(base_info.kind, TypeKind::Interface { .. }) {
                self.emit_error(
                    codes::INVALID_BASE_TYPE,
                    None,
                    format!(
                        "interface `{iface_name}` cannot inherit from non-interface type `{}`",
                        base.name
                    ),
                );
            }
        }
    }

    fn validate_struct_bases(&mut self, struct_name: &str, bases: &[BaseTypeBinding]) {
        for base in bases {
            let Some(base_info) = self.resolve_type_info(&base.name) else {
                continue;
            };
            match &base_info.kind {
                TypeKind::Interface { .. } => {
                    self.validate_accessibility(
                        struct_name,
                        Visibility::Public,
                        base_info.visibility,
                        &base.name,
                    );
                }
                _ => {
                    self.emit_error(
                        codes::INVALID_BASE_TYPE,
                        None,
                        format!(
                            "struct `{struct_name}` cannot inherit from non-interface type `{}`",
                            base.name
                        ),
                    );
                }
            }
        }
    }

    fn detect_interface_cycles(&mut self) {
        let interfaces: Vec<String> = self
            .types
            .iter()
            .filter_map(|(name, infos)| {
                infos
                    .iter()
                    .any(|info| matches!(info.kind, TypeKind::Interface { .. }))
                    .then(|| name.clone())
            })
            .collect();
        let mut visiting = Vec::new();
        let mut visited = HashSet::new();
        let mut reported = HashSet::new();
        for iface in interfaces {
            self.detect_interface_cycle_from(&iface, &mut visiting, &mut visited, &mut reported);
        }
    }

    fn detect_interface_cycle_from(
        &mut self,
        iface: &str,
        stack: &mut Vec<String>,
        visited: &mut HashSet<String>,
        reported: &mut HashSet<String>,
    ) {
        if visited.contains(iface) {
            return;
        }
        stack.push(iface.to_string());
        let Some(info) = self.resolve_type_info(iface).cloned() else {
            stack.pop();
            return;
        };
        let Some(bases) = (match &info.kind {
            TypeKind::Interface { bases, .. } => Some(bases.clone()),
            _ => None,
        }) else {
            stack.pop();
            return;
        };

        for base in bases {
            if let Some(pos) = stack.iter().position(|entry| entry == &base.name) {
                let mut cycle: Vec<String> = stack[pos..].to_vec();
                cycle.push(base.name.clone());
                for member in &cycle {
                    if reported.insert(member.clone()) {
                        self.emit_error(
                            codes::INTERFACE_CYCLE,
                            None,
                            format!(
                                "interface `{}` participates in an inheritance cycle: {}",
                                member,
                                cycle.join(" -> ")
                            ),
                        );
                    }
                }
                continue;
            }
            self.detect_interface_cycle_from(&base.name, stack, visited, reported);
        }

        stack.pop();
        visited.insert(iface.to_string());
    }

    fn validate_accessibility(
        &mut self,
        derived: &str,
        derived_vis: Visibility,
        base_vis: Visibility,
        base_name: &str,
    ) {
        if matches!(derived_vis, Visibility::Public) && !matches!(base_vis, Visibility::Public) {
            self.emit_error(
                codes::INACCESSIBLE_BASE,
                None,
                format!(
                    "public type `{derived}` cannot inherit from less accessible `{base_name}`"
                ),
            );
        }
    }

    pub(super) fn check_interface_fulfillment(&mut self) {
        let type_names: Vec<String> = self.types.keys().cloned().collect();
        for name in type_names {
            let Some(infos) = self.types.get(&name).cloned() else {
                continue;
            };
            for info in infos {
                let TypeKind::Class {
                    methods,
                    bases,
                    kind,
                    properties,
                    ..
                } = &info.kind
                else {
                    continue;
                };
                let is_error_type = matches!(kind, ClassKind::Error);
                let interface_closure = self.collect_interface_closure(bases);
                for base in bases {
                    let base_name = base.name.as_str();
                    let Some(interface_info) = self.resolve_type_info(base_name).cloned() else {
                        self.diagnostics.push(diagnostics::error(
                            codes::UNKNOWN_INTERFACE,
                            format!("type `{name}` implements unknown interface `{base_name}`"),
                            None,
                        ));
                        continue;
                    };
                    let Some((iface_methods, iface_properties)) = (match &interface_info.kind {
                        TypeKind::Interface {
                            methods,
                            properties,
                            bases: _,
                        } => Some((methods.clone(), properties.clone())),
                        other => {
                            if is_error_type && matches!(other, TypeKind::Class { .. }) {
                                self.diagnostics.push(diagnostics::error(
                                    codes::ERROR_INHERITANCE,
                                    format!(
                                        "error type `{name}` cannot inherit from non-error type `{base_name}`"
                                    ),
                                    None,
                                ));
                            }
                            None
                        }
                    }) else {
                        continue;
                    };
                    let substitution = self.build_type_substitution(&interface_info, &base.expr);
                    for iface_method_id in &iface_methods {
                        let iface_method = self.signatures.get(*iface_method_id);
                        let method_name = strip_receiver(&iface_method.name).to_string();
                        let expected_params: Vec<String> = iface_method
                            .param_types
                            .iter()
                            .map(|ty| instantiate_type_name(ty, &substitution))
                            .collect();
                        let expected_return =
                            instantiate_type_name(&iface_method.return_type, &substitution);

                        let mut name_match: Option<&FunctionSignature> = None;
                        let mut implemented = false;
                        for id in methods {
                            let method = self.signatures.get(*id);
                            if strip_receiver(&method.name) != method_name {
                                continue;
                            }
                            if method.param_types == expected_params
                                && method.return_type == expected_return
                            {
                                implemented = true;
                                break;
                            }
                            if name_match.is_none() {
                                name_match = Some(method);
                            }
                        }
                        if implemented {
                            continue;
                        }

                        if let Some(candidate) = name_match {
                            self.diagnostics.push(diagnostics::error(
                                codes::INTERFACE_METHOD_SIGNATURE_MISMATCH,
                                format!(
                                    "type `{name}` implements `{base_name}` but method `{method_name}` has signature `{}`; expected `{}`",
                                    format_signature(&candidate.param_types, &candidate.return_type),
                                    format_signature(&expected_params, &expected_return),
                                ),
                                candidate.span.or(iface_method.span),
                            ));
                            continue;
                        }

                        if self.try_apply_interface_default(
                            name.as_str(),
                            base_name,
                            &method_name,
                            &interface_closure,
                            None,
                        ) {
                            continue;
                        }
                        self.diagnostics.push(diagnostics::error(
                            codes::MISSING_INTERFACE_METHOD,
                            format!(
                                "type `{name}` is missing implementation for `{}` required by `{base_name}`",
                                method_name
                            ),
                            None,
                        ));
                    }
                    for iface_property in iface_properties {
                        let Some(class_prop) = properties
                            .iter()
                            .find(|prop| prop.name == iface_property.name)
                        else {
                            self.diagnostics.push(diagnostics::error(
                                codes::MISSING_INTERFACE_PROPERTY,
                                format!(
                                    "type `{name}` is missing property `{}` required by `{base_name}`",
                                    iface_property.name
                                ),
                                iface_property.span,
                            ));
                            continue;
                        };

                        if iface_property.is_static != class_prop.is_static {
                            let descriptor = if iface_property.is_static {
                                "static"
                            } else {
                                "instance"
                            };
                            self.diagnostics.push(diagnostics::error(
                                codes::PROPERTY_STATIC_MISMATCH,
                                format!(
                                    "property `{}` in type `{name}` must be declared {descriptor} to match `{base_name}`",
                                    iface_property.name
                                ),
                                class_prop.span.or(iface_property.span),
                            ));
                        }

                        let expected_type =
                            instantiate_type_name(&iface_property.ty, &substitution);

                        if expected_type != class_prop.ty {
                            self.diagnostics.push(diagnostics::error(
                                codes::PROPERTY_TYPE_MISMATCH,
                                format!(
                                    "property `{}` in type `{name}` has type `{}` but `{base_name}` requires `{}`",
                                    iface_property.name,
                                    class_prop.ty,
                                    expected_type
                                ),
                                class_prop.span.or(iface_property.span),
                            ));
                        }

                        if iface_property.accessors.has_get() && !class_prop.accessors.has_get() {
                            self.diagnostics.push(diagnostics::error(
                                codes::PROPERTY_ACCESSOR_CONFLICT,
                                format!(
                                    "property `{}` in type `{name}` is missing a `get` accessor required by `{base_name}`",
                                    iface_property.name
                                ),
                                class_prop.span.or(iface_property.span),
                            ));
                        }

                        if iface_property.accessors.has_set() && !class_prop.accessors.has_set() {
                            self.diagnostics.push(diagnostics::error(
                                codes::PROPERTY_ACCESSOR_CONFLICT,
                                format!(
                                    "property `{}` in type `{name}` is missing a `set` accessor required by `{base_name}`",
                                    iface_property.name
                                ),
                                class_prop.span.or(iface_property.span),
                            ));
                        }

                        if iface_property.accessors.has_init() && !class_prop.accessors.has_init() {
                            self.diagnostics.push(diagnostics::error(
                                codes::PROPERTY_ACCESSOR_CONFLICT,
                                format!(
                                    "property `{}` in type `{name}` is missing an `init` accessor required by `{base_name}`",
                                    iface_property.name
                                ),
                                class_prop.span.or(iface_property.span),
                            ));
                        }
                    }
                }
            }
        }
    }

    pub(super) fn validate_dependency_injection(&mut self) {
        if self.di_manifest.services.is_empty() {
            return;
        }

        let services_snapshot = self.di_manifest.services.clone();
        let mut services_by_name: HashMap<String, DiService> = HashMap::new();
        for service in &services_snapshot {
            services_by_name.insert(service.name.clone(), service.clone());
            if matches!(service.lifetime, DiLifetime::ThreadLocal) {
                self.emit_error(
                    codes::DI_THREADLOCAL_UNSUPPORTED,
                    service.span,
                    format!(
                        "DI0003: `ThreadLocal` lifetime is not yet supported for service `{}`",
                        service.name
                    ),
                );
            }
        }

        for service in &services_snapshot {
            for dependency in &service.dependencies {
                self.validate_di_dependency(service, dependency, &services_by_name);
            }
        }
    }

    fn validate_di_dependency(
        &mut self,
        service: &DiService,
        dependency: &DiDependency,
        services: &HashMap<String, DiService>,
    ) {
        if dependency
            .requested_lifetime
            .is_some_and(|lifetime| lifetime == DiLifetime::ThreadLocal)
        {
            self.emit_error(
                codes::DI_THREADLOCAL_UNSUPPORTED,
                dependency.span,
                format!(
                    "DI0003: `ThreadLocal` lifetime is not yet supported for dependency `{}`",
                    dependency.target
                ),
            );
        }

        let dependency_service = services.get(&dependency.target);
        if dependency_service.is_none() && !dependency.optional {
            self.emit_error(
                codes::DI_MISSING_REGISTRATION,
                dependency.span,
                format!(
                    "DI0001: no service registration found for `{}` required by {}",
                    dependency.target,
                    describe_injection_site(&dependency.site)
                ),
            );
            return;
        }

        if matches!(service.lifetime, DiLifetime::ThreadLocal) {
            return;
        }

        if matches!(service.lifetime, DiLifetime::Singleton) {
            let effective_lifetime = dependency
                .requested_lifetime
                .or_else(|| dependency_service.map(|svc| svc.lifetime));
            if let Some(lifetime) = effective_lifetime {
                if lifetime != DiLifetime::Singleton {
                    self.emit_error(
                        codes::DI_SINGLETON_LIFETIME,
                        dependency.span,
                        format!(
                            "DI0002: singleton service `{}` cannot depend on `{}` with `{}` lifetime",
                            service.name,
                            dependency.target,
                            lifetime_label(lifetime)
                        ),
                    );
                }
            }
        }
    }

    pub(super) fn ensure_auto_trait(&mut self, check: AutoTraitCheck<'_>) {
        if self.context_provides_auto_trait(check.function, check.ty, check.kind) {
            return;
        }
        let traits = self.type_layouts.resolve_auto_traits(check.ty);
        let status = match check.kind {
            AutoTraitKind::ThreadSafe => traits.thread_safe,
            AutoTraitKind::Shareable => traits.shareable,
            AutoTraitKind::Copy => traits.copy,
        };

        if matches!(status, AutoTraitStatus::Yes) {
            return;
        }

        let target_desc = if check.target.is_empty() {
            "this usage".to_string()
        } else {
            format!("`{}`", check.target)
        };

        let (code, mut message) = match status {
            AutoTraitStatus::No => match check.origin {
                AutoTraitConstraintOrigin::ThreadSpawn => (
                    codes::THREADSAFE_REQUIRED,
                    format!(
                        "`Thread::Spawn` in `{}` requires `{}` to implement ThreadSafe",
                        check.function, check.ty
                    ),
                ),
                _ => (
                    codes::AUTO_TRAIT_REQUIRED,
                    format!(
                        "type `{}` captured by {target_desc} in `{}` does not implement {}",
                        check.ty,
                        check.function,
                        check.kind.display_name()
                    ),
                ),
            },
            AutoTraitStatus::Unknown => match check.origin {
                AutoTraitConstraintOrigin::ThreadSpawn => (
                    codes::THREADSAFE_REQUIRED,
                    format!(
                        "`Thread::Spawn` in `{}` cannot prove `{}` is ThreadSafe",
                        check.function, check.ty
                    ),
                ),
                _ => (
                    codes::AUTO_TRAIT_UNPROVEN,
                    format!(
                        "cannot prove type `{}` implements {} required by {target_desc} in `{}`",
                        check.ty,
                        check.kind.display_name(),
                        check.function
                    ),
                ),
            },
            AutoTraitStatus::Yes => return,
        };

        if matches!(
            check.kind,
            AutoTraitKind::ThreadSafe | AutoTraitKind::Shareable
        ) {
            message.push_str(
                " Consider guarding the value with `std.sync::Mutex`, `std.sync::RwLock`, or an atomic primitive.",
            );
        }

        self.emit_error(code, check.span, message);
    }

    fn context_provides_auto_trait(&self, context: &str, ty: &str, kind: AutoTraitKind) -> bool {
        if context.is_empty() || ty.is_empty() {
            return false;
        }
        let Some(param) = self.lookup_generic_param_for_auto_trait(context, ty) else {
            return false;
        };
        Self::generic_param_supports_auto_trait(param, kind)
    }

    fn lookup_generic_param_for_auto_trait<'b>(
        &'b self,
        context: &str,
        ty: &str,
    ) -> Option<&'b GenericParam> {
        let mut candidates = vec![ty];
        if let Some(stripped) = ty.strip_suffix('?') {
            candidates.push(stripped);
        }
        for candidate in candidates {
            if let Some(param) = self.lookup_context_generic_param(Some(context), candidate) {
                return Some(param);
            }
        }
        None
    }

    fn generic_param_supports_auto_trait(param: &GenericParam, kind: AutoTraitKind) -> bool {
        let Some(data) = param.as_type() else {
            return false;
        };
        data.constraints
            .iter()
            .any(|constraint| match &constraint.kind {
                GenericConstraintKind::AutoTrait(required) => {
                    Self::auto_trait_requirement_matches(*required, kind)
                }
                _ => false,
            })
    }

    const fn auto_trait_requirement_matches(
        required: AutoTraitConstraint,
        kind: AutoTraitKind,
    ) -> bool {
        matches!(
            (required, kind),
            (AutoTraitConstraint::ThreadSafe, AutoTraitKind::ThreadSafe)
                | (AutoTraitConstraint::Shareable, AutoTraitKind::Shareable)
        )
    }

    pub(super) fn type_implements_interface_by_name(&self, ty: &str, interface: &str) -> bool {
        if let (Some(ty_expr), Some(interface_expr)) =
            (parse_type_text(ty), parse_type_text(interface))
        {
            self.type_implements_interface_expr(ty_expr, interface_expr)
        } else {
            self.type_implements_interface_by_name_legacy(ty, interface)
        }
    }

    fn type_implements_interface_by_name_legacy(&self, ty: &str, interface: &str) -> bool {
        if type_names_equivalent(ty, interface) {
            return true;
        }
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(ty.to_string());

        while let Some(current) = queue.pop_front() {
            if !visited.insert(current.clone()) {
                continue;
            }
            if type_names_equivalent(&current, interface) {
                return true;
            }
            let Some(info) = self.resolve_type_info(&current) else {
                continue;
            };
            match &info.kind {
                TypeKind::Class { bases, .. } => {
                    for base in bases {
                        if type_names_equivalent(&base.name, interface) {
                            return true;
                        }
                        queue.push_back(base.name.clone());
                    }
                }
                TypeKind::Interface { bases, .. } => {
                    for base in bases {
                        if type_names_equivalent(&base.name, interface) {
                            return true;
                        }
                        queue.push_back(base.name.clone());
                    }
                }
                TypeKind::Struct { bases, .. } => {
                    for base in bases {
                        if type_names_equivalent(&base.name, interface) {
                            return true;
                        }
                        queue.push_back(base.name.clone());
                    }
                }
                _ => {}
            }
        }

        false
    }

    fn type_implements_interface_expr(&self, ty_expr: TypeExpr, interface_expr: TypeExpr) -> bool {
        if self
            .interface_variance_compatible(&ty_expr, &interface_expr)
            .unwrap_or(false)
        {
            return true;
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(ty_expr);

        while let Some(current) = queue.pop_front() {
            let key = format_type_expr(&current);
            if !visited.insert(key.clone()) {
                continue;
            }
            if self
                .interface_variance_compatible(&current, &interface_expr)
                .unwrap_or(false)
            {
                return true;
            }
            let base_name = base_type_name(&current.name).to_string();
            let Some(info) = self.resolve_type_info(&base_name) else {
                continue;
            };
            let subst = self.build_type_substitution(info, &current);
            match &info.kind {
                TypeKind::Class { bases, .. }
                | TypeKind::Interface { bases, .. }
                | TypeKind::Struct { bases, .. } => {
                    for binding in bases {
                        let instantiated = self.instantiate_base_expr(binding, &subst);
                        queue.push_back(instantiated);
                    }
                }
                _ => {}
            }
        }

        false
    }

    pub(super) fn type_is_subclass_of_name(&self, ty: &str, base: &str) -> bool {
        if type_names_equivalent(ty, base) {
            return true;
        }
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(ty.to_string());

        while let Some(current) = queue.pop_front() {
            if !visited.insert(current.clone()) {
                continue;
            }
            let Some(info) = self.resolve_type_info(&current) else {
                continue;
            };
            if let TypeKind::Class { bases, .. } = &info.kind {
                for candidate in bases {
                    if type_names_equivalent(&candidate.name, base) {
                        return true;
                    }
                    if let Some(base_info) = self.resolve_type_info(&candidate.name) {
                        if matches!(base_info.kind, TypeKind::Class { .. }) {
                            queue.push_back(candidate.name.clone());
                        }
                    }
                }
            }
        }

        false
    }

    fn build_type_substitution(&self, info: &TypeInfo, inst: &TypeExpr) -> HashMap<String, String> {
        let mut map = HashMap::new();
        let Some(generics) = info.generics.as_ref() else {
            return map;
        };
        let Some(args) = inst.generic_arguments() else {
            return map;
        };

        for (param, arg) in generics.params.iter().zip(args.iter()) {
            if let GenericParamKind::Type(_) = &param.kind {
                if let Some(arg_ty) = arg.ty() {
                    map.insert(param.name.clone(), format_type_expr(arg_ty));
                }
            }
        }

        map
    }

    fn instantiate_base_expr(
        &self,
        binding: &BaseTypeBinding,
        subst: &HashMap<String, String>,
    ) -> TypeExpr {
        if subst.is_empty() {
            return binding.expr.clone();
        }
        let rendered = render_type_expr(&binding.expr, subst);
        parse_type_text(&rendered).unwrap_or_else(|| binding.expr.clone())
    }

    fn interface_variance_compatible(&self, source: &TypeExpr, target: &TypeExpr) -> Option<bool> {
        let source_base = base_type_name(&source.name).to_string();
        let target_base = base_type_name(&target.name).to_string();
        if !type_names_equivalent(&source_base, &target_base) {
            return Some(false);
        }
        let info = self.resolve_type_info(&target_base)?;
        let Some(generics) = info.generics.as_ref() else {
            return Some(true);
        };
        if generics.params.is_empty() {
            return Some(true);
        }

        let source_args = collect_generic_arg_values(source);
        let target_args = collect_generic_arg_values(target);
        if source_args.len() != generics.params.len() || target_args.len() != generics.params.len()
        {
            return Some(false);
        }

        for (param, (src_arg, tgt_arg)) in generics
            .params
            .iter()
            .zip(source_args.iter().zip(target_args.iter()))
        {
            match (&param.kind, src_arg, tgt_arg) {
                (
                    GenericParamKind::Type(data),
                    GenericArgValue::Type(src_ty),
                    GenericArgValue::Type(tgt_ty),
                ) => {
                    let permitted = match data.variance {
                        Variance::Invariant => type_names_equivalent(&src_ty.name, &tgt_ty.name),
                        Variance::Covariant => self.type_argument_is_subtype(src_ty, tgt_ty),
                        Variance::Contravariant => self.type_argument_is_subtype(tgt_ty, src_ty),
                    };
                    if !permitted {
                        return Some(false);
                    }
                }
                (
                    GenericParamKind::Const(_),
                    GenericArgValue::Const(src_val),
                    GenericArgValue::Const(tgt_val),
                ) => {
                    if src_val != tgt_val {
                        return Some(false);
                    }
                }
                _ => return Some(false),
            }
        }

        Some(true)
    }

    fn type_argument_is_subtype(&self, source: &TypeExpr, target: &TypeExpr) -> bool {
        if type_names_equivalent(&source.name, &target.name) {
            return true;
        }
        let source_rendered = format_type_expr(source);
        let target_rendered = format_type_expr(target);
        self.type_is_subclass_of_name(source_rendered.as_str(), target_rendered.as_str())
            || self.type_implements_interface_by_name(
                source_rendered.as_str(),
                target_rendered.as_str(),
            )
    }
}

fn describe_injection_site(site: &DiInjectionSite) -> String {
    match site {
        DiInjectionSite::ConstructorParameter {
            constructor,
            parameter,
        } => {
            format!("parameter `{parameter}` of constructor `{constructor}`")
        }
        DiInjectionSite::Property { property } => format!("property `{property}`"),
    }
}

fn lifetime_label(lifetime: DiLifetime) -> &'static str {
    match lifetime {
        DiLifetime::Singleton => "Singleton",
        DiLifetime::Scoped => "Scoped",
        DiLifetime::Transient => "Transient",
        DiLifetime::ThreadLocal => "ThreadLocal",
    }
}

#[derive(Clone)]
enum GenericArgValue {
    Type(TypeExpr),
    Const(String),
}

fn collect_generic_arg_values(expr: &TypeExpr) -> Vec<GenericArgValue> {
    expr.generic_arguments()
        .map(|args| {
            args.iter()
                .map(|arg| {
                    if let Some(arg_ty) = arg.ty() {
                        GenericArgValue::Type(arg_ty.clone())
                    } else {
                        GenericArgValue::Const(arg.expression().text.clone())
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn render_type_expr(expr: &TypeExpr, subst: &HashMap<String, String>) -> String {
    if expr.generic_arguments().is_none()
        && expr.tuple_elements().is_none()
        && expr.fn_signature().is_none()
        && expr.trait_object().is_none()
    {
        if let Some(replacement) = subst.get(expr.name.as_str()) {
            return replacement.clone();
        }
        return expr.name.clone();
    }

    if let Some(args) = expr.generic_arguments() {
        let mut text = base_type_name(&expr.name).to_string();
        text.push('<');
        for (index, arg) in args.iter().enumerate() {
            if index > 0 {
                text.push(',');
            }
            if let Some(arg_ty) = arg.ty() {
                text.push_str(&render_type_expr(arg_ty, subst));
            } else {
                text.push_str(&arg.expression().text);
            }
        }
        text.push('>');
        return text;
    }

    expr.name.clone()
}

fn format_type_expr(expr: &TypeExpr) -> String {
    render_type_expr(expr, &HashMap::new())
}

fn instantiate_type_name(ty: &str, subst: &HashMap<String, String>) -> String {
    if subst.is_empty() {
        return ty.to_string();
    }
    if let Some(expr) = parse_type_text(ty) {
        return render_type_expr(&expr, subst);
    }
    subst.get(ty).cloned().unwrap_or_else(|| ty.to_string())
}

fn format_signature(params: &[String], return_type: &str) -> String {
    let param_text = if params.is_empty() {
        String::from("()")
    } else {
        format!("({})", params.join(", "))
    };
    format!("{param_text} -> {return_type}")
}
