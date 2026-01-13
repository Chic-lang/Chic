#![cfg(test)]

use super::helpers::body_contains_call;
use crate::codegen::wasm::emitter::function::FunctionEmitter;
use crate::codegen::wasm::runtime_hooks::{ALL_RUNTIME_HOOKS, RuntimeHook};
use crate::mir::{
    Abi, BasicBlock, BlockId, FnSig, FunctionKind, LocalDecl, LocalKind, MirBody, MirFunction, Ty,
};
use crate::perf::{TraceLevel, Tracepoint, trace_id};
use std::collections::HashMap;

#[test]
fn emits_trace_calls() {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.blocks.push(BasicBlock {
        id: BlockId(0),
        statements: Vec::new(),
        terminator: Some(crate::mir::Terminator::Return),
        span: None,
    });

    let function = MirFunction {
        name: "Demo::trace_me".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
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
        span: None,
        optimization_hints: crate::frontend::attributes::OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    };

    let mut functions = HashMap::new();
    for (idx, hook) in ALL_RUNTIME_HOOKS.iter().enumerate() {
        functions.insert(hook.qualified_name(), idx as u32);
    }
    let trace_enter = *functions
        .get(&RuntimeHook::TraceEnter.qualified_name())
        .expect("trace_enter hook present");
    let trace_exit = *functions
        .get(&RuntimeHook::TraceExit.qualified_name())
        .expect("trace_exit hook present");
    functions.insert(function.name.clone(), functions.len() as u32);

    let mut string_literals = HashMap::new();
    string_literals.insert(
        crate::mir::StrId::new(0),
        crate::codegen::wasm::module_builder::WasmStrLiteral { offset: 0, len: 5 },
    );

    let tracepoint = Tracepoint {
        function: function.name.clone(),
        label: "trace".into(),
        label_id: Some(crate::mir::StrId::new(0)),
        level: TraceLevel::Perf,
        trace_id: trace_id(&function.name, "trace"),
        span: None,
        budget: None,
    };

    let layouts = super::common::wasm_layouts();
    let function_return_tys = HashMap::new();
    let mut emitter = FunctionEmitter::new(
        &function,
        &functions,
        &function_return_tys,
        None,
        &layouts,
        Some(&string_literals),
        None,
        None,
        None,
        None,
        &[],
        &[],
        None,
        None,
        Some(&tracepoint),
    )
    .expect("create emitter");

    let body = emitter.emit_body().expect("emit body");
    assert!(
        body_contains_call(&body, trace_enter),
        "call to trace_enter should be present"
    );
    assert!(
        body_contains_call(&body, trace_exit),
        "call to trace_exit should be present"
    );
}
