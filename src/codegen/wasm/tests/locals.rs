#![allow(unused_imports)]

use super::common::*;
use crate::codegen::wasm::emitter::function::{LocalRepresentation, plan_locals};
use crate::codegen::wasm::types::ValueType;
use crate::mir::{
    Abi, BasicBlock, BlockId, FnSig, FunctionKind, LocalDecl, LocalKind, MirBody, MirFunction,
    ParamMode, Terminator, Ty, TypeLayoutTable,
};
use std::collections::HashSet;

#[test]
fn pointer_parameters_use_i32_value_type() {
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(
        LocalDecl::new(
            Some("dest".into()),
            Ty::named("double"),
            true,
            None,
            LocalKind::Arg(0),
        )
        .with_param_mode(ParamMode::Ref),
    );
    let mut block = BasicBlock::new(BlockId(0), None);
    block.terminator = Some(Terminator::Return);
    body.blocks.push(block);

    let function = MirFunction {
        name: "Demo::PointerParam".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::named("double")],
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

    let layouts = wasm_layouts();
    let plan = plan_locals(
        &function,
        &layouts,
        &HashSet::new(),
        function.body.arg_count,
        false,
    )
    .expect("plan locals");
    assert_eq!(
        plan.representations[1],
        LocalRepresentation::PointerParam,
        "expected pointer parameter representation"
    );
    assert_eq!(
        plan.value_types[1],
        Some(ValueType::I32),
        "pointer parameters should be tracked as i32 addresses"
    );
}

#[test]
fn return_slots_use_signature_type() {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unknown,
        false,
        None,
        LocalKind::Return,
    ));
    let mut block = BasicBlock::new(BlockId(0), None);
    block.terminator = Some(Terminator::Return);
    body.blocks.push(block);

    let function = MirFunction {
        name: "Demo::ReturnDouble".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::named("double"),
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

    let layouts = wasm_layouts();
    let plan = plan_locals(
        &function,
        &layouts,
        &HashSet::new(),
        function.body.arg_count,
        false,
    )
    .expect("plan locals");
    assert_eq!(
        plan.value_types[0],
        Some(ValueType::F64),
        "return slot should adopt the signature's scalar type"
    );
}
