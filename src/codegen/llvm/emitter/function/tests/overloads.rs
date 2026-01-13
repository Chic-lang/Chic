use super::helpers::{function_ir, test_target};
use crate::codegen::CpuIsaTier;
use crate::codegen::llvm::emitter::function::builder::emit_function;
use crate::codegen::llvm::emitter::metadata_pool::MetadataRegistry;
use crate::codegen::llvm::signatures::build_signatures;
use crate::mir::{
    Abi, BasicBlock, BlockId, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl, LocalId,
    LocalKind, MirBody, MirFunction, MirModule, Operand, Place, Terminator, Ty,
};
use std::collections::{BTreeSet, HashMap, HashSet};

#[test]
fn emit_call_distinguishes_overload_symbols() {
    let module = overload_module();
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("build llvm signatures");
    let function = module
        .functions
        .iter()
        .find(|func| func.name.ends_with("::CallBoth"))
        .expect("CallBoth function");
    let sig = signatures.get(&function.name).expect("CallBoth signature");
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
        &HashSet::new(),
        &module.trait_vtables,
        &module.class_vtables,
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
    .expect("emit overload caller");
    let body = function_ir(&out, &sig.symbol);
    assert!(
        body.contains("call i32 @Demo__Math__Combine("),
        "expected direct call to first overload: {body}"
    );
    assert!(
        body.contains("call i32 @Demo__Math__Combine_1("),
        "expected direct call to second overload: {body}"
    );
}

fn overload_module() -> MirModule {
    let mut module = MirModule::default();
    module.functions.push(overload_fn("Demo::Math::Combine"));
    module.functions.push(overload_fn("Demo::Math::Combine#1"));
    module.functions.push(caller_fn());
    module
}

fn overload_fn(name: &str) -> MirFunction {
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.blocks.push(BasicBlock {
        id: body.entry(),
        statements: Vec::new(),
        terminator: Some(Terminator::Return),
        span: None,
    });
    MirFunction {
        name: name.into(),
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

fn caller_fn() -> MirFunction {
    let mut body = MirBody::new(1, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("tmp0".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Temp,
    ));
    body.locals.push(LocalDecl::new(
        Some("tmp1".into()),
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
