//! Async state machine lowering scaffold.
//!
//! This module will eventually lower Chic async functions into native
//! poll/drop routines that the runtime executor can drive. The current
//! implementation only collects lowering plans so subsequent patches can
//! synthesise frame layouts and poll entry points.

use crate::frontend::diagnostics::Span;
use crate::mir::builder::{LoweringDiagnostic, MirStatement, MirStatementKind};
use crate::mir::layout::{
    AutoTraitOverride, AutoTraitSet, FieldLayout, MIN_ALIGN, StructLayout, TypeLayout,
    TypeLayoutTable, TypeRepr, align_to, pointer_align, pointer_size,
};
use crate::mir::state::{AsyncStateMachine, GeneratorStateMachine};
use crate::mir::{
    ASYNC_DIAG_FRAME_LIMIT, ASYNC_DIAG_NO_CAPTURE, ASYNC_DIAG_STACK_ONLY, AsyncFramePolicy,
    NoCaptureMode,
};
use crate::mir::{
    Abi, BasicBlock, BlockId, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl, LocalId,
    LocalKind, MirBody, MirFunction, MirModule, Operand, ParamMode, Place, PointerTy, Rvalue,
    Terminator, Ty,
};

const AWAIT_READY: u32 = 1;

/// Entry point for async lowering. The pass is intentionally incremental: it
/// analyses every async function and records the data required to build future
/// state machines. Subsequent steps will consume the collected plans to emit
/// poll/drop glue and rewrite the original async entry points.
pub fn lower_async_functions(module: &mut MirModule) -> Vec<LoweringDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut stubs = Vec::new();
    let mut artifacts = Vec::new();
    for index in 0..module.functions.len() {
        let plan_data = {
            let function = &module.functions[index];
            if let Some(machine) = function.body.async_machine.as_ref() {
                AsyncPlanData::from(function, machine)
            } else if let Some(generator) = function.body.generator.as_ref() {
                AsyncPlanData::from_generator(function, generator)
            } else {
                continue;
            }
        };
        log_plan(&plan_data);
        let frame_ty = register_frame_layout(module, &plan_data);
        let function = &module.functions[index];
        let analysis = analyse_async_frame(&plan_data, function, &frame_ty, &module.type_layouts);
        enforce_async_policy(&plan_data, function, &analysis, &mut diagnostics);
        if promotion_logging_enabled(&plan_data.policy) {
            log_async_analysis(&plan_data, &analysis);
        }
        artifacts.push(plan_data.artifact(
            index,
            frame_ty.clone(),
            analysis.metrics.clone(),
            &analysis,
        ));
        stubs.push(synthesise_poll_function(&plan_data, plan_data.span));
        stubs.push(synthesise_drop_function(&plan_data, plan_data.span));
    }
    module.functions.extend(stubs);
    module.async_plans = artifacts;
    diagnostics
}

fn log_plan(plan: &AsyncPlanData) {
    tracing::debug!(
        target: "async_lowering",
        function = %plan.function_name,
        frame_fields = plan.frame_fields.len(),
        suspend_points = plan.suspend_points.len(),
        "analysed async function"
    );
    if std::env::var("CHIC_DEBUG_ASYNC_READY").is_ok() {
        eprintln!(
            "[chic-debug] async plan {}: suspend_points={}, frame_fields={}",
            plan.function_name,
            plan.suspend_points.len(),
            plan.frame_fields.len()
        );
    }
}
#[derive(Clone)]
struct AsyncPlanData {
    function_name: String,
    span: Option<Span>,
    frame_fields: Vec<AsyncFrameFieldPlan>,
    suspend_points: Vec<AsyncSuspendPlan>,
    context_local: Option<LocalId>,
    policy: AsyncFramePolicy,
}

impl AsyncPlanData {
    fn from(function: &MirFunction, machine: &AsyncStateMachine) -> Self {
        let frame_fields = machine
            .frame_fields
            .iter()
            .map(|field| AsyncFrameFieldPlan {
                local: field.local,
                name: field.name.clone(),
                ty: field.ty.clone(),
            })
            .collect();
        let suspend_points = machine
            .suspend_points
            .iter()
            .map(|point| AsyncSuspendPlan {
                id: point.id,
                await_block: point.await_block,
                resume_block: point.resume_block,
                drop_block: point.drop_block,
                future_local: point.future,
                destination_local: point.destination,
            })
            .collect();
        Self {
            function_name: function.name.clone(),
            span: function.body.span,
            frame_fields,
            suspend_points,
            context_local: machine.context_local,
            policy: machine.policy.clone(),
        }
    }

    fn from_generator(function: &MirFunction, generator: &GeneratorStateMachine) -> Self {
        let frame_fields = generator
            .yields
            .iter()
            .filter_map(|point| {
                point.value.map(|local| AsyncFrameFieldPlan {
                    local,
                    name: Some(format!("yield_{}", point.id)),
                    ty: Ty::Unknown,
                })
            })
            .collect();
        let suspend_points = generator
            .yields
            .iter()
            .map(|point| AsyncSuspendPlan {
                id: point.id,
                await_block: point.yield_block,
                resume_block: point.resume_block,
                drop_block: point.drop_block,
                future_local: point.value.unwrap_or(LocalId(0)),
                destination_local: None,
            })
            .collect();
        Self {
            function_name: function.name.clone(),
            span: function.body.span,
            frame_fields,
            suspend_points,
            context_local: None,
            policy: AsyncFramePolicy::default(),
        }
    }

    fn artifact(
        &self,
        function_index: usize,
        frame_type: String,
        metrics: AsyncFrameMetrics,
        analysis: &AsyncFrameAnalysis,
    ) -> AsyncLoweringArtifact {
        let mut resume_states = Vec::new();
        for (index, point) in self.suspend_points.iter().enumerate() {
            resume_states.push(AsyncResumeStatePlan {
                state_id: (index as u32) + 1,
                resume_block: point.resume_block,
                drop_block: point.drop_block,
            });
        }
        AsyncLoweringArtifact {
            function_index,
            function_name: self.function_name.clone(),
            frame_type,
            context_local: self.context_local,
            policy: self.policy.clone(),
            metrics,
            frame_fields: self.frame_fields.clone(),
            suspend_points: self.suspend_points.clone(),
            resume_states,
            state_count: (self.suspend_points.len() as u32).saturating_add(1),
            poll_fn: format!("{}::poll", self.function_name),
            drop_fn: format!("{}::drop", self.function_name),
            implicit_promotion: !analysis.captured_non_args.is_empty(),
            captured_arguments: analysis
                .captured_args
                .iter()
                .map(|info| info.name.clone())
                .collect(),
            captured_locals: analysis
                .captured_non_args
                .iter()
                .map(|info| info.name.clone())
                .collect(),
        }
    }
}

fn register_frame_layout(module: &mut MirModule, plan: &AsyncPlanData) -> String {
    let frame_name = format!("{}::AsyncFrame", plan.function_name);
    if module.type_layouts.types.contains_key(&frame_name) {
        return frame_name;
    }
    let mut fields = Vec::new();
    fields.push(FieldLayout {
        name: "State".into(),
        ty: Ty::named("int"),
        index: 0,
        offset: None,
        span: None,
        mmio: None,
        display_name: None,
        is_required: false,
        is_nullable: false,
        is_readonly: false,
        view_of: None,
    });
    fields.push(FieldLayout {
        name: "Context".into(),
        ty: Ty::named("Std.Async.RuntimeContext"),
        index: 1,
        offset: None,
        span: None,
        mmio: None,
        display_name: None,
        is_required: false,
        is_nullable: false,
        is_readonly: false,
        view_of: None,
    });
    for (idx, field) in plan.frame_fields.iter().enumerate() {
        fields.push(FieldLayout {
            name: field
                .name
                .clone()
                .unwrap_or_else(|| format!("local_{}", field.local.0)),
            ty: field.ty.clone(),
            index: (idx + 2) as u32,
            offset: None,
            span: None,
            mmio: None,
            display_name: None,
            is_required: false,
            is_nullable: matches!(field.ty, Ty::Nullable(_)),
            is_readonly: false,
            view_of: None,
        });
    }
    let mut layout = StructLayout {
        name: frame_name.clone(),
        repr: TypeRepr::Default,
        packing: None,
        fields,
        positional: Vec::new(),
        list: None,
        size: None,
        align: None,
        is_readonly: false,
        is_intrinsic: false,
        allow_cross_inline: false,
        auto_traits: AutoTraitSet::all_unknown(),
        overrides: AutoTraitOverride::default(),
        mmio: None,
        dispose: None,
        class: None,
    };
    assign_frame_layout_sizes(&mut layout, &module.type_layouts);
    module
        .type_layouts
        .types
        .insert(frame_name.clone(), TypeLayout::Struct(layout));
    frame_name
}

fn assign_frame_layout_sizes(layout: &mut StructLayout, table: &TypeLayoutTable) {
    let mut offset = 0usize;
    let mut max_align = MIN_ALIGN;
    for field in &mut layout.fields {
        let (size, align) = table
            .size_and_align_for_ty(&field.ty)
            .unwrap_or((pointer_size(), pointer_align()));
        let align = align.max(MIN_ALIGN);
        offset = align_to(offset, align);
        field.offset = Some(offset);
        offset = offset.saturating_add(size);
        if align > max_align {
            max_align = align;
        }
    }
    layout.align = Some(max_align.max(MIN_ALIGN));
    layout.size = Some(align_to(offset, max_align.max(MIN_ALIGN)));
}

fn analyse_async_frame(
    plan: &AsyncPlanData,
    function: &MirFunction,
    frame_type: &str,
    layouts: &TypeLayoutTable,
) -> AsyncFrameAnalysis {
    let mut captured_args = Vec::new();
    let mut captured_non_args = Vec::new();
    for field in &plan.frame_fields {
        let Some(decl) = function.body.local(field.local).cloned() else {
            let fallback = CapturedLocalInfo {
                name: field
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("local_{}", field.local.0)),
                span: plan.span,
                kind: LocalKind::Local,
                param_mode: None,
            };
            captured_non_args.push(fallback);
            continue;
        };

        let info = CapturedLocalInfo {
            name: decl
                .name
                .clone()
                .unwrap_or_else(|| format!("_{}", field.local.0)),
            span: decl.span.or(plan.span),
            kind: decl.kind,
            param_mode: decl.param_mode,
        };
        if matches!(info.kind, LocalKind::Arg(_)) {
            captured_args.push(info);
        } else {
            captured_non_args.push(info);
        }
    }

    let frame_layout = layouts.size_and_align_for_ty(&Ty::named(frame_type.to_string()));
    let metrics = AsyncFrameMetrics {
        frame_size: frame_layout.map(|(size, _)| size),
        frame_align: frame_layout.map(|(_, align)| align),
        capture_count: plan.frame_fields.len(),
        arg_capture_count: captured_args.len(),
        non_arg_capture_count: captured_non_args.len(),
        suspend_points: plan.suspend_points.len(),
    };

    AsyncFrameAnalysis {
        metrics,
        captured_args,
        captured_non_args,
    }
}

fn enforce_async_policy(
    plan: &AsyncPlanData,
    function: &MirFunction,
    analysis: &AsyncFrameAnalysis,
    diagnostics: &mut Vec<LoweringDiagnostic>,
) {
    let function_span = function.span.or(plan.span);

    if plan.policy.stack_only.is_some() {
        let limit = plan
            .policy
            .frame_limit
            .as_ref()
            .map(|limit| limit.bytes)
            .unwrap_or(DEFAULT_STACK_ONLY_LIMIT);
        if let Some(size) = analysis.metrics.frame_size {
            if (size as u64) > limit {
                diagnostics.push(async_diagnostic(
                    ASYNC_DIAG_STACK_ONLY,
                    format!(
                        "async frame for `{}` is {} bytes, exceeding stack-only budget of {} bytes",
                        plan.function_name, size, limit
                    ),
                    plan.policy
                        .frame_limit
                        .as_ref()
                        .and_then(|limit| limit.span)
                        .or(plan.policy.stack_only.as_ref().and_then(|src| src.span))
                        .or(function_span),
                ));
            }
        }
        if !analysis.captured_non_args.is_empty() {
            let offender = &analysis.captured_non_args[0];
            diagnostics.push(async_diagnostic(
                ASYNC_DIAG_STACK_ONLY,
                format!(
                    "async frame for `{}` stores `{}` across awaits, preventing @stack_only from keeping the frame on the stack",
                    plan.function_name, offender.name
                ),
                offender
                    .span
                    .or(plan.policy.stack_only.as_ref().and_then(|src| src.span))
                    .or(function_span),
            ));
        }
    } else if !analysis.captured_non_args.is_empty()
        && (plan.policy.is_configured() || std::env::var("CHIC_WARN_ASYNC_PROMOTION").is_ok())
    {
        let offender = &analysis.captured_non_args[0];
        diagnostics.push(async_diagnostic(
            ASYNC_DIAG_STACK_ONLY,
            format!(
                "async frame for `{}` captures `{}` across await and will be promoted; add `@stack_only`/`@no_capture` to enforce stack residency",
                plan.function_name, offender.name
            ),
            offender.span.or(function_span),
        ));
    }

    if let Some(limit) = &plan.policy.frame_limit {
        match analysis.metrics.frame_size {
            Some(size) if (size as u64) > limit.bytes => diagnostics.push(async_diagnostic(
                ASYNC_DIAG_FRAME_LIMIT,
                format!(
                    "async frame for `{}` estimated at {} bytes exceeds @frame_limit({})",
                    plan.function_name, size, limit.bytes
                ),
                limit.span.or(function_span),
            )),
            Some(_) => {}
            None => diagnostics.push(async_diagnostic(
                ASYNC_DIAG_FRAME_LIMIT,
                format!(
                    "async frame for `{}` has unknown size; cannot enforce @frame_limit({})",
                    plan.function_name, limit.bytes
                ),
                limit.span.or(function_span),
            )),
        }
    } else if let Some(size) = analysis.metrics.frame_size {
        if (size as u64) > DEFAULT_FRAME_WARN_LIMIT {
            diagnostics.push(async_diagnostic(
                ASYNC_DIAG_FRAME_LIMIT,
                format!(
                    "async frame for `{}` estimated at {} bytes exceeds the {}-byte default warning budget",
                    plan.function_name, size, DEFAULT_FRAME_WARN_LIMIT
                ),
                function_span,
            ));
        }
    }

    if let Some(no_capture) = &plan.policy.no_capture {
        if !analysis.captured_non_args.is_empty() {
            let offender = &analysis.captured_non_args[0];
            diagnostics.push(async_diagnostic(
                ASYNC_DIAG_NO_CAPTURE,
                format!(
                    "async frame for `{}` captures `{}` across await, violating @no_capture",
                    plan.function_name, offender.name
                ),
                offender.span.or(no_capture.span).or(function_span),
            ));
        } else if matches!(no_capture.mode, NoCaptureMode::MoveOnly) {
            if let Some(offender) = analysis
                .captured_args
                .iter()
                .chain(analysis.captured_non_args.iter())
                .find(|info| matches!(info.param_mode, Some(ParamMode::Ref | ParamMode::Out)))
            {
                diagnostics.push(async_diagnostic(
                    ASYNC_DIAG_NO_CAPTURE,
                    format!(
                        "async frame for `{}` retains reference capture `{}` despite @no_capture(move)",
                        plan.function_name, offender.name
                    ),
                    offender.span.or(no_capture.span).or(function_span),
                ));
            }
        }
    }
}

fn promotion_logging_enabled(policy: &AsyncFramePolicy) -> bool {
    policy.log_promotion || std::env::var("CHIC_DEBUG_ASYNC_PROMOTION").is_ok()
}

fn log_async_analysis(plan: &AsyncPlanData, analysis: &AsyncFrameAnalysis) {
    let size_text = analysis
        .metrics
        .frame_size
        .map_or_else(|| "unknown".into(), |size| format!("{size}"));
    let align_text = analysis
        .metrics
        .frame_align
        .map_or_else(|| "unknown".into(), |align| format!("{align}"));
    eprintln!(
        "[async-promotion] {}: size={} align={} captures={} (args={}, locals={}), suspend_points={}",
        plan.function_name,
        size_text,
        align_text,
        analysis.total_captures(),
        analysis.captured_args.len(),
        analysis.captured_non_args.len(),
        analysis.metrics.suspend_points
    );
}

fn async_diagnostic(
    code: &str,
    message: impl Into<String>,
    span: Option<Span>,
) -> LoweringDiagnostic {
    LoweringDiagnostic {
        message: format!("[{code}] {}", message.into()),
        span,
    }
}

fn synthesise_poll_function(plan: &AsyncPlanData, span: Option<Span>) -> MirFunction {
    let mut body = MirBody::new(2, span);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("uint"),
        false,
        span,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("__header".into()),
        Ty::Pointer(Box::new(PointerTy::new(
            Ty::named("Std.Async.FutureHeader"),
            true,
        ))),
        true,
        span,
        LocalKind::Arg(0),
    ));
    body.locals.push(LocalDecl::new(
        Some("__ctx".into()),
        Ty::Pointer(Box::new(PointerTy::new(
            Ty::named("Std.Async.RuntimeContext"),
            true,
        ))),
        false,
        span,
        LocalKind::Arg(1),
    ));
    let mut entry = BasicBlock::new(BlockId(0), span);
    entry.statements.push(MirStatement {
        span,
        kind: MirStatementKind::Assign {
            place: Place::new(LocalId(0)),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::UInt(
                AWAIT_READY as u128,
            )))),
        },
    });
    entry.terminator = Some(Terminator::Return);
    body.blocks.push(entry);

    let name = format!("{}::poll", plan.function_name);
    let sig = FnSig {
        params: vec![
            Ty::Pointer(Box::new(PointerTy::new(
                Ty::named("Std.Async.FutureHeader"),
                true,
            ))),
            Ty::Pointer(Box::new(PointerTy::new(
                Ty::named("Std.Async.RuntimeContext"),
                true,
            ))),
        ],
        ret: Ty::named("uint"),
        abi: Abi::Chic,
        effects: Vec::new(),

        lends_to_return: None,

        variadic: false,
    };
    MirFunction {
        name,
        kind: FunctionKind::Function,
        signature: sig,
        body,
        is_async: false,
        async_result: None,
        is_generator: false,
        span,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    }
}

fn synthesise_drop_function(plan: &AsyncPlanData, span: Option<Span>) -> MirFunction {
    let mut body = MirBody::new(1, span);
    body.locals.push(LocalDecl::new(
        None,
        Ty::Unit,
        false,
        span,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("__header".into()),
        Ty::Pointer(Box::new(PointerTy::new(
            Ty::named("Std.Async.FutureHeader"),
            true,
        ))),
        true,
        span,
        LocalKind::Arg(0),
    ));
    let mut entry = BasicBlock::new(BlockId(0), span);
    entry.terminator = Some(Terminator::Return);
    body.blocks.push(entry);
    MirFunction {
        name: format!("{}::drop", plan.function_name),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::Pointer(Box::new(PointerTy::new(
                Ty::named("Std.Async.FutureHeader"),
                true,
            )))],
            ret: Ty::Unit,
            abi: Abi::Chic,
            effects: Vec::new(),

            lends_to_return: None,

            variadic: false,
        },
        body,
        is_async: false,
        async_result: None,
        is_generator: false,
        span,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    }
}

#[derive(Debug, Clone)]
pub struct AsyncLoweringArtifact {
    pub function_index: usize,
    pub function_name: String,
    pub frame_type: String,
    pub context_local: Option<LocalId>,
    pub policy: AsyncFramePolicy,
    pub metrics: AsyncFrameMetrics,
    pub frame_fields: Vec<AsyncFrameFieldPlan>,
    pub suspend_points: Vec<AsyncSuspendPlan>,
    pub resume_states: Vec<AsyncResumeStatePlan>,
    pub state_count: u32,
    pub poll_fn: String,
    pub drop_fn: String,
    pub implicit_promotion: bool,
    pub captured_arguments: Vec<String>,
    pub captured_locals: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AsyncFrameFieldPlan {
    pub local: LocalId,
    pub name: Option<String>,
    pub ty: Ty,
}

#[derive(Debug, Clone)]
pub struct AsyncSuspendPlan {
    pub id: usize,
    pub await_block: BlockId,
    pub resume_block: BlockId,
    pub drop_block: BlockId,
    pub future_local: LocalId,
    pub destination_local: Option<LocalId>,
}

#[derive(Debug, Clone)]
pub struct AsyncResumeStatePlan {
    pub state_id: u32,
    pub resume_block: BlockId,
    pub drop_block: BlockId,
}

#[derive(Debug, Clone, Default)]
pub struct AsyncFrameMetrics {
    pub frame_size: Option<usize>,
    pub frame_align: Option<usize>,
    pub capture_count: usize,
    pub arg_capture_count: usize,
    pub non_arg_capture_count: usize,
    pub suspend_points: usize,
}

#[derive(Debug)]
struct CapturedLocalInfo {
    name: String,
    span: Option<Span>,
    kind: LocalKind,
    param_mode: Option<ParamMode>,
}

#[derive(Debug)]
struct AsyncFrameAnalysis {
    metrics: AsyncFrameMetrics,
    captured_args: Vec<CapturedLocalInfo>,
    captured_non_args: Vec<CapturedLocalInfo>,
}

impl AsyncFrameAnalysis {
    fn total_captures(&self) -> usize {
        self.captured_args.len() + self.captured_non_args.len()
    }
}

const DEFAULT_STACK_ONLY_LIMIT: u64 = 8 * 1024;
const DEFAULT_FRAME_WARN_LIMIT: u64 = 64 * 1024;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::data::module_metadata::ModuleAttributes;
    use crate::mir::state::GeneratorYieldPoint;
    use std::collections::HashMap;

    #[test]
    fn async_lowering_emits_poll_and_drop_shims() {
        let mut body = MirBody::new(0, None);
        body.locals.push(LocalDecl::new(
            Some("_ret".into()),
            Ty::Unit,
            false,
            None,
            LocalKind::Return,
        ));
        body.blocks.push(BasicBlock::new(BlockId(0), None));
        body.async_machine = Some(AsyncStateMachine {
            suspend_points: Vec::new(),
            pinned_locals: Vec::new(),
            cross_locals: Vec::new(),
            frame_fields: Vec::new(),
            result_local: None,
            result_ty: None,
            context_local: None,
            policy: AsyncFramePolicy::default(),
        });
        let function = MirFunction {
            name: "Demo::Async".into(),
            kind: FunctionKind::Function,
            signature: FnSig::empty(),
            body,
            is_async: true,
            async_result: None,
            is_generator: false,
            span: None,
            optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
            extern_spec: None,
            is_weak: false,
            is_weak_import: false,
        };
        let type_layouts = TypeLayoutTable::default();
        let primitive_registry = type_layouts.primitive_registry.clone();
        let mut module = MirModule {
            functions: vec![function],
            test_cases: Vec::new(),
            default_arguments: Vec::new(),
            statics: Vec::new(),
            type_layouts,
            primitive_registry,
            interned_strs: Vec::new(),
            exports: Vec::new(),
            attributes: ModuleAttributes::default(),
            trait_vtables: Vec::new(),
            class_vtables: Vec::new(),
            interface_defaults: Vec::new(),
            type_variance: HashMap::new(),
            async_plans: Vec::new(),
        };

        let diagnostics = lower_async_functions(&mut module);
        assert!(
            diagnostics.is_empty(),
            "unexpected diagnostics: {diagnostics:?}"
        );

        let poll = module
            .functions
            .iter()
            .find(|f| f.name.ends_with("::poll"))
            .expect("poll shim missing");
        assert_eq!(poll.signature.params.len(), 2);
        match &poll.signature.params[0] {
            Ty::Pointer(ptr) => {
                assert_eq!(ptr.element.canonical_name(), "Std.Async.FutureHeader");
            }
            other => panic!("expected header pointer param, found {other:?}"),
        }
        match &poll.signature.params[1] {
            Ty::Pointer(ptr) => {
                assert_eq!(ptr.element.canonical_name(), "Std.Async.RuntimeContext");
            }
            other => panic!("expected runtime context pointer param, found {other:?}"),
        }
        assert_eq!(poll.signature.ret.canonical_name(), "uint");

        let drop = module
            .functions
            .iter()
            .find(|f| f.name.ends_with("::drop"))
            .expect("drop shim missing");
        assert_eq!(drop.signature.params.len(), 1);
        assert!(matches!(drop.signature.ret, Ty::Unit));

        let plan = module
            .async_plans
            .iter()
            .find(|plan| plan.function_name == "Demo::Async")
            .expect("async plan missing");
        assert!(plan.poll_fn.ends_with("Demo::Async::poll"));
        assert!(plan.drop_fn.ends_with("Demo::Async::drop"));
    }

    #[test]
    fn generator_lowering_reuses_async_plans() {
        let mut body = MirBody::new(0, None);
        body.locals.push(LocalDecl::new(
            Some("_ret".into()),
            Ty::Unit,
            false,
            None,
            LocalKind::Return,
        ));
        body.locals.push(LocalDecl::new(
            Some("yielded".into()),
            Ty::named("int"),
            false,
            None,
            LocalKind::Local,
        ));
        body.blocks.push(BasicBlock::new(BlockId(0), None));
        body.blocks.push(BasicBlock::new(BlockId(1), None));
        body.blocks.push(BasicBlock::new(BlockId(2), None));
        body.generator = Some(GeneratorStateMachine {
            yields: vec![GeneratorYieldPoint {
                id: 0,
                yield_block: BlockId(0),
                resume_block: BlockId(1),
                drop_block: BlockId(2),
                value: Some(LocalId(1)),
                span: None,
            }],
        });

        let function = MirFunction {
            name: "Demo::Iter::Numbers".into(),
            kind: FunctionKind::Function,
            signature: FnSig::empty(),
            body,
            is_async: false,
            async_result: None,
            is_generator: true,
            span: None,
            optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
            extern_spec: None,
            is_weak: false,
            is_weak_import: false,
        };
        let type_layouts = TypeLayoutTable::default();
        let primitive_registry = type_layouts.primitive_registry.clone();
        let mut module = MirModule {
            functions: vec![function],
            test_cases: Vec::new(),
            default_arguments: Vec::new(),
            statics: Vec::new(),
            type_layouts,
            primitive_registry,
            interned_strs: Vec::new(),
            exports: Vec::new(),
            attributes: ModuleAttributes::default(),
            trait_vtables: Vec::new(),
            class_vtables: Vec::new(),
            interface_defaults: Vec::new(),
            type_variance: HashMap::new(),
            async_plans: Vec::new(),
        };

        let diagnostics = lower_async_functions(&mut module);
        assert!(
            diagnostics.is_empty(),
            "unexpected diagnostics: {diagnostics:?}"
        );

        let plan = module
            .async_plans
            .iter()
            .find(|plan| plan.function_name == "Demo::Iter::Numbers")
            .expect("generator lowering should emit async plan");
        assert_eq!(plan.state_count, 2);
        assert_eq!(plan.suspend_points.len(), 1);
        assert_eq!(plan.suspend_points[0].await_block, BlockId(0));

        let poll = module
            .functions
            .iter()
            .find(|f| f.name.ends_with("::poll"))
            .expect("poll shim missing for generator");
        assert!(poll.name.contains("Numbers::poll"));
        let drop = module
            .functions
            .iter()
            .find(|f| f.name.ends_with("::drop"))
            .expect("drop shim missing for generator");
        assert!(drop.name.contains("Numbers::drop"));
    }
}
