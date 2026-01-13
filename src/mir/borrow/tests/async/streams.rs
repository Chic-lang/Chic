use super::super::util::BorrowCheckResultExt;
use super::common::{async_harness, single_suspend_point};
use crate::mir::data::{
    BasicBlock, BlockId, ConstOperand, ConstValue, LocalKind, Operand, Place, Rvalue, Statement,
    StatementKind, Terminator, Ty,
};

#[test]
fn rejects_unpinned_stream_across_await() {
    let harness = async_harness("Borrow::StreamAwaitUnpinned");
    let mut case = harness.case();
    let stream = case.push_local(
        Some("stream"),
        Ty::named("Std::Accelerator::Stream<Std::Accelerator::Host>"),
        true,
        LocalKind::Arg(0),
    );
    case.body_mut().arg_count = 1;
    let future = case.push_local(
        Some("future"),
        Ty::named("Std::Async::Future"),
        true,
        LocalKind::Local,
    );

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(future),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(0)))),
        },
    });
    entry.terminator = Some(Terminator::Await {
        future: Place::new(future),
        destination: None,
        resume: BlockId(1),
        drop: BlockId(2),
    });
    case.body_mut().blocks.push(entry);

    let mut resume = BasicBlock::new(BlockId(1), None);
    resume.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(resume);

    let mut drop = BasicBlock::new(BlockId(2), None);
    drop.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(drop);

    let mut machine = single_suspend_point(
        future,
        None,
        BlockId(0),
        BlockId(1),
        BlockId(2),
        Vec::new(),
        None,
    );
    machine.cross_locals.push(stream);

    let case = case.with_async_machine(machine);
    case.run().expect_message("must be pinned");
}

#[test]
fn allows_pinned_stream_across_await() {
    let harness = async_harness("Borrow::StreamAwaitPinned");
    let mut case = harness.case();
    let stream = case.push_local(
        Some("stream"),
        Ty::named("Std::Accelerator::Stream<Std::Accelerator::Host>"),
        true,
        LocalKind::Arg(0),
    );
    if let Some(local) = case.body_mut().local_mut(stream) {
        local.is_pinned = true;
    }
    case.body_mut().arg_count = 1;
    let future = case.push_local(
        Some("future"),
        Ty::named("Std::Async::Future"),
        true,
        LocalKind::Local,
    );

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(future),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(0)))),
        },
    });
    entry.terminator = Some(Terminator::Await {
        future: Place::new(future),
        destination: None,
        resume: BlockId(1),
        drop: BlockId(2),
    });
    case.body_mut().blocks.push(entry);

    let mut resume = BasicBlock::new(BlockId(1), None);
    resume.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(resume);

    let mut drop = BasicBlock::new(BlockId(2), None);
    drop.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(drop);

    let mut machine = single_suspend_point(
        future,
        None,
        BlockId(0),
        BlockId(1),
        BlockId(2),
        vec![stream],
        None,
    );
    machine.cross_locals.push(stream);

    let case = case.with_async_machine(machine);
    let result = case.run();
    assert!(
        result
            .diagnostics
            .iter()
            .all(|diag| !diag.message.contains("pinned")),
        "unexpected pinned diagnostic: {:?}",
        result.diagnostics
    );
}
