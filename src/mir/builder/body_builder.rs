use self::closures::analysis::{CapturedLocal, LambdaLoweringBody, analyze_captures};
use self::closures::environment::{
    LambdaParameterInfo, closure_temp_operand, register_closure_layout,
};
use self::closures::lowering::{ClosureEnvironmentInfo, ClosureInfo};
use self::expressions::collect_path_segments;
use super::module_lowering::driver::TypeDeclInfo;
use super::module_lowering::traits::TraitLoweringInfo;
use super::static_registry::StaticRegistry;
use super::symbol_index::{
    ConstSymbol, FieldSymbol, FunctionParamSymbol, PropertySymbol, SymbolIndex,
};
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::accessibility::{AccessContext, AccessFailure, AccessResult};
use crate::drop_glue::{drop_glue_symbol_for, drop_type_identity};
use crate::frontend::ast::{
    Block, Expression, FunctionDecl, GenericParamKind, GenericParams, PropertyAccessorKind,
    Statement, StatementKind, TypeExpr,
};
use crate::frontend::import_resolver::ImportResolver;
use crate::frontend::local_functions::{local_function_env_name, local_function_symbol};
use crate::frontend::parser::parse_type_expression_text;
use crate::frontend::{attributes::ConditionalAttribute, conditional};
use crate::mir::async_types::task_result_ty;
use crate::mir::builder::support::resolve_type_layout_name;
use crate::mir::builder::{FunctionSpecialization, specialised_function_name};
use crate::mir::operators::OperatorRegistry;
use crate::mir::{
    AliasContract, AsyncFramePolicy, AtomicOrdering, ConstOperand, DebugNote, GenericArg,
    InterpolatedStringSegment,
};
use crate::mir::{StructLayout, UnionLayout};
use crate::primitives::PrimitiveRegistry;
use crate::syntax::expr::ExprNode;
use crate::threading::ThreadRuntimeMode;
use crate::typeck::BorrowEscapeCategory;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[allow(dead_code)]
pub(super) fn impl_trait_bounds_from_type_expr(expr: &TypeExpr) -> Option<Vec<String>> {
    let object = expr.trait_object().filter(|object| object.opaque_impl)?;
    let mut bounds = Vec::new();
    for bound in &object.bounds {
        let ty = Ty::from_type_expr(bound);
        bounds.push(ty.canonical_name());
    }
    if bounds.is_empty() {
        None
    } else {
        Some(bounds)
    }
}

/// Helper macro for body builder submodules so new helpers can attach to
/// `BodyBuilder<'a>` without repeating the lifetime bound boilerplate.
macro_rules! body_builder_impl {
    ($($items:tt)*) => {
        impl<'a> BodyBuilder<'a> {
            $($items)*
        }
    };
}

body_builder_impl! {
    pub(crate) fn push_debug_note(&mut self, message: String, span: Option<Span>) {
        self.body.debug_notes.push(DebugNote { message, span });
    }

    pub(crate) fn atomic_order_from_expression(
        &mut self,
        expression: &Expression,
        context: &str,
    ) -> Option<AtomicOrdering> {
        let Some(node) = expression.node.as_ref() else {
            self.emit_atomic_ordering_error(expression.span, context);
            return None;
        };
        self.atomic_order_from_expr_node(node, expression.span, context)
    }

    pub(crate) fn atomic_order_from_expr_node(
        &mut self,
        node: &ExprNode,
        span: Option<Span>,
        context: &str,
    ) -> Option<AtomicOrdering> {
        fn collect(node: &ExprNode, out: &mut Vec<String>) -> bool {
            match node {
                ExprNode::Identifier(name) => {
                    out.push(name.clone());
                    true
                }
                ExprNode::Member { base, member, .. } => {
                    if collect(base, out) {
                        out.push(member.clone());
                        true
                    } else {
                        false
                    }
                }
                ExprNode::Parenthesized(inner) => collect(inner, out),
                _ => false,
            }
        }

        let mut segments = Vec::new();
        if !collect(node, &mut segments) {
            self.emit_atomic_ordering_error(span, context);
            return None;
        }
        self.atomic_order_from_segments(&segments, span, context)
    }

    pub(crate) fn atomic_order_from_operand(
        &mut self,
        operand: &Operand,
        span: Option<Span>,
        context: &str,
    ) -> Option<AtomicOrdering> {
        match operand {
            Operand::Const(constant) => {
                if let ConstValue::Enum {
                    type_name,
                    variant,
                    ..
                } = &constant.value
                {
                    if Self::is_memory_order_type(type_name) {
                        return AtomicOrdering::from_variant(variant.as_str());
                    }
                }
                if let Some(name) = constant.symbol_name() {
                    if let Some(order) = Self::atomic_order_from_path(name) {
                        return Some(order);
                    }
                }
            }
            Operand::Pending(pending) => {
                if let Some(order) = Self::atomic_order_from_path(&pending.repr) {
                    return Some(order);
                }
            }
            _ => {}
        }
        self.emit_atomic_ordering_error(span, context);
        None
    }

    pub(crate) fn atomic_order_from_segments(
        &mut self,
        segments: &[String],
        span: Option<Span>,
        context: &str,
    ) -> Option<AtomicOrdering> {
        if segments.is_empty() {
            self.emit_atomic_ordering_error(span, context);
            return None;
        }
        let variant = segments.last().expect("non-empty segments");
        let type_segments = &segments[..segments.len().saturating_sub(1)];
        if type_segments.is_empty() {
            self.emit_atomic_ordering_error(span, context);
            return None;
        }
        let type_name = type_segments.join("::");
        if !Self::is_memory_order_type(&type_name) {
            self.emit_atomic_ordering_error(span, context);
            return None;
        }
        AtomicOrdering::from_variant(variant.as_str())
    }

    pub(crate) fn atomic_order_from_path(path: &str) -> Option<AtomicOrdering> {
        let canonical = path.replace("::", ".");
        let mut segments: Vec<&str> = canonical
            .split('.')
            .filter(|segment| !segment.is_empty())
            .collect();
        if segments.len() < 2 {
            return None;
        }
        let variant = segments.pop()?.to_string();
        let type_name = segments.join("::");
        if !Self::is_memory_order_type(&type_name) {
            return None;
        }
        AtomicOrdering::from_variant(&variant)
    }

    pub(crate) fn emit_atomic_ordering_error(&mut self, span: Option<Span>, context: &str) {
        self.diagnostics.push(LoweringDiagnostic {
            message: format!(
                "{context} expects a `Std.Sync.MemoryOrder` value"
            ),
            span,
        });
    }

    pub(crate) fn is_memory_order_type(name: &str) -> bool {
        let canonical = name.replace("::", ".").trim_matches('.').to_string();
        canonical
            .split('.')
            .last()
            .is_some_and(|segment| segment == "MemoryOrder")
    }

    pub(crate) fn is_atomic_type_name(name: &str) -> bool {
        let canonical = name.replace("::", ".");
        canonical.starts_with("Std.Sync.Atomic") || canonical.starts_with("std.sync.Atomic")
    }
}

mod async_support;
mod casts;
mod checked;
mod closures;
mod coercions;
mod constructors;
pub(crate) mod drop_lowering;
mod expressions;
mod finalize;
mod fixed;
mod goto;
mod lock;
mod loops;
mod nameof;
mod null_coalesce;
mod orchestrator;
mod pattern;
mod region;
mod resource_dispatch;
mod returns;
pub(super) mod span_conversions;
pub(super) mod state;
mod statements;
mod switch;

mod throw;
mod try_catch;
mod try_flow;
mod using;

pub(super) use state::AssignmentSourceKind;
pub(super) use state::LoopContext;

pub(super) struct BodyBuilder<'a> {
    body: MirBody,
    locals: Vec<LocalDecl>,
    blocks: Vec<BasicBlock>,
    scopes: Vec<ScopeFrame>,
    loop_stack: Vec<LoopContext>,
    switch_stack: Vec<SwitchContext>,
    try_stack: Vec<TryContext>,
    current_block: BlockId,
    diagnostics: Vec<LoweringDiagnostic>,
    temp_counter: usize,
    constraints: Vec<TypeConstraint>,
    function_name: String,
    self_type_name: Option<String>,
    return_type: Ty,
    opaque_return: Option<OpaqueReturnInfo>,
    declared_effects: Vec<Ty>,
    suspend_points: Vec<AsyncSuspendPoint>,
    generator_points: Vec<GeneratorYieldPoint>,
    async_result_ty: Option<Ty>,
    async_result_local: Option<LocalId>,
    async_context_local: Option<LocalId>,
    async_policy: AsyncFramePolicy,
    next_suspend_id: usize,
    next_foreach_id: usize,
    next_yield_id: usize,
    next_fixed_id: usize,
    next_using_id: usize,
    next_lock_id: usize,
    next_closure_id: usize,
    next_borrow_id: usize,
    next_region_id: usize,
    next_exception_id: usize,
    next_local_function_id: usize,
    is_async: bool,
    is_generator: bool,
    unsafe_depth: usize,
    lock_depth: usize,
    atomic_depth: usize,
    unchecked_depth: usize,
    first_class_spans: bool,
    in_guard_expression: bool,
    generic_param_index: HashMap<String, usize>,
    type_layouts: &'a mut TypeLayoutTable,
    type_visibilities: &'a HashMap<String, TypeDeclInfo>,
    primitive_registry: &'a PrimitiveRegistry,
    operator_registry: &'a OperatorRegistry,
    string_interner: &'a mut StringInterner,
    symbol_index: &'a SymbolIndex,
    trait_registry: &'a HashMap<String, TraitLoweringInfo>,
    default_arguments: DefaultArgumentStore,
    namespace: Option<String>,
    current_package: Option<String>,
    function_packages: &'a HashMap<String, String>,
    import_resolver: &'a ImportResolver,
    static_import_types: Vec<String>,
    exception_regions: Vec<ExceptionRegion>,
    labels: HashMap<String, LabelState>,
    pending_gotos: HashMap<String, Vec<PendingGoto>>,
    match_binding_counter: usize,
    async_cross_locals: HashSet<LocalId>,
    function_kind: FunctionKind,
    nested_functions: Vec<MirFunction>,
    closure_registry: HashMap<String, closures::ClosureInfo>,
    closure_fn_signatures: HashMap<String, FnTy>,
    capture_cache: closures::analysis::CaptureCache,
    local_function_table: Vec<LocalFunctionEntry>,
    vectorize_decimal: bool,
    thread_runtime_mode: ThreadRuntimeMode,
    conditional_defines: conditional::ConditionalDefines,
    static_registry: &'a StaticRegistry,
    class_bases: &'a HashMap<String, Vec<String>>,
    class_virtual_slots: &'a HashMap<String, HashMap<String, u32>>,
    lending_return_params: Vec<LocalId>,
    lending_return_names: Option<Vec<String>>,
    generic_specializations: Rc<RefCell<Vec<FunctionSpecialization>>>,
    ffi_pointer_context: bool,
}

fn collect_generic_param_names(generics: Option<&GenericParams>) -> Vec<String> {
    generics
        .map(|params| {
            params
                .params
                .iter()
                .map(|param| param.name.clone())
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Clone)]
struct LocalFunctionEntry {
    name: String,
    symbol: String,
    decl: FunctionDecl,
    ordinal: usize,
    span: Option<Span>,
    lowered: bool,
    capture_ty_name: Option<String>,
    captures: Vec<CapturedLocal>,
}

#[derive(Clone)]
pub(super) struct OpaqueReturnInfo {
    bounds: Vec<String>,
    declared_span: Option<Span>,
    inferred: Option<String>,
    unknown_spans: Vec<Option<Span>>,
}

impl OpaqueReturnInfo {
    pub(super) fn new(bounds: Vec<String>, declared_span: Option<Span>) -> Self {
        Self {
            bounds,
            declared_span,
            inferred: None,
            unknown_spans: Vec::new(),
        }
    }
}

pub(super) fn opaque_return_info_from_ty(ty: &Ty, span: Option<Span>) -> Option<OpaqueReturnInfo> {
    match ty {
        Ty::TraitObject(object) if object.opaque_impl => {
            Some(OpaqueReturnInfo::new(object.traits.clone(), span))
        }
        Ty::Nullable(inner) => opaque_return_info_from_ty(inner, span),
        Ty::Ref(reference) => opaque_return_info_from_ty(&reference.element, span),
        Ty::Array(array) => opaque_return_info_from_ty(&array.element, span),
        Ty::Vec(vec) => opaque_return_info_from_ty(&vec.element, span),
        Ty::Span(span_ty) => opaque_return_info_from_ty(&span_ty.element, span),
        Ty::ReadOnlySpan(span_ty) => opaque_return_info_from_ty(&span_ty.element, span),
        Ty::Rc(rc) => opaque_return_info_from_ty(&rc.element, span),
        Ty::Arc(arc) => opaque_return_info_from_ty(&arc.element, span),
        Ty::Tuple(tuple) => tuple
            .elements
            .iter()
            .find_map(|elem| opaque_return_info_from_ty(elem, span)),
        Ty::Fn(fn_ty) => {
            if let Some(info) = opaque_return_info_from_ty(fn_ty.ret.as_ref(), span) {
                return Some(info);
            }
            fn_ty
                .params
                .iter()
                .find_map(|param| opaque_return_info_from_ty(param, span))
        }
        Ty::Named(named) => {
            for arg in named.args() {
                if let Some(ty) = arg.as_type() {
                    if let Some(info) = opaque_return_info_from_ty(ty, span) {
                        return Some(info);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

impl<'a> BodyBuilder<'a> {
    #[expect(
        clippy::too_many_arguments,
        reason = "Body builder constructor needs multiple lowering parameters."
    )]
    pub(super) fn new(
        signature: &FnSig,
        span: Option<Span>,
        function_name: &str,
        is_async: bool,
        is_unsafe: bool,
        generic_param_names: Vec<String>,
        type_layouts: &'a mut TypeLayoutTable,
        type_visibilities: &'a HashMap<String, TypeDeclInfo>,
        primitive_registry: &'a PrimitiveRegistry,
        default_arguments: DefaultArgumentStore,
        namespace: Option<&str>,
        current_package: Option<String>,
        function_packages: &'a HashMap<String, String>,
        operator_registry: &'a OperatorRegistry,
        string_interner: &'a mut StringInterner,
        symbol_index: &'a SymbolIndex,
        import_resolver: &'a ImportResolver,
        static_registry: &'a StaticRegistry,
        class_bases: &'a HashMap<String, Vec<String>>,
        class_virtual_slots: &'a HashMap<String, HashMap<String, u32>>,
        trait_registry: &'a HashMap<String, TraitLoweringInfo>,
        function_kind: FunctionKind,
        vectorize_decimal: bool,
        thread_runtime_mode: ThreadRuntimeMode,
        lends_to_return: Option<Vec<String>>,
        opaque_return: Option<OpaqueReturnInfo>,
        generic_specializations: Rc<RefCell<Vec<FunctionSpecialization>>>,
    ) -> Self {
        let mut async_result_ty = if is_async {
            task_result_ty(&signature.ret)
        } else {
            None
        };
        if async_result_ty.is_none() && is_async && matches!(function_kind, FunctionKind::Testcase)
        {
            async_result_ty = Some(Ty::named("bool"));
        }

        let mut return_ty = signature.ret.clone();
        if !is_async && matches!(function_kind, FunctionKind::Testcase) {
            // Testcases are evaluated by the runner via a boolean result. Even when parsing
            // `testcase` as a `void`-returning declaration (so fluent assertions can be used
            // without explicit `return` statements), we lower the MIR signature as `bool` and
            // default the return slot to `true` so reaching the end of the body implies success.
            return_ty = Ty::named("bool");
        }
        if !is_async
            && matches!(function_kind, FunctionKind::Testcase)
            && std::env::var_os("CHIC_DEBUG_LOWER_TESTCASE_RET").is_some()
        {
            eprintln!(
                "[lower-testcase-ret] {} signature_ret={:?} lowered_ret={:?}",
                function_name, signature.ret, return_ty
            );
        }
        if is_async {
            if let Some(result) = async_result_ty.clone() {
                return_ty = Ty::named_generic("Std::Async::Task", vec![GenericArg::Type(result)]);
            }
        }

        let mut body = MirBody::new(signature.params.len(), span);
        let mut locals = Vec::new();
        let return_local = LocalDecl::new(
            Some(String::from("_ret")),
            return_ty.clone(),
            false,
            span,
            LocalKind::Return,
        );
        locals.push(return_local);

        let entry = BasicBlock::new(BlockId(0), span);
        body.blocks.push(entry.clone());

        let mut generic_param_index = HashMap::new();
        for name in generic_param_names {
            let idx = generic_param_index.len();
            generic_param_index.insert(name, idx);
        }

        let self_type_name = match function_kind {
            FunctionKind::Method | FunctionKind::Constructor => {
                let mut segments = function_name.split("::").collect::<Vec<_>>();
                if segments.len() < 2 {
                    None
                } else {
                    segments.pop();
                    Some(segments.join("::"))
                }
            }
            _ => None,
        };

        let mut namespace_owned = namespace.map(|ns| ns.replace('.', "::"));
        if let (Some(ns), Some(self_ty)) = (namespace_owned.as_deref(), self_type_name.as_deref()) {
            if ns == self_ty {
                namespace_owned = self_ty
                    .rsplit_once("::")
                    .map(|(prefix, _)| prefix.to_string());
            }
        }
        let static_import_types =
            Self::collect_static_imports(import_resolver, namespace_owned.as_deref());

        let mut builder = Self {
            body,
            locals,
            blocks: vec![entry],
            scopes: vec![ScopeFrame::default()],
            loop_stack: Vec::new(),
            switch_stack: Vec::new(),
            try_stack: Vec::new(),
            current_block: BlockId(0),
            diagnostics: Vec::new(),
            temp_counter: 0,
            constraints: Vec::new(),
            function_name: function_name.to_string(),
            self_type_name,
            return_type: return_ty,
            opaque_return,
            declared_effects: signature.effects.clone(),
            suspend_points: Vec::new(),
            generator_points: Vec::new(),
            async_result_ty,
            async_result_local: None,
            async_context_local: None,
            async_policy: AsyncFramePolicy::default(),
            next_suspend_id: 0,
            next_foreach_id: 0,
            next_yield_id: 0,
            next_fixed_id: 0,
            next_using_id: 0,
            next_lock_id: 0,
            next_closure_id: 0,
            next_borrow_id: 0,
            next_region_id: 0,
            next_exception_id: 0,
            next_local_function_id: 0,
            is_async,
            is_generator: false,
            unsafe_depth: if is_unsafe { 1 } else { 0 },
            lock_depth: 0,
            atomic_depth: 0,
            unchecked_depth: 0,
            first_class_spans: crate::language::first_class_spans_enabled(),
            in_guard_expression: false,
            generic_param_index,
            type_layouts,
            type_visibilities,
            primitive_registry,
            operator_registry,
            string_interner,
            symbol_index,
            trait_registry,
            default_arguments,
            namespace: namespace_owned,
            current_package,
            function_packages,
            import_resolver,
            static_import_types,
            exception_regions: Vec::new(),
            labels: HashMap::new(),
            pending_gotos: HashMap::new(),
            match_binding_counter: 0,
            async_cross_locals: HashSet::new(),
            function_kind,
            nested_functions: Vec::new(),
            closure_registry: HashMap::new(),
            closure_fn_signatures: HashMap::new(),
            capture_cache: closures::analysis::CaptureCache::default(),
            local_function_table: Vec::new(),
            vectorize_decimal,
            thread_runtime_mode,
            conditional_defines: conditional::active_defines(),
            static_registry,
            class_bases,
            class_virtual_slots,
            lending_return_params: Vec::new(),
            lending_return_names: lends_to_return,
            generic_specializations,
            ffi_pointer_context: false,
        };

        builder.initialise_async_runtime_locals(span);
        builder
    }

    fn current_self_type_name(&self) -> Option<String> {
        self.self_type_name.clone()
    }

    fn owner_package(&self, owner: &str) -> Option<&str> {
        self.type_visibilities
            .get(owner)
            .and_then(|info| info.package.as_deref())
    }

    fn owner_namespace<'b>(&'b self, owner: &str, explicit: Option<&'b str>) -> Option<&'b str> {
        explicit.or_else(|| {
            self.type_visibilities
                .get(owner)
                .and_then(|info| info.namespace.as_deref())
        })
    }

    fn function_package(&self, symbol: &FunctionSymbol) -> Option<&str> {
        if let Some(pkg) = self.function_packages.get(&symbol.qualified) {
            return Some(pkg.as_str());
        }
        if let Some(owner) = symbol.owner.as_deref() {
            return self.owner_package(owner);
        }
        None
    }

    #[allow(dead_code)]
    fn build_access_context<'b>(
        &'b self,
        receiver_type: Option<&'b str>,
        is_instance: bool,
    ) -> AccessContext<'b> {
        AccessContext {
            current_package: self.current_package.as_deref(),
            current_type: self.self_type_name.as_deref(),
            current_namespace: self.namespace.as_deref(),
            receiver_type,
            is_instance,
        }
    }

    fn check_member_access(
        &self,
        visibility: Visibility,
        owner: &str,
        owner_package: Option<&str>,
        owner_namespace: Option<&str>,
        receiver_type: Option<&str>,
        is_instance: bool,
    ) -> AccessResult {
        let ctx = AccessContext {
            current_package: self.current_package.as_deref(),
            current_type: self.self_type_name.as_deref(),
            current_namespace: self.namespace.as_deref(),
            receiver_type,
            is_instance,
        };

        crate::accessibility::check_access(
            visibility,
            owner,
            owner_package,
            owner_namespace,
            &ctx,
            |lhs, rhs| lhs == rhs,
            |derived, base| self.inherits_from(derived, base),
        )
    }

    fn access_denial_reason(
        &self,
        owner: &str,
        package: Option<&str>,
        failure: AccessFailure,
    ) -> String {
        let current_type = self.current_self_type_name();
        match failure {
            AccessFailure::Private => format!("private to `{owner}`"),
            AccessFailure::InternalPackage => package
                .map(|pkg| format!("internal to `{pkg}`"))
                .unwrap_or_else(|| "internal to its declaring package".to_string()),
            AccessFailure::ProtectedInheritance => {
                format!("protected and requires access from `{owner}` or derived types")
            }
            AccessFailure::ProtectedReceiver => current_type
                .map(|ty| {
                    format!(
                        "protected members must be accessed through `{ty}` or derived receivers"
                    )
                })
                .unwrap_or_else(|| {
                    "protected members must be accessed through the accessing derived type"
                        .to_string()
                }),
            AccessFailure::ProtectedInternalUnavailable => {
                "protected internal members require the same package or an inheritance path"
                    .to_string()
            }
            AccessFailure::PrivateProtectedUnavailable => {
                "private protected members require a derived type in the same package".to_string()
            }
        }
    }

    pub(super) fn member_accessible(
        &mut self,
        visibility: Visibility,
        owner: &str,
        owner_package: Option<&str>,
        owner_namespace: Option<&str>,
        receiver_type: Option<&str>,
        is_instance: bool,
        span: Option<Span>,
        descriptor: &str,
    ) -> bool {
        let package = owner_package.or_else(|| self.owner_package(owner));
        let owner_namespace = self.owner_namespace(owner, owner_namespace);
        let result = self.check_member_access(
            visibility,
            owner,
            package,
            owner_namespace,
            receiver_type,
            is_instance,
        );
        if result.allowed {
            return true;
        }
        let reason = self.access_denial_reason(
            owner,
            package,
            result.failure.unwrap_or(AccessFailure::InternalPackage),
        );
        self.diagnostics.push(LoweringDiagnostic {
            message: format!("{descriptor} is not accessible ({reason})"),
            span,
        });
        false
    }

    pub(super) fn async_result_ty(&self) -> Option<&Ty> {
        self.async_result_ty.as_ref()
    }

    fn initialise_async_runtime_locals(&mut self, span: Option<Span>) {
        if !self.is_async {
            return;
        }
        self.ensure_async_context_local(span);
    }

    pub(super) fn set_async_policy(&mut self, policy: AsyncFramePolicy) {
        self.async_policy = policy;
    }

    fn ensure_async_context_local(&mut self, span: Option<Span>) -> Option<LocalId> {
        if !self.is_async {
            return None;
        }
        if let Some(local) = self.async_context_local {
            return Some(local);
        }
        let ty = Ty::named("Std.Async.RuntimeContext");
        self.ensure_ty_layout_for_ty(&ty);
        let decl = LocalDecl::new(Some("__async_ctx".into()), ty, false, span, LocalKind::Temp);
        let id = self.push_local(decl);
        self.async_context_local = Some(id);
        Some(id)
    }

    pub(super) fn ensure_async_result_local(&mut self, span: Option<Span>) -> Option<LocalId> {
        if self.async_result_ty.is_none() {
            return None;
        }
        if let Some(local) = self.async_result_local {
            return Some(local);
        }
        let ty = self.async_result_ty.clone().unwrap_or(Ty::Unit);
        let mut decl = LocalDecl::new(Some("async_result".into()), ty, true, span, LocalKind::Temp);
        decl.is_nullable = matches!(decl.ty, Ty::Nullable(_));
        let id = self.push_local(decl);
        self.async_result_local = Some(id);
        Some(id)
    }

    pub(super) fn lower_parameters(&mut self, parameters: &[Parameter]) {
        for (index, param) in parameters.iter().enumerate() {
            let mutable = matches!(param.binding, BindingModifier::Ref | BindingModifier::Out);
            let name = Some(param.name.clone());
            let span = param
                .name_span
                .or(param.ty.span)
                .or(param.default_span)
                .or(self.body.span);
            let ty = Ty::from_type_expr(&param.ty);
            self.ensure_ty_layout_for_ty(&ty);
            let mode = match param.binding {
                BindingModifier::In => ParamMode::In,
                BindingModifier::Ref => ParamMode::Ref,
                BindingModifier::Out => ParamMode::Out,
                BindingModifier::Value => ParamMode::Value,
            };
            let mut decl = LocalDecl::new(name, ty, mutable, span, LocalKind::Arg(index))
                .with_param_mode(mode);
            let alias = Self::alias_contract_for_parameter(&decl.ty, mode);
            decl.aliasing = alias;
            if param.binding_nullable || param.ty.is_nullable() {
                decl.is_nullable = true;
            }
            if is_pin_type_name(&param.ty.name) {
                decl.is_pinned = true;
            }
            let id = self.push_local(decl);
            self.bind_name(&param.name, id);
            if param.name == "self" {
                self.bind_name("this", id);
            } else if param.name == "this" {
                self.bind_name("self", id);
            }
        }

        if let Some(names) = self.lending_return_names.take() {
            self.lending_return_params = names
                .into_iter()
                .filter_map(|name| self.lookup_name(&name))
                .collect();
        }
    }

    fn alias_contract_for_parameter(ty: &Ty, mode: ParamMode) -> AliasContract {
        let mut contract = AliasContract::default();
        match mode {
            ParamMode::In => {
                contract.noalias = true;
                contract.nocapture = true;
                contract.readonly = true;
            }
            ParamMode::Ref => {
                contract.noalias = true;
                contract.nocapture = true;
            }
            ParamMode::Out => {
                contract.noalias = true;
                contract.nocapture = true;
                contract.writeonly = true;
            }
            ParamMode::Value => {}
        }
        Self::apply_pointer_qualifiers(&mut contract, ty);
        contract
    }

    fn apply_pointer_qualifiers(contract: &mut AliasContract, ty: &Ty) {
        match ty {
            Ty::Pointer(pointer) => {
                if pointer.qualifiers.restrict {
                    contract.restrict = true;
                    contract.noalias = true;
                }
                if pointer.qualifiers.noalias {
                    contract.noalias = true;
                }
                if pointer.qualifiers.readonly {
                    contract.readonly = true;
                }
                if pointer.qualifiers.expose_address {
                    contract.expose_address = true;
                }
                if let Some(alignment) = pointer.qualifiers.alignment {
                    contract.alignment = Some(
                        contract
                            .alignment
                            .map_or(alignment, |current| current.max(alignment)),
                    );
                }
            }
            Ty::Nullable(inner) => Self::apply_pointer_qualifiers(contract, inner),
            _ => {}
        }
    }

    fn pointer_has_expose_address(ty: &Ty) -> bool {
        match ty {
            Ty::Pointer(pointer) => pointer.qualifiers.expose_address,
            Ty::Nullable(inner) => Self::pointer_has_expose_address(inner),
            _ => false,
        }
    }

    fn predeclare_block_local_functions(&mut self, block: &Block) {
        self.predeclare_statement_list(&block.statements);
    }

    fn predeclare_statement_list(&mut self, statements: &[Statement]) {
        for statement in statements {
            if let StatementKind::LocalFunction(local) = &statement.kind {
                let entry_index = self.register_local_function_entry(local.clone(), statement.span);
                self.bind_local_function_name(&local.name, entry_index, statement.span);
            }
        }
    }

    fn register_local_function_entry(&mut self, local: FunctionDecl, span: Option<Span>) -> usize {
        let ordinal = self.next_local_function_id;
        self.next_local_function_id += 1;
        let symbol = local_function_symbol(&self.function_name, ordinal, &local.name);
        let entry = LocalFunctionEntry {
            name: local.name.clone(),
            symbol: symbol.clone(),
            decl: local,
            ordinal,
            span,
            lowered: false,
            capture_ty_name: None,
            captures: Vec::new(),
        };
        let index = self.local_function_table.len();
        self.local_function_table.push(entry);
        index
    }

    fn bind_local_function_name(&mut self, name: &str, entry_index: usize, span: Option<Span>) {
        if let Some(frame) = self.scopes.last_mut() {
            if frame
                .local_functions
                .insert(name.to_string(), entry_index)
                .is_some()
            {
                self.diagnostics.push(LoweringDiagnostic {
                    message: format!("duplicate local function `{name}` in this scope"),
                    span,
                });
            }
        }
    }

    fn lookup_local_function_entry(&self, name: &str) -> Option<usize> {
        for frame in self.scopes.iter().rev() {
            if let Some(index) = frame.local_functions.get(name) {
                return Some(*index);
            }
        }
        None
    }

    fn instantiate_local_function(
        &mut self,
        entry_index: usize,
        span: Option<Span>,
    ) -> Option<Operand> {
        if !self.ensure_local_function_lowered(entry_index) {
            return None;
        }
        let entry = &self.local_function_table[entry_index];
        if entry.captures.is_empty() {
            return Some(Operand::Const(ConstOperand::new(ConstValue::Symbol(
                entry.symbol.clone(),
            ))));
        }
        let ty_name = entry
            .capture_ty_name
            .as_ref()
            .cloned()
            .unwrap_or_else(|| local_function_env_name(&self.function_name, entry.ordinal));
        let captures = entry.captures.clone();
        let span_hint = entry.span;
        Some(closure_temp_operand(
            self,
            span.or(span_hint),
            &ty_name,
            &captures,
        ))
    }

    fn ensure_local_function_lowered(&mut self, entry_index: usize) -> bool {
        if self.local_function_table[entry_index].lowered {
            return true;
        }
        self.lower_local_function(entry_index).is_some()
    }

    fn lower_local_function(&mut self, entry_index: usize) -> Option<()> {
        let entry = self.local_function_table.get(entry_index)?.clone();
        let Some(body) = entry.decl.body.clone() else {
            self.diagnostics.push(LoweringDiagnostic {
                message: format!("local function `{}` must provide a body", entry.name),
                span: entry.span,
            });
            return None;
        };

        let lowering_body = LambdaLoweringBody::Block(body.clone());
        let captures = analyze_captures(self, &lowering_body);
        let capture_ty_name = if captures.is_empty() {
            None
        } else {
            let ty_name = local_function_env_name(&self.function_name, entry.ordinal);
            register_closure_layout(self, &ty_name, &captures);
            Some(ty_name)
        };
        let param_info = self.convert_function_parameters(&entry.decl);
        let closure_info = self.lower_nested_local_function(
            &entry,
            &captures,
            &param_info,
            capture_ty_name.clone(),
        )?;
        if let Some(env_name) = &capture_ty_name {
            self.register_closure_info(env_name.clone(), closure_info);
        }
        if let Some(entry_mut) = self.local_function_table.get_mut(entry_index) {
            entry_mut.lowered = true;
            entry_mut.capture_ty_name = capture_ty_name;
            entry_mut.captures = captures;
        }
        Some(())
    }

    fn convert_function_parameters(&mut self, func: &FunctionDecl) -> Vec<LambdaParameterInfo> {
        func.signature
            .parameters
            .iter()
            .map(|param| {
                let ty = Ty::from_type_expr(&param.ty);
                self.ensure_ty_layout_for_ty(&ty);
                let mode = match param.binding {
                    BindingModifier::In => ParamMode::In,
                    BindingModifier::Ref => ParamMode::Ref,
                    BindingModifier::Out => ParamMode::Out,
                    BindingModifier::Value => ParamMode::Value,
                };
                LambdaParameterInfo {
                    name: param.name.clone(),
                    ty,
                    mode,
                    mutable: matches!(mode, ParamMode::Ref | ParamMode::Out),
                    is_nullable: param.binding_nullable || param.ty.is_nullable(),
                    default: param.default.clone(),
                }
            })
            .collect()
    }

    fn lower_nested_local_function(
        &mut self,
        entry: &LocalFunctionEntry,
        captures: &[CapturedLocal],
        params: &[LambdaParameterInfo],
        capture_ty_name: Option<String>,
    ) -> Option<ClosureInfo> {
        let mut param_tys = Vec::with_capacity(captures.len() + params.len());
        for capture in captures {
            param_tys.push(capture.ty.clone());
        }
        for param in params {
            param_tys.push(param.ty.clone());
        }

        let mut effects = Vec::new();
        if let Some(throws) = entry.decl.signature.throws.as_ref() {
            for effect in &throws.types {
                let ty = Ty::from_type_expr(effect);
                self.ensure_ty_layout_for_ty(&ty);
                effects.push(ty);
            }
        }

        let lends_to_return = entry
            .decl
            .signature
            .lends_to_return
            .as_ref()
            .map(|clause| clause.targets.clone());
        let mut sig = FnSig {
            params: param_tys,
            ret: Ty::from_type_expr(&entry.decl.signature.return_type),
            abi: Abi::Chic,
            effects,

            lends_to_return: None,

            variadic: false,
        };
        self.ensure_ty_layout_for_ty(&sig.ret);
        let generic_param_names = collect_generic_param_names(entry.decl.generics.as_ref());

        let nested_span = entry.decl.body.as_ref().and_then(|block| block.span);
        let opaque_return = opaque_return_info_from_ty(&sig.ret, nested_span);
        let mut nested_builder = BodyBuilder::new(
            &sig,
            nested_span,
            &entry.symbol,
            entry.decl.is_async,
            entry.decl.is_unsafe,
            generic_param_names,
            self.type_layouts,
            self.type_visibilities,
            self.primitive_registry,
            self.default_arguments.clone(),
            self.namespace.as_deref(),
            self.current_package.clone(),
            self.function_packages,
            self.operator_registry,
            self.string_interner,
            self.symbol_index,
            self.import_resolver,
            self.static_registry,
            self.class_bases,
            self.class_virtual_slots,
            self.trait_registry,
            FunctionKind::Function,
            false,
            self.thread_runtime_mode,
            lends_to_return,
            opaque_return,
            self.generic_specializations.clone(),
        );

        for (index, capture) in captures.iter().enumerate() {
            nested_builder.ensure_ty_layout_for_ty(&capture.ty);
            let mut decl = LocalDecl::new(
                Some(capture.name.clone()),
                capture.ty.clone(),
                capture.is_mutable,
                entry.span,
                LocalKind::Arg(index),
            )
            .with_param_mode(ParamMode::Value);
            if capture.is_nullable {
                decl.is_nullable = true;
            }
            let id = nested_builder.push_local(decl);
            nested_builder.bind_name(&capture.name, id);
        }

        for (idx, param) in params.iter().enumerate() {
            nested_builder.ensure_ty_layout_for_ty(&param.ty);
            let mut decl = LocalDecl::new(
                Some(param.name.clone()),
                param.ty.clone(),
                param.mutable,
                entry.span,
                LocalKind::Arg(captures.len() + idx),
            )
            .with_param_mode(param.mode);
            if param.is_nullable {
                decl.is_nullable = true;
            }
            let id = nested_builder.push_local(decl);
            nested_builder.bind_name(&param.name, id);
        }

        if let Some(body) = entry.decl.body.as_ref() {
            nested_builder.lower_block(body);
        }

        let (body, mut diagnostics, mut constraints, nested_functions) = nested_builder.finish();
        if let Some(ret_local) = body.locals.first() {
            sig.ret = ret_local.ty.clone();
        }
        let mut info = ClosureInfo {
            invoke_symbol: entry.symbol.clone(),
            capture_fields: captures.iter().map(|c| c.name.clone()).collect(),
            fn_ty: FnTy::with_modes(
                params.iter().map(|param| param.ty.clone()).collect(),
                params.iter().map(|param| param.mode).collect(),
                sig.ret.clone(),
                Abi::Chic,
                false,
            ),
            environment: capture_ty_name.as_ref().map(|ty_name| {
                let ty = Ty::named(ty_name.clone());
                let canonical = ty.canonical_name();
                let drop_glue_symbol = if self.type_layouts.type_requires_drop(&canonical) {
                    Some(drop_glue_symbol_for(&canonical))
                } else {
                    None
                };
                let layout_info = self.type_layouts.size_and_align_for_ty(&ty);
                ClosureEnvironmentInfo {
                    drop_glue_symbol,
                    env_size: layout_info.map(|(size, _)| size),
                    env_align: layout_info.map(|(_, align)| align),
                    env_ty_name: Some(canonical),
                }
            }),
            params: params
                .iter()
                .map(|param| FunctionParamSymbol {
                    name: param.name.clone(),
                    has_default: param.default.is_some(),
                    mode: param.mode,
                    is_extension_this: false,
                })
                .collect(),
            capture_ty_name: capture_ty_name.clone(),
        };

        let nested_function = MirFunction {
            name: entry.symbol.clone(),
            kind: FunctionKind::Function,
            signature: sig,
            body,
            is_async: entry.decl.is_async,
            async_result: None,
            is_generator: false,
            span: entry.span,
            optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
            extern_spec: None,
            is_weak: false,
            is_weak_import: false,
        };

        self.register_nested_function(nested_function);
        self.register_nested_functions(nested_functions);
        self.diagnostics.append(&mut diagnostics);
        self.constraints.append(&mut constraints);

        if captures.is_empty() {
            info.environment = None;
        }

        Some(info)
    }

    fn lower_local_function_statement(&mut self, statement: &AstStatement) {
        if let AstStatementKind::LocalFunction(local) = &statement.kind {
            if let Some(index) = self.lookup_local_function_entry(&local.name) {
                self.ensure_local_function_lowered(index);
            }
        }
    }
}
