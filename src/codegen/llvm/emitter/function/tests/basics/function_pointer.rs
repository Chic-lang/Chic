use super::super::helpers::{function_ir, test_target};
use crate::chic_kind::ChicKind;
use crate::codegen::CodegenOptions;
use crate::codegen::llvm::emit_module;
use crate::codegen::llvm::signatures::build_signatures;
use crate::mir::{
    Abi, BasicBlock, BlockId, FnSig, FnTy, FunctionKind, LocalDecl, LocalId, LocalKind, MirBody,
    MirFunction, MirModule, Operand, Place, Terminator, Ty,
};
use crate::perf::PerfMetadata;

#[test]
fn emits_indirect_call_for_function_pointer() {
    let fn_ty = FnTy::new(vec![Ty::named("int")], Ty::named("int"), Abi::Chic);
    let mut layouts = crate::mir::TypeLayoutTable::default();
    layouts.ensure_fn_layout(&fn_ty);
    let mut body = MirBody::new(2, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("callback".into()),
        Ty::Fn(fn_ty.clone()),
        false,
        None,
        LocalKind::Arg(0),
    ));
    body.locals.push(LocalDecl::new(
        Some("value".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Arg(1),
    ));

    body.blocks.push(BasicBlock {
        id: BlockId(0),
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Copy(Place::new(LocalId(1))),
            args: vec![Operand::Copy(Place::new(LocalId(2)))],
            arg_modes: vec![crate::mir::ParamMode::Value],
            destination: Some(Place::new(LocalId(0))),
            target: BlockId(1),
            unwind: None,

            dispatch: None,
        }),
        span: None,
    });
    body.blocks.push(BasicBlock {
        id: BlockId(1),
        statements: Vec::new(),
        terminator: Some(Terminator::Return),
        span: None,
    });

    let function = MirFunction {
        name: "Root::Apply".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::Fn(fn_ty), Ty::named("int")],
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
    };

    let mut module = MirModule::default();
    module.type_layouts = layouts;
    module.functions.push(function);
    let target = test_target();
    let signatures = build_signatures(&module, None, &target).expect("signatures");
    let options = CodegenOptions::default();
    let perf = PerfMetadata::default();

    let ir = emit_module(
        &module,
        None,
        &perf,
        &signatures,
        None,
        ChicKind::StaticLibrary,
        target.triple(),
        &target,
        &options,
        &[],
        &[],
        &[],
        &[],
    )
    .expect("emit module");
    let func_ir = function_ir(&ir, "Root__Apply");

    assert!(
        func_ir.contains("getelementptr"),
        "expected field projection on fn pointer: {func_ir}"
    );
    assert!(
        func_ir.contains("call i32"),
        "expected call through function pointer"
    );
}
