use crate::decimal::Decimal128;
use crate::frontend::diagnostics::Span;
use crate::mir::FloatValue;
use crate::mir::*;
use crate::mmio::AddressSpaceId;

pub(crate) fn float_constant_function() -> MirFunction {
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
        Ty::named("float"),
        false,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(1)),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Float(
                FloatValue::from_f64(1.5),
            )))),
        },
    });
    entry.terminator = Some(Terminator::Return);
    body.blocks.push(entry);

    MirFunction {
        name: "Demo::FloatConst".into(),
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

pub(crate) fn pending_rvalue_function() -> MirFunction {
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

    // Exercise awaited resume lowering that still surfaces as a pending rvalue.
    let pending_span = Span::new(32, 59);
    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: Some(pending_span),
        kind: StatementKind::Assign {
            place: Place::new(LocalId(1)),
            value: Rvalue::Pending(PendingRvalue {
                repr: "pending::<AwaitResume(Demo::AsyncJob)>".into(),
                span: Some(pending_span),
            }),
        },
    });
    entry.terminator = Some(Terminator::Return);
    body.blocks.push(entry);

    MirFunction {
        name: "Demo::PendingRvalue".into(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_rvalue_fixture_exposes_repr_and_span() {
        let function = pending_rvalue_function();
        let block = function
            .body
            .blocks
            .first()
            .expect("pending fixture should have an entry block");
        let statement = block
            .statements
            .first()
            .expect("pending fixture should contain an assignment");
        let span = statement
            .span
            .expect("pending fixture should provide a statement span");

        match &statement.kind {
            StatementKind::Assign { value, .. } => match value {
                Rvalue::Pending(pending) => {
                    assert_eq!(
                        pending.repr, "pending::<AwaitResume(Demo::AsyncJob)>",
                        "fixture should mirror the awaited resume repr planned by lowering"
                    );
                    assert_eq!(
                        pending.span,
                        Some(span),
                        "pending rvalue should retain the recorded source span"
                    );
                }
                other => panic!("expected pending rvalue, found {other:?}"),
            },
            other => panic!("expected assignment statement, found {other:?}"),
        }
    }
}

pub(crate) fn borrow_operand_function() -> MirFunction {
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
    body.locals.push(LocalDecl::new(
        Some("handle".into()),
        Ty::named("&int"),
        true,
        None,
        LocalKind::Local,
    ));

    let borrow_operand = BorrowOperand {
        kind: BorrowKind::Shared,
        place: Place::new(LocalId(1)),
        region: RegionVar(0),
        span: None,
    };

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(LocalId(1)),
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Borrow {
            borrow_id: BorrowId(0),
            kind: BorrowKind::Shared,
            place: Place::new(LocalId(1)),
            region: RegionVar(0),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(2)),
            value: Rvalue::Use(Operand::Borrow(borrow_operand)),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Drop {
            place: Place::new(LocalId(1)),
            target: BlockId(0),
            unwind: None,
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageDead(LocalId(2)),
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageDead(LocalId(1)),
    });
    entry.terminator = Some(Terminator::Return);
    body.blocks.push(entry);

    MirFunction {
        name: "Demo::BorrowOperand".into(),
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

pub(crate) fn decimal_intrinsic_function() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    ));
    body.locals.push(LocalDecl::new(
        Some("lhs".into()),
        Ty::named("decimal"),
        true,
        None,
        LocalKind::Local,
    ));
    body.locals.push(LocalDecl::new(
        Some("rhs".into()),
        Ty::named("decimal"),
        true,
        None,
        LocalKind::Local,
    ));
    body.locals.push(LocalDecl::new(
        Some("result".into()),
        Ty::named("Std::Numeric::Decimal::DecimalIntrinsicResult"),
        true,
        None,
        LocalKind::Local,
    ));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(1)),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Decimal(
                Decimal128::parse_literal("1.25").expect("decimal literal"),
            )))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(2)),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Decimal(
                Decimal128::parse_literal("2.75").expect("decimal literal"),
            )))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(3)),
            value: Rvalue::DecimalIntrinsic(DecimalIntrinsic {
                kind: DecimalIntrinsicKind::Add,
                lhs: Operand::Copy(Place::new(LocalId(1))),
                rhs: Operand::Copy(Place::new(LocalId(2))),
                addend: None,
                rounding: Operand::Const(ConstOperand::new(ConstValue::Enum {
                    type_name: "Std::Numeric::Decimal::DecimalRoundingMode".into(),
                    variant: "TiesToEven".into(),
                    discriminant: 0,
                })),
                vectorize: Operand::Const(ConstOperand::new(ConstValue::Enum {
                    type_name: "Std::Numeric::Decimal::DecimalVectorizeHint".into(),
                    variant: "None".into(),
                    discriminant: 0,
                })),
            }),
        },
    });
    entry.terminator = Some(Terminator::Return);
    body.blocks.push(entry);

    MirFunction {
        name: "Demo::DecimalIntrinsic".into(),
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

pub(crate) fn needs_drop_function() -> MirFunction {
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
        Ty::named("Demo::NeedsDrop"),
        false,
        None,
        LocalKind::Temp,
    ));

    let mut block = BasicBlock::new(BlockId(0), None);
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(LocalId(1)),
    });
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Drop {
            place: Place::new(LocalId(1)),
            target: BlockId(0),
            unwind: None,
        },
    });
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageDead(LocalId(1)),
    });
    block.terminator = Some(Terminator::Return);
    body.blocks.push(block);

    MirFunction {
        name: "Demo::Drop".into(),
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

pub(crate) fn mmio_function() -> MirFunction {
    let mut body = MirBody::new(0, None);
    body.locals.push(LocalDecl::new(
        Some("_ret".into()),
        Ty::named("int"),
        true,
        None,
        LocalKind::Return,
    ));

    let spec = MmioOperand {
        base_address: 0x4000,
        offset: 0x08,
        width_bits: 32,
        access: MmioAccess::ReadWrite,
        endianness: MmioEndianness::Little,
        address_space: AddressSpaceId::DEFAULT,
        requires_unsafe: false,
        ty: Ty::named("int"),
        name: Some("Data".into()),
    };

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::MmioStore {
            target: spec.clone(),
            value: Operand::Const(ConstOperand::new(ConstValue::UInt(0xABCD))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(0)),
            value: Rvalue::Use(Operand::Mmio(spec)),
        },
    });
    entry.terminator = Some(Terminator::Return);
    body.blocks.push(entry);

    MirFunction {
        name: "Demo::Mmio".into(),
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

pub(crate) fn mmio_function_in_apb_space() -> MirFunction {
    let mut function = mmio_function();
    let apb_id = AddressSpaceId::from_name("apb");
    for block in &mut function.body.blocks {
        for statement in &mut block.statements {
            if let StatementKind::MmioStore { target, .. } = &mut statement.kind {
                target.address_space = apb_id;
            }
            if let StatementKind::Assign { value, .. } = &mut statement.kind {
                if let Rvalue::Use(Operand::Mmio(spec)) = value {
                    spec.address_space = apb_id;
                }
            }
        }
    }
    function
}

pub(crate) fn pending_operand_function() -> MirFunction {
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
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(LocalId(1)),
            value: Rvalue::Use(Operand::Pending(PendingOperand {
                category: ValueCategory::Pending,
                repr: "unresolved".into(),
                span: None,
                info: None,
            })),
        },
    });
    entry.terminator = Some(Terminator::Return);
    body.blocks.push(entry);

    MirFunction {
        name: "Demo::PendingOperand".into(),
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
