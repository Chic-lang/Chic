use crate::mir::{
    Abi, BasicBlock, BlockId, ConstOperand, ConstValue, FnSig, FunctionKind, LocalDecl, LocalId,
    LocalKind, MirBody, MirFunction, MirModule, Operand, Place, Terminator, Ty,
};

pub(crate) fn linalg_dpbusd_module() -> MirModule {
    let mut body = MirBody::new(3, None);
    body.locals.push(LocalDecl::new(
        None,
        Ty::named("i32x16"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        None,
        Ty::named("i32x16"),
        false,
        None,
        LocalKind::Arg(0),
    ));
    body.locals.push(LocalDecl::new(
        None,
        Ty::named("i32x16"),
        false,
        None,
        LocalKind::Arg(1),
    ));
    body.locals.push(LocalDecl::new(
        None,
        Ty::named("i32x16"),
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
                "std.linalg.int8x64.dpbusd".into(),
            ))),
            args,
            arg_modes,
            destination: Some(Place::new(LocalId(0))),
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
        name: "Root::Dpbusd".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![
                Ty::named("i32x16"),
                Ty::named("i32x16"),
                Ty::named("i32x16"),
            ],
            ret: Ty::named("i32x16"),
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
