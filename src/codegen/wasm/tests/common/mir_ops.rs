use crate::mir::*;

pub(crate) fn function_with_unary_ops() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("value".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));
    body.locals.push(LocalDecl::new(
        Some("flag".into()),
        Ty::named("bool"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(1)),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(5)))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(1)),
            value: Rvalue::Unary {
                op: UnOp::Neg,
                operand: Operand::Copy(Place::new(LocalId(1))),
                rounding: None,
            },
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(2)),
            value: Rvalue::Unary {
                op: UnOp::Not,
                operand: Operand::Const(ConstOperand::new(ConstValue::Bool(true))),
                rounding: None,
            },
        },
    });
    entry.terminator = Some(Terminator::Return);
    body.blocks.push(entry);

    MirFunction {
        name: "Demo::Unary".into(),
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

pub(crate) fn function_with_binary_ops() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("value".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(1)),
            value: Rvalue::Binary {
                op: BinOp::Add,
                lhs: Operand::Const(ConstOperand::new(ConstValue::Int(1))),
                rhs: Operand::Const(ConstOperand::new(ConstValue::Int(2))),
                rounding: None,
            },
        },
    });
    entry.terminator = Some(Terminator::Return);
    body.blocks.push(entry);

    MirFunction {
        name: "Demo::Binary".into(),
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

pub(crate) fn function_with_flag_ops() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("Flags::Permissions"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("value".into()),
        Ty::named("Flags::Permissions"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(1)),
            value: Rvalue::Binary {
                op: BinOp::BitOr,
                lhs: Operand::Const(ConstOperand::new(ConstValue::Int(1))),
                rhs: Operand::Const(ConstOperand::new(ConstValue::Int(2))),
                rounding: None,
            },
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(1)),
            value: Rvalue::Binary {
                op: BinOp::BitAnd,
                lhs: Operand::Copy(Place::new(LocalId(1))),
                rhs: Operand::Const(ConstOperand::new(ConstValue::Int(1))),
                rounding: None,
            },
        },
    });
    entry.terminator = Some(Terminator::Return);
    body.blocks.push(entry);

    MirFunction {
        name: "Flags::Combine".into(),
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

pub(crate) fn numeric_try_add_function() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("bool"),
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("sum".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));

    let intrinsic = NumericIntrinsic {
        kind: NumericIntrinsicKind::TryAdd,
        width: NumericWidth::W32,
        signed: true,
        symbol: "Std::Int32::TryAdd".into(),
        operands: vec![
            Operand::Const(ConstOperand::new(ConstValue::Int(1))),
            Operand::Const(ConstOperand::new(ConstValue::Int(2))),
        ],
        out: Some(Place::new(LocalId(1))),
    };

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(0)),
            value: Rvalue::NumericIntrinsic(intrinsic),
        },
    });
    entry.terminator = Some(Terminator::Return);
    body.blocks.push(entry);

    MirFunction {
        name: "Std::Int32::TryAdd".into(),
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

pub(crate) fn numeric_leading_zero_byte_function() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        false,
        None,
        LocalKind::Return,
    ));

    let intrinsic = NumericIntrinsic {
        kind: NumericIntrinsicKind::LeadingZeroCount,
        width: NumericWidth::W8,
        signed: false,
        symbol: "Std::Byte::LeadingZeroCount".into(),
        operands: vec![Operand::Const(ConstOperand::new(ConstValue::Int(0)))],
        out: None,
    };

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(0)),
            value: Rvalue::NumericIntrinsic(intrinsic),
        },
    });
    entry.terminator = Some(Terminator::Return);
    body.blocks.push(entry);

    MirFunction {
        name: "Std::Byte::LeadingZeroCount".into(),
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
