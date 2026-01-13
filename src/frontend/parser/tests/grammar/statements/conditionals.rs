use super::super::common::FunctionFixture;
use crate::frontend::ast::{ClassMember, Item, StatementKind};
use crate::frontend::parser::tests::fixtures::parse_ok;
use crate::syntax::expr::ExprNode;

#[test]
fn parses_control_flow_statements() {
    let source = r"
namespace Control;

public void Flow(Span<int> data)
{
    var total = 0;
    if (total == 0) { total = 1; } else { total = 2; }
    while (total < 10) { total++; }
    do { total--; } while (total > 0);
    for (var i = 0; i < 3; i++) { total += i; }
    foreach (var item in data) { total += item; }
    switch (total)
    {
        case 0:
            break;
        case 1:
            break;
        default:
            break;
    }
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    let statements = fixture.statements();
    assert!(matches!(
        statements[0].kind,
        StatementKind::VariableDeclaration(_)
    ));
    assert!(matches!(statements[1].kind, StatementKind::If(_)));
    assert!(matches!(statements[2].kind, StatementKind::While { .. }));
    assert!(matches!(statements[3].kind, StatementKind::DoWhile { .. }));
    assert!(matches!(statements[4].kind, StatementKind::For(_)));
    assert!(matches!(statements[5].kind, StatementKind::Foreach(_)));
    assert!(matches!(statements[6].kind, StatementKind::Switch(_)));
}

#[test]
fn parses_return_without_expression() {
    let source = r"
public void ExitEarly()
{
    return;
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    match &fixture.statements()[0].kind {
        StatementKind::Return { expression } => assert!(
            expression.is_none(),
            "expected empty return expression, found {expression:?}"
        ),
        other => panic!("expected return statement, found {other:?}"),
    }
}

#[test]
fn parses_async_function() {
    let source = r"
namespace AsyncTests;

public async Task<int> FetchAsync()
{
    return await Task.Get();
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    assert!(
        fixture.function().is_async,
        "function should be marked async"
    );
}

#[test]
fn parses_async_class_method() {
    let source = r"
namespace AsyncTests;

public class Client
{
    public async Task ConnectAsync()
    {
        await Socket.Connect();
    }
}
";

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());
    let class = match &parse.module.items[0] {
        Item::Class(c) => c,
        other => panic!("expected class, found {other:?}"),
    };
    let method = match &class.members[0] {
        ClassMember::Method(func) => func,
        ClassMember::Field(field) => panic!("expected method, found field {field:?}"),
        ClassMember::Constructor(ctor) => panic!("expected method, found constructor {ctor:?}"),
        ClassMember::Property(prop) => panic!("expected method, found property {prop:?}"),
        ClassMember::Const(constant) => {
            panic!("expected method, found const member {constant:?}")
        }
    };
    assert!(method.is_async, "method should be marked async");
}

#[test]
fn parses_testcase() {
    let source = r"
testcase CalculatesArea()
{
    Assert.That(Circle.Area(1.0)).IsCloseTo(3.14159, tolerance: 1e-6);
}
";

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());

    match &parse.module.items[0] {
        Item::TestCase(test) => {
            assert!(!test.body.statements.is_empty());
        }
        other => panic!("expected testcase, found {other:?}"),
    }
}

#[test]
fn parses_async_testcase() {
    let source = r"
async testcase LoadsData()
{
    await Data.Refresh();
}
";

    let parse = parse_ok(source);
    assert!(parse.diagnostics.is_empty());
    match &parse.module.items[0] {
        Item::TestCase(test) => {
            assert!(test.is_async);
            let signature = test
                .signature
                .as_ref()
                .expect("expected testcase signature");
            assert_eq!(signature.return_type.name, "Task");
        }
        other => panic!("expected testcase, found {other:?}"),
    }
}

#[test]
fn parses_yield_return_and_break_expressions() {
    let source = r"
public IEnumerable<int> Emit(bool flag)
{
    if (flag)
    {
        yield return 42;
    }
    yield break;
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    match &fixture.statements()[0].kind {
        StatementKind::If(if_stmt) => match &if_stmt.then_branch.kind {
            StatementKind::Block(block) => {
                assert_eq!(block.statements.len(), 1);
                match &block.statements[0].kind {
                    StatementKind::YieldReturn { expression } => {
                        assert_eq!(expression.text.trim(), "42");
                    }
                    other => panic!("expected yield return, found {other:?}"),
                }
            }
            other => panic!("expected block in then-branch, found {other:?}"),
        },
        other => panic!("expected if statement, found {other:?}"),
    }
    match &fixture.statements()[1].kind {
        StatementKind::YieldBreak => {}
        other => panic!("expected yield break, found {other:?}"),
    }
}

#[test]
fn parses_result_propagation_in_return_statement() {
    let source = r"
public Result<int> Demo(Result<int> input)
{
    return input?;
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    let return_stmt = match &fixture.statements()[0].kind {
        StatementKind::Return {
            expression: Some(expr),
        } => expr,
        other => panic!("expected return statement with expression, found {other:?}"),
    };
    assert_eq!(return_stmt.text.trim(), "input?");
    let node = return_stmt
        .node
        .as_ref()
        .expect("expected parsed expression node");
    match node {
        ExprNode::TryPropagate { expr, .. } => match expr.as_ref() {
            ExprNode::Identifier(name) if name == "input" => {}
            other => panic!(
                "expected identifier operand, found {other:?}",
                other = other
            ),
        },
        other => panic!("expected TryPropagate node, found {other:?}", other = other),
    }
}
