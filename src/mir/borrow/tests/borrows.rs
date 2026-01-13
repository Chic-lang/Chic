mod fixtures;

use super::util::{BorrowCheckResultExt, BorrowTestHarness};
use crate::mir::data::{
    BasicBlock, BlockId, BorrowId, BorrowKind, ConstOperand, ConstValue, LocalKind, Operand, Place,
    RegionVar, Rvalue, Statement, StatementKind, Terminator, Ty,
};
use fixtures::{readonly_span_case, readonly_span_slice_case, span_case, span_slice_case};

#[test]
fn readonly_span_borrow_blocks_unique_array_borrow() {
    readonly_span_case(false).expect_message("conflicting borrow");
}

#[test]
fn readonly_span_release_allows_unique_array_borrow() {
    let result = readonly_span_case(true);
    assert!(
        result.diagnostics.is_empty(),
        "expected readonly span views to release before unique borrow, got {:?}",
        result.diagnostics
    );
}

#[test]
fn readonly_span_slice_propagates_borrow() {
    readonly_span_slice_case(false).expect_message("conflicting borrow");
}

#[test]
fn readonly_span_slice_release_allows_unique_borrow() {
    let result = readonly_span_slice_case(true);
    assert!(
        result.diagnostics.is_empty(),
        "expected dropping slices to allow unique borrow, got {:?}",
        result.diagnostics
    );
}

#[test]
fn span_borrow_blocks_unique_array_borrow() {
    span_case(false).expect_message("conflicting borrow");
}

#[test]
fn span_release_allows_unique_array_borrow() {
    let result = span_case(true);
    assert!(
        result.diagnostics.is_empty(),
        "expected mutable span views to release before unique borrow, got {:?}",
        result.diagnostics
    );
}

#[test]
fn span_slice_propagates_borrow() {
    span_slice_case(false).expect_message("conflicting borrow");
}

#[test]
fn span_slice_release_allows_unique_borrow() {
    let result = span_slice_case(true);
    assert!(
        result.diagnostics.is_empty(),
        "expected releasing span slices to allow unique borrow, got {:?}",
        result.diagnostics
    );
}

fn unique_conflict_case() -> BorrowTestHarness {
    BorrowTestHarness::new("Borrow::Conflicts")
}

#[expect(
    clippy::too_many_lines,
    reason = "Conflicting reborrow case requires explicit MIR scaffolding."
)]
#[test]
fn detects_conflicting_unique_reborrow() {
    let harness = unique_conflict_case();
    let mut case = harness.case();
    let value = case.push_local(Some("value"), Ty::named("int"), true, LocalKind::Local);
    let tmp1 = case.push_local(Some("tmp"), Ty::named("int"), true, LocalKind::Local);
    let tmp2 = case.push_local(Some("tmp"), Ty::named("int"), true, LocalKind::Local);

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(value),
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(value),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(1)))),
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
        kind: StatementKind::Borrow {
            borrow_id: BorrowId(1),
            kind: BorrowKind::Unique,
            place: Place::new(value),
            region: RegionVar(1),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Borrow {
            borrow_id: BorrowId(2),
            kind: BorrowKind::Unique,
            place: Place::new(value),
            region: RegionVar(2),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(tmp1),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(2)))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(tmp2),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(3)))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(case.return_slot()),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Unit))),
        },
    });
    entry.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(entry);

    case.run().expect_message("conflicting borrow");
}

#[test]
fn detects_conflicting_unique_borrows() {
    let harness = unique_conflict_case();
    let mut case = harness.case();
    let value = case.push_local(Some("value"), Ty::named("int"), true, LocalKind::Local);

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
            kind: BorrowKind::Unique,
            place: Place::new(value),
            region: RegionVar(0),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Borrow {
            borrow_id: BorrowId(1),
            kind: BorrowKind::Unique,
            place: Place::new(value),
            region: RegionVar(1),
        },
    });
    entry.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(entry);

    case.run().expect_message("conflicting borrow");
}

#[expect(
    clippy::too_many_lines,
    reason = "Shared reborrow scenario uses explicit MIR fixture for clarity."
)]
#[test]
fn allows_shared_reborrow() {
    let harness = BorrowTestHarness::new("Borrow::Shared");
    let mut case = harness.case();
    let value = case.push_local(Some("value"), Ty::named("int"), true, LocalKind::Local);

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::StorageLive(value),
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(value),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(0)))),
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
        kind: StatementKind::Borrow {
            borrow_id: BorrowId(1),
            kind: BorrowKind::Shared,
            place: Place::new(value),
            region: RegionVar(1),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(case.return_slot()),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Unit))),
        },
    });
    entry.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(entry);

    let result = case.run();
    assert!(
        result.diagnostics.is_empty(),
        "shared reborrows should succeed: {:?}",
        result.diagnostics
    );
}
