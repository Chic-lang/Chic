use super::util::{BorrowCheckResultExt, BorrowTestHarness};
use crate::frontend::diagnostics::Severity;
use crate::frontend::parser::parse_module;
use crate::mir::ProjectionElem;
use crate::mir::borrow::borrow_check_function_with_layouts;
use crate::mir::data::{
    BasicBlock, BlockId, BorrowId, BorrowKind, ConstOperand, ConstValue, LocalKind, Operand,
    ParamMode, Place, RegionVar, Rvalue, Statement, StatementKind, Terminator, Ty,
};
use crate::mir::layout::{
    AutoTraitOverride, AutoTraitSet, FieldLayout, StructLayout, TypeLayout, TypeRepr,
};
use crate::mir::lower_module;

#[test]
fn detects_unassigned_out_parameter() {
    let harness = BorrowTestHarness::new("Borrow::OutParam");
    let mut case = harness.case();
    case.body_mut().arg_count = 1;
    case.push_local_with_mode(
        Some("out"),
        Ty::named("int"),
        true,
        LocalKind::Arg(0),
        Some(ParamMode::Out),
    );

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(entry);

    let result = case.run();
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("was not assigned")),
        "expected diagnostic about out parameter assignment, got {:?}",
        result.diagnostics
    );
    assert!(
        result
            .diagnostics
            .iter()
            .all(|diag| diag.severity == Severity::Error)
    );
}

#[test]
fn constructor_self_param_is_considered_initialized() {
    let harness = BorrowTestHarness::new("Borrow::CtorSelf").mark_constructor();
    let mut case = harness.case();
    case.body_mut().arg_count = 1;
    case.push_local_with_mode(
        Some("self"),
        Ty::named("Widget"),
        true,
        LocalKind::Arg(0),
        Some(ParamMode::Out),
    );
    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(entry);

    let result = case.run();
    assert!(
        result.diagnostics.is_empty(),
        "expected constructor self parameter to be treated as initialized, got {:?}",
        result.diagnostics
    );
}

#[test]
fn rejects_assignment_to_in_parameter() {
    let harness = BorrowTestHarness::new("Borrow::InParam");
    let mut case = harness.case();
    case.body_mut().arg_count = 1;
    let param = case.push_local_with_mode(
        Some("param"),
        Ty::named("int"),
        false,
        LocalKind::Arg(0),
        Some(ParamMode::In),
    );

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(param),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(1)))),
        },
    });
    entry.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(entry);

    case.run().expect_message("cannot assign to `in` parameter");
}

#[test]
fn allows_out_parameter_assigned_multiple_times() {
    let harness = BorrowTestHarness::new("Borrow::OutParamReassign");
    let mut case = harness.case();
    case.body_mut().arg_count = 1;
    let out = case.push_local_with_mode(
        Some("out"),
        Ty::named("int"),
        true,
        LocalKind::Arg(0),
        Some(ParamMode::Out),
    );

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(out),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(1)))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(out),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(2)))),
        },
    });
    entry.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(entry);

    let result = case.run();
    assert!(
        result.diagnostics.is_empty(),
        "expected out parameter to allow reassignment, got {:?}",
        result.diagnostics
    );
}

#[test]
fn zero_init_satisfies_out_parameter_assignment() {
    let harness = BorrowTestHarness::new("Borrow::ZeroInitOut");
    let mut case = harness.case();
    case.body_mut().arg_count = 1;
    let out = case.push_local_with_mode(
        Some("out"),
        Ty::named("int"),
        true,
        LocalKind::Arg(0),
        Some(ParamMode::Out),
    );

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::ZeroInit {
            place: Place::new(out),
        },
    });
    entry.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(entry);

    let result = case.run();
    assert!(
        result.diagnostics.is_empty(),
        "zero init should satisfy out parameter assignment, got {:?}",
        result.diagnostics
    );
}

#[test]
fn detects_use_of_uninitialized_local() {
    let harness = BorrowTestHarness::new("Borrow::Uninit").with_return_type(Ty::named("int"));
    let mut case = harness.case();
    let value = case.push_local(Some("value"), Ty::named("int"), true, LocalKind::Local);

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(case.return_slot()),
            value: Rvalue::Use(Operand::Copy(Place::new(value))),
        },
    });
    entry.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(entry);

    case.run().expect_message("before it is assigned");
}

#[test]
fn immutable_reassignment_uses_lcl0002() {
    let harness = BorrowTestHarness::new("Borrow::ImmutableReassign");
    let mut case = harness.case();
    let value = case.push_local(Some("value"), Ty::named("int"), false, LocalKind::Local);

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(value),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(1)))),
        },
    });
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(value),
            value: Rvalue::Use(Operand::Const(ConstOperand::new(ConstValue::Int(2)))),
        },
    });
    entry.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(entry);

    let result = case.run();
    let diag = result
        .diagnostics
        .iter()
        .find(|diag| {
            diag.code
                .as_ref()
                .is_some_and(|code| code.code == "LCL0002")
        })
        .expect("expected LCL0002 for immutable reassignment");
    assert!(
        diag.message.contains("immutable binding `value`"),
        "diagnostic should mention immutable binding name: {diag:?}"
    );
}

#[test]
fn mutable_borrow_of_immutable_binding_produces_lcl0002() {
    let harness = BorrowTestHarness::new("Borrow::ImmutableBorrow");
    let mut case = harness.case();
    let value = case.push_local(Some("value"), Ty::named("int"), false, LocalKind::Local);

    let mut entry = BasicBlock::new(BlockId(0), None);
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
            kind: BorrowKind::Unique,
            place: Place::new(value),
            region: RegionVar(0),
        },
    });
    entry.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(entry);

    let result = case.run();
    let diag = result
        .diagnostics
        .iter()
        .find(|diag| {
            diag.code
                .as_ref()
                .is_some_and(|code| code.code == "LCL0002")
        })
        .unwrap_or_else(|| {
            panic!(
                "expected LCL0002 for mutable borrow of immutable binding, got {:?}",
                result.diagnostics
            )
        });
    assert!(
        diag.message.contains("immutable binding `value`"),
        "diagnostic should mention immutable binding name: {diag:?}"
    );
}

fn borrow_check_lowered(function_name_suffix: &str, source: &str) -> Vec<String> {
    let Ok(parsed) = parse_module(source) else {
        panic!("parse failed");
    };
    let lowering = lower_module(&parsed.module);
    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with(function_name_suffix))
        .unwrap_or_else(|| panic!("missing {function_name_suffix} function"));
    let result = borrow_check_function_with_layouts(func, &lowering.module.type_layouts);
    result
        .diagnostics
        .iter()
        .map(|diag| diag.message.clone())
        .collect()
}

#[test]
fn reports_union_view_mismatch_on_read() {
    let diagnostics = borrow_check_lowered(
        "::Read",
        r"
namespace Graphics;

public union Pixel
{
public int Value;
public float AsFloat;
}

public float Read()
{
var p = new Pixel();
p.Value = 42;
return p.AsFloat;
}
",
    );
    assert!(
        diagnostics
            .iter()
            .any(|message| message.contains("cannot read union view")),
        "expected union view mismatch diagnostic, got {:?}",
        diagnostics
    );
}

#[test]
fn reports_union_readonly_assignment() {
    let diagnostics = borrow_check_lowered(
        "::Write",
        r"
namespace Graphics;

public union Pixel
{
public readonly int Value;
public int Other;
}

public void Write()
{
var p = new Pixel();
p.Value = 1;
}
",
    );
    assert!(
        diagnostics
            .iter()
            .any(|message| message.contains("readonly union view")),
        "expected readonly assignment diagnostic, got {:?}",
        diagnostics
    );
}

#[test]
fn reports_union_inactive_read() {
    let diagnostics = borrow_check_lowered(
        "::ReadWithoutInit",
        r"
namespace Graphics;

public union Pixel
{
public int Value;
public float AsFloat;
}

public float ReadWithoutInit()
{
var p = new Pixel();
return p.AsFloat;
}
",
    );
    assert!(
        diagnostics
            .iter()
            .any(|message| message.contains("is not active")),
        "expected union inactive diagnostic, got {:?}",
        diagnostics
    );
}

#[test]
fn reports_move_of_owner_field_with_live_view_dependency() {
    let mut harness = BorrowTestHarness::new("Borrow::ViewMove").with_return_type(Ty::String);
    harness.layouts_mut().types.insert(
        "Buffer".into(),
        TypeLayout::Struct(StructLayout {
            name: "Buffer".into(),
            repr: TypeRepr::Default,
            packing: None,
            fields: vec![
                FieldLayout {
                    name: "View".into(),
                    ty: Ty::Str,
                    index: 0,
                    offset: None,
                    span: None,
                    mmio: None,
                    display_name: None,
                    is_required: false,
                    is_nullable: false,
                    is_readonly: false,
                    view_of: Some("Data".into()),
                },
                FieldLayout {
                    name: "Data".into(),
                    ty: Ty::String,
                    index: 1,
                    offset: None,
                    span: None,
                    mmio: None,
                    display_name: None,
                    is_required: false,
                    is_nullable: false,
                    is_readonly: false,
                    view_of: None,
                },
            ],
            positional: Vec::new(),
            list: None,
            size: None,
            align: None,
            is_readonly: false,
            is_intrinsic: false,
            allow_cross_inline: false,
            auto_traits: AutoTraitSet::all_unknown(),
            overrides: AutoTraitOverride::default(),
            mmio: None,
            dispose: None,
            class: None,
        }),
    );

    let mut case = harness.case();
    case.body_mut().arg_count = 1;
    let buffer = case.push_local(Some("buffer"), Ty::named("Buffer"), true, LocalKind::Arg(0));

    let mut entry = BasicBlock::new(BlockId(0), None);
    entry.statements.push(Statement {
        span: None,
        kind: StatementKind::Assign {
            place: Place::new(case.return_slot()),
            value: Rvalue::Use(Operand::Move(Place {
                local: buffer,
                projection: vec![ProjectionElem::Field(1)],
            })),
        },
    });
    entry.terminator = Some(Terminator::Return);
    case.body_mut().blocks.push(entry);

    let result = case.run();
    result.expect_message("dependent view field(s)");
}
