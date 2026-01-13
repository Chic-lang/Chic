use super::util::{BorrowCheckResultExt, BorrowTestHarness};
use crate::mir::data::{
    BasicBlock, BlockId, BorrowId, BorrowKind, ConstOperand, ConstValue, LocalKind, Operand, Place,
    RegionVar, Rvalue, Statement, StatementKind, Terminator, Ty,
};

fn moves_harness() -> BorrowTestHarness {
    BorrowTestHarness::new("Borrow::Moves")
}

#[expect(
    clippy::too_many_lines,
    reason = "Explicit MIR fixture keeps borrow/move ordering visible."
)]
#[test]
fn detects_move_while_shared_borrow_active() {
    let harness = moves_harness();
    let mut case = harness.case();
    let value = case.push_local(Some("x"), Ty::named("int"), true, LocalKind::Local);

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(value),
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(value),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(5)))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Borrow {
            borrow_id: BorrowId(0),
            kind: BorrowKind::Shared,
            place: Place::new(value),
            region: RegionVar(0),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(case.return_slot()),
            value: Rvalue::Use(Operand::Move(Place::new(value))),
        },
    });
    entry.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(entry);

    let result = case.run();
    result.expect_message("cannot move");
}

#[test]
fn move_while_borrowed_in_switch_int_reports_error() {
    let harness = moves_harness();
    let mut case = harness.case();
    let discr = case.push_local(Some("x"), Ty::named("int"), true, LocalKind::Local);

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(discr),
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(discr),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(1)))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Borrow {
            borrow_id: BorrowId(0),
            kind: BorrowKind::Shared,
            place: Place::new(discr),
            region: RegionVar(0),
        },
    });
    entry.terminator = Some(Terminator::SwitchInt {
        discr: Operand::Move(Place::new(discr)),
        targets: vec![(0, BlockId(1))],
        otherwise: BlockId(2),
    });
    case.body_mut().blocks.push(entry);

    let mut on_zero = BasicBlock::new(BlockId(1), None);
    on_zero.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(on_zero);

    let mut fallback = BasicBlock::new(BlockId(2), None);
    fallback.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(fallback);

    let result = case.run();
    result.expect_message("cannot move");
}

#[test]
fn detects_use_after_move() {
    let harness = moves_harness();
    let mut case = harness.case();
    let value = case.push_local(Some("value"), Ty::named("int"), true, LocalKind::Local);
    let tmp = case.push_local(Some("tmp"), Ty::named("int"), true, LocalKind::Local);

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(value),
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(value),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(42)))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(case.return_slot()),
            value: Rvalue::Use(Operand::Move(Place::new(value))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(tmp),
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(tmp),
            value: Rvalue::Use(Operand::Copy(Place::new(value))),
        },
    });
    entry.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(entry);

    let result = case.run();
    result.expect_message("use of `value`");
}

#[test]
fn rejects_move_of_pinned_local() {
    let harness = BorrowTestHarness::new("Borrow::PinnedMove").with_return_type(Ty::named("int"));
    let mut case = harness.case();
    let pinned = case.push_local(Some("pinned"), Ty::named("int"), true, LocalKind::Local);
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
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(5)))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(case.return_slot()),
            value: Rvalue::Use(Operand::Move(Place::new(pinned))),
        },
    });
    entry.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(entry);

    let result = case.run();
    result.expect_message("pinned binding");
}
