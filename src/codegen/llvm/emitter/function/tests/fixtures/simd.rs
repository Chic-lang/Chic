use crate::mir::{
    Abi, BasicBlock, BlockId, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl, LocalId,
    LocalKind, MirBody, MirFunction, MirModule, Operand, Place, Terminator, Ty,
};

pub(crate) fn simd_fma_module() -> MirModule {
    let mut body = MirBody::new(3, None);
    body.locals.push(LocalDecl::new(
        None,
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        None,
        Ty::named("f32x8"),
        false,
        None,
        LocalKind::Arg(0),
    ));
    body.locals.push(LocalDecl::new(
        None,
        Ty::named("f32x8"),
        false,
        None,
        LocalKind::Arg(1),
    ));
    body.locals.push(LocalDecl::new(
        None,
        Ty::named("f32x8"),
        false,
        None,
        LocalKind::Arg(2),
    ));

    let args = vec![
        Operand::Copy(Place::new(LocalId(1))),
        Operand::Copy(Place::new(LocalId(2))),
        Operand::Copy(Place::new(LocalId(3))),
    ];
    let arg_modes = vec![crate::mir::ParamMode::Value; args.len()];
    let entry = BasicBlock {
        id: BlockId(0),
        statements: Vec::new(),
        terminator: Some(Terminator::Call {
            func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
                "std.simd.f32x8.fma".into(),
            ))),
            args,
            arg_modes,
            destination: None,
            target: BlockId(1),
            unwind: None,
            dispatch: None,
        }),
        span: None,
    };
    let exit = BasicBlock {
        id: BlockId(1),
        statements: Vec::new(),
        terminator: Some(Terminator::Return),
        span: None,
    };
    body.blocks.push(entry);
    body.blocks.push(exit);

    let function = MirFunction {
        name: "Root::SimdFma".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![Ty::named("f32x8"), Ty::named("f32x8"), Ty::named("f32x8")],
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

    let mut module = MirModule::default();
    module.functions.push(function);
    module
}
