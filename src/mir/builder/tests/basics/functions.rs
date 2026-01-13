use super::helper::*;
use super::*;

#[expect(
    clippy::too_many_lines,
    reason = "End-to-end lowering test requires explicit assertions for clarity."
)]
#[test]
fn lowers_simple_function_into_mir() {
    let source = r"
namespace Math;

public int Add(int a, int b)
{
let total = a + b;
return total;
}
";
    let lowering = lower(source);
    assert_eq!(lowering.module.functions.len(), 1);

    let func = &lowering.module.functions[0];
    assert_eq!(func.name, "Math::Add");
    assert_eq!(func.kind, FunctionKind::Function);
    assert_eq!(func.signature.params.len(), 2);
    assert_eq!(func.body.locals.len(), 5); // _ret, a, b, total, temp

    let body = &func.body;
    assert_eq!(body.blocks.len(), 1);
    let block = &body.blocks[0];
    assert_eq!(block.statements.len(), 7);

    assert!(matches!(block.terminator, Some(Terminator::Return)));

    let storage_live = &block.statements[0];
    assert!(matches!(
        storage_live.kind,
        MirStatementKind::StorageLive(LocalId(3))
    ));

    let temp_live = &block.statements[1];
    assert!(matches!(
        temp_live.kind,
        MirStatementKind::StorageLive(LocalId(4))
    ));

    let temp_assign = &block.statements[2];
    match &temp_assign.kind {
        MirStatementKind::Assign { place, value } => {
            assert_eq!(place.local.0, 4);
            match value {
                Rvalue::Binary { op, lhs, rhs, .. } => {
                    assert!(matches!(op, BinOp::Add));
                    assert!(matches!(lhs, Operand::Copy(_)));
                    assert!(matches!(rhs, Operand::Copy(_)));
                }
                other => panic!("expected binary rvalue, found {other:?}"),
            }
        }
        other => panic!("expected assign, found {other:?}"),
    }

    let assign = &block.statements[3];
    match &assign.kind {
        MirStatementKind::Assign { place, value } => {
            assert_eq!(place.local.0, 3);
            match value {
                Rvalue::Use(Operand::Copy(copy_place)) => {
                    assert_eq!(copy_place.local.0, 4);
                }
                other => panic!("expected use of temp, found {other:?}"),
            }
        }
        other => panic!("expected assign, found {other:?}"),
    }

    let ret_assign = &block.statements[4];
    match &ret_assign.kind {
        MirStatementKind::Assign { place, value } => {
            assert_eq!(place.local.0, 0);
            assert!(
                matches!(value, Rvalue::Use(Operand::Copy(copy_place)) if copy_place.local.0 == 3)
            );
        }
        other => panic!("expected return assign, found {other:?}"),
    }
    assert!(
        matches!(
            block.statements[5].kind,
            MirStatementKind::StorageDead(LocalId(4))
        ),
        "expected temp local to be StorageDead"
    );
    assert!(
        matches!(
            block.statements[6].kind,
            MirStatementKind::StorageDead(LocalId(3))
        ),
        "expected scoped local to be StorageDead"
    );

    verify_body(body).require("body verification");
}

#[test]
fn expression_bodied_function_lowers_to_return_assignment() {
    let source = r"
namespace Sample;

public int Double(int value) => value * 2;
";
    let lowering = lower_no_diagnostics(source);

    let func = lowering
        .module
        .functions
        .iter()
        .find(|f| f.name == "Sample::Double")
        .require("missing Sample::Double lowering");
    let block = &func.body.blocks[0];
    assert!(
        matches!(block.terminator, Some(Terminator::Return)),
        "expression-bodied function should emit a return terminator"
    );

    let assigns_to_return = block
        .statements
        .iter()
        .filter(|stmt| matches!(&stmt.kind, MirStatementKind::Assign { place, .. } if place.local.0 == 0))
        .count();
    assert_eq!(
        assigns_to_return, 1,
        "expression-bodied function should assign into the return slot exactly once"
    );
}

#[test]
fn lowering_includes_file_scoped_namespace_prefix() {
    let source = r"
namespace Root.Base;

public void TopLevel() { }

namespace Services
{
    public void Nested() { }
}
";

    let lowering = lower_no_diagnostics(source);
    let names: Vec<_> = lowering
        .module
        .functions
        .iter()
        .map(|func| func.name.as_str())
        .collect();
    assert!(
        names.contains(&"Root::Base::TopLevel"),
        "missing Root::Base::TopLevel in {:?}",
        names
    );
    assert!(
        names.contains(&"Root::Base::Services::Nested"),
        "missing Root::Base::Services::Nested in {:?}",
        names
    );
}

#[test]
fn lowers_local_function_without_captures() {
    let source = r"
namespace Demo;

public int Run(int value)
{
    function int Twice(int input)
    {
        return input * 2;
    }

    return Twice(value);
}
";
    let lowering = lower_no_diagnostics(source);

    let names: Vec<_> = lowering
        .module
        .functions
        .iter()
        .map(|func| func.name.as_str())
        .collect();
    assert!(
        names.contains(&"Demo::Run::local$0::Twice"),
        "missing lowered local function symbol in {names:?}"
    );

    let run = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == "Demo::Run")
        .require("missing Demo::Run lowering");
    let mut call_found = false;
    for block in &run.body.blocks {
        if let Some(Terminator::Call { func, args, .. }) = &block.terminator {
            if let Operand::Const(constant) = func {
                if constant.symbol_name() == Some("Demo::Run::local$0::Twice") {
                    call_found = true;
                    assert_eq!(
                        args.len(),
                        1,
                        "non-capturing local function should forward only the user argument"
                    );
                    break;
                }
            }
        }
    }
    assert!(call_found, "expected call into local function symbol");
}

#[test]
fn lowers_local_function_with_capture() {
    let source = r"
namespace Demo;

public int Run(int value)
{
    function int AddValue(int delta)
    {
        return value + delta;
    }

    return AddValue(2);
}
";
    let lowering = lower_no_diagnostics(source);

    let run = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == "Demo::Run")
        .require("missing Demo::Run lowering");

    let mut capture_call = None;
    for block in &run.body.blocks {
        if let Some(Terminator::Call { func, args, .. }) = &block.terminator {
            if let Operand::Const(constant) = func {
                if constant.symbol_name() == Some("Demo::Run::local$0::AddValue") {
                    capture_call = Some(args.clone());
                    break;
                }
            }
        }
    }

    let args = capture_call.require("expected call into capturing local function");
    assert_eq!(
        args.len(),
        2,
        "capturing local function should receive capture plus user argument"
    );
    let nested = lowering
        .module
        .functions
        .iter()
        .find(|func| func.name == "Demo::Run::local$0::AddValue")
        .require("missing nested local function lowering");
    assert_eq!(
        nested.signature.params.len(),
        2,
        "nested local function should accept capture plus delta parameter"
    );
}

#[test]
fn reports_unknown_identifier_in_expression() {
    let source = r"
namespace Sample;

public int Fail(int a)
{
let total = missing + 1;
return total;
}
";
    let lowering = lower(source);
    assert!(
        lowering
            .diagnostics
            .iter()
            .any(|diag| diag.message.contains("unknown identifier `missing`"))
    );

    let func = &lowering.module.functions[0];
    let body = &func.body;
    let block = &body.blocks[0];
    let assign = block
        .statements
        .iter()
        .find(|stmt| matches!(stmt.kind, MirStatementKind::Assign { .. }))
        .require("expected assignment statement");
    match &assign.kind {
        MirStatementKind::Assign { value, .. } => {
            assert!(matches!(value, Rvalue::Pending(_)));
        }
        other => panic!("expected assign, found {other:?}"),
    }
}
