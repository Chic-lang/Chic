use crate::mir::*;

pub(crate) fn match_with_projection_function() -> MirFunction {
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
        Ty::named("Demo::Pair"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut match_place = Place::new(LocalId(1));
    match_place.projection.push(ProjectionElem::Field(0));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::Match {
        value: match_place,
        arms: vec![MatchArm {
            pattern: Pattern::Wildcard,
            guard: None,
            bindings: Vec::new(),
            target: BlockId(1),
        }],
        otherwise: BlockId(1),
    });
    body.blocks.push(entry);

    let mut target = BasicBlock::new(BlockId(1), None);
    target.terminator = Some(Terminator::Return);
    body.blocks.push(target);

    MirFunction {
        name: "Demo::ProjectedMatch".into(),
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

pub(crate) fn match_literal_function() -> MirFunction {
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
    entry.terminator = Some(Terminator::Match {
        value: Place::new(LocalId(1)),
        arms: vec![MatchArm {
            pattern: Pattern::Literal(ConstValue::Int(1)),
            guard: None,
            bindings: Vec::new(),
            target: BlockId(1),
        }],
        otherwise: BlockId(2),
    });
    body.blocks.push(entry);

    let mut arm_block = BasicBlock::new(BlockId(1), None);
    arm_block.terminator = Some(Terminator::Return);
    body.blocks.push(arm_block);

    let mut default_block = BasicBlock::new(BlockId(2), None);
    default_block.terminator = Some(Terminator::Return);
    body.blocks.push(default_block);

    MirFunction {
        name: "Demo::LiteralMatch".into(),
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

pub(crate) fn function_with_wildcard_match() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
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
    entry.terminator = Some(Terminator::Match {
        value: Place::new(LocalId(1)),
        arms: vec![MatchArm {
            pattern: Pattern::Wildcard,
            guard: None,
            bindings: Vec::new(),
            target: BlockId(1),
        }],
        otherwise: BlockId(2),
    });
    body.blocks.push(entry);

    for id in 1..=2 {
        let mut block = BasicBlock::new(BlockId(id), None);
        block.terminator = Some(Terminator::Return);
        body.blocks.push(block);
    }

    MirFunction {
        name: "Demo::MatchWildcard".into(),
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

pub(crate) fn match_enum_function() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("value".into()),
        Ty::named("Demo::Choice"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::Match {
        value: Place::new(LocalId(1)),
        arms: vec![MatchArm {
            pattern: Pattern::Enum {
                path: vec!["Demo".into(), "Choice".into()],
                variant: "First".into(),
                fields: VariantPatternFields::Unit,
            },
            guard: None,
            bindings: Vec::new(),
            target: BlockId(1),
        }],
        otherwise: BlockId(2),
    });
    body.blocks.push(entry);

    let mut matched = BasicBlock::new(BlockId(1), None);
    matched.terminator = Some(Terminator::Return);
    body.blocks.push(matched);

    let mut fallback = BasicBlock::new(BlockId(2), None);
    fallback.terminator = Some(Terminator::Return);
    body.blocks.push(fallback);

    MirFunction {
        name: "Demo::EnumMatch".into(),
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

pub(crate) fn function_with_complex_match() -> MirFunction {
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
        Ty::named("Demo::Pair"),
        true,
        None,
        LocalKind::Local,
    ));
    body.locals.push(LocalDecl::new(
        Some("x".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::Match {
        value: Place::new(LocalId(1)),
        arms: vec![MatchArm {
            pattern: Pattern::Struct {
                path: vec!["Demo".into(), "Pair".into()],
                fields: vec![
                    PatternField {
                        name: "X".into(),
                        pattern: Pattern::Binding(BindingPattern {
                            name: "x".into(),
                            mutability: PatternBindingMutability::Mutable,
                            mode: PatternBindingMode::Value,
                        }),
                    },
                    PatternField {
                        name: "Y".into(),
                        pattern: Pattern::Wildcard,
                    },
                ],
            },
            guard: None,
            bindings: vec![PatternBinding {
                name: "x".into(),
                local: LocalId(2),
                projection: vec![PatternProjectionElem::FieldNamed("X".into())],
                span: None,
                mutability: PatternBindingMutability::Mutable,
                mode: PatternBindingMode::Value,
            }],
            target: BlockId(1),
        }],
        otherwise: BlockId(2),
    });
    body.blocks.push(entry);

    let mut guard = BasicBlock::new(BlockId(1), None);
    guard.terminator = Some(Terminator::Return);
    body.blocks.push(guard);

    let mut default = BasicBlock::new(BlockId(2), None);
    default.terminator = Some(Terminator::Return);
    body.blocks.push(default);

    MirFunction {
        name: "Demo::ComplexMatch".into(),
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

pub(crate) fn guarded_match_function(
    name: &str,
    value_const: i128,
    guard_const: bool,
    success_result: i128,
    fallback_result: i128,
) -> MirFunction {
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
        Some("guard".into()),
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
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(
                value_const,
            )))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(2)),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Bool(
                guard_const,
            )))),
        },
    });
    entry.terminator = Some(Terminator::SwitchInt {
        discr: Operand::Copy(Place::new(LocalId(1))),
        targets: vec![(1, BlockId(2))],
        otherwise: BlockId(1),
    });
    body.blocks.push(entry);

    let mut fallback = BasicBlock::new(BlockId(1), None);
    fallback.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(0)),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(
                fallback_result,
            )))),
        },
    });
    fallback.terminator = Some(Terminator::Return);
    body.blocks.push(fallback);

    let mut binding = BasicBlock::new(BlockId(2), None);
    binding.terminator = Some(Terminator::Goto { target: BlockId(3) });
    body.blocks.push(binding);

    let mut guard_block = BasicBlock::new(BlockId(3), None);
    guard_block.terminator = Some(Terminator::SwitchInt {
        discr: Operand::Copy(Place::new(LocalId(2))),
        targets: vec![(1, BlockId(4))],
        otherwise: BlockId(1),
    });
    body.blocks.push(guard_block);

    let mut success = BasicBlock::new(BlockId(4), None);
    success.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(0)),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(
                success_result,
            )))),
        },
    });
    success.terminator = Some(Terminator::Return);
    body.blocks.push(success);

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
