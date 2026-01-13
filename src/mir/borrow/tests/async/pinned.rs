use super::super::util::BorrowCheckResultExt;
use super::common::{async_harness, register_layout, single_suspend_point};
use crate::mir::borrow::BorrowCheckResult;
use crate::mir::data::{
    ArrayTy, BasicBlock, BlockId, BorrowId, BorrowKind, BorrowOperand, ConstOperand, ConstValue,
    LocalKind, Operand, ParamMode, Place, RegionVar, Rvalue, SpanTy, Statement, StatementKind,
    Terminator, Ty,
};
use crate::mir::layout::{AutoTraitOverride, AutoTraitSet, AutoTraitStatus};

#[expect(
    clippy::too_many_lines,
    reason = "Pinned async scenario requires explicit MIR construction."
)]
#[test]
fn allows_unique_borrow_across_await_when_pinned() {
    let mut harness = async_harness("Borrow::PinnedAwait");
    register_layout(
        &mut harness,
        "Future",
        AutoTraitSet::all_yes(),
        AutoTraitOverride::default(),
        None,
        None,
    );

    let mut case = harness.case();
    let pinned = case.push_local(Some("pinned"), Ty::named("Future"), true, LocalKind::Local);
    if let Some(local) = case.body_mut().local_mut(pinned) {
        local.is_pinned = true;
    }

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(pinned),
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(pinned),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(0)))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Borrow {
            borrow_id: BorrowId(1),
            kind: BorrowKind::Unique,
            place: Place::new(pinned),
            region: RegionVar(1),
        },
    });
    entry.terminator = Some(Terminator::Await {
        future: Place::new(pinned),
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

    let case = case.with_async_machine(single_suspend_point(
        pinned,
        None,
        BlockId(0),
        BlockId(1),
        BlockId(2),
        vec![pinned],
        None,
    ));

    let result = case.run();
    assert!(
        result
            .diagnostics
            .iter()
            .all(|diag| !diag.message.contains("cannot await")),
        "expected pinned unique borrow to be allowed across await: {:?}",
        result.diagnostics
    );
}

#[expect(
    clippy::too_many_lines,
    reason = "Pinned async scenario requires explicit MIR setup."
)]
#[test]
fn detects_pinned_await_without_thread_safe() {
    let mut harness = async_harness("Borrow::PinnedNoThreadSafe");
    register_layout(
        &mut harness,
        "Demo::PinnedCell",
        AutoTraitSet::new(
            AutoTraitStatus::No,
            AutoTraitStatus::Yes,
            AutoTraitStatus::No,
        ),
        AutoTraitOverride::default(),
        Some(4_usize),
        Some(4_usize),
    );

    let mut case = harness.case();
    let pinned = case.push_local(
        Some("pinned"),
        Ty::named("Demo::PinnedCell"),
        true,
        LocalKind::Local,
    );
    if let Some(local) = case.body_mut().local_mut(pinned) {
        local.is_pinned = true;
    }

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(pinned),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(0)))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Borrow {
            borrow_id: BorrowId(0),
            kind: BorrowKind::Unique,
            place: Place::new(pinned),
            region: RegionVar(0),
        },
    });
    entry.terminator = Some(Terminator::Await {
        future: Place::new(pinned),
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

    let case = case.with_async_machine(single_suspend_point(
        pinned,
        None,
        BlockId(0),
        BlockId(1),
        BlockId(2),
        vec![pinned],
        None,
    ));

    case.run().expect_message("ThreadSafe");
}

fn span_async_case(pinned_array: bool) -> BorrowCheckResult {
    let mut harness = async_harness("Borrow::SpanAwait");
    register_layout(
        &mut harness,
        "Future",
        AutoTraitSet::all_yes(),
        AutoTraitOverride::default(),
        None,
        None,
    );

    let mut case = harness.case();
    let array = case.push_local(
        Some("array"),
        Ty::Array(ArrayTy::new(Box::new(Ty::named("int")), 1)),
        true,
        LocalKind::Local,
    );
    if pinned_array {
        if let Some(local) = case.body_mut().local_mut(array) {
            local.is_pinned = true;
        }
    }
    let view = case.push_local(
        Some("view"),
        Ty::Span(SpanTy::new(Box::new(Ty::named("int")))),
        true,
        LocalKind::Local,
    );
    let future = case.push_local(Some("future"), Ty::named("Future"), true, LocalKind::Local);

    let mut init = BasicBlock::new(BlockId(0), None);
    init.terminator = Some(Terminator::Call {
        func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
            "Std::Collections::Array::AsSpan".into(),
        ))),
        args: vec![Operand::Borrow(BorrowOperand {
            kind: BorrowKind::Unique,
            place: Place::new(array),
            region: RegionVar(0),
            span: None,
        })],
        arg_modes: vec![ParamMode::Ref],
        destination: Some(Place::new(view)),
        target: BlockId(1),
        unwind: None,
        dispatch: None,
    });
    case.body_mut().blocks.push(init);

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

    let mut drop = BasicBlock::new(BlockId(3), None);
    drop.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(drop);

    let pinned = if pinned_array {
        vec![array]
    } else {
        Vec::new()
    };
    let machine = single_suspend_point(
        future,
        None,
        BlockId(1),
        BlockId(2),
        BlockId(3),
        pinned,
        None,
    );
    let case = case.with_async_machine(machine);

    case.run()
}

#[test]
fn span_unique_borrow_requires_pin_across_await() {
    span_async_case(false).expect_message("cannot await");
}

#[test]
fn span_unique_borrow_allows_pin_across_await() {
    let result = span_async_case(true);
    assert!(
        result
            .diagnostics
            .iter()
            .all(|diag| !diag.message.contains("cannot await")),
        "expected pinned span borrow to allow await: {:?}",
        result.diagnostics
    );
}
