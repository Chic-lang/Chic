#![cfg(test)]

use super::helpers::*;
use crate::mir::{
    Abi, BasicBlock, BlockId, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl, LocalId,
    LocalKind, MirBody, MirFunction, Operand, Place, Terminator, Ty,
};

#[test]
fn emit_body_calls_distinct_overload_indices() {
    let function = overload_caller();
    let body = emit_body_using_layouts(super::common::wasm_layouts(), function, |indices| {
        indices.insert("Demo::Math::Combine".into(), 3);
        indices.insert("Demo::Math::Combine#1".into(), 4);
        None
    });
    assert!(
        body_contains_call(&body, 3),
        "expected wasm call to first overload"
    );
    assert!(
        body_contains_call(&body, 4),
        "expected wasm call to second overload"
    );
}

fn overload_caller() -> MirFunction {
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("lhs".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Temp,
    ));
    body.locals.push(LocalDecl::new(
        Some("rhs".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Temp,
    ));
    body.blocks.push(BasicBlock {
        id: body.entry(),
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                "Demo::Math::Combine".into(),
            ))),
            args: Vec::new(),
            arg_modes: Vec::new(),
            destination: Some(Place::new(LocalId(1))),
            target: BlockId(1),
            unwind: None,
            dispatch: None,
        }),
        span: None,
    });
    body.blocks.push(BasicBlock {
        id: BlockId(1),
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                "Demo::Math::Combine#1".into(),
            ))),
            args: Vec::new(),
            arg_modes: Vec::new(),
            destination: Some(Place::new(LocalId(2))),
            target: BlockId(2),
            unwind: None,
            dispatch: None,
        }),
        span: None,
    });
    body.blocks.push(BasicBlock {
        id: BlockId(2),
        statements: Vec::new(),
        terminator: Some(Terminator::Return),
        span: None,
    });
    MirFunction {
        name: "Demo::Use::CallBoth".into(),
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
    }
}
