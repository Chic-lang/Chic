use crate::mir::*;

pub(crate) fn call_function_with_operand(func_operand: Operand) -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("tmp".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::Call {
        func: func_operand,
        args: Vec::new(),
        arg_modes: Vec::new(),
        destination: None,
        target: BlockId(1),
        unwind: None,

        dispatch: None,
    });
    body.blocks.push(entry);

    let mut target = BasicBlock::new(BlockId(1), None);
    target.terminator = Some(Terminator::Return);
    body.blocks.push(target);

    MirFunction {
        name: "Demo::CallSite".into(),
        kind: FunctionKind::Function,
        signature: FnSig::empty(),
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

pub(crate) fn call_function_with_string_operand(name: &str) -> MirFunction {
    call_function_with_operand(Operand::Const(ConstOperand::new(ConstValue::Symbol(
        name.into(),
    ))))
}

pub(crate) fn switch_int_overflow_function() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::SwitchInt {
        discr: Operand::Const(ConstOperand::new(ConstValue::Int(0))),
        targets: vec![(i128::from(i32::MAX) + 1, BlockId(1))],
        otherwise: BlockId(1),
    });
    body.blocks.push(entry);

    let mut target = BasicBlock::new(BlockId(1), None);
    target.terminator = Some(Terminator::Return);
    body.blocks.push(target);

    MirFunction {
        name: "Demo::SwitchOverflow".into(),
        kind: FunctionKind::Function,
        signature: FnSig::empty(),
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
