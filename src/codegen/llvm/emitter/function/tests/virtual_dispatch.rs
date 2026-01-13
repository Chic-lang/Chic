use super::helpers::{function_ir, test_target};
use crate::codegen::CpuIsaTier;
use crate::codegen::llvm::emitter::function::builder::emit_function;
use crate::codegen::llvm::emitter::metadata_pool::MetadataRegistry;
use crate::codegen::llvm::signatures::build_signatures;
use crate::mir::{
    Abi, AutoTraitOverride, AutoTraitSet, BasicBlock, BlockId, CallDispatch, ClassVTable,
    ClassVTableSlot, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl, LocalId, LocalKind,
    MirBody, MirFunction, MirModule, Operand, ParamMode, Place, Rvalue, Statement, StatementKind,
    StructLayout, Terminator, Ty, TypeLayout, TypeRepr,
};
use std::collections::{BTreeSet, HashMap, HashSet};

#[test]
fn emit_virtual_call_loads_receiver_header() {
    let module = class_dispatch_module();
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let vtable_symbols: HashSet<String> = module
        .class_vtables
        .iter()
        .map(|table| table.symbol.clone())
        .collect();
    let function = module
        .functions
        .iter()
        .find(|func| func.name == "Demo::Tests::Invoke")
        .expect("Invoke function");
    let sig = signatures.get(&function.name).expect("Invoke signature");
    let mut out = String::new();
    let mut metadata = MetadataRegistry::new();
    emit_function(
        &mut out,
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut BTreeSet::new(),
        &vtable_symbols,
        module.trait_vtables.as_slice(),
        module.class_vtables.as_slice(),
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &HashMap::new(),
        &module.type_layouts,
        &mut metadata,
        None,
    )
    .expect("emit invoke");
    let body = function_ir(&out, &sig.symbol);
    assert!(
        body.contains("getelementptr inbounds ptr, ptr"),
        "dynamic virtual dispatch should index through the receiver vtable: {body}"
    );
    assert!(
        body.contains("load ptr, ptr"),
        "dynamic virtual dispatch should load function pointer from slot: {body}"
    );
}

#[test]
fn emit_virtual_call_uses_base_vtable_for_explicit_base_dispatch() {
    let module = class_dispatch_module();
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let vtable_symbols: HashSet<String> = module
        .class_vtables
        .iter()
        .map(|table| table.symbol.clone())
        .collect();
    let function = module
        .functions
        .iter()
        .find(|func| func.name == "Demo::Dog::SpeakAsBase")
        .expect("SpeakAsBase function");
    let sig = signatures
        .get(&function.name)
        .expect("SpeakAsBase signature");
    let mut out = String::new();
    let mut metadata = MetadataRegistry::new();
    emit_function(
        &mut out,
        function,
        sig,
        &sig.symbol,
        "dso_local",
        &signatures,
        &mut BTreeSet::new(),
        &vtable_symbols,
        module.trait_vtables.as_slice(),
        module.class_vtables.as_slice(),
        CpuIsaTier::Baseline,
        &[CpuIsaTier::Baseline],
        target.arch(),
        &target,
        module.statics.as_slice(),
        &HashMap::new(),
        &module.type_layouts,
        &mut metadata,
        None,
    )
    .expect("emit base call");
    let body = function_ir(&out, &sig.symbol);
    assert!(
        body.contains("__class_vtable_Demo__Dog"),
        "base call should reference the recorded class vtable global: {body}"
    );
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
