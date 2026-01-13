use super::super::common::FunctionFixture;
use crate::frontend::ast::{StatementKind, UsingResource};
use crate::frontend::parser::tests::fixtures::parse_fail;

#[test]
fn parses_region_statement() {
    let source = r"
namespace Regions;

public void Build()
{
    region workspace {
        var buffer = 1;
    }
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    let region = match &fixture.statements()[0].kind {
        StatementKind::Region { name, body } => {
            assert_eq!(name, "workspace");
            assert_eq!(body.statements.len(), 1);
            body
        }
        other => panic!("expected region statement, found {other:?}"),
    };
    assert!(
        matches!(
            region.statements[0].kind,
            StatementKind::VariableDeclaration(_)
        ),
        "region body should include parsed statements"
    );
}

#[test]
fn parses_resource_and_exception_statements() {
    let source = r"
public void Manage(ref Span<byte> data, IDisposable other)
{
    using (other) { }
    using var guard = other;
    try
    {
        checked { }
    }
catch (Exception err) when (err.ShouldRetry())
    {
        throw;
    }
    finally
    {
        lock (other) { }
    }
    unchecked { }
    fixed (let ptr = data.Pin())
    {
        unsafe { goto Done; }
    }
Done:
    return;
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    let statements = fixture.statements();
    assert!(matches!(statements[0].kind, StatementKind::Using(_)));
    assert!(matches!(statements[1].kind, StatementKind::Using(_)));
    assert!(matches!(statements[2].kind, StatementKind::Try(_)));
    assert!(matches!(
        statements[3].kind,
        StatementKind::Unchecked { .. }
    ));
    assert!(matches!(statements[4].kind, StatementKind::Fixed(_)));
    assert!(matches!(statements[5].kind, StatementKind::Labeled { .. }));
}

#[test]
fn parses_atomic_statements_with_optional_ordering() {
    let source = r"
public void Update(ref Std.Sync.Atomic<int> counter)
{
    atomic { }
    atomic(Std.Sync.MemoryOrder.AcqRel)
    {
        counter.FetchAdd(1, Std.Sync.MemoryOrder.AcqRel);
    }
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    assert_eq!(fixture.statements().len(), 2);

    match &fixture.statements()[0].kind {
        StatementKind::Atomic { ordering, body } => {
            assert!(
                ordering.is_none(),
                "plain atomic block should default to SeqCst"
            );
            assert!(body.statements.is_empty());
        }
        other => panic!("expected atomic statement, found {other:?}"),
    }

    match &fixture.statements()[1].kind {
        StatementKind::Atomic { ordering, body } => {
            let order_expr = ordering.as_ref().expect("expected explicit ordering");
            assert!(
                order_expr.text.contains("Std.Sync.MemoryOrder.AcqRel"),
                "unexpected ordering expression: {}",
                order_expr.text
            );
            assert!(
                !body.statements.is_empty(),
                "atomic block should carry nested statements"
            );
        }
        other => panic!("expected atomic statement with ordering, found {other:?}"),
    }
}

#[test]
fn atomic_statement_requires_block() {
    let source = r"
public void Invalid(ref Std.Sync.Atomic<int> counter)
{
    atomic Std.Sync.MemoryOrder.Relaxed;
}
";

    let diagnostics = parse_fail(source);
    assert_eq!(diagnostics.len(), 1);
    assert!(
        diagnostics[0]
            .message
            .to_lowercase()
            .contains("atomic statement requires a block"),
        "unexpected diagnostic: {:?}",
        diagnostics[0]
    );
}

#[test]
fn parses_using_expression_resource() {
    let source = r"
public void Close(IDisposable other)
{
    using (other)
    {
        return;
    }
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    match &fixture.statements()[0].kind {
        StatementKind::Using(using) => {
            match &using.resource {
                UsingResource::Expression(expr) => assert_eq!(expr.text.trim(), "other"),
                other => panic!("expected expression resource, found {other:?}"),
            }
            assert!(using.body.is_some(), "using expression should wrap a body");
        }
        other => panic!("expected using statement, found {other:?}"),
    }
}

#[test]
fn using_expression_reports_pin_attribute_misuse() {
    let source = r"
public void Manage(IDisposable other)
{
    using (@pin other)
    {
        other.Dispose();
    }
}
";

    let diagnostics = parse_fail(source);
    assert!(
        diagnostics.iter().any(|diag| diag
            .message
            .contains("`@pin` attribute is only supported on variable declarations")),
        "expected @pin misuse diagnostic, found {:#?}",
        diagnostics
    );
}

#[test]
fn parses_nested_using_statements() {
    let source = r"
public void Manage()
{
    using (var primary = AcquirePrimary())
    using (AcquireSecondary())
    {
        using (var inner = AcquireInner())
        {
            inner.Dispose();
        }
    }
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    let outer = match &fixture.statements()[0].kind {
        StatementKind::Using(using) => using,
        other => panic!("expected outer using statement, found {other:?}"),
    };
    assert!(
        matches!(outer.resource, UsingResource::Declaration(_)),
        "outer using should bind a declaration"
    );
    let chained_stmt = outer
        .body
        .as_ref()
        .expect("outer using should carry nested body");
    let chained = match &chained_stmt.kind {
        StatementKind::Using(inner) => inner,
        other => panic!("expected chained using statement, found {other:?}"),
    };
    assert!(
        matches!(chained.resource, UsingResource::Expression(_)),
        "chained using should accept expression resource"
    );
    let inner_body = chained
        .body
        .as_ref()
        .expect("chained using should wrap its block");
    match &inner_body.kind {
        StatementKind::Block(block) => {
            assert_eq!(block.statements.len(), 1);
            assert!(
                matches!(block.statements[0].kind, StatementKind::Using(_)),
                "expected nested using inside the block"
            );
        }
        other => panic!("expected block body for chained using, found {other:?}"),
    }
}

#[test]
fn parses_fixed_statement() {
    let source = r"
public void Pin(Span<byte> data)
{
    fixed (let ptr = data.Pin())
    {
        ptr[0] = 1;
    }
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    match &fixture.statements()[0].kind {
        StatementKind::Fixed(fixed_stmt) => {
            let decl = &fixed_stmt.declaration;
            assert!(
                !decl.is_pinned,
                "parser should not implicitly pin fixed declarations without @pin"
            );
            assert!(
                decl.type_annotation.is_none(),
                "fixed declarations should be let/var without an explicit type"
            );
            assert_eq!(decl.declarators.len(), 1);
            assert_eq!(decl.declarators[0].name, "ptr");
        }
        other => panic!("expected fixed statement, found {other:?}"),
    }
}

#[test]
fn parses_nested_unsafe_and_fixed() {
    let source = r"
public void PinNested(byte[] data)
{
    unsafe
    {
        fixed (let ptr = data.Pin())
        {
            ptr[0] = 1;
        }
    }
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    match &fixture.statements()[0].kind {
        StatementKind::Unsafe { body } => match &body.kind {
            StatementKind::Block(block) => {
                assert_eq!(block.statements.len(), 1);
                assert!(matches!(block.statements[0].kind, StatementKind::Fixed(_)));
            }
            other => panic!("expected unsafe block, found {other:?}"),
        },
        other => panic!("expected unsafe statement, found {other:?}"),
    }
}
