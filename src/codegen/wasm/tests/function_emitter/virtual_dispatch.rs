#![cfg(test)]

use super::common::*;
use crate::codegen::wasm::module_builder::FunctionSignature;
use crate::mir::{
    Abi, AutoTraitOverride, AutoTraitSet, BasicBlock, BlockId, CallDispatch, ClassVTable,
    ClassVTableSlot, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl, LocalId, LocalKind,
    MirBody, MirFunction, MirModule, Operand, ParamMode, Place, Rvalue, Statement, StatementKind,
    StructLayout, Terminator, Ty, TypeLayout, TypeRepr,
};
use std::collections::HashMap;

#[test]
fn emit_virtual_call_loads_receiver_header() {
    let module = class_dispatch_module();
    let harness = WasmFunctionHarness::from_module(module);
    let signature_map = signature_indices(&harness);
    let body = harness
        .emit_body_with("Demo::Tests::Invoke", |_| Some(signature_map.clone()))
        .expect("emit invoke");
    assert!(
        contains_bytes(&body, &[0x28]),
        "dynamic virtual dispatch should load the vtable pointer from the receiver header"
    );
    assert!(
        contains_bytes(&body, &[0x11]),
        "dynamic virtual dispatch should call_indirect through the function table"
    );
}

#[test]
fn emit_virtual_call_reads_base_table_address() {
    let module = class_dispatch_module();
    let harness = WasmFunctionHarness::from_module(module);
    let signature_map = signature_indices(&harness);
    let body = harness
        .emit_body_with("Demo::Dog::SpeakAsBase", |_| Some(signature_map.clone()))
        .expect("emit base call");
    assert!(
        contains_bytes(&body, &[0x41]),
        "base dispatch should materialize the class vtable address"
    );
}

fn signature_indices(harness: &WasmFunctionHarness) -> HashMap<FunctionSignature, u32> {
    harness
        .module()
        .functions
        .iter()
        .enumerate()
        .map(|(idx, func)| {
            (
                FunctionSignature::from_mir(func, &harness.module().type_layouts),
                idx as u32,
            )
        })
        .collect()
}

fn class_dispatch_module() -> MirModule {
    let mut module = MirModule::default();
    register_class_layout(&mut module, "Demo::Animal");
    register_class_layout(&mut module, "Demo::Dog");
    module.class_vtables.push(ClassVTable {
        type_name: "Demo::Animal".into(),
        symbol: "__class_vtable_Demo__Animal".into(),
        version: 0,
        slots: vec![
            ClassVTableSlot {
                slot_index: 0,
                member: "Speak".into(),
                accessor: None,
                symbol: "Demo::Animal::Speak".into(),
            },
            ClassVTableSlot {
                slot_index: 1,
                member: "Chain".into(),
                accessor: None,
                symbol: "Demo::Animal::Chain".into(),
            },
        ],
    });
    module.class_vtables.push(ClassVTable {
        type_name: "Demo::Dog".into(),
        symbol: "__class_vtable_Demo__Dog".into(),
        version: 0,
        slots: vec![
            ClassVTableSlot {
                slot_index: 0,
                member: "Speak".into(),
                accessor: None,
                symbol: "Demo::Dog::Speak".into(),
            },
            ClassVTableSlot {
                slot_index: 1,
                member: "Chain".into(),
                accessor: None,
                symbol: "Demo::Dog::Chain".into(),
            },
        ],
    });
    module.functions.push(simple_method("Demo::Animal::Speak"));
    module.functions.push(simple_method("Demo::Animal::Chain"));
    module.functions.push(simple_method("Demo::Dog::Speak"));
    module.functions.push(simple_method("Demo::Dog::Chain"));
    module.functions.push(invoke_function());
    module.functions.push(base_call_function());
    module
}

fn register_class_layout(module: &mut MirModule, name: &str) {
    module.type_layouts.types.insert(
        name.to_string(),
        TypeLayout::Class(StructLayout {
            name: name.to_string(),
            repr: TypeRepr::Default,
            packing: None,
            fields: Vec::new(),
            positional: Vec::new(),
            list: None,
            size: Some(0),
            align: Some(1),
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );
}

fn simple_method(name: &str) -> MirFunction {
    let mut body = MirBody::new(2, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("self".into()),
        Ty::named("ptr"),
        false,
        None,
        LocalKind::Arg(0),
    ));
    body.blocks.push(BasicBlock {
        id: body.entry(),
        statements: vec![Statement {
            span: None,
            kind: StatementKind::Assign {
                place: Place::new(LocalId(0)),
                value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(1.into())))),
            },
        }],
        terminator: Some(Terminator::Return),
        span: None,
    });
    MirFunction {
        name: name.into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::named("Demo::Animal")],
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

fn invoke_function() -> MirFunction {
    let mut body = MirBody::new(2, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("target".into()),
        Ty::named("Demo::Animal"),
        false,
        None,
        LocalKind::Arg(0),
    ));
    body.blocks.push(BasicBlock {
        id: body.entry(),
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                "Demo::Dog::Speak".into(),
            ))),
            args: vec![Operand::Copy(Place::new(LocalId(1)))],
            arg_modes: vec![ParamMode::Value],
            destination: Some(Place::new(LocalId(0))),
            target: BlockId(1),
            unwind: None,
            dispatch: Some(CallDispatch::Virtual(crate::mir::VirtualDispatch {
                slot_index: 0,
                receiver_index: 0,
                base_owner: None,
            })),
        }),
        span: None,
    });
    body.blocks.push(BasicBlock {
        id: BlockId(1),
        statements: Vec::new(),
        terminator: Some(Terminator::Return),
        span: None,
    });
    MirFunction {
        name: "Demo::Tests::Invoke".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::named("Demo::Animal")],
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

fn base_call_function() -> MirFunction {
    let mut body = MirBody::new(2, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("self".into()),
        Ty::named("Demo::Dog"),
        false,
        None,
        LocalKind::Arg(0),
    ));
    body.blocks.push(BasicBlock {
        id: body.entry(),
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                "Demo::Animal::Speak".into(),
            ))),
            args: vec![Operand::Copy(Place::new(LocalId(1)))],
            arg_modes: vec![ParamMode::Value],
            destination: Some(Place::new(LocalId(0))),
            target: BlockId(1),
            unwind: None,
            dispatch: Some(CallDispatch::Virtual(crate::mir::VirtualDispatch {
                slot_index: 0,
                receiver_index: 0,
                base_owner: Some("Demo::Dog".into()),
            })),
        }),
        span: None,
    });
    body.blocks.push(BasicBlock {
        id: BlockId(1),
        statements: Vec::new(),
        terminator: Some(Terminator::Return),
        span: None,
    });
    MirFunction {
        name: "Demo::Dog::SpeakAsBase".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::named("Demo::Dog")],
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
