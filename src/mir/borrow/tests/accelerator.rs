use super::util::{BorrowCheckResultExt, BorrowTestHarness};
use crate::mir::data::{
    AcceleratorCopyKind, BasicBlock, BlockId, ConstOperand, ConstValue, LocalKind, Operand, Place,
    Rvalue, Statement, StatementKind, Terminator, Ty,
};

#[test]
fn enqueued_copy_holds_borrows_until_wait() {
    let harness = BorrowTestHarness::new("Borrow::EnqueueCopyBorrow");
    let mut case = harness.case();
    let stream = case.push_local(
        Some("stream"),
        Ty::named("Std::Accelerator::Stream<Std::Accelerator::Host>"),
        true,
        LocalKind::Arg(0),
    );
    case.body_mut().arg_count = 1;
    let buffer = case.push_local(Some("buffer"), Ty::named("int"), true, LocalKind::Local);
    let event = case.push_local(
        Some("event"),
        Ty::named("Std::Accelerator::Event"),
        true,
        LocalKind::Local,
    );
    let tmp = case.push_local(Some("tmp"), Ty::named("int"), true, LocalKind::Temp);

    let mut block = BasicBlock::new(BlockId(0), None);
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(buffer),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(1)))),
        },
    });
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::EnqueueCopy {
            stream: Place::new(stream),
            dst: Place::new(buffer),
            src: Place::new(buffer),
            bytes: Operand::Const(ConstOperand::new(ConstValue::Int(4))),
            kind: AcceleratorCopyKind::HostToDevice,
            completion: Some(Place::new(event)),
        },
    });
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(tmp),
            value: Rvalue::Use(Operand::Move(Place::new(buffer))),
        },
    });
    block.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(block);

    case.run()
        .expect_message("cannot move `buffer` while Shared borrow is active");
}

#[test]
fn wait_event_releases_stream_borrows() {
    let harness = BorrowTestHarness::new("Borrow::EnqueueCopyWait");
    let mut case = harness.case();
    let stream = case.push_local(
        Some("stream"),
        Ty::named("Std::Accelerator::Stream<Std::Accelerator::Host>"),
        true,
        LocalKind::Arg(0),
    );
    case.body_mut().arg_count = 1;
    let buffer = case.push_local(Some("buffer"), Ty::named("int"), true, LocalKind::Local);
    let event = case.push_local(
        Some("event"),
        Ty::named("Std::Accelerator::Event"),
        true,
        LocalKind::Local,
    );
    let tmp = case.push_local(Some("tmp"), Ty::named("int"), true, LocalKind::Temp);

    let mut block = BasicBlock::new(BlockId(0), None);
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(buffer),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(1)))),
        },
    });
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::EnqueueCopy {
            stream: Place::new(stream),
            dst: Place::new(buffer),
            src: Place::new(buffer),
            bytes: Operand::Const(ConstOperand::new(ConstValue::Int(4))),
            kind: AcceleratorCopyKind::HostToDevice,
            completion: Some(Place::new(event)),
        },
    });
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::WaitEvent {
            event: Place::new(event),
            stream: Some(Place::new(stream)),
        },
    });
    block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(tmp),
            value: Rvalue::Use(Operand::Move(Place::new(buffer))),
        },
    });
    block.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(block);

    let result = case.run();
    assert!(
        result
            .diagnostics
            .iter()
            .all(|diag| !diag.message.contains("cannot move `buffer`")),
        "expected wait to release borrow, got {:?}",
        result.diagnostics
    );
}
