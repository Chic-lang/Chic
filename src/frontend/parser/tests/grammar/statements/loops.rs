use super::super::common::FunctionFixture;
use crate::frontend::ast::StatementKind;

#[test]
fn parses_for_statement_with_multiple_iterators() {
    let source = r"
public void Iterate(int[] items)
{
    for (var i = 0, j = 0; i < items.Length; i++, j++)
    {
    }
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    let for_stmt = match &fixture.statements()[0].kind {
        StatementKind::For(stmt) => stmt,
        other => panic!("expected for statement, found {other:?}"),
    };
    assert_eq!(
        for_stmt.iterator.len(),
        2,
        "expected two iterator expressions"
    );
    assert_eq!(for_stmt.iterator[0].text.trim(), "i++");
    assert_eq!(for_stmt.iterator[1].text.trim(), "j++");
}

#[test]
fn parses_for_statement_with_empty_clauses() {
    let source = r"
public void Spin()
{
    for (;;) { break; }
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    let for_stmt = match &fixture.statements()[0].kind {
        StatementKind::For(stmt) => stmt,
        other => panic!("expected for statement, found {other:?}"),
    };
    assert!(
        for_stmt.initializer.is_none(),
        "initializer should be empty"
    );
    assert!(for_stmt.condition.is_none(), "condition should be empty");
    assert!(
        for_stmt.iterator.is_empty(),
        "iterator list should be empty"
    );
}

#[test]
fn parses_iterator_statements() {
    let source = r"
public IEnumerable<int> Iterate(Span<int> data)
{
    foreach (var item in data)
    {
        yield return item;
    }
    yield break;
}
";

    let fixture = FunctionFixture::new(source);
    fixture.assert_no_diagnostics();
    let statements = fixture.statements();
    assert!(matches!(statements[0].kind, StatementKind::Foreach(_)));
    assert!(matches!(statements[1].kind, StatementKind::YieldBreak));
}
