#![cfg(test)]

use super::common::*;
use crate::frontend::parser::Visibility;
use crate::mir::{
    Abi, BasicBlock, BlockId, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl, LocalId,
    LocalKind, MirBody, MirFunction, MirModule, Operand, Place, Rvalue, Statement, StatementKind,
    StaticId, StaticVar, Terminator, Ty,
};

fn module_with_static_members() -> (WasmFunctionHarness, StaticId) {
    let static_id = StaticId(0);
    let mut module = MirModule::default();
    module.statics.push(StaticVar {
        id: static_id,
        qualified: "Demo::Config::Version".into(),
        owner: Some("Demo::Config".into()),
        namespace: Some("Demo".into()),
        ty: Ty::named("int"),
        visibility: Visibility::Public,
        is_readonly: false,
        threadlocal: false,
        is_weak: false,
        is_extern: false,
        is_import: false,
        is_weak_import: false,
        link_library: None,
        extern_spec: None,
        span: None,
        initializer: Some(ConstValue::Int(7)),
    });
    module.functions.push(read_static_fn(static_id));
    module.functions.push(write_static_fn(static_id));
    (WasmFunctionHarness::from_module(module), static_id)
}

fn read_static_fn(id: StaticId) -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.blocks.push(BasicBlock {
        id: BlockId(0),
        statements: vec![Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(LocalId(0)),
                value: Rvalue::StaticLoad { id },
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    });
    MirFunction {
        name: "Demo::Config::ReadVersion".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::named("int"),
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

fn write_static_fn(id: StaticId) -> MirFunction {
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
        statements: vec![Statement {
            span: None,
            kind: StatementKind::StaticStore {
                id,
                value: Operand::Const(ConstOperand::new(ConstValue::Int(42))),
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    });
    MirFunction {
        name: "Demo::Config::WriteVersion".into(),
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

#[test]
fn static_load_emits_memory_read() {
    let (harness, _) = module_with_static_members();
    let body = harness
        .emit_body_with("Demo::Config::ReadVersion", |_| None)
        .expect("emit read body");
    assert!(
        contains_bytes(&body, &[0x28, 0x00, 0x00]),
        "expected i32.load for static read"
    );
    assert!(
        contains_bytes(&body, &[0x41, 0x00, 0x28, 0x00, 0x00]),
        "expected pointer constant before static load"
    );
}

#[test]
fn static_store_emits_memory_write() {
    let (harness, _) = module_with_static_members();
    let body = harness
        .emit_body_with("Demo::Config::WriteVersion", |_| None)
        .expect("emit write body");
    assert!(
        contains_bytes(&body, &[0x36, 0x00, 0x00]),
        "expected i32.store for static write"
    );
}

fn module_with_module_statics() -> (WasmFunctionHarness, StaticId) {
    let static_id = StaticId(0);
    let mut module = MirModule::default();
    module.statics.push(StaticVar {
        id: static_id,
        qualified: "Demo::Answer".into(),
        owner: None,
        namespace: Some("Demo".into()),
        ty: Ty::named("int"),
        visibility: Visibility::Public,
        is_readonly: false,
        threadlocal: false,
        is_weak: false,
        is_extern: false,
        is_import: false,
        is_weak_import: false,
        link_library: None,
        extern_spec: None,
        span: None,
        initializer: Some(ConstValue::Int(3)),
    });
    module.functions.push(module_static_read_fn(static_id));
    module.functions.push(module_static_write_fn(static_id));
    (WasmFunctionHarness::from_module(module), static_id)
}

fn module_static_read_fn(id: StaticId) -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.blocks.push(BasicBlock {
        id: BlockId(0),
        statements: vec![Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(LocalId(0)),
                value: Rvalue::StaticLoad { id },
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    });
    MirFunction {
        name: "Demo::ReadAnswer".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: Vec::new(),
            ret: Ty::named("int"),
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

fn module_static_write_fn(id: StaticId) -> MirFunction {
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
        statements: vec![Statement {
            span: None,
            kind: StatementKind::StaticStore {
                id,
                value: Operand::Const(ConstOperand::new(ConstValue::Int(5))),
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    });
    MirFunction {
        name: "Demo::WriteAnswer".into(),
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

#[test]
fn module_static_load_emits_memory_read() {
    let (harness, _) = module_with_module_statics();
    let body = harness
        .emit_body_with("Demo::ReadAnswer", |_| None)
        .expect("emit read body");
    assert!(
        contains_bytes(&body, &[0x28, 0x00, 0x00]),
        "expected i32.load for module static read"
    );
}

#[test]
fn module_static_store_emits_memory_write() {
    let (harness, _) = module_with_module_statics();
    let body = harness
        .emit_body_with("Demo::WriteAnswer", |_| None)
        .expect("emit write body");
    assert!(
        contains_bytes(&body, &[0x36, 0x00, 0x00]),
        "expected i32.store for module static write"
    );
}
