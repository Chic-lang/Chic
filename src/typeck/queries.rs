use super::arena::{TraitFulfillmentReport, TypeChecker};
use super::{
    AutoTraitConstraintOrigin, AutoTraitKind, BorrowEscapeCategory, ConstraintKind,
    TypeCheckResult, TypeConstraint,
};
use crate::frontend::ast::{Module, TypeExpr};
use crate::frontend::diagnostics::{Diagnostic, Span};
use crate::frontend::import_resolver::Resolution as ImportResolution;
use crate::mir::{ParamMode, TypeLayoutTable};
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct TypeckQueries<'a> {
    module: &'a Module,
    type_layouts: &'a TypeLayoutTable,
    full_cache: RefCell<Option<TypeCheckResult>>,
    constraints_cache: RefCell<HashMap<ConstraintCacheKey, Vec<Diagnostic>>>,
    trait_cache: RefCell<Option<TraitFulfillmentReport>>,
    resolve_cache: RefCell<HashMap<ResolveKey, ImportResolution>>,
    package_context: crate::typeck::PackageContext,
}

impl<'a> TypeckQueries<'a> {
    #[must_use]
    pub fn new(module: &'a Module, type_layouts: &'a TypeLayoutTable) -> Self {
        Self::new_with_context(
            module,
            type_layouts,
            crate::typeck::PackageContext::default(),
        )
    }

    #[must_use]
    pub fn new_with_context(
        module: &'a Module,
        type_layouts: &'a TypeLayoutTable,
        package_context: crate::typeck::PackageContext,
    ) -> Self {
        Self {
            module,
            type_layouts,
            full_cache: RefCell::new(None),
            constraints_cache: RefCell::new(HashMap::new()),
            trait_cache: RefCell::new(None),
            resolve_cache: RefCell::new(HashMap::new()),
            package_context,
        }
    }

    #[must_use]
    pub fn check_module(&self, constraints: &[TypeConstraint]) -> TypeCheckResult {
        if let Some(cached) = self.full_cache.borrow().clone() {
            return cached;
        }
        let result = TypeChecker::new_with_context(
            self.module,
            self.type_layouts,
            self.package_context.clone(),
        )
        .run_full(constraints);
        self.full_cache.replace(Some(result.clone()));
        result
    }

    #[must_use]
    pub fn check_constraints(&self, constraints: &[TypeConstraint]) -> Vec<Diagnostic> {
        let key = constraints_key(constraints);
        if let Some(cached) = self.constraints_cache.borrow().get(&key) {
            return cached.clone();
        }
        let diagnostics = TypeChecker::new_with_context(
            self.module,
            self.type_layouts,
            self.package_context.clone(),
        )
        .run_constraints_only(constraints);
        self.constraints_cache
            .borrow_mut()
            .insert(key, diagnostics.clone());
        diagnostics
    }

    #[must_use]
    pub fn trait_fulfillment(&self) -> TraitFulfillmentReport {
        if let Some(cached) = self.trait_cache.borrow().clone() {
            return cached;
        }
        let report = TypeChecker::new_with_context(
            self.module,
            self.type_layouts,
            self.package_context.clone(),
        )
        .run_trait_checks();
        self.trait_cache.replace(Some(report.clone()));
        report
    }

    #[must_use]
    pub fn resolve_type_expr(
        &self,
        expr: &TypeExpr,
        namespace: Option<&str>,
        context_type: Option<&str>,
    ) -> ImportResolution {
        let key = ResolveKey::new(expr, namespace, context_type);
        if let Some(cached) = self.resolve_cache.borrow().get(&key) {
            return clone_resolution(cached);
        }
        let mut checker = TypeChecker::new_with_context(
            self.module,
            self.type_layouts,
            self.package_context.clone(),
        );
        let result = checker.resolve_type_for_expr(expr, namespace, context_type);
        self.resolve_cache
            .borrow_mut()
            .insert(key, clone_resolution(&result));
        result
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ConstraintCacheKey(u64);

#[derive(Hash, Eq, PartialEq)]
struct ResolveKey {
    expr_repr: String,
    namespace: Option<String>,
    context: Option<String>,
}

impl ResolveKey {
    fn new(expr: &TypeExpr, namespace: Option<&str>, context: Option<&str>) -> Self {
        Self {
            expr_repr: format!("{expr:?}"),
            namespace: namespace.map(str::to_owned),
            context: context.map(str::to_owned),
        }
    }
}

fn constraints_key(constraints: &[TypeConstraint]) -> ConstraintCacheKey {
    let mut hasher = DefaultHasher::new();
    constraints.len().hash(&mut hasher);
    for constraint in constraints {
        hash_constraint_kind(&mut hasher, &constraint.kind);
        hash_span(&mut hasher, constraint.span);
    }
    ConstraintCacheKey(hasher.finish())
}

fn hash_constraint_kind(hasher: &mut DefaultHasher, kind: &ConstraintKind) {
    std::mem::discriminant(kind).hash(hasher);
    match kind {
        ConstraintKind::ParameterType {
            function,
            param,
            ty,
        } => {
            function.hash(hasher);
            param.hash(hasher);
            ty.hash(hasher);
        }
        ConstraintKind::VariableInit {
            function,
            name,
            declared,
            expr,
        } => {
            function.hash(hasher);
            name.hash(hasher);
            declared.hash(hasher);
            expr.hash(hasher);
        }
        ConstraintKind::ReturnType { function, ty } => {
            function.hash(hasher);
            ty.hash(hasher);
        }
        ConstraintKind::ImplTraitBound {
            function,
            opaque_ty,
            bound,
        } => {
            function.hash(hasher);
            opaque_ty.hash(hasher);
            bound.hash(hasher);
        }
        ConstraintKind::ImplementsInterface {
            type_name,
            interface,
        } => {
            type_name.hash(hasher);
            interface.hash(hasher);
        }
        ConstraintKind::ExtensionTarget { extension, target } => {
            extension.hash(hasher);
            target.hash(hasher);
        }
        ConstraintKind::RequiresAutoTrait {
            function,
            target,
            ty,
            trait_kind,
            origin,
        } => {
            function.hash(hasher);
            target.hash(hasher);
            ty.hash(hasher);
            hash_auto_trait_kind(hasher, trait_kind);
            hash_auto_trait_origin(hasher, origin);
        }
        ConstraintKind::ThreadingBackendAvailable {
            function,
            backend,
            call,
        } => {
            function.hash(hasher);
            backend.hash(hasher);
            call.hash(hasher);
        }
        ConstraintKind::RandomDuplication { function } => {
            function.hash(hasher);
        }
        ConstraintKind::EffectEscape { function, effect } => {
            function.hash(hasher);
            effect.hash(hasher);
        }
        ConstraintKind::BorrowEscape {
            function,
            parameter,
            parameter_mode,
            escape,
        } => {
            function.hash(hasher);
            parameter.hash(hasher);
            hash_param_mode(hasher, parameter_mode);
            hash_borrow_escape_category(hasher, escape);
        }
        ConstraintKind::RequiresTrait {
            function,
            ty,
            trait_name,
        } => {
            function.hash(hasher);
            ty.hash(hasher);
            trait_name.hash(hasher);
        }
    }
}

fn hash_auto_trait_kind(hasher: &mut DefaultHasher, kind: &AutoTraitKind) {
    match kind {
        AutoTraitKind::ThreadSafe => 0u8.hash(hasher),
        AutoTraitKind::Shareable => 1u8.hash(hasher),
        AutoTraitKind::Copy => 2u8.hash(hasher),
    }
}

fn hash_auto_trait_origin(hasher: &mut DefaultHasher, origin: &AutoTraitConstraintOrigin) {
    match origin {
        AutoTraitConstraintOrigin::Generic => 0u8.hash(hasher),
        AutoTraitConstraintOrigin::AsyncSuspend => 1u8.hash(hasher),
        AutoTraitConstraintOrigin::ThreadSpawn => 2u8.hash(hasher),
    }
}

fn hash_param_mode(hasher: &mut DefaultHasher, mode: &ParamMode) {
    let value = match mode {
        ParamMode::Value => 0u8,
        ParamMode::In => 1u8,
        ParamMode::Ref => 2u8,
        ParamMode::Out => 3u8,
    };
    value.hash(hasher);
}

fn hash_borrow_escape_category(hasher: &mut DefaultHasher, escape: &BorrowEscapeCategory) {
    match escape {
        BorrowEscapeCategory::Return => 0u8.hash(hasher),
        BorrowEscapeCategory::Store { target } => {
            1u8.hash(hasher);
            target.hash(hasher);
        }
        BorrowEscapeCategory::Capture { closure } => {
            2u8.hash(hasher);
            closure.hash(hasher);
        }
    }
}

fn hash_span(hasher: &mut DefaultHasher, span: Option<Span>) {
    match span {
        Some(span) => {
            span.start.hash(hasher);
            span.end.hash(hasher);
        }
        None => {
            0usize.hash(hasher);
        }
    }
}

fn clone_resolution(resolution: &ImportResolution) -> ImportResolution {
    match resolution {
        ImportResolution::Found(name) => ImportResolution::Found(name.clone()),
        ImportResolution::Ambiguous(candidates) => ImportResolution::Ambiguous(candidates.clone()),
        ImportResolution::NotFound => ImportResolution::NotFound,
    }
}
