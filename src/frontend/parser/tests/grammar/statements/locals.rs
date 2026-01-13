use super::super::common::FunctionFixture;
use crate::frontend::ast::StatementKind;
use crate::frontend::parser::tests::fixtures::parse_fail;

#[test]
fn parses_pin_attribute_on_local_declaration() {
    let source = r"
namespace Geometry;

public async Task<int> Work()
{
    @pin var buffer = Allocate();
    return await buffer;
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    let body = fixture.body();
    match &body.statements[0].kind {
        StatementKind::VariableDeclaration(decl) => {
            assert!(decl.is_pinned, "variable should be marked pinned");
            assert_eq!(decl.declarators.len(), 1);
            assert_eq!(decl.declarators[0].name, "buffer");
        }
        other => panic!("expected variable declaration, found {other:?}"),
    }
}

#[test]
fn parses_local_function_statement() {
    let source = r"
public int Outer(int seed)
{
    function int Helper(int value)
    {
        return value + seed;
    }

    return Helper(seed);
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    let statement = &fixture.statements()[0];
    let local = match &statement.kind {
        StatementKind::LocalFunction(local) => local,
        other => panic!("expected local function, found {other:?}"),
    };
    assert_eq!(local.name, "Helper");
    assert_eq!(local.signature.parameters.len(), 1);
    assert!(
        local.body.is_some(),
        "local function should carry a body in the AST"
    );
    assert!(
        statement.attributes.is_none(),
        "statement attributes should remain empty for local functions"
    );
}

#[test]
fn local_function_attributes_attach_to_function() {
    let source = r"
public void Owner()
{
    @memoize
    function int Cached()
    {
        return 1;
    }
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    let body = fixture.body();
    let local = match &body.statements[0].kind {
        StatementKind::LocalFunction(local) => local,
        other => panic!("expected local function, found {other:?}"),
    };
    let attrs = &local.attributes;
    assert_eq!(
        attrs.len(),
        1,
        "local function should own statement attributes"
    );
    assert_eq!(attrs[0].name, "memoize");
}

#[test]
fn reports_attribute_misuse_for_local_function_pin() {
    let source = r"
public void Owner()
{
    @pin
    function int Bad()
    {
        return 0;
    }
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics
            .iter()
            .any(|diag| diag.message.contains("local function")),
        "expected misuse diagnostic mentioning local functions: {diagnostics:?}"
    );
}

#[test]
fn parses_async_local_function_modifier() {
    let source = r"
public void Driver()
{
    async function int Worker()
    {
        return 42;
    }
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    let body = fixture.body();
    let local = match &body.statements[0].kind {
        StatementKind::LocalFunction(local) => local,
        other => panic!("expected local function, found {other:?}"),
    };
    assert!(local.is_async, "local function should be marked async");
}

#[test]
fn local_declaration_rejects_borrow_qualifier() {
    let source = r"
public void Demo()
{
    let ref int value = 0;
}
";
    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("`ref` qualifier is only supported on parameters and receivers")),
        "expected borrow qualifier diagnostic, found {diagnostics:?}"
    );
}

#[test]
fn pattern_binding_rejects_borrow_qualifier() {
    let source = r"
public bool Check(object value)
{
    return value is ref var other;
}
";
    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("`ref` qualifier is only supported on parameters and receivers")),
        "expected borrow qualifier diagnostic, found {diagnostics:?}"
    );
}

#[test]
fn typed_local_rejected_with_explicit_code() {
    let source = r"
public void Demo(Range range)
{
    RangeBounds bounds = SpanGuards.ResolveRangeInclusive(range, this.Length);
}
";
    let diagnostics = parse_fail(source);
    let diag = diagnostics
        .iter()
        .find(|diag| {
            diag.code
                .as_ref()
                .is_some_and(|code| code.code == "LCL0001")
        })
        .expect("expected LCL0001 for typed locals");
    assert!(
        diag.message.contains("let") && diag.message.contains("var"),
        "typed local diagnostic should mention let/var: {diag:?}"
    );
}

#[test]
fn typed_foreach_binding_is_rejected() {
    let source = r"
public void Demo(ReadOnlySpan<int> values)
{
    foreach (RangeBounds value in values)
    {
    }
}
";
    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .code
            .as_ref()
            .is_some_and(|code| code.code == "LCL0001")),
        "expected foreach typed binding to produce LCL0001: {diagnostics:?}"
    );
}
