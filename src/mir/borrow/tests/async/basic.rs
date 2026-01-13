use super::super::util::BorrowCheckResultExt;
use super::common::{async_harness, register_layout, single_suspend_point};
use crate::frontend::diagnostics::{FileId, Span};
use crate::mir::data::{
    BasicBlock, BlockId, BorrowId, BorrowKind, ConstOperand, ConstValue, LocalKind, Operand, Place,
    RegionVar, Rvalue, Statement, StatementKind, Terminator, Ty,
};
use crate::mir::layout::{AutoTraitOverride, AutoTraitSet, AutoTraitStatus};

#[expect(
    clippy::too_many_lines,
    reason = "Explicit MIR fixture keeps borrow/shareable interactions visible."
)]
#[test]
fn await_requires_shareable_borrow() {
    let mut harness = async_harness("Borrow::AwaitShareable");
    register_layout(
        &mut harness,
        "Demo::MutableCell",
        AutoTraitSet::new(
            AutoTraitStatus::Yes,
            AutoTraitStatus::No,
            AutoTraitStatus::No,
        ),
        AutoTraitOverride {
            thread_safe: Some(true),
            shareable: Some(false),
            copy: None,
        },
        None,
        None,
    );

    let mut case = harness.case();
    case.body_mut().arg_count = 1;
    let cell = case.push_local(
        Some("cell"),
        Ty::named("Demo::MutableCell"),
        true,
        LocalKind::Local,
    );
    let future = case.push_local(Some("future"), Ty::Unknown, true, LocalKind::Local);

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(cell),
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(cell),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Unknown))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(future),
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(future),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Unit))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Borrow {
            borrow_id: BorrowId(0),
            kind: BorrowKind::Shared,
            place: Place::new(cell),
            region: RegionVar(0),
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
    resume.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(case.return_slot()),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Unit))),
        },
    });
    resume.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(resume);

    let mut drop_block = BasicBlock::new(BlockId(2), None);
    drop_block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(case.return_slot()),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Unit))),
        },
    });
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

    case.run().expect_message("Shareable");
}

#[expect(
    clippy::too_many_lines,
    reason = "Async borrow checker fixture needs explicit MIR setup."
)]
#[test]
fn detects_unique_borrow_across_await() {
    let harness = async_harness("Borrow::UniqueAwait");
    let mut case = harness.case();
    let future = case.push_local(Some("future"), Ty::named("Future"), true, LocalKind::Local);

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(future),
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(future),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(0)))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Borrow {
            borrow_id: BorrowId(1),
            kind: BorrowKind::Unique,
            place: Place::new(future),
            region: RegionVar(1),
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
    resume.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(case.return_slot()),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Unit))),
        },
    });
    resume.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(resume);

    let mut drop = BasicBlock::new(BlockId(2), None);
    drop.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(case.return_slot()),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Unit))),
        },
    });
    drop.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(drop);

    let case = case.with_async_machine(single_suspend_point(
        future,
        None,
        BlockId(0),
        BlockId(1),
        BlockId(2),
        Vec::new(),
        None,
    ));

    case.run().expect_message("cannot await");
}

#[expect(
    clippy::too_many_lines,
    reason = "Await assignment regression test needs explicit block setup."
)]
#[test]
fn await_destination_counts_as_assignment() {
    let harness = async_harness("Borrow::AwaitDestination");
    let mut case = harness.case();
    let future = case.push_local(Some("future"), Ty::named("Future"), true, LocalKind::Local);
    let value = case.push_local(Some("value"), Ty::named("int"), true, LocalKind::Local);

    let mut await_block = BasicBlock::new(BlockId(0), None);
    await_block.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(future),
    });
    await_block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(future),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(0)))),
        },
    });
    await_block.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(value),
    });
    await_block.terminator = Some(Terminator::Await {
        future: Place::new(future),
        destination: Some(Place::new(value)),
        resume: BlockId(1),
        drop: BlockId(2),
    });
    case.body_mut().blocks.push(await_block);

    let mut resume_block = BasicBlock::new(BlockId(1), None);
    resume_block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(case.return_slot()),
            value: Rvalue::Use(Operand::Copy(Place::new(value))),
        },
    });
    resume_block.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(resume_block);

    let mut drop_block = BasicBlock::new(BlockId(2), None);
    drop_block.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(case.return_slot()),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(0)))),
        },
    });
    drop_block.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(drop_block);

    let case = case.with_async_machine(single_suspend_point(
        future,
        Some(value),
        BlockId(0),
        BlockId(1),
        BlockId(2),
        Vec::new(),
        Some(Span {
            file_id: FileId::UNKNOWN,
            start: 10,
            end: 20,
        }),
    ));

    let result = case.run();
    assert!(
        result.diagnostics.is_empty(),
        "await destination should count as assignment: {:?}",
        result.diagnostics
    );
}
