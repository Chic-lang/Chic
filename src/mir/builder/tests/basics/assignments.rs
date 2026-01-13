use super::helper::*;
use super::*;
use crate::mir::module_metadata::StdProfile;

#[test]
fn lowers_compound_assignment_expression() {
    let source = r"
namespace Sample;

public int UseAddAssign(int value)
{
var total = value;
total += 5;
return total;
}
";
    let lowering = lower_no_diagnostics(source);

    let func = &lowering.module.functions[0];
    let block = &func.body.blocks[0];
    assert_eq!(block.statements.len(), 5);

    if let MirStatementKind::Assign { place, value } = &block.statements[2].kind {
        assert_eq!(place.local.0, 2);
        match value {
            Rvalue::Binary { op, lhs, rhs, .. } => {
                assert!(matches!(op, BinOp::Add));
                assert!(matches!(lhs, Operand::Copy(_)));
                match rhs {
                    Operand::Const(ConstOperand {
                        value: ConstValue::Int(5),
                        ..
                    }) => {}
                    other => panic!("expected const 5, got {other:?}"),
                }
            }
            other => panic!("expected binary rvalue, found {other:?}"),
        }
    } else {
        panic!("expected assign statement for temp");
    }

    if let MirStatementKind::Assign { place, value } = &block.statements[3].kind {
        assert_eq!(place.local.0, 0);
        assert!(matches!(
            value,
            Rvalue::Use(Operand::Copy(copy_place)) if copy_place.local.0 == 2
        ));
    } else {
        panic!("expected return assignment");
    }
    assert!(
        matches!(
            block.statements[4].kind,
            MirStatementKind::StorageDead(LocalId(2))
        ),
        "scoped local should be StorageDead at end of block"
    );
}

#[test]
fn lowers_member_assignment() {
    let source = r"
namespace Sample;

public void Update(Point p)
{
p.X = 42;
}

public struct Point
{
public int X;
public int Y;
}
";
    let lowering = lower_no_diagnostics(source);

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name.ends_with("::Update"))
        .require("missing Update function");
    let assign = func.body.blocks[0]
        .statements
        .iter()
        .find(|stmt| matches!(stmt.kind, MirStatementKind::Assign { .. }))
        .require("missing assignment statement");

    match &assign.kind {
        MirStatementKind::Assign { place, value } => {
            assert_eq!(place.local.0, 1, "expected parameter local for `p`");
            assert_eq!(place.projection.len(), 1);
            match &place.projection[0] {
                ProjectionElem::FieldNamed(name) => assert_eq!(name, "X"),
                other => panic!("expected named field projection, found {other:?}"),
            }
            match value {
                Rvalue::Use(Operand::Const(ConstOperand {
                    value: ConstValue::Int(value),
                    ..
                })) => {
                    assert_eq!(*value, 42);
                }
                other => panic!("expected const assignment, found {other:?}"),
            }
        }
        other => panic!("expected assign statement, found {other:?}"),
    }
}

#[test]
fn lowers_index_assignment() {
    let source = r"
namespace Sample;

public void Set(Vec<int> values, int index)
{
values[index] = index;
}
";
    let lowering = lower_no_diagnostics(source);

    let func = find_function(&lowering, "Set");
    match &func.signature.params[0] {
        Ty::Vec(vec_ty) => {
            assert!(matches!(
                *vec_ty.element,
                Ty::Named(ref name) if name.as_str() == "int"
            ));
        }
        other => panic!("expected Vec<int> parameter type, found {other:?}"),
    }
    let assign = func.body.blocks[0]
        .statements
        .iter()
        .find(|stmt| matches!(stmt.kind, MirStatementKind::Assign { .. }))
        .require("missing assignment");

    match &assign.kind {
        MirStatementKind::Assign { place, value } => {
            assert_eq!(place.local.0, 1, "expected local for `values` parameter");
            assert_eq!(place.projection.len(), 1);
            match &place.projection[0] {
                ProjectionElem::Index(local) => {
                    let decl = &func.body.locals[local.0];
                    assert!(
                        matches!(decl.kind, LocalKind::Arg(1)),
                        "expected index parameter local, found {:?}",
                        decl.kind
                    );
                }
                other => panic!("expected index projection, found {other:?}"),
            }
            match value {
                Rvalue::Use(Operand::Copy(copy_place)) => {
                    assert_eq!(copy_place.local.0, 2, "expected copy of index parameter");
                }
                other => panic!("expected use of index operand, found {other:?}"),
            }
        }
        other => panic!("expected assign statement, found {other:?}"),
    }
}

#[test]
fn lowers_multi_dimension_array_index_assignment() {
    let source = r"
namespace Sample;

public void Set(Array<int>[,] matrix, int row, int column)
{
matrix[row, column] = row + column;
}
";
    let lowering = lower_no_diagnostics(source);
    let func = find_function(&lowering, "Set");

    let place = func.body.blocks[0]
        .statements
        .iter()
        .find_map(|stmt| {
            if let MirStatementKind::Assign { place, .. } = &stmt.kind {
                let index_count = place
                    .projection
                    .iter()
                    .filter(|proj| matches!(proj, ProjectionElem::Index(_)))
                    .count();
                if index_count == 2 {
                    return Some(place.clone());
                }
            }
            None
        })
        .require("missing matrix index assignment");

    let index_count = place
        .projection
        .iter()
        .filter(|proj| matches!(proj, ProjectionElem::Index(_)))
        .count();
    assert_eq!(index_count, 2, "expected exactly two index projections");
}

#[test]
fn lowers_array_variable_declaration() {
    let source = r"
namespace Sample;

public void Use()
{
let Array<int>[,] data;
}
";
    let lowering = lower_no_diagnostics(source);
    let func = find_function(&lowering, "Use");

    let data_local = func
        .body
        .locals
        .iter()
        .find(|decl| decl.name.as_deref() == Some("data"))
        .require("missing data local");

    match &data_local.ty {
        Ty::Array(array) => {
            assert_eq!(array.rank, 2, "expected two-dimensional array rank");
            assert!(matches!(
                *array.element,
                Ty::Named(ref name) if name.as_str() == "int"
            ));
        }
        other => panic!("expected Array<int>[,] type, found {other:?}"),
    }
}

#[test]
fn lowers_call_expression_in_assignment() {
    let source = r"
namespace Calls;

public int Use(int value)
{
let total = Increment(value);
return total;
}

public int Increment(int value)
{
return value + 1;
}
";
    let lowering = lower_no_diagnostics(source);
    let func = find_function(&lowering, "Use");
    let entry = &func.body.blocks[0];
    match &entry.terminator {
        Some(Terminator::Call {
            destination,
            target,
            arg_modes: _,
            ..
        }) => {
            assert!(destination.is_some());
            assert_eq!(target.0, 1);
        }
        other => panic!("expected call terminator, found {other:?}"),
    }
}

#[test]
fn lowers_module_scoped_function_call() {
    let source = r#"
public int Helper(int value)
{
    return value + 1;
}

public int Entry(int input)
{
    return Helper(input);
}
"#;
    let lowering = lower_no_diagnostics(source);
    let func = find_function(&lowering, "Entry");
    let entry_block = &func.body.blocks[0];
    match &entry_block.terminator {
        Some(Terminator::Call { destination, .. }) => {
            assert!(destination.is_some(), "call should store return value");
        }
        other => panic!("expected call terminator, found {other:?}"),
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "integration-style MIR lowering test requires full control-flow fixture"
)]
#[test]
fn lowers_member_access_to_projection() {
    let source = r"
namespace Sample;

public int GetX(Point p)
{
return p.X;
}

public struct Point
{
public int X;
public int Y;
}
";
    let lowering = lower_no_diagnostics(source);
    let func = find_function(&lowering, "GetX");
    let return_assign = func
        .body
        .blocks
        .iter()
        .flat_map(|block| block.statements.iter())
        .find(|statement| {
            if let MirStatementKind::Assign { place, .. } = &statement.kind {
                place.local == LocalId(0)
            } else {
                false
            }
        })
        .require("missing return assignment");

    let MirStatementKind::Assign { value, .. } = &return_assign.kind else {
        panic!("expected assign statement for return");
    };
    match value {
        Rvalue::Use(Operand::Copy(place)) => {
            assert_eq!(place.projection.len(), 1);
            match &place.projection[0] {
                ProjectionElem::FieldNamed(name) => assert_eq!(name, "X"),
                other => panic!("expected named field projection, found {other:?}"),
            }
        }
        other => panic!("expected use of place operand, found {other:?}"),
    }
}

#[test]
fn static_member_assignment_stays_pending() {
    let source = r"
namespace Sample;

public void Configure()
{
Logger.Level = 2;
}

";
    let lowering = lower_no_diagnostics(source);
    let func = find_function(&lowering, "Configure");
    let block = &func.body.blocks[0];
    let pending = block
        .statements
        .iter()
        .find(|stmt| matches!(stmt.kind, MirStatementKind::Eval(_)))
        .require("expected pending eval statement");
    match &pending.kind {
        MirStatementKind::Eval(pending) => {
            assert!(
                pending.repr.contains("Logger.Level"),
                "expected pending text to contain Logger.Level, found {:?}",
                pending.repr
            );
        }
        _ => unreachable!(),
    }
}

#[test]
fn member_call_passes_receiver_argument() {
    let source = r"
namespace Calls;

public void Invoke(Widget widget)
{
widget.Run();
}

public class Widget
{
public void Run() { }
}
";
    let lowering = lower_no_diagnostics(source);
    let func = find_function(&lowering, "Invoke");
    let Terminator::Call {
        args, arg_modes: _, ..
    } = func.body.blocks[0]
        .terminator
        .as_ref()
        .require("expected call terminator in entry block")
    else {
        panic!("expected call terminator");
    };
    assert!(
        matches!(args.first(), Some(Operand::Copy(place)) if place.local == LocalId(1)),
        "expected receiver argument referencing parameter: {args:?}"
    );
}

#[test]
fn static_member_call_does_not_inject_receiver() {
    let source = r"
namespace Calls;

public void Invoke()
{
Widget.Run();
}

public class Widget
{
public void Run() { }
}
";
    let lowering = lower_no_diagnostics(source);
    let func = find_function(&lowering, "Invoke");
    let Terminator::Call {
        args, arg_modes: _, ..
    } = func.body.blocks[0]
        .terminator
        .as_ref()
        .require("expected call terminator in entry block")
    else {
        panic!("expected call terminator");
    };
    assert!(
        args.is_empty(),
        "expected no receiver argument for unresolved static call, got {args:?}"
    );
}

#[test]
fn lowers_export_and_no_std_attributes() {
    let source = r#"
#![no_std]
namespace Kernel;

public class Entry
{
    @export("_start")
    public static void Start() { }
}

public void Helper() { }
"#;

    let parsed = parse_module(source).require("parse");
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        parsed.diagnostics
    );
    let lowering = lower_no_diagnostics(source);

    assert_eq!(lowering.module.attributes.std_profile, StdProfile::NoStd);
    assert_eq!(lowering.module.exports.len(), 1);
    let export = &lowering.module.exports[0];
    assert_eq!(export.function, "Kernel::Entry::Start");
    assert_eq!(export.symbol, "_start");
}

#[test]
fn lowers_vec_length_into_len_rvalue() {
    let source = r#"
namespace Sample;

public usize GetLength(Vec<int> data)
{
    return data.Length;
}
"#;
    let lowering = lower_no_diagnostics(source);

    let func = &lowering.module.functions[0];
    assert_eq!(func.body.blocks.len(), 1);
    let block = &func.body.blocks[0];

    let (result_local, source_place) = block
        .statements
        .iter()
        .find_map(|stmt| match &stmt.kind {
            MirStatementKind::Assign {
                place,
                value: Rvalue::Len(origin),
            } => Some((place.local, origin.clone())),
            _ => None,
        })
        .expect("expected length assignment");

    let length_decl = func
        .body
        .locals
        .get(result_local.0)
        .expect("length local should exist");
    assert_eq!(length_decl.ty.canonical_name(), "usize");
    assert!(!length_decl.is_nullable);

    assert_eq!(source_place.local.0, 1, "length should use parameter place");
    assert!(source_place.projection.is_empty());

    let return_assign = block
        .statements
        .iter()
        .rev()
        .find_map(|stmt| match &stmt.kind {
            MirStatementKind::Assign { place, value } if place.local.0 == 0 => Some(value),
            _ => None,
        })
        .expect("expected return assignment");
    assert!(matches!(
        return_assign,
        Rvalue::Use(Operand::Copy(copy_place)) if copy_place.local == result_local
    ));

    verify_body(&func.body).require("body verification");
}

#[test]
fn lowers_array_length_into_len_rvalue() {
    let source = r#"
namespace Sample;

public usize Count(Array<int> data)
{
    return data.Length;
}
"#;
    let lowering = lower_no_diagnostics(source);

    let func = &lowering.module.functions[0];
    assert_eq!(func.body.blocks.len(), 1);
    let block = &func.body.blocks[0];

    let (result_local, source_place) = block
        .statements
        .iter()
        .find_map(|stmt| match &stmt.kind {
            MirStatementKind::Assign {
                place,
                value: Rvalue::Len(origin),
            } => Some((place.local, origin.clone())),
            _ => None,
        })
        .expect("expected length assignment");

    let length_decl = func
        .body
        .locals
        .get(result_local.0)
        .expect("length local should exist");
    assert_eq!(length_decl.ty.canonical_name(), "usize");
    assert!(!length_decl.is_nullable);

    assert_eq!(source_place.local.0, 1, "length should use parameter place");
    assert!(source_place.projection.is_empty());

    let return_assign = block
        .statements
        .iter()
        .rev()
        .find_map(|stmt| match &stmt.kind {
            MirStatementKind::Assign { place, value } if place.local.0 == 0 => Some(value),
            _ => None,
        })
        .expect("expected return assignment");
    assert!(matches!(
        return_assign,
        Rvalue::Use(Operand::Copy(copy_place)) if copy_place.local == result_local
    ));

    verify_body(&func.body).require("body verification");
}
