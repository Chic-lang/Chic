use super::arena::{EffectConstraintRecord, TypeChecker, TypeKind};
use super::diagnostics::{self, codes};
use super::helpers::{base_type_name, canonical_type_name, type_expr_path};
use super::traits::AutoTraitCheck;
use super::{ConstraintKind, TypeConstraint};
use crate::frontend::ast::{GenericConstraintKind, GenericParam, Parameter, TypeExpr};
use crate::frontend::diagnostics::Span;
use crate::frontend::import_resolver::Resolution as ImportResolution;
use crate::frontend::parser::parse_type_expression_text;
use crate::threading;
use std::collections::HashSet;

impl<'a> TypeChecker<'a> {
    pub(super) fn check_constraints(&mut self, constraints: &[TypeConstraint]) {
        for constraint in constraints {
            match &constraint.kind {
                ConstraintKind::ParameterType { function, ty, .. } => {
                    self.ensure_type_exists(ty, None, Some(function.as_str()), constraint.span);
                }
                ConstraintKind::ReturnType { function, ty, .. } => {
                    self.ensure_type_exists(ty, None, Some(function.as_str()), constraint.span);
                }
                ConstraintKind::ImplTraitBound {
                    function,
                    opaque_ty,
                    bound,
                } => self.ensure_impl_trait_bound(function, opaque_ty, bound, constraint.span),
                ConstraintKind::VariableInit {
                    function,
                    declared,
                    expr,
                    ..
                } => {
                    if let Some(ty) = declared {
                        self.ensure_type_exists(ty, None, Some(function.as_str()), constraint.span);
                    } else {
                        self.ensure_type_exists(
                            expr,
                            None,
                            Some(function.as_str()),
                            constraint.span,
                        );
                    }
                }
                ConstraintKind::ImplementsInterface {
                    type_name,
                    interface,
                } => {
                    if !self.has_type(type_name) {
                        self.emit_error(
                            codes::UNKNOWN_TYPE,
                            constraint.span,
                            format!(
                                "type `{type_name}` not found while processing interface constraints"
                            ),
                        );
                    }
                    if !self.has_type(interface) {
                        self.emit_error(
                            codes::UNKNOWN_INTERFACE,
                            constraint.span,
                            format!("interface `{interface}` not defined"),
                        );
                    }
                }
                ConstraintKind::ExtensionTarget { target, .. } => {
                    self.ensure_type_exists(target, None, None, constraint.span);
                }
                ConstraintKind::RequiresAutoTrait {
                    function,
                    target,
                    ty,
                    trait_kind,
                    origin,
                } => {
                    self.ensure_type_exists(ty, None, Some(function.as_str()), constraint.span);
                    self.ensure_auto_trait(AutoTraitCheck {
                        function,
                        target,
                        ty,
                        kind: *trait_kind,
                        origin: *origin,
                        span: constraint.span,
                    });
                }
                ConstraintKind::EffectEscape { function, effect } => {
                    if !(effect.eq_ignore_ascii_case("random")
                        || effect.eq_ignore_ascii_case("network"))
                    {
                        self.ensure_type_exists(
                            effect,
                            None,
                            Some(function.as_str()),
                            constraint.span,
                        );
                    }
                    let entry = self.inferred_effects.entry(function.clone()).or_default();
                    if entry.iter().any(|existing| existing.effect == *effect) {
                        continue;
                    }
                    entry.push(EffectConstraintRecord {
                        effect: effect.clone(),
                        span: constraint.span,
                    });
                }
                ConstraintKind::ThreadingBackendAvailable {
                    function,
                    backend,
                    call,
                } => self.ensure_thread_backend_available(function, backend, call, constraint.span),
                ConstraintKind::RandomDuplication { function } => {
                    self.emit_error(
                        codes::RANDOM_DUPLICATED,
                        constraint.span,
                        format!(
                            "RNG handle is duplicated in `{}`; use split to create independent streams",
                            function
                        ),
                    );
                }
                ConstraintKind::BorrowEscape {
                    function,
                    parameter,
                    parameter_mode,
                    escape,
                } => {
                    self.report_borrow_escape(
                        function,
                        parameter,
                        *parameter_mode,
                        escape,
                        constraint.span,
                    );
                }
                ConstraintKind::RequiresTrait {
                    function,
                    ty,
                    trait_name,
                } => {
                    if !self.type_satisfies_trait(function, ty, trait_name) {
                        self.emit_error(
                            codes::UNKNOWN_TYPE,
                            constraint.span,
                            format!(
                                "type `{ty}` must implement trait `{trait_name}` in `{function}`"
                            ),
                        );
                    }
                }
            }
        }
    }

    pub(super) fn ensure_type_exists(
        &mut self,
        ty: &str,
        namespace: Option<&str>,
        context_type: Option<&str>,
        span: Option<Span>,
    ) {
        if let Some(expr) = parse_type_text(ty) {
            self.ensure_type_expr(&expr, namespace, context_type, span);
            return;
        }

        let base = base_type_name(ty);
        if base == "Self"
            || self.builtin_types.contains(base)
            || self
                .type_layouts
                .primitive_registry
                .lookup_by_name(base)
                .is_some()
        {
            return;
        }
        if self.resolve_type_info(base).is_some() {
            return;
        }
        self.emit_error(codes::UNKNOWN_TYPE, span, format!("unknown type `{ty}`"));
    }

    fn type_satisfies_trait(&self, owner: &str, ty: &str, trait_name: &str) -> bool {
        let base_ty = base_type_name(ty);
        if self.numeric_builtin_supports_trait(base_ty, trait_name) {
            return true;
        }

        if let Some(param) = self.generic_param_in_owner(owner, base_ty) {
            if let Some(type_param) = param.as_type() {
                for constraint in &type_param.constraints {
                    if let GenericConstraintKind::Type(expr) = &constraint.kind {
                        if self.trait_constraint_matches(trait_name, expr) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    fn trait_constraint_matches(&self, expected: &str, constraint: &TypeExpr) -> bool {
        if self.trait_names_match(expected, &constraint.name) {
            return true;
        }
        self.interface_inherits_trait(&constraint.name, expected, &mut HashSet::new())
    }

    fn interface_inherits_trait(
        &self,
        candidate: &str,
        expected: &str,
        visited: &mut HashSet<String>,
    ) -> bool {
        if !visited.insert(candidate.to_string()) {
            return false;
        }
        let Some(info) = self.resolve_type_info(candidate) else {
            return false;
        };
        let TypeKind::Interface { bases, .. } = &info.kind else {
            return false;
        };
        for base in bases {
            if self.trait_names_match(expected, &base.name)
                || self.interface_inherits_trait(&base.name, expected, visited)
            {
                return true;
            }
        }
        false
    }

    fn trait_names_match(&self, expected: &str, candidate: &str) -> bool {
        let expected_short = diagnostics::simple_name(expected).to_ascii_lowercase();
        let candidate_short = diagnostics::simple_name(candidate).to_ascii_lowercase();
        expected_short == candidate_short
    }

    fn numeric_builtin_supports_trait(&self, ty: &str, trait_name: &str) -> bool {
        let numeric = matches!(
            ty.to_ascii_lowercase().as_str(),
            "sbyte"
                | "byte"
                | "short"
                | "ushort"
                | "int"
                | "uint"
                | "long"
                | "ulong"
                | "nint"
                | "nuint"
                | "float"
                | "double"
                | "decimal"
        );
        if !numeric {
            return false;
        }

        let accepted = [
            "icomparable",
            "iequatable",
            "iequalityoperators",
            "icomparisonoperators",
            "iadditionoperators",
            "isubtractionoperators",
            "imultiplyoperators",
            "idivisionoperators",
            "imodulusoperators",
            "iunarynegationoperators",
            "iunaryplusoperators",
            "iincrementoperators",
            "idecrementoperators",
            "iadditiveidentity",
            "imultiplicativeidentity",
            "iminmaxvalue",
            "inumberbase",
            "inumber",
            "isignednumber",
            "ibinarynumber",
            "ibinaryinteger",
            "ibitwiseoperators",
            "ishiftoperators",
            "iformattable",
            "ispanformattable",
            "iutf8spanformattable",
            "iparsable",
            "ispanparsable",
            "iutf8spanparsable",
            "iconvertible",
        ];
        let trait_short = diagnostics::simple_name(trait_name).to_ascii_lowercase();
        accepted.contains(&trait_short.as_str())
    }

    pub(super) fn ensure_type_expr(
        &mut self,
        expr: &TypeExpr,
        namespace: Option<&str>,
        context_type: Option<&str>,
        span: Option<Span>,
    ) {
        if expr.is_trait_object() {
            self.ensure_trait_object(expr, namespace, context_type, span);
            return;
        }

        if let Some(signature) = expr.fn_signature() {
            for param in &signature.params {
                self.ensure_type_expr(param, namespace, context_type, span);
            }
            self.ensure_type_expr(
                signature.return_type.as_ref(),
                namespace,
                context_type,
                span,
            );
            return;
        }

        let resolution = self.resolve_type_for_expr(expr, namespace, context_type);
        let resolved_name = match resolution {
            ImportResolution::Found(name) => name,
            ImportResolution::Ambiguous(candidates) => {
                self.emit_error(
                    codes::AMBIGUOUS_TYPE,
                    span,
                    format!(
                        "type `{}` resolves to multiple candidates: {}",
                        expr.name,
                        candidates.join(", ")
                    ),
                );
                expr.name.replace('.', "::")
            }
            ImportResolution::NotFound => {
                if expr.pointer_depth() > 0 || expr.array_ranks().next().is_some() {
                    type_expr_path(expr).unwrap_or_else(|| expr.name.replace('.', "::"))
                } else {
                    expr.name.replace('.', "::")
                }
            }
        };

        let base = base_type_name(&resolved_name);
        if base == "Self"
            || base.eq_ignore_ascii_case("var")
            || self.builtin_types.contains(base)
            || self
                .type_layouts
                .primitive_registry
                .lookup_by_name(base)
                .is_some()
        {
            return;
        }

        let expected_arity = expr.generic_arguments().map(|args| args.len());
        let Some(info) = self
            .resolve_type_info_with_arity(&resolved_name, expected_arity)
            .cloned()
        else {
            if self.symbol_index.contains_type(&resolved_name) {
                return;
            }
            if self.context_declares_generic(context_type, &resolved_name) {
                return;
            }
            self.emit_error(
                codes::UNKNOWN_TYPE,
                span,
                format!("unknown type `{}`", resolved_name),
            );
            return;
        };

        if let Some(args) = expr.generic_arguments() {
            self.validate_generic_arguments(&resolved_name, &info, args, context_type, span);
        } else if info
            .generics
            .as_ref()
            .is_some_and(|params| !params.params.is_empty())
        {
            let expected = info
                .generics
                .as_ref()
                .map_or(0, |params| params.params.len());
            self.emit_error(
                codes::GENERIC_ARGUMENT_MISMATCH,
                span,
                format!(
                    "type `{}` requires {} type argument{}",
                    resolved_name,
                    expected,
                    if expected == 1 { "" } else { "s" }
                ),
            );
        }
    }

    pub(super) fn ensure_thread_backend_available(
        &mut self,
        function: &str,
        backend: &str,
        call: &str,
        span: Option<Span>,
    ) {
        if threading::threads_supported() {
            return;
        }
        self.emit_error(
            codes::THREADS_UNAVAILABLE_ON_TARGET,
            span,
            format!(
                "`{call}` in `{function}` is unavailable for backend `{backend}`; \
                 gate the call or target a backend with native threading support"
            ),
        );
    }

    fn ensure_trait_object(
        &mut self,
        expr: &TypeExpr,
        namespace: Option<&str>,
        context_type: Option<&str>,
        span: Option<Span>,
    ) {
        let Some(trait_object) = expr.trait_object() else {
            return;
        };
        let is_impl = expr.is_impl_trait();
        let context_label = if is_impl { "impl" } else { "dyn" };
        let enforce_object_safety = !is_impl;
        for bound in &trait_object.bounds {
            match self.resolve_type_for_expr(bound, namespace, context_type) {
                ImportResolution::Found(resolved) => {
                    let Some(info) = self.traits.get(&resolved) else {
                        self.emit_error(
                            codes::TRAIT_FEATURE_UNAVAILABLE,
                            span,
                            format!(
                                "type `{resolved}` is not a trait and cannot appear after `{context_label}`"
                            ),
                        );
                        continue;
                    };
                    if !enforce_object_safety || info.object_safety.is_object_safe() {
                        continue;
                    }
                    if let Some(reason) = info.object_safety.describe() {
                        let diag_span = info.object_safety.violation_span().or(span);
                        self.emit_error(
                            codes::TRAIT_OBJECT_UNSAFE,
                            diag_span,
                            format!(
                                "trait `{resolved}` cannot be used as `{context_label}` because {reason}"
                            ),
                        );
                    }
                }
                ImportResolution::Ambiguous(candidates) => {
                    self.emit_error(
                        codes::AMBIGUOUS_TYPE,
                        span,
                        format!(
                            "trait `{}` resolves to multiple candidates: {}",
                            bound.name,
                            candidates.join(", ")
                        ),
                    );
                }
                ImportResolution::NotFound => {
                    self.emit_error(
                        codes::UNKNOWN_TYPE,
                        span,
                        format!(
                            "trait `{}` referenced by `{context_label}` is not defined",
                            bound.name
                        ),
                    );
                }
            }
        }
    }

    pub(super) fn ensure_unique_parameter_names(
        &mut self,
        params: &[Parameter],
        function_name: &str,
        span: Option<Span>,
    ) {
        let mut seen = HashSet::new();
        for param in params {
            if !seen.insert(param.name.clone()) {
                self.emit_error(
                    codes::PARAMETER_NAME_DUPLICATE,
                    span,
                    format!(
                        "parameter `{}` appears multiple times in `{}`",
                        param.name, function_name
                    ),
                );
            }
        }
    }

    pub(super) fn context_declares_generic(
        &self,
        context_type: Option<&str>,
        candidate: &str,
    ) -> bool {
        let Some(mut ctx) = context_type else {
            return false;
        };
        loop {
            let owner = base_type_name(ctx);
            if self.pending_generics_contain(owner, candidate) {
                return true;
            }
            if self.function_generics_contain(owner, candidate) {
                return true;
            }
            if let Some(entries) = self.types.get(owner) {
                if entries.iter().any(|info| {
                    info.generics.as_ref().is_some_and(|params| {
                        params.params.iter().any(|param| param.name == candidate)
                    })
                }) {
                    return true;
                }
            }
            if let Some(pos) = owner.rfind("::") {
                ctx = &owner[..pos];
            } else {
                break;
            }
        }
        false
    }

    fn ensure_impl_trait_bound(
        &mut self,
        function: &str,
        opaque_ty: &str,
        bound: &str,
        span: Option<Span>,
    ) {
        let trait_name = bound.replace('.', "::");
        let trait_base = trait_name
            .rsplit("::")
            .next()
            .unwrap_or(trait_name.as_str());
        let resolved_trait = if self.traits.contains_key(&trait_name) {
            Some(trait_name.clone())
        } else {
            self.traits
                .keys()
                .find(|candidate| candidate.rsplit("::").next() == Some(trait_base))
                .cloned()
        };
        let Some(resolved_trait) = resolved_trait else {
            self.emit_error(
                codes::UNKNOWN_TYPE,
                span,
                format!(
                    "opaque type `{opaque_ty}` in `{function}` requires trait `{trait_name}`, which is not defined"
                ),
            );
            return;
        };
        if self.trait_impl_for(opaque_ty, &resolved_trait) {
            return;
        }
        self.emit_error(
            codes::IMPL_TRAIT_BOUND_UNSATISFIED,
            span,
            format!(
                "opaque return type `{opaque_ty}` in `{function}` does not implement required trait `{resolved_trait}`"
            ),
        );
    }

    fn trait_impl_for(&self, ty: &str, trait_name: &str) -> bool {
        let target = ty.replace('.', "::");
        let target_base = target.rsplit("::").next().unwrap_or(target.as_str());
        let trait_key = trait_name.replace('.', "::");
        let trait_base = trait_key.rsplit("::").next().unwrap_or(trait_key.as_str());
        self.impls.iter().any(|info| {
            let impl_trait = info.trait_name.as_ref().map(|name| name.replace('.', "::"));
            let impl_trait_base = impl_trait
                .as_deref()
                .and_then(|name| name.rsplit("::").next());
            let impl_target = canonical_type_name(&info.target);
            let impl_target_base = impl_target
                .rsplit("::")
                .next()
                .unwrap_or(impl_target.as_str());
            (impl_trait.as_deref() == Some(trait_key.as_str())
                || impl_trait_base == Some(trait_base))
                && (impl_target == target || impl_target_base == target_base)
        })
    }

    pub(super) fn lookup_context_generic_param(
        &self,
        context_type: Option<&str>,
        candidate: &str,
    ) -> Option<&GenericParam> {
        let mut ctx = context_type?;
        loop {
            let owner = base_type_name(ctx);
            if let Some(param) = self.generic_param_in_owner(owner, candidate) {
                return Some(param);
            }
            if let Some(pos) = owner.rfind("::") {
                ctx = &owner[..pos];
            } else {
                break;
            }
        }
        None
    }
}

pub(super) fn parse_type_text(text: &str) -> Option<TypeExpr> {
    let trimmed = text.trim();
    if let Some(expr) = parse_type_expression_text(trimmed) {
        return Some(expr);
    }
    if trimmed.starts_with('(') && trimmed.ends_with(')') && trimmed.len() > 2 {
        if let Some(expr) = parse_type_text(&trimmed[1..trimmed.len() - 1]) {
            return Some(expr);
        }
    }
    if trimmed.contains("::") {
        let substituted = trimmed.replace("::", ".");
        if let Some(mut expr) = parse_type_expression_text(&substituted) {
            expr.name = expr.name.replace('.', "::");
            return Some(expr);
        }
    }
    None
}
