use chic::frontend::attributes::OptimizationHints;
use chic::mir::{
    Abi, AcceleratorBuilder, AcceleratorCopyKind, AsyncFramePolicy, AsyncStateMachine,
    AsyncSuspendPoint, BasicBlock, BlockId, ConstOperand, ConstValue, FnSig, FunctionKind,
    LocalDecl, LocalId, LocalKind, MirBody, MirFunction, Operand, Place, Rvalue, Statement,
    StatementKind, Terminator, Ty, borrow_check_function,
};

fn async_overlap(pinned_streams: bool) -> MirFunction {
    let mut body = MirBody::new(2, None);
    let ret = LocalDecl::new(
        Some("_ret".into()),
        Ty::Unit,
        false,
        None,
        LocalKind::Return,
    );
    let mut stream_a = LocalDecl::new(
        Some("stream_a".into()),
        Ty::named("Std::Accelerator::Stream<Std::Accelerator::Host>"),
        true,
        None,
        LocalKind::Arg(0),
    );
    let mut stream_b = LocalDecl::new(
        Some("stream_b".into()),
        Ty::named("Std::Accelerator::Stream<Std::Accelerator::PinnedHost>"),
        true,
        None,
        LocalKind::Arg(1),
    );
    if pinned_streams {
        stream_a.is_pinned = true;
        stream_b.is_pinned = true;
    }
    let event_a = LocalDecl::new(
        Some("event_a".into()),
        Ty::named("Std::Accelerator::Event"),
        true,
        None,
        LocalKind::Local,
    );
    let event_b = LocalDecl::new(
        Some("event_b".into()),
        Ty::named("Std::Accelerator::Event"),
        true,
        None,
        LocalKind::Local,
    );
    let buffer_a = LocalDecl::new(
        Some("buffer_a".into()),
        Ty::named("Std::Accelerator::Host"),
        true,
        None,
        LocalKind::Local,
    );
    let buffer_b = LocalDecl::new(
        Some("buffer_b".into()),
        Ty::named("Std::Accelerator::PinnedHost"),
        true,
        None,
        LocalKind::Local,
    );
    let future = LocalDecl::new(
        Some("future".into()),
        Ty::named("Std::Async::Future"),
        true,
        None,
        LocalKind::Local,
    );

    body.locals = vec![
        ret, stream_a, stream_b, event_a, event_b, buffer_a, buffer_b, future,
    ];

    let stream_a_id = LocalId(1);
    let stream_b_id = LocalId(2);
    let event_a_id = LocalId(3);
    let event_b_id = LocalId(4);
    let buffer_a_id = LocalId(5);
    let buffer_b_id = LocalId(6);
    let future_id = LocalId(7);

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(buffer_a_id),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(0)))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(buffer_b_id),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(0)))),
        },
    });
    {
        let mut accel = AcceleratorBuilder::new(&mut body);
        accel.enqueue_copy(
            &mut entry,
            Place::new(stream_a_id),
            Place::new(buffer_a_id),
            Place::new(buffer_a_id),
            Operand::Const(ConstOperand::new(ConstValue::Int(16))),
            AcceleratorCopyKind::HostToDevice,
            Some(Place::new(event_a_id)),
            None,
        );
        accel.enqueue_kernel(
            &mut entry,
            Place::new(stream_b_id),
            Operand::Const(ConstOperand::new(ConstValue::Symbol("Demo::Kernel".into()))),
            vec![
                Operand::Const(ConstOperand::new(ConstValue::Int(1))),
                Operand::Copy(Place::new(buffer_b_id)),
            ],
            Some(Place::new(event_b_id)),
            None,
        );
    }
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(future_id),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(0)))),
        },
    });
    entry.terminator = Some(Terminator::Await {
        future: Place::new(future_id),
        destination: None,
        resume: BlockId(1),
        drop: BlockId(2),
    });

    let mut resume = BasicBlock::new(BlockId(1), None);
    resume.statements.push(Statement {
        span: None,
        kind: StatementKind::WaitEvent {
            event: Place::new(event_a_id),
            stream: Some(Place::new(stream_a_id)),
        },
    });
    resume.statements.push(Statement {
        span: None,
        kind: StatementKind::WaitEvent {
            event: Place::new(event_b_id),
            stream: Some(Place::new(stream_b_id)),
        },
    });
    resume.terminator = Some(Terminator::Return);

    let mut drop = BasicBlock::new(BlockId(2), None);
    drop.terminator = Some(Terminator::Return);

    body.blocks = vec![entry, resume, drop];
    body.async_machine = Some(AsyncStateMachine {
        suspend_points: vec![AsyncSuspendPoint {
            id: 0,
            await_block: BlockId(0),
            resume_block: BlockId(1),
            drop_block: BlockId(2),
            future: future_id,
            destination: None,
            span: None,
        }],
        pinned_locals: if pinned_streams {
            vec![stream_a_id, stream_b_id]
        } else {
            Vec::new()
        },
        cross_locals: if pinned_streams {
            Vec::new()
        } else {
            vec![stream_a_id, stream_b_id]
        },
        frame_fields: Vec::new(),
        result_local: None,
        result_ty: None,
        context_local: None,
        policy: AsyncFramePolicy::default(),
    });

    MirFunction {
        name: "Demo::AsyncAccelerator".into(),
        kind: FunctionKind::Function,
        signature: FnSig {
            params: vec![
                Ty::named("Std::Accelerator::Stream<Std::Accelerator::Host>"),
                Ty::named("Std::Accelerator::Stream<Std::Accelerator::PinnedHost>"),
            ],
            ret: Ty::Unit,
            abi: Abi::Chic,
            effects: Vec::new(),

            lends_to_return: None,
            variadic: false,
        },
        body,
        is_async: true,
        async_result: None,
        is_generator: false,
        span: None,
        optimization_hints: OptimizationHints::default(),
        extern_spec: None,
        is_weak: false,
        is_weak_import: false,
    }
}

#[test]
fn overlapping_streams_wait_after_suspend() {
    let function = async_overlap(true);
    let result = borrow_check_function(&function);
    assert!(
        result.is_ok(),
        "expected borrow check to pass: {:?}",
        result.diagnostics
    );
    assert_eq!(function.body.stream_metadata.len(), 2);
    let waits = &function.body.blocks[1].statements;
    let wait_count = waits
        .iter()
        .filter(|stmt| matches!(stmt.kind, StatementKind::WaitEvent { .. }))
        .count();
    assert_eq!(wait_count, 2);
    assert!(matches!(
        waits.get(0).map(|stmt| &stmt.kind),
        Some(StatementKind::WaitEvent { event, .. }) if event.local == LocalId(3)
    ));
    assert!(matches!(
        waits.get(1).map(|stmt| &stmt.kind),
        Some(StatementKind::WaitEvent { event, .. }) if event.local == LocalId(4)
    ));
}

#[test]
fn unpinned_streams_fail_async_capture() {
    let function = async_overlap(false);
    let result = borrow_check_function(&function);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("must be pinned")),
        "expected pinned diagnostic, got {:?}",
        result.diagnostics
    );
}
