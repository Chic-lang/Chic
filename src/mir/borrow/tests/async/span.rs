use super::super::util::BorrowCheckResultExt;
use super::common::{async_harness, single_suspend_point};
use crate::mir::data::{
    BasicBlock, BlockId, BorrowKind, BorrowOperand, ConstOperand, ConstValue, LocalKind, Operand,
    ParamMode, Place, RegionVar, Rvalue, SpanTy, Statement, StatementKind, Terminator, Ty,
};

#[test]
fn stack_alloc_span_cannot_cross_await() {
    let harness = async_harness("Borrow::SpanStackAllocAwait");
    let mut case = harness.case();
    let element_ty = Ty::named("int");
    let span_local = case.push_local(
        Some("buffer"),
        Ty::Span(SpanTy::new(Box::new(element_ty.clone()))),
        true,
        LocalKind::Local,
    );
    let len_local = case.push_local(Some("len"), Ty::named("int"), true, LocalKind::Local);
    let future = case.push_local(Some("future"), Ty::named("Future"), true, LocalKind::Local);

    let mut await_block = BasicBlock::new(BlockId(0), None);
    await_block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(len_local),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(4)))),
        },
    });
    await_block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(span_local),
            value: Rvalue::SpanStackAlloc {
                element: element_ty.clone(),
                length: Operand::Copy(Place::new(len_local)),
                source: None,
            },
        },
    });
    await_block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(future),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(0)))),
        },
    });
    await_block.terminator = Some(Terminator::Await {
        future: Place::new(future),
        destination: None,
        resume: BlockId(1),
        drop: BlockId(2),
    });
    case.body_mut().blocks.push(await_block);

    let mut resume = BasicBlock::new(BlockId(1), None);
    resume.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(resume);

    let mut drop_block = BasicBlock::new(BlockId(2), None);
    drop_block.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(drop_block);

    let case = case.with_async_machine(single_suspend_point(
        future,
        None,
        BlockId(0),
        BlockId(1),
        BlockId(2),
        Vec::new(),
        None,
    ));

    case.run().expect_message("stack-allocated span");
}

#[test]
fn stack_alloc_slice_cannot_cross_await() {
    let harness = async_harness("Borrow::SpanStackAllocSliceAwait");
    let mut case = harness.case();
    let element_ty = Ty::named("int");
    let span_local = case.push_local(
        Some("buffer"),
        Ty::Span(SpanTy::new(Box::new(element_ty.clone()))),
        true,
        LocalKind::Local,
    );
    let slice_local = case.push_local(
        Some("window"),
        Ty::Span(SpanTy::new(Box::new(element_ty.clone()))),
        true,
        LocalKind::Local,
    );
    let len_local = case.push_local(Some("len"), Ty::named("int"), true, LocalKind::Local);
    let future = case.push_local(Some("future"), Ty::named("Future"), true, LocalKind::Local);

    let mut call_block = BasicBlock::new(BlockId(0), None);
    call_block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(len_local),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(4)))),
        },
    });
    call_block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(span_local),
            value: Rvalue::SpanStackAlloc {
                element: element_ty.clone(),
                length: Operand::Copy(Place::new(len_local)),
                source: None,
            },
        },
    });
    call_block.terminator = Some(Terminator::Call {
        func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
            "Std::Span::SpanIntrinsics::chic_rt_span_slice_mut".into(),
        ))),
        args: vec![Operand::Borrow(BorrowOperand {
            kind: BorrowKind::Unique,
            place: Place::new(span_local),
            region: RegionVar(0),
            span: None,
        })],
        arg_modes: vec![ParamMode::Ref],
        destination: Some(Place::new(slice_local)),
        target: BlockId(1),
        unwind: None,
        dispatch: None,
    });
    case.body_mut().blocks.push(call_block);

    let mut await_block = BasicBlock::new(BlockId(1), None);
    await_block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(future),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(0)))),
        },
    });
    await_block.terminator = Some(Terminator::Await {
        future: Place::new(future),
        destination: None,
        resume: BlockId(2),
        drop: BlockId(3),
    });
    case.body_mut().blocks.push(await_block);

    let mut resume = BasicBlock::new(BlockId(2), None);
    resume.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(resume);

    let mut drop_block = BasicBlock::new(BlockId(3), None);
    drop_block.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(drop_block);

    let case = case.with_async_machine(single_suspend_point(
        future,
        None,
        BlockId(1),
        BlockId(2),
        BlockId(3),
        Vec::new(),
        None,
    ));

    case.run().expect_message("stack-allocated span");
}

#[test]
fn stack_alloc_span_dropped_before_await_is_allowed() {
    let harness = async_harness("Borrow::SpanStackAllocReleased");
    let mut case = harness.case();
    let element_ty = Ty::named("int");
    let span_local = case.push_local(
        Some("scratch"),
        Ty::Span(SpanTy::new(Box::new(element_ty.clone()))),
        true,
        LocalKind::Local,
    );
    let future = case.push_local(Some("future"), Ty::named("Future"), true, LocalKind::Local);

    let mut await_block = BasicBlock::new(BlockId(0), None);
    await_block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(span_local),
            value: Rvalue::SpanStackAlloc {
                element: element_ty,
                length: Operand::Const(ConstOperand::new(ConstValue::Int(1))),
                source: None,
            },
        },
    });
    await_block.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageDead(span_local),
    });
    await_block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(future),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(0)))),
        },
    });
    await_block.terminator = Some(Terminator::Await {
        future: Place::new(future),
        destination: None,
        resume: BlockId(1),
        drop: BlockId(2),
    });
    case.body_mut().blocks.push(await_block);

    let mut resume = BasicBlock::new(BlockId(1), None);
    resume.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(resume);

    let mut drop_block = BasicBlock::new(BlockId(2), None);
    drop_block.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(drop_block);

    let case = case.with_async_machine(single_suspend_point(
        future,
        None,
        BlockId(0),
        BlockId(1),
        BlockId(2),
        Vec::new(),
        None,
    ));

    let result = case.run();
    assert!(
        result.diagnostics.is_empty(),
        "expected stackalloc to be released before await, got {:?}",
        result.diagnostics
    );
}
