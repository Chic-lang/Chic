use crate::mir::borrow::BorrowCheckResult;
use crate::mir::data::{
    ArrayTy, BasicBlock, BlockId, BorrowId, BorrowKind, BorrowOperand, ConstOperand, ConstValue,
    LocalKind, Operand, ParamMode, Place, ReadOnlySpanTy, RegionVar, Rvalue, SpanTy, Statement,
    StatementKind, Terminator, Ty,
};

use super::super::util::BorrowTestHarness;

pub(super) fn readonly_span_case(release_before_borrow: bool) -> BorrowCheckResult {
    let array_ty = Ty::Array(ArrayTy::new(Box::new(Ty::named("int")), 1));
    let span_ty = Ty::ReadOnlySpan(ReadOnlySpanTy::new(Box::new(Ty::named("int"))));

    let harness = BorrowTestHarness::new("Borrow::ReadonlySpan");
    let mut case = harness.case();
    case.body_mut().arg_count = 0;

    let array = case.push_local(Some("array"), array_ty, true, LocalKind::Arg(0));
    let view = case.push_local(Some("view"), span_ty, true, LocalKind::Local);

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::Call {
        func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
            "Std::Collections::Array::AsReadOnlySpan".into(),
        ))),
        args: vec![Operand::Borrow(BorrowOperand {
            kind: BorrowKind::Shared,
            place: Place::new(array),
            region: RegionVar(0),
            span: None,
        })],
        arg_modes: vec![ParamMode::In],
        destination: Some(Place::new(view)),
        target: BlockId(1),
        unwind: None,

        dispatch: None,
    });
    case.body_mut().blocks.push(entry);

    let mut follow = BasicBlock::new(BlockId(1), None);
    if release_before_borrow {
        follow.statements.push(Statement {
            span: None,
            kind: StatementKind::StorageDead(view),
        });
    }
    follow.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(case.return_slot()),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Unit))),
        },
    });
    follow.statements.push(Statement {
        span: None,
        kind: StatementKind::Borrow {
            borrow_id: BorrowId(1),
            kind: BorrowKind::Unique,
            place: Place::new(array),
            region: RegionVar(1),
        },
    });
    follow.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(follow);

    case.run()
}

pub(super) fn readonly_span_slice_case(release_before_borrow: bool) -> BorrowCheckResult {
    let array_ty = Ty::Array(ArrayTy::new(Box::new(Ty::named("int")), 1));
    let span_ty = Ty::ReadOnlySpan(ReadOnlySpanTy::new(Box::new(Ty::named("int"))));

    let harness = BorrowTestHarness::new("Borrow::ReadonlySpanSlice");
    let mut case = harness.case();
    case.body_mut().arg_count = 1;

    let array = case.push_local(Some("array"), array_ty, true, LocalKind::Arg(0));
    let view = case.push_local(Some("view"), span_ty.clone(), true, LocalKind::Local);
    let slice = case.push_local(Some("slice"), span_ty, true, LocalKind::Local);

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::Call {
        func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
            "Std::Collections::Array::AsReadOnlySpan".into(),
        ))),
        args: vec![Operand::Borrow(BorrowOperand {
            kind: BorrowKind::Shared,
            place: Place::new(array),
            region: RegionVar(0),
            span: None,
        })],
        arg_modes: vec![ParamMode::In],
        destination: Some(Place::new(view)),
        target: BlockId(1),
        unwind: None,

        dispatch: None,
    });
    case.body_mut().blocks.push(entry);

    let mut slice_block = BasicBlock::new(BlockId(1), None);
    slice_block.terminator = Some(Terminator::Call {
        func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
            "Std::Span::SpanIntrinsics::chic_rt_span_slice_readonly".into(),
        ))),
        args: vec![Operand::Borrow(BorrowOperand {
            kind: BorrowKind::Shared,
            place: Place::new(view),
            region: RegionVar(1),
            span: None,
        })],
        arg_modes: vec![ParamMode::In],
        destination: Some(Place::new(slice)),
        target: BlockId(2),
        unwind: None,

        dispatch: None,
    });
    case.body_mut().blocks.push(slice_block);

    let mut tail = BasicBlock::new(BlockId(2), None);
    if release_before_borrow {
        tail.statements.push(Statement {
            span: None,
            kind: StatementKind::StorageDead(slice),
        });
        tail.statements.push(Statement {
            span: None,
            kind: StatementKind::StorageDead(view),
        });
    }
    tail.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(case.return_slot()),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Unit))),
        },
    });
    tail.statements.push(Statement {
        span: None,
        kind: StatementKind::Borrow {
            borrow_id: BorrowId(5),
            kind: BorrowKind::Unique,
            place: Place::new(array),
            region: RegionVar(2),
        },
    });
    tail.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(tail);

    case.run()
}

pub(super) fn span_case(release_before_borrow: bool) -> BorrowCheckResult {
    let array_ty = Ty::Array(ArrayTy::new(Box::new(Ty::named("int")), 1));
    let span_ty = Ty::Span(SpanTy::new(Box::new(Ty::named("int"))));

    let harness = BorrowTestHarness::new("Borrow::Span");
    let mut case = harness.case();
    case.body_mut().arg_count = 0;

    let array = case.push_local(Some("array"), array_ty, true, LocalKind::Arg(0));
    let view = case.push_local(Some("view"), span_ty, true, LocalKind::Local);

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::Call {
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
    case.body_mut().blocks.push(entry);

    let mut follow = BasicBlock::new(BlockId(1), None);
    if release_before_borrow {
        follow.statements.push(Statement {
            span: None,
            kind: StatementKind::StorageDead(view),
        });
    }
    follow.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(case.return_slot()),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Unit))),
        },
    });
    follow.statements.push(Statement {
        span: None,
        kind: StatementKind::Borrow {
            borrow_id: BorrowId(10),
            kind: BorrowKind::Unique,
            place: Place::new(array),
            region: RegionVar(1),
        },
    });
    follow.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(follow);

    case.run()
}

pub(super) fn span_slice_case(release_before_borrow: bool) -> BorrowCheckResult {
    let array_ty = Ty::Array(ArrayTy::new(Box::new(Ty::named("int")), 1));
    let span_ty = Ty::Span(SpanTy::new(Box::new(Ty::named("int"))));

    let harness = BorrowTestHarness::new("Borrow::SpanSlice");
    let mut case = harness.case();
    case.body_mut().arg_count = 1;

    let array = case.push_local(Some("array"), array_ty, true, LocalKind::Arg(0));
    let view = case.push_local(Some("view"), span_ty.clone(), true, LocalKind::Local);
    let slice = case.push_local(Some("slice"), span_ty, true, LocalKind::Local);

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::Call {
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
    case.body_mut().blocks.push(entry);

    let mut slice_block = BasicBlock::new(BlockId(1), None);
    slice_block.terminator = Some(Terminator::Call {
        func: Operand::Const(ConstOperand::new(ConstValue::Symbol(
            "Std::Span::SpanIntrinsics::chic_rt_span_slice_mut".into(),
        ))),
        args: vec![Operand::Borrow(BorrowOperand {
            kind: BorrowKind::Unique,
            place: Place::new(view),
            region: RegionVar(1),
            span: None,
        })],
        arg_modes: vec![ParamMode::Ref],
        destination: Some(Place::new(slice)),
        target: BlockId(2),
        unwind: None,
        dispatch: None,
    });
    case.body_mut().blocks.push(slice_block);

    let mut tail = BasicBlock::new(BlockId(2), None);
    if release_before_borrow {
        tail.statements.push(Statement {
            span: None,
            kind: StatementKind::StorageDead(slice),
        });
        tail.statements.push(Statement {
            span: None,
            kind: StatementKind::StorageDead(view),
        });
    }
    tail.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(case.return_slot()),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Unit))),
        },
    });
    tail.statements.push(Statement {
        span: None,
        kind: StatementKind::Borrow {
            borrow_id: BorrowId(12),
            kind: BorrowKind::Unique,
            place: Place::new(array),
            region: RegionVar(2),
        },
    });
    tail.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(tail);

    case.run()
}
